//! Validation and source selection for variable exports.
use crate::{Error, ExportWarning};
use fc_model::{Axis, DesignLocation, FeaturesSource, Font, LayerKind, MasterId, Tag};
use std::collections::HashSet;

pub(crate) fn validate_export(font: &Font) -> Result<(), Error> {
    font.validate_designspace()
        .map_err(|e| Error::InvalidSource(e.to_string()))?;
    if font.features.source_of_truth == FeaturesSource::ImportedTables
        && [Tag(*b"GSUB"), Tag(*b"GPOS"), Tag(*b"GDEF"), Tag(*b"BASE")]
            .iter()
            .any(|tag| font.extra.raw_tables.contains_key(tag))
    {
        return Err(Error::InvalidSource("convert imported layout to editable feature source before compiling a new variable design space".into()));
    }
    Ok(())
}

/// Keep only the listed masters, then shrink the design space around them.
///
/// Every axis is clamped to the span the retained masters actually cover, an
/// axis with no remaining span is removed entirely, and a named instance that
/// falls outside the retained space is reported and dropped instead of being
/// compiled at a position no master describes.
pub(crate) fn master_subset(
    font: &Font,
    subset: &[MasterId],
) -> Result<(Font, Vec<ExportWarning>), Error> {
    let ids: HashSet<_> = subset.iter().collect();
    if ids.is_empty() || ids.len() != subset.len() || ids.iter().any(|id| font.master(id).is_none())
    {
        return Err(Error::InvalidSource(
            "master subset must contain unique existing master IDs".into(),
        ));
    }
    let mut selected = font.clone();
    selected.masters.retain(|m| ids.contains(&m.id));
    retain_master_layers(&mut selected, &ids);
    let warnings = shrink(&mut selected)?;
    Ok((selected, warnings))
}

/// Keep only the listed axes, dropping masters that are off-default elsewhere.
pub(crate) fn axis_subset(font: &Font, keep: &[Tag]) -> Result<(Font, Vec<ExportWarning>), Error> {
    let tags: HashSet<_> = keep.iter().copied().collect();
    if tags.is_empty() || tags.len() != keep.len() {
        return Err(Error::InvalidSource(
            "axis subset must contain unique axis tags".into(),
        ));
    }
    if let Some(missing) = keep
        .iter()
        .find(|tag| !font.axes.iter().any(|a| a.tag == **tag))
    {
        return Err(Error::InvalidSource(format!(
            "axis subset names `{missing}`, which this design space does not define"
        )));
    }
    let dropped: Vec<Tag> = font
        .axes
        .iter()
        .map(|axis| axis.tag)
        .filter(|tag| !tags.contains(tag))
        .collect();
    let mut selected = font.clone();
    if !dropped.is_empty() {
        let defaults: Vec<(Tag, f64)> = font
            .axes
            .iter()
            .filter(|axis| dropped.contains(&axis.tag))
            .map(|axis| (axis.tag, axis.user_to_design(axis.default)))
            .collect();
        let at_default = |location: &DesignLocation| {
            defaults
                .iter()
                .all(|(tag, default)| location.get(tag).copied().unwrap_or(*default) == *default)
        };
        let retained: HashSet<MasterId> = selected
            .masters
            .iter()
            .filter(|master| at_default(&master.location))
            .map(|master| master.id.clone())
            .collect();
        selected
            .masters
            .retain(|master| retained.contains(&master.id));
        let borrowed: HashSet<&MasterId> = retained.iter().collect();
        retain_master_layers(&mut selected, &borrowed);
        if selected.masters.is_empty() {
            return Err(Error::InvalidSource(
                "the axis subset leaves no master at the remaining axes' default location".into(),
            ));
        }
    }
    let warnings = shrink(&mut selected)?;
    Ok((selected, warnings))
}

