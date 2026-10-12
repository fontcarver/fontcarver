use crate::Error;
use fc_model::{Axis, Font, Instance, LayerId, LayerKind, Master, MasterId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use ts_rs::TS;

/// Atomic design-space metadata and source-layer editing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum DesignSpaceCommand {
    /// Replace named sparse-source coordinates without allocating glyph layers.
    VirtualMasters {
        /// All named virtual positions in design coordinates.
        #[ts(type = "Array<VirtualMaster>")]
        masters: Vec<fc_model::VirtualMaster>,
    },
    /// Duplicate one selected glyph source at a named virtual position.
    VirtualSource {
        /// Glyph receiving a sparse intermediate source.
        #[ts(type = "string")]
        glyph: fc_model::GlyphName,
        /// Existing layer whose geometry seeds the new source.
        #[ts(type = "string")]
        source: LayerId,
        /// Index into the current named virtual positions.
        index: usize,
    },
    /// Replace a glyph's local smart-component axes.
    SmartAxes {
        /// Glyph whose reusable geometry is parameterized.
        #[ts(type = "string")]
        glyph: fc_model::GlyphName,
        /// Complete local axis definitions.
        #[ts(type = "Array<SmartAxis>")]
        axes: Vec<fc_model::SmartAxis>,
    },
    /// Duplicate an outline source or change an existing supplemental layer's role.
    Layer {
        /// Glyph containing the source.
        #[ts(type = "string")]
        glyph: fc_model::GlyphName,
        /// Existing layer to copy or edit.
        #[ts(type = "string")]
        source: LayerId,
        /// Copy to a fresh identity instead of editing the source's metadata.
        duplicate: bool,
        /// Intermediate or conditional destination role.
        #[ts(type = "LayerKind")]
        kind: LayerKind,
        /// Display name.
        name: String,
    },
    /// Replace one alternate region's condition set in every source that shares it.
    ///
    /// An alternate region is identified by its canonical condition set, so this
    /// keeps the sources of one region consistent in a single undo entry instead
    /// of leaving some of them behind on the previous condition.
    Conditions {
        /// Glyph whose alternate region changes.
        #[ts(type = "string")]
        glyph: fc_model::GlyphName,
        /// Condition set to replace, in any order.
        #[ts(type = "Array<BracketCondition>")]
        from: Vec<fc_model::BracketCondition>,
        /// Replacement condition set, in any order.
        #[ts(type = "Array<BracketCondition>")]
        to: Vec<fc_model::BracketCondition>,
        /// Display name for every affected layer.
        name: String,
    },
    /// Reorder one glyph's shapes, in one source or in every source.
    ///
    /// Shape order is the interpolation contract: reordering a single source is
    /// how a "shape shifter" is repaired, and reordering every source at once is
    /// the all-masters variant. A source with a different shape count is
    /// reported instead of being reordered.
    ReorderShapes {
        /// Glyph whose shape order changes.
        #[ts(type = "string")]
        glyph: fc_model::GlyphName,
        /// Source layer to reorder; absent reorders every source layer.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional, type = "string")]
        layer: Option<LayerId>,
        /// New positions: `order[i]` is the current index that moves to `i`.
        order: Vec<usize>,
        /// Reorder components instead of contours.
        #[serde(default)]
        #[ts(as = "Option<bool>", optional)]
        components: bool,
    },
    /// Align contour directions, ordering and starts while preserving all geometry.
    Repair {
        /// Glyphs to repair atomically; incompatible topology remains unchanged.
        #[ts(type = "Array<string>")]
        glyphs: Vec<fc_model::GlyphName>,
    },
    /// Replace metadata, cloning added masters and removing deleted masters' layers.
    Set {
        /// Explicit old-to-new axis tags; source-layer references migrate atomically.
        #[serde(default)]
        axis_renames: std::collections::BTreeMap<String, String>,
        /// Ordered axes in user coordinates.
        #[ts(type = "Array<Axis>")]
        axes: Vec<Axis>,
        /// Complete master list in design coordinates.
        #[ts(type = "Array<Master>")]
        masters: Vec<Master>,
        /// Complete named-instance list in design coordinates.
        #[ts(type = "Array<Instance>")]
        instances: Vec<Instance>,
        /// Optional replacement of virtual positions; omission preserves existing positions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional, type = "Array<VirtualMaster>")]
        virtual_masters: Option<Vec<fc_model::VirtualMaster>>,
        /// Existing master whose layers seed newly added masters.
        #[ts(type = "string")]
        copy_from: MasterId,
    },
}

