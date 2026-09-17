//! Trusted post-validation feature activation; never deserialized from commands.

use std::collections::HashSet;

use fc_model::{
    FeatureAnchorState, FeatureGenerationSettings, FeatureKerningState, Features, Font,
    LayoutConversion,
};

use super::Document;
use crate::{
    command::{self, Impact},
    undo::{self, UndoEntry},
    Change, Error,
};

/// A compiler-validated source or proven layout conversion ready to commit.
///
/// This type deliberately implements neither `Deserialize` nor the wire
/// command interface. Only trusted Rust orchestration may construct it after
/// compiling every affected master, or validating the exact conversion token.
/// Construction is not proof: direct Rust callers have the same obligation as
/// the API handlers. Document revision checks run again at the commit boundary.
#[derive(Clone, Debug)]
pub struct PreparedFeatureChange {
    source: Features,
    settings: FeatureGenerationSettings,
    conversion: Option<Option<LayoutConversion>>,
    kerning: Option<FeatureKerningState>,
    anchors: Vec<FeatureAnchorState>,
}

impl PreparedFeatureChange {
    /// Prepare accepted source and settings, clearing its saved draft on commit.
    ///
    /// The caller must validate this exact candidate before invoking this.
    pub fn new(source: Features, settings: FeatureGenerationSettings) -> Self {
        Self {
            source,
            settings,
            conversion: None,
            kerning: None,
            anchors: Vec::new(),
        }
    }

    /// Set or clear native conversion history after a validated convert/revert.
    pub fn with_conversion(mut self, conversion: Option<LayoutConversion>) -> Self {
        self.conversion = Some(conversion);
        self
    }

    /// Atomically install the model spacing covered by the conversion proof.
    pub fn with_kerning(mut self, kerning: FeatureKerningState) -> Self {
        self.kerning = Some(kerning);
        self
    }

    /// Atomically install the model anchors covered by the conversion proof.
    pub fn with_anchors(mut self, anchors: Vec<FeatureAnchorState>) -> Self {
        self.anchors = anchors;
        self
    }
}

impl Document {
    /// Atomically commit a trusted validated candidate after rechecking inputs.
    ///
    /// Generic deserializable commands cannot reach this authority-changing
    /// path. Compile and conversion services validate outside the session lock,
    /// then call this with revisions from the exact snapshot they validated.
    pub fn commit_prepared_features(
        &mut self,
        expected_generation: u64,
        expected_compile_revision: u64,
        prepared: PreparedFeatureChange,
        label: impl Into<String>,
    ) -> Result<Change, Error> {
        if self.generation != expected_generation
            || self.compile_revision != expected_compile_revision
        {
            return Err(Error::InvalidCommand(
                "stale feature activation: document or compile inputs changed".into(),
            ));
        }
        validate_model_changes(&self.font, &prepared)?;
        let impact = Impact {
            feature_source: true,
            layout: true,
            feature_anchors: prepared
                .anchors
                .iter()
                .map(|state| (state.glyph.clone(), state.layer.clone()))
                .collect(),
            ..Default::default()
        };
        let inverse = undo::capture(&self.font, &impact);
        let before_state = self.current_state;
        let active_changed = self.font.features != prepared.source
            || self.font.feature_settings != prepared.settings;
        self.font.features = prepared.source;
        self.font.feature_settings = prepared.settings;
        self.font.feature_draft = None;
        if let Some(conversion) = prepared.conversion {
            self.font.layout_conversion = conversion;
        }
        if let Some(state) = prepared.kerning {
            for record in state.masters {
                if let Some(master) = self
                    .font
                    .masters
                    .iter_mut()
                    .find(|master| master.id == record.master)
                {
                    master.kerning_h = record.kerning;
                }
            }
            self.font.kerning_groups = state.groups;
            for record in state.memberships {
                if let Some(glyph) = self.font.glyph_mut(&record.glyph) {
                    glyph.kern_group_side1 = record.side1;
                    glyph.kern_group_side2 = record.side2;
                }
            }
        }
        for state in prepared.anchors {
            if let Some(layer) = self.font.glyph_mut(&state.glyph).and_then(|glyph| {
                glyph
                    .layers
                    .iter_mut()
                    .find(|layer| layer.id == state.layer)
            }) {
                layer.anchors = state.anchors;
            }
        }
        let forward = undo::capture(&self.font, &impact);
        if inverse == forward {
            return Ok(Change::default());
        }
        // Draft/metadata-only application need not invalidate active shaping.
        let layout = active_changed
            || inverse.iter().zip(&forward).any(|(before, after)| {
                !matches!(before, crate::Snapshot::Features(_)) && before != after
            });
        if let Err(error) = command::bump_feature_revision(&mut self.font) {
            undo::restore(&mut self.font, &inverse);
            return Err(error);
        }
        let forward = undo::capture(&self.font, &impact);
        let change = Change {
            feature_source: true,
            layout,
            ..Default::default()
        };
        self.finalize_active_edit();
        let after_state = self.next_state;
        self.next_state = self.next_state.saturating_add(1);
        self.current_state = after_state;
        self.dirty = self.current_state != self.saved_state;
        self.undo.push(UndoEntry::new(
            label.into(),
            inverse,
            forward,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            change.clone(),
            false,
            before_state,
            after_state,
        ));
        self.generation = self.generation.saturating_add(1);
        if layout {
            self.compile_revision = self.compile_revision.saturating_add(1);
        }
        self.emit(change.clone());
        Ok(change)
    }
}