/// Keep only the named instances the configuration lists.
pub(crate) fn instance_subset(
    font: &Font,
    keep: &[String],
) -> Result<(Font, Vec<ExportWarning>), Error> {
    let mut selected = font.clone();
    let mut warnings = Vec::new();
    let wanted: HashSet<&str> = keep.iter().map(String::as_str).collect();
    if wanted.len() != keep.len() {
        return Err(Error::InvalidSource(
            "instance subset must contain unique instance names".into(),
        ));
    }
    if let Some(missing) = keep
        .iter()
        .find(|name| !font.instances.iter().any(|i| i.name == **name))
    {
        return Err(Error::InvalidSource(format!(
            "instance subset names `{missing}`, which this design space does not define"
        )));
    }
    let before = selected.instances.len();
    selected
        .instances
        .retain(|instance| wanted.contains(instance.name.as_str()));
    if selected.instances.len() != before {
        warnings.push(ExportWarning {
            code: "designspace-instances-subset",
            message: format!(
                "{} of {before} named instances were kept by the export configuration",
                selected.instances.len()
            ),
        });
    }
    Ok((selected, warnings))
}

fn retain_master_layers(font: &mut Font, ids: &HashSet<&MasterId>) {
    for glyph in font.glyphs.values_mut() {
        glyph.layers.retain(|layer| match &layer.kind {
            LayerKind::Master(id)
            | LayerKind::Background(id)
            | LayerKind::Smart { master: id, .. }
            | LayerKind::Brace { master: id, .. }
            | LayerKind::Bracket { master: id, .. }
            | LayerKind::Backup { of: id, .. } => ids.contains(id),
            LayerKind::Custom(_) => true,
        });
    }
}