pub(super) fn apply(font: &mut Font, command: &DesignSpaceCommand) -> Result<(), Error> {
    let mut candidate = font.clone();
    apply_candidate(&mut candidate, command)?;
    *font = candidate;
    Ok(())
}

fn apply_candidate(font: &mut Font, command: &DesignSpaceCommand) -> Result<(), Error> {
    if let DesignSpaceCommand::VirtualMasters { masters } = command {
        font.virtual_masters = masters.clone();
        font.validate_designspace()?;
        return Ok(());
    }
    if let DesignSpaceCommand::VirtualSource {
        glyph,
        source,
        index,
    } = command
    {
        let position = font.virtual_masters.get(*index).ok_or_else(|| {
            Error::InvalidCommand("virtual master position does not exist".into())
        })?;
        let command = DesignSpaceCommand::Layer {
            glyph: glyph.clone(),
            source: source.clone(),
            duplicate: true,
            name: position.name.clone(),
            kind: LayerKind::Brace {
                master: font.design_default_master()?.id.clone(),
                location: font.complete_location(&position.location)?,
            },
        };
        return apply_candidate(font, &command);
    }
    if let DesignSpaceCommand::SmartAxes { glyph, axes } = command {
        let target = font
            .glyph_mut(glyph)
            .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
        target.smart_axes = axes.clone();
        target.validate_smart_axes()?;
        for component in font
            .glyphs
            .values()
            .flat_map(|g| &g.layers)
            .flat_map(|l| &l.components)
            .filter(|c| c.base == *glyph)
        {
            if let Some(parameters) = &component.smart {
                for (name, value) in parameters {
                    if !axes.iter().any(|axis| {
                        axis.name == *name
                            && value.is_finite()
                            && *value >= axis.min
                            && *value <= axis.max
                    }) {
                        return Err(Error::InvalidCommand(
                            "existing component parameters fall outside the new local axes".into(),
                        ));
                    }
                }
            }
        }
        return Ok(());
    }
    if let DesignSpaceCommand::Layer {
        glyph,
        source,
        duplicate,
        kind,
        name,
    } = command
    {
        if !matches!(
            kind,
            LayerKind::Brace { .. } | LayerKind::Bracket { .. } | LayerKind::Smart { .. }
        ) {
            return Err(Error::InvalidCommand(
                "choose an intermediate or conditional layer role".into(),
            ));
        }
        let target = font
            .glyph_mut(glyph)
            .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
        let index = target
            .layers
            .iter()
            .position(|layer| layer.id == *source)
            .ok_or_else(|| Error::MissingLayer {
                glyph: glyph.clone(),
                layer: source.clone(),
            })?;
        if !duplicate && matches!(target.layers[index].kind, LayerKind::Master(_)) {
            return Err(Error::InvalidCommand(
                "duplicate a master layer before changing its role".into(),
            ));
        }
        let mut layer = target.layers[index].clone();
        layer.kind = kind.clone();
        layer.name = name.clone();
        if *duplicate {
            if target.layers.len() >= 4096 {
                return Err(Error::InvalidCommand("glyph exceeds 4096 layers".into()));
            }
            layer.id = LayerId::new();
            target.layers.push(layer);
        } else {
            target.layers[index] = layer;
        }
        font.validate_designspace()?;
        return Ok(());
    }
    if let DesignSpaceCommand::Conditions {
        glyph,
        from,
        to,
        name,
    } = command
    {
        let canonical = |conditions: &[fc_model::BracketCondition]| {
            let mut sorted = conditions.to_vec();
            sorted.sort_by_key(|condition| condition.axis);
            sorted
        };
        let source = canonical(from);
        let replacement =
            LayerKind::from_bracket_conditions(font.design_default_master()?.id.clone(), to)?;
        let Some(target) = replacement.bracket_conditions() else {
            return Err(Error::InvalidCommand(
                "an alternate region needs a condition set".into(),
            ));
        };
        let glyph_data = font
            .glyph_mut(glyph)
            .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
        let mut changed = 0usize;
        for layer in &mut glyph_data.layers {
            if layer.kind.bracket_conditions().as_deref() != Some(source.as_slice()) {
                continue;
            }
            let LayerKind::Bracket { master, .. } = &layer.kind else {
                continue;
            };
            layer.kind = LayerKind::from_bracket_conditions(master.clone(), &target)?;
            layer.name = name.clone();
            changed += 1;
        }
        if changed == 0 {
            return Err(Error::InvalidCommand(
                "no alternate source uses that condition set".into(),
            ));
        }
        font.validate_designspace()?;
        return Ok(());
    }
    if let DesignSpaceCommand::ReorderShapes {
        glyph,
        layer,
        order,
        components,
    } = command
    {
        let target = font
            .glyph_mut(glyph)
            .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
        let sources: Vec<usize> = match layer {
            Some(selected) => vec![target
                .layers
                .iter()
                .position(|candidate| candidate.id == *selected)
                .ok_or_else(|| Error::MissingLayer {
                    glyph: glyph.clone(),
                    layer: selected.clone(),
                })?],
            None => target
                .layers
                .iter()
                .enumerate()
                .filter(|(_, layer)| {
                    matches!(
                        layer.kind,
                        LayerKind::Master(_)
                            | LayerKind::Brace { .. }
                            | LayerKind::Bracket { .. }
                            | LayerKind::Smart { .. }
                    )
                })
                .map(|(index, _)| index)
                .collect(),
        };
        if sources.is_empty() {
            return Err(Error::InvalidCommand(
                "the glyph has no source layers to reorder".into(),
            ));
        }
        let count = |layer: &fc_model::Layer| {
            if *components {
                layer.components.len()
            } else {
                layer.contours.len()
            }
        };
        let expected = count(&target.layers[sources[0]]);
        if order.len() != expected
            || order.iter().any(|index| *index >= expected)
            || order.iter().collect::<std::collections::HashSet<_>>().len() != order.len()
        {
            return Err(Error::InvalidCommand(
                "shape order must be a permutation of the current shapes".into(),
            ));
        }
        if sources
            .iter()
            .any(|index| count(&target.layers[*index]) != expected)
        {
            return Err(Error::InvalidCommand(
                "sources have different shape counts; repair topology before reordering".into(),
            ));
        }
        for index in sources {
            let layer = &mut target.layers[index];
            if *components {
                let current = layer.components.clone();
                layer.components = order
                    .iter()
                    .map(|position| current[*position].clone())
                    .collect();
            } else {
                let current = layer.contours.clone();
                layer.contours = order
                    .iter()
                    .map(|position| current[*position].clone())
                    .collect();
                // Point-indexed data describes the old order.
                layer.tt_instructions = None;
                layer.hints.hint_masks.clear();
                for point in layer.contours.iter_mut().flat_map(|c| &mut c.points) {
                    point.tt_index = fc_model::TtIndex::Unnumbered;
                }
            }
        }
        return Ok(());
    }
    if let DesignSpaceCommand::Repair { glyphs } = command {
        let default = font.design_default_master()?.id.clone();
        if glyphs.len() > 4096 {
            return Err(Error::InvalidCommand(
                "repair at most 4096 glyphs per operation".into(),
            ));
        }
        for name in glyphs {
            let glyph = font
                .glyph_mut(name)
                .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
            let reference = fc_geom::canonical_source(
                glyph
                    .master_layer(&default)
                    .ok_or_else(|| Error::InvalidCommand("default source is missing".into()))?,
            );
            let mut alternates: Vec<(Vec<fc_model::BracketCondition>, fc_model::Layer)> =
                Vec::new();
            for layer in &glyph.layers {
                let (LayerKind::Bracket { master, .. }, Some(key)) =
                    (&layer.kind, layer.kind.bracket_conditions())
                else {
                    continue;
                };
                if let Some((_, existing)) = alternates.iter_mut().find(|(old, _)| *old == key) {
                    if *master == default {
                        *existing = fc_geom::canonical_source(layer);
                    }
                } else {
                    alternates.push((key, fc_geom::canonical_source(layer)));
                }
            }
            for layer in &mut glyph.layers {
                let conditions = layer.kind.bracket_conditions();
                let source = match &layer.kind {
                    LayerKind::Master(_) | LayerKind::Brace { .. } | LayerKind::Smart { .. } => {
                        Some(&reference)
                    }
                    LayerKind::Bracket { .. } => alternates
                        .iter()
                        .find(|(key, _)| Some(key.as_slice()) == conditions.as_deref())
                        .map(|(_, layer)| layer),
                    _ => None,
                };
                if let Some(source) = source {
                    *layer = fc_geom::repair_compatibility(source, layer)
                        .map_err(|e| Error::Geometry(e.to_string()))?;
                }
            }
        }
        return Ok(());
    }
    let DesignSpaceCommand::Set {
        axis_renames,
        axes,
        masters,
        instances,
        virtual_masters,
        copy_from,
    } = command
    else {
        return Err(Error::InvalidCommand("unknown design-space command".into()));
    };
    let copy_master = font
        .master(copy_from)
        .ok_or_else(|| Error::MissingMaster(copy_from.clone()))?;
    let old_ids: HashSet<_> = font.masters.iter().map(|m| m.id.clone()).collect();
    let new_ids: HashSet<_> = masters.iter().map(|m| m.id.clone()).collect();
    // Existing kerning has its own authority and validation. Metadata edits must
    // not become a back door for replacing imported layout or malformed pairs.
    for master in masters {
        let source = font.master(&master.id).unwrap_or(copy_master);
        if !crate::kerning::same_records(&master.kerning_h, &source.kerning_h)
            || !crate::kerning::same_records(&master.kerning_v, &source.kerning_v)
        {
            return Err(Error::InvalidCommand(
                "edit kerning through its dedicated commands".into(),
            ));
        }
    }
    let added: Vec<_> = masters
        .iter()
        .filter(|m| !old_ids.contains(&m.id))
        .map(|m| m.id.clone())
        .collect();
    let removed = old_ids.iter().any(|id| !new_ids.contains(id));
    if !added.is_empty() || removed {
        for glyph in font.glyphs.values_mut() {
            let source = glyph.master_layer(copy_from).cloned();
            let local_sources:Vec<_>=glyph.layers.iter().filter(|layer| matches!(&layer.kind,LayerKind::Smart {master,..} if master == copy_from)).cloned().collect();
            for id in &added {
                let mut layer = source.clone().ok_or_else(|| {
                    Error::InvalidCommand(format!(
                        "{} has no source layer to duplicate",
                        glyph.name
                    ))
                })?;
                layer.id = LayerId::new();
                layer.kind = LayerKind::Master(id.clone());
                layer.name.clear();
                glyph.layers.push(layer);
                for source in &local_sources {
                    let mut layer = source.clone();
                    layer.id = LayerId::new();
                    if let LayerKind::Smart { master, .. } = &mut layer.kind {
                        *master = id.clone();
                    }
                    glyph.layers.push(layer);
                }
            }
            glyph.layers.retain(|layer| match &layer.kind {
                LayerKind::Master(id)
                | LayerKind::Background(id)
                | LayerKind::Smart { master: id, .. }
                | LayerKind::Brace { master: id, .. }
                | LayerKind::Bracket { master: id, .. }
                | LayerKind::Backup { of: id, .. } => new_ids.contains(id),
                LayerKind::Custom(_) => true,
            });
        }
    }
    migrate_axis_references(font, axes, axis_renames)?;
    font.axes = axes.clone();
    font.masters = masters.clone();
    font.instances = instances.clone();
    if let Some(virtual_masters) = virtual_masters {
        font.virtual_masters = virtual_masters.clone();
    }
    font.validate_designspace()?;
    Ok(())
}