fn validate_model_changes(font: &Font, prepared: &PreparedFeatureChange) -> Result<(), Error> {
    if let Some(state) = &prepared.kerning {
        validate_kerning_state(font, state)?;
        let mut seen = HashSet::new();
        for record in &state.masters {
            if font.master(&record.master).is_none() {
                return Err(Error::MissingMaster(record.master.clone()));
            }
            if !seen.insert(record.master.clone())
                || record.kerning.0.values().any(|value| !value.is_finite())
            {
                return Err(Error::InvalidCommand(
                    "converted master kerning must be unique and finite".into(),
                ));
            }
            for (left, right) in record.kerning.0.keys() {
                for (side, groups) in [(left, &state.groups.side1), (right, &state.groups.side2)] {
                    match side {
                        fc_model::KernSide::Glyph(name) if font.glyph(name).is_none() => {
                            return Err(Error::MissingGlyph(name.clone()))
                        }
                        fc_model::KernSide::Group(name) if !groups.contains_key(name) => {
                            return Err(Error::InvalidCommand(format!(
                                "converted kerning refers to missing group `{name}`"
                            )))
                        }
                        _ => {}
                    }
                }
            }
        }
        for members in state
            .groups
            .side1
            .values()
            .chain(state.groups.side2.values())
        {
            for name in members {
                if font.glyph(name).is_none() {
                    return Err(Error::MissingGlyph(name.clone()));
                }
            }
        }
        let mut seen = HashSet::new();
        for membership in &state.memberships {
            if font.glyph(&membership.glyph).is_none() {
                return Err(Error::MissingGlyph(membership.glyph.clone()));
            }
            if !seen.insert(&membership.glyph) {
                return Err(Error::InvalidCommand(
                    "duplicate converted glyph membership".into(),
                ));
            }
        }
    }
    let mut seen = HashSet::new();
    for state in &prepared.anchors {
        let glyph = font
            .glyph(&state.glyph)
            .ok_or_else(|| Error::MissingGlyph(state.glyph.clone()))?;
        if glyph.layer(&state.layer).is_none() {
            return Err(Error::MissingLayer {
                glyph: state.glyph.clone(),
                layer: state.layer.clone(),
            });
        }
        if !seen.insert((&state.glyph, &state.layer))
            || state
                .anchors
                .iter()
                .any(|anchor| !anchor.x.is_finite() || !anchor.y.is_finite())
        {
            return Err(Error::InvalidCommand(
                "converted anchor layers must be unique and coordinates finite".into(),
            ));
        }
    }
    Ok(())
}

fn validate_kerning_state(font: &Font, state: &FeatureKerningState) -> Result<(), Error> {
    let memberships: std::collections::HashMap<_, _> = state
        .memberships
        .iter()
        .map(|record| (&record.glyph, record))
        .collect();
    let masters: std::collections::HashMap<_, _> = state
        .masters
        .iter()
        .map(|record| (&record.master, &record.kerning))
        .collect();
    use crate::kerning::KerningGroupSide;
    for (groups, side, first) in [
        (&state.groups.side1, KerningGroupSide::Side1, true),
        (&state.groups.side2, KerningGroupSide::Side2, false),
    ] {
        let mut member_groups = std::collections::HashMap::new();
        for (name, members) in groups {
            side.validate_name(name)?;
            for member in members {
                if font.glyph(member).is_none() {
                    return Err(Error::MissingGlyph(member.clone()));
                }
                if member_groups.insert(member, name).is_some() {
                    return Err(Error::InvalidCommand(format!(
                        "glyph `{member}` belongs to multiple converted groups on one side"
                    )));
                }
            }
        }
        for glyph in font.glyphs.values() {
            let override_membership = memberships.get(&glyph.name);
            let assigned = if first {
                override_membership.map_or(glyph.kern_group_side1.as_ref(), |membership| {
                    membership.side1.as_ref()
                })
            } else {
                override_membership.map_or(glyph.kern_group_side2.as_ref(), |membership| {
                    membership.side2.as_ref()
                })
            };
            let expected = member_groups.get(&glyph.name).copied();
            if assigned != expected {
                return Err(Error::InvalidCommand(format!(
                    "converted group membership disagrees for glyph `{}`",
                    glyph.name
                )));
            }
        }
    }
    // Group replacement is font-global, so unchanged masters must still refer
    // to groups and glyphs present after the transaction too.
    for master in &font.masters {
        let kerning = masters
            .get(&master.id)
            .copied()
            .unwrap_or(&master.kerning_h);
        for (left, right) in kerning.0.keys() {
            for (side, groups) in [(left, &state.groups.side1), (right, &state.groups.side2)] {
                match side {
                    fc_model::KernSide::Glyph(name) if font.glyph(name).is_none() => return Err(Error::MissingGlyph(name.clone())),
                    fc_model::KernSide::Group(name) if !groups.contains_key(name) => return Err(Error::InvalidCommand(format!("converted groups leave master `{}` referring to missing group `{name}`", master.id))),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
