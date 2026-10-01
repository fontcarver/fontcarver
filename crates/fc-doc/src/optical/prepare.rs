use std::collections::{BTreeMap, BTreeSet, HashSet};

use fc_model::{Font, GlyphName, KernSide, MasterId};
use kurbo::{BezPath, Shape};

use super::{
    profile::{invalid, Profile},
    OpticalExample, OpticalOperation, OpticalOptions, OpticalProposal, OpticalProposalEntry,
    OpticalSelection, OpticalSkip, OpticalSkipReason,
};
use crate::{Command, Error, MetricsSource};

const MAX_LAYERS: usize = 512;
const MAX_PAIRS: usize = 16_384;

/// Prepare deterministic optical edits on an owned snapshot without changing a document.
///
/// Uses 256 analytic curve scanlines and caps the aggregate segment-sample work.
/// Sidebearings subtract half the mean silhouette recession from a density-scaled
/// target. Pair kerning approaches a mean gap while preserving a sampled clearance
/// floor. This is an optical starting point, not a claim of continuous collision
/// detection or script-specific typographic judgment.
pub fn prepare(mut font: Font, options: &OpticalOptions) -> Result<OpticalProposal, Error> {
    if !options.density.is_finite() || !(0.25..=4.).contains(&options.density) {
        return Err(invalid("optical density must be between 0.25 and 4"));
    }
    if options.glyphs.is_empty() || options.glyphs.len() > 128 {
        return Err(invalid(
            "select between one and 128 glyphs for optical spacing",
        ));
    }
    let names: BTreeSet<_> = options.glyphs.iter().cloned().collect();
    for name in &names {
        if font.glyph(name).is_none() {
            return Err(Error::MissingGlyph(name.clone()));
        }
    }
    if let Some(id) = &options.master {
        if font.master(id).is_none() {
            return Err(Error::MissingMaster(id.clone()));
        }
    }
    let masters: Vec<_> = font
        .masters
        .iter()
        .filter(|m| options.master.as_ref().is_none_or(|id| *id == m.id))
        .map(|m| (m.id.clone(), m.name.clone()))
        .collect();
    if masters.is_empty() || names.len() * masters.len() > MAX_LAYERS {
        return Err(invalid(
            "optical operation exceeds 512 source layers or has no master",
        ));
    }
    if options.operation == OpticalOperation::Kerning {
        crate::kerning::check_horizontal_editing(&font).map_err(|e| invalid(e.to_string()))?;
        if names.len() * names.len() * masters.len() > MAX_PAIRS {
            return Err(invalid(
                "automatic kerning exceeds 16384 pairs across all masters",
            ));
        }
    }
    if let Some(accepted) = &options.accepted {
        if accepted.len() > MAX_PAIRS {
            return Err(invalid(
                "an optical review cannot accept more than 16384 proposals",
            ));
        }
    }
    let mut result = OpticalProposal {
        commands: Vec::new(),
        glyph_count: names.len() as u32,
        changed_count: 0,
        skipped: Vec::new(),
        examples: Vec::new(),
        proposals: Vec::new(),
    };
    let mut budget = 16_000_000usize;
    for (master, master_name) in masters {
        match options.operation {
            OpticalOperation::Spacing => spacing(
                &mut font,
                &names,
                &master,
                &master_name,
                options,
                &mut result,
                &mut budget,
            )?,
            OpticalOperation::Kerning => kerning(
                &font,
                &names,
                &master,
                &master_name,
                options,
                &mut result,
                &mut budget,
            )?,
        }
    }
    // A selection the recomputed proposal no longer holds means the review was
    // made against a different document; nothing is applied.
    if let Some(accepted) = &options.accepted {
        let proposed: BTreeSet<OpticalSelection> =
            result.proposals.iter().map(OpticalSelection::of).collect();
        if let Some(missing) = accepted.iter().find(|entry| !proposed.contains(entry)) {
            return Err(invalid(format!(
                "the reviewed proposal for {}{} no longer exists; preview again",
                missing.glyph,
                missing
                    .right
                    .as_ref()
                    .map(|right| format!("/{right}"))
                    .unwrap_or_default()
            )));
        }
    }
    Ok(result)
}

/// Whether one proposed change is in the reviewed acceptance, if there is one.
fn is_accepted(options: &OpticalOptions, entry: &OpticalProposalEntry) -> bool {
    match &options.accepted {
        None => true,
        Some(accepted) => {
            let key = OpticalSelection::of(entry);
            accepted.contains(&key)
        }
    }
}

fn skip(
    result: &mut OpticalProposal,
    glyph: &GlyphName,
    right: Option<&GlyphName>,
    master: &str,
    reason: OpticalSkipReason,
) {
    result.skipped.push(OpticalSkip {
        glyph: glyph.to_string(),
        right: right.map(ToString::to_string),
        master: master.to_owned(),
        reason,
    });
}

struct Source {
    path: BezPath,
    width: f64,
    layer: fc_model::LayerId,
}