/// Clamp every axis to the retained masters' span, drop the axes that collapse.
fn shrink(font: &mut Font) -> Result<Vec<ExportWarning>, Error> {
    let mut warnings = Vec::new();
    let original = font.axes.clone();
    let locations = font
        .masters
        .iter()
        .map(|master| font.complete_location(&master.location))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| Error::InvalidSource(error.to_string()))?;
    let mut dropped = Vec::new();
    let mut axes = Vec::new();
    for axis in &original {
        let values: Vec<f64> = locations
            .iter()
            .map(|location| {
                location
                    .get(&axis.tag)
                    .copied()
                    .unwrap_or_else(|| axis.user_to_design(axis.default))
            })
            .collect();
        let low = values.iter().copied().fold(f64::INFINITY, f64::min);
        let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if !low.is_finite() || !high.is_finite() {
            return Err(Error::InvalidSource(format!(
                "{} has a non-finite master coordinate",
                axis.tag
            )));
        }
        if low == high {
            dropped.push(axis.tag);
            continue;
        }
        axes.push(clamped_axis(axis, low, high));
    }
    if axes.is_empty() {
        return Err(Error::InvalidSource(
            "the selected sources do not vary on any axis; export a static master or instance instead"
                .into(),
        ));
    }
    if !dropped.is_empty() {
        warnings.push(ExportWarning {
            code: "designspace-axis-dropped",
            message: format!(
                "{} carried no remaining variation in this export configuration and was dropped",
                dropped
                    .iter()
                    .map(Tag::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    let retained: HashSet<Tag> = axes.iter().map(|axis| axis.tag).collect();
    font.axes = axes;
    let trim = |location: &mut DesignLocation| {
        location.retain(|tag, _| retained.contains(tag));
    };
    for master in &mut font.masters {
        trim(&mut master.location);
    }
    for master in &mut font.virtual_masters {
        trim(&mut master.location);
    }
    let bounds: Vec<(Tag, f64, f64)> = font
        .axes
        .iter()
        .map(|axis| {
            (
                axis.tag,
                axis.user_to_design(axis.min),
                axis.user_to_design(axis.max),
            )
        })
        .collect();
    let inside = |location: &DesignLocation, defaults: &[(Tag, f64)]| {
        defaults
            .iter()
            .all(|(tag, default)| location.get(tag).copied().unwrap_or(*default) == *default)
            && bounds.iter().all(|(tag, low, high)| {
                location
                    .get(tag)
                    .copied()
                    .is_none_or(|value| value >= *low && value <= *high)
            })
    };
    let dropped_defaults: Vec<(Tag, f64)> = original
        .iter()
        .filter(|axis| dropped.contains(&axis.tag))
        .map(|axis| (axis.tag, axis.user_to_design(axis.default)))
        .collect();
    let before = font.instances.len();
    font.instances
        .retain(|instance| inside(&instance.location, &dropped_defaults));
    for instance in &mut font.instances {
        trim(&mut instance.location);
    }
    font.virtual_masters
        .retain(|master| inside(&master.location, &[]));
    let removed = before - font.instances.len();
    if removed > 0 {
        warnings.push(ExportWarning {
            code: "designspace-instances-outside-subset",
            message: format!(
                "{removed} named instances fall outside the retained design space and were not written"
            ),
        });
    }
    // A conditional layer whose axes no longer exist would apply
    // unconditionally, and one that still has a bounded axis keeps only the
    // ranges on the axes that survived.
    let mut conditional_dropped = 0usize;
    let mut conditions_trimmed = 0usize;
    for glyph in font.glyphs.values_mut() {
        for layer in &mut glyph.layers {
            let (LayerKind::Bracket { master, .. }, Some(conditions)) =
                (&layer.kind, layer.kind.bracket_conditions())
            else {
                continue;
            };
            let master = master.clone();
            let kept: Vec<_> = conditions
                .iter()
                .copied()
                .filter(|condition| retained.contains(&condition.axis))
                .collect();
            if kept.len() == conditions.len() {
                continue;
            }
            conditions_trimmed += conditions.len() - kept.len();
            match LayerKind::from_bracket_conditions(master, &kept) {
                Ok(kind) => layer.kind = kind,
                // No bounded axis survives: the layer would apply everywhere.
                Err(_) => layer.kind = LayerKind::Custom("dropped-alternate".into()),
            }
        }
        glyph.layers.retain(|layer| match &layer.kind {
            LayerKind::Custom(name) if name == "dropped-alternate" => {
                conditional_dropped += 1;
                false
            }
            LayerKind::Brace { location, .. } => {
                let keep = inside(location, &dropped_defaults);
                if !keep {
                    conditional_dropped += 1;
                }
                keep
            }
            _ => true,
        });
        for layer in &mut glyph.layers {
            if let LayerKind::Brace { location, .. } = &mut layer.kind {
                trim(location);
            }
        }
    }
    if conditions_trimmed > 0 {
        warnings.push(ExportWarning {
            code: "designspace-conditions-trimmed",
            message: format!(
                "{conditions_trimmed} alternate-layer condition ranges named an axis this export configuration dropped and were removed"
            ),
        });
    }
    if conditional_dropped > 0 {
        warnings.push(ExportWarning {
            code: "designspace-conditional-layers-dropped",
            message: format!(
                "{conditional_dropped} intermediate or alternate layers lie outside the retained design space and were not compiled"
            ),
        });
    }
    if font.design_default_master().is_err() {
        let axis = font
            .axes
            .iter()
            .find(|axis| {
                !font.masters.iter().any(|master| {
                    master
                        .location
                        .get(&axis.tag)
                        .copied()
                        .unwrap_or_else(|| axis.user_to_design(axis.default))
                        == axis.user_to_design(axis.default)
                })
            })
            .map_or_else(
                || "the retained axes".to_owned(),
                |axis| axis.tag.to_string(),
            );
        return Err(Error::InvalidSource(format!(
            "the export configuration keeps no master at the default value of {axis}; include the default master"
        )));
    }
    font.validate_designspace()
        .map_err(|e| Error::InvalidSource(e.to_string()))?;
    Ok(warnings)
}

/// Rebuild one axis over `[low, high]` design units, keeping the mapping exact.
fn clamped_axis(axis: &Axis, low: f64, high: f64) -> Axis {
    let mut result = axis.clone();
    let min = axis.design_to_user(low);
    let max = axis.design_to_user(high);
    result.min = min;
    result.max = max;
    result.default = axis.default.clamp(min, max);
    if !axis.map.is_empty() {
        let mut map = vec![(min, low)];
        map.extend(
            axis.map
                .iter()
                .copied()
                .filter(|(user, _)| *user > min && *user < max),
        );
        map.push((max, high));
        result.map = map;
    }
    result
}