fn migrate_axis_references(
    font: &mut Font,
    axes: &[Axis],
    renames: &std::collections::BTreeMap<String, String>,
) -> Result<(), Error> {
    use std::str::FromStr;
    let mut mapping = std::collections::BTreeMap::new();
    let mut destinations = HashSet::new();
    for (old, new) in renames {
        let old = fc_model::Tag::from_str(old)?;
        let new = fc_model::Tag::from_str(new)?;
        if !font.axes.iter().any(|axis| axis.tag == old)
            || !axes.iter().any(|axis| axis.tag == new)
            || !destinations.insert(new)
        {
            return Err(Error::InvalidCommand(
                "axis renames require distinct existing source and destination axes".into(),
            ));
        }
        mapping.insert(old, new);
    }
    if mapping.is_empty() {
        return Ok(());
    }
    for master in &mut font.virtual_masters {
        let mut migrated = fc_model::DesignLocation::new();
        for (tag, value) in &master.location {
            if migrated
                .insert(mapping.get(tag).copied().unwrap_or(*tag), *value)
                .is_some()
            {
                return Err(Error::InvalidCommand(
                    "axis rename collides with a virtual coordinate".into(),
                ));
            }
        }
        master.location = migrated;
    }
    for layer in font.glyphs.values_mut().flat_map(|glyph| &mut glyph.layers) {
        match &mut layer.kind {
            LayerKind::Brace { location, .. } => {
                let mut migrated = fc_model::DesignLocation::new();
                for (tag, value) in location.iter() {
                    if migrated
                        .insert(mapping.get(tag).copied().unwrap_or(*tag), *value)
                        .is_some()
                    {
                        return Err(Error::InvalidCommand(
                            "axis rename collides with an existing source coordinate".into(),
                        ));
                    }
                }
                *location = migrated;
            }
            LayerKind::Bracket { axis, extra, .. } => {
                if let Some(new) = mapping.get(axis) {
                    *axis = *new;
                }
                for condition in extra.iter_mut() {
                    if let Some(new) = mapping.get(&condition.axis) {
                        condition.axis = *new;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