fn source(
    font: &Font,
    name: &GlyphName,
    master: &MasterId,
    spacing: bool,
    result: &mut OpticalProposal,
    master_name: &str,
) -> Result<Option<Source>, Error> {
    let glyph = font
        .glyph(name)
        .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
    let layer = glyph
        .master_layer(master)
        .ok_or_else(|| invalid(format!("{name} has no source for {master_name}")))?;
    if glyph.locked {
        skip(result, name, None, master_name, OpticalSkipReason::Locked);
        return Ok(None);
    }
    if !layer.width.is_finite() || !(0. ..=65535.).contains(&layer.width) {
        return Err(invalid(format!("{name} has an invalid advance width")));
    }
    if spacing && crate::metrics::resolve(font, name, master)?.source == MetricsSource::Key {
        skip(
            result,
            name,
            None,
            master_name,
            OpticalSkipReason::MetricsKey,
        );
        return Ok(None);
    }
    if layer.width == 0. {
        skip(
            result,
            name,
            None,
            master_name,
            OpticalSkipReason::EmptyOrMark,
        );
        return Ok(None);
    }
    let path = fc_geom::flatten_layer(font, name, master)?;
    if path.elements().is_empty()
        || path.bounding_box().width() <= 0.
        || path.bounding_box().height() <= 0.
    {
        skip(
            result,
            name,
            None,
            master_name,
            OpticalSkipReason::EmptyOrMark,
        );
        return Ok(None);
    }
    if !path.is_finite() || path.elements().len() > 4096 {
        return Err(invalid(format!(
            "{name} has invalid geometry or exceeds 4096 resolved segments"
        )));
    }
    Ok(Some(Source {
        path,
        width: layer.width,
        layer: layer.id.clone(),
    }))
}

fn example(
    name: &GlyphName,
    right: Option<&GlyphName>,
    master: &str,
    source: &Source,
    before: [f64; 3],
    after: [f64; 3],
) -> OpticalExample {
    let b = source.path.bounding_box();
    OpticalExample {
        glyph: name.to_string(),
        right: right.map(ToString::to_string),
        master: master.to_owned(),
        before,
        after,
        outline: source.path.to_svg(),
        after_outline: None,
        after_bounds: None,
        bounds: [b.x0, b.y0, b.x1, b.y1],
    }
}

#[allow(clippy::too_many_arguments)]
fn spacing(
    font: &mut Font,
    names: &BTreeSet<GlyphName>,
    master: &MasterId,
    master_name: &str,
    options: &OpticalOptions,
    result: &mut OpticalProposal,
    budget: &mut usize,
) -> Result<(), Error> {
    let ordered = dependency_order(font, names, master)?;
    // Store pre-edit metrics for review before selected components' bases move.
    let mut originals = BTreeMap::new();
    for name in &ordered {
        if let Some(source) = source(font, name, master, true, result, master_name)? {
            originals.insert(name.clone(), source);
        }
    }
    for name in ordered {
        let Some(original) = originals.get(&name) else {
            continue;
        };
        let path = fc_geom::flatten_layer(font, &name, master)?;
        let b = path.bounding_box();
        let profile = Profile::sample(&path, (b.y0, b.y1), budget)?;
        let Some((lsb, rsb)) =
            profile.sidebearings(f64::from(font.info.units_per_em), options.density)
        else {
            skip(
                result,
                &name,
                None,
                master_name,
                OpticalSkipReason::EmptyOrMark,
            );
            continue;
        };
        let width = b.width() + lsb + rsb;
        if !width.is_finite() || !(0. ..=65535.).contains(&width) {
            return Err(invalid(format!(
                "optical advance for {name} exceeds the OpenType range"
            )));
        }
        let before_bounds = original.path.bounding_box();
        let before = [
            before_bounds.x0,
            original.width - before_bounds.x1,
            original.width,
        ];
        let after = [lsb, rsb, width];
        let current_width = font
            .glyph(&name)
            .and_then(|g| g.master_layer(master))
            .ok_or_else(|| invalid("optical source disappeared"))?
            .width;
        if (b.x0 - lsb).abs() <= 1e-8 && (current_width - b.x1 - rsb).abs() <= 1e-8 {
            continue;
        }
        let entry = OpticalProposalEntry {
            glyph: name.to_string(),
            right: None,
            master: master.clone(),
            master_name: master_name.to_owned(),
            before,
            after,
        };
        let accepted = is_accepted(options, &entry);
        result.proposals.push(entry);
        if !accepted {
            // A rejected layer must not move, so dependents keep measuring the
            // snapshot as it stands.
            continue;
        }
        let command = Command::SetMetrics {
            glyph: name.clone(),
            layer: original.layer.clone(),
            width: None,
            lsb: Some(lsb),
            rsb: Some(rsb),
        };
        // The private snapshot ensures downstream components are measured exactly
        // as the ordered atomic batch will measure them when committed.
        crate::command::apply_to_font(font, &command, &[])?;
        result.commands.push(command);
        result.changed_count += 1;
        if result.examples.len() < 16 {
            let mut reviewed = example(&name, None, master_name, original, before, after);
            let final_path = fc_geom::flatten_layer(font, &name, master)?;
            let final_bounds = final_path.bounding_box();
            reviewed.after_outline = Some(final_path.to_svg());
            reviewed.after_bounds = Some([
                final_bounds.x0,
                final_bounds.y0,
                final_bounds.x1,
                final_bounds.y1,
            ]);
            result.examples.push(reviewed);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn kerning(
    font: &Font,
    names: &BTreeSet<GlyphName>,
    master: &MasterId,
    master_name: &str,
    options: &OpticalOptions,
    result: &mut OpticalProposal,
    budget: &mut usize,
) -> Result<(), Error> {
    let mut sources = BTreeMap::new();
    let mut band = (f64::INFINITY, f64::NEG_INFINITY);
    for name in names {
        if let Some(source) = source(font, name, master, false, result, master_name)? {
            let b = source.path.bounding_box();
            band.0 = band.0.min(b.y0);
            band.1 = band.1.max(b.y1);
            sources.insert(name.clone(), source);
        }
    }
    let profiles: BTreeMap<_, _> = sources
        .iter()
        .map(|(name, source)| Ok((name.clone(), Profile::sample(&source.path, band, budget)?)))
        .collect::<Result<_, Error>>()?;
    for (left, left_source) in &sources {
        for right in sources.keys() {
            let existing = crate::kerning::resolve_stored_horizontal(font, left, right, master)?;
            if existing.is_some() && !options.replace_existing {
                skip(
                    result,
                    left,
                    Some(right),
                    master_name,
                    OpticalSkipReason::ExistingKerning,
                );
                continue;
            }
            let (Some(left_profile), Some(right_profile)) =
                (profiles.get(left), profiles.get(right))
            else {
                return Err(invalid("optical profile disappeared"));
            };
            let Some(value) = left_profile.kerning(
                right_profile,
                left_source.width,
                f64::from(font.info.units_per_em),
                options.density,
            ) else {
                skip(
                    result,
                    left,
                    Some(right),
                    master_name,
                    OpticalSkipReason::NoOverlap,
                );
                continue;
            };
            if !value.is_finite() || !(-32768. ..=32767.).contains(&value) {
                return Err(invalid(format!(
                    "automatic kerning for {left}/{right} exceeds the OpenType range"
                )));
            }
            let before = existing.as_ref().map_or(0., |pair| pair.value);
            if (before - value).abs() <= 1e-8 {
                continue;
            }
            let entry = OpticalProposalEntry {
                glyph: left.to_string(),
                right: Some(right.to_string()),
                master: master.clone(),
                master_name: master_name.to_owned(),
                before: [before, 0., 0.],
                after: [value, 0., 0.],
            };
            let accepted = is_accepted(options, &entry);
            result.proposals.push(entry);
            if !accepted {
                continue;
            }
            result.commands.push(Command::SetHorizontalKerning {
                master: master.clone(),
                left: KernSide::Glyph(left.clone()),
                right: KernSide::Glyph(right.clone()),
                value: Some(value),
            });
            result.changed_count += 1;
            if result.examples.len() < 16 {
                result.examples.push(example(
                    left,
                    Some(right),
                    master_name,
                    left_source,
                    [before, 0., 0.],
                    [value, 0., 0.],
                ));
            }
        }
    }
    Ok(())
}

fn dependency_order(
    font: &Font,
    selected: &BTreeSet<GlyphName>,
    master: &MasterId,
) -> Result<Vec<GlyphName>, Error> {
    struct Walk<'a> {
        font: &'a Font,
        selected: &'a BTreeSet<GlyphName>,
        master: &'a MasterId,
        visiting: HashSet<GlyphName>,
        done: HashSet<GlyphName>,
        ordered: Vec<GlyphName>,
        remaining: usize,
    }
    impl Walk<'_> {
        fn visit(&mut self, name: &GlyphName) -> Result<(), Error> {
            if self.done.contains(name) {
                return Ok(());
            }
            if self.visiting.len() >= 64 || !self.visiting.insert(name.clone()) {
                return Err(invalid(
                    "cyclic or excessively deep optical spacing components",
                ));
            }
            let glyph = self
                .font
                .glyph(name)
                .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
            for layer in &glyph.layers {
                if !matches!(&layer.kind, fc_model::LayerKind::Master(id) | fc_model::LayerKind::Smart {master:id, ..} if id == self.master)
                {
                    continue;
                }
                for component in &layer.components {
                    self.remaining = self
                        .remaining
                        .checked_sub(1)
                        .ok_or_else(|| invalid("optical spacing component work limit"))?;
                    self.visit(&component.base)?;
                }
            }
            self.visiting.remove(name);
            self.done.insert(name.clone());
            if self.selected.contains(name) {
                self.ordered.push(name.clone());
            }
            Ok(())
        }
    }
    let mut walk = Walk {
        font,
        selected,
        master,
        visiting: HashSet::new(),
        done: HashSet::new(),
        ordered: Vec::new(),
        remaining: 100_000,
    };
    for name in selected {
        walk.visit(name)?;
    }
    Ok(walk.ordered)
}
