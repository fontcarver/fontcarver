//! Resolve a static design-space instance before compiling its outlines and layout.
use std::collections::{BTreeSet, HashMap};

use fc_geom::variation::{interpolate_glyph, normalized_location};
use fc_model::{DesignLocation, Font, Instance, Kerning, LayerKind, MasterMetrics};
use fontdrasil::{
    types::Tag,
    variations::{RoundingBehaviour, VariationModel},
};

use crate::Error;

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::InvalidSource(error.to_string())
}

/// Create an independent, single-master font at a checked design location.
/// Conditional glyphs, anchors, advances, master metrics and kerning are resolved.
/// Point-indexed programs are discarded because interpolation changes their input.
pub fn instantiate(
    font: &Font,
    location: &DesignLocation,
    instance: Option<&Instance>,
) -> Result<Font, Error> {
    crate::variable::validate_export(font)?;
    let location = font.complete_location(location).map_err(invalid)?;
    let mut result = font.clone();
    let mut master = font.design_default_master().map_err(invalid)?.clone();
    let source_locations: Vec<_> = font
        .masters
        .iter()
        .map(|m| normalized_location(&font.axes, &m.location))
        .collect();
    let model = VariationModel::new(
        source_locations.iter().cloned().collect(),
        font.axes.iter().map(|a| Tag::new(&a.tag.0)).collect(),
    );
    let at = normalized_location(&font.axes, &location);
    let scalar = |values: Vec<f64>| -> Result<f64, Error> {
        if values.iter().any(|n| !n.is_finite()) {
            return Err(invalid("non-finite instance metric"));
        }
        let values: HashMap<_, _> = source_locations
            .iter()
            .cloned()
            .zip(values.into_iter().map(|n| vec![n]))
            .collect();
        let deltas = model
            .deltas_with_rounding::<f64, f64>(&values, RoundingBehaviour::None)
            .map_err(invalid)?;
        let value = model
            .interpolate_from_deltas(&at, &deltas)
            .first()
            .copied()
            .ok_or_else(|| invalid("missing interpolated metric"))?;
        if !value.is_finite() {
            return Err(invalid("non-finite interpolated metric"));
        }
        Ok(value)
    };
    let mut metrics = MasterMetrics::default();
    macro_rules! required {
        ($($field:ident),+ $(,)?) => { $(metrics.$field = scalar(font.masters.iter().map(|m| m.metrics.$field).collect())?;)+ };
    }
    required!(ascender, descender, x_height, cap_height, italic_angle);
    let defaults: Vec<_> = font
        .masters
        .iter()
        .map(|m| crate::instance_metrics::defaults(&m.metrics, font.info.units_per_em))
        .collect();
    macro_rules! optional {
        ($($field:ident),+ $(,)?) => { $(
            if font.masters.iter().any(|m| m.metrics.$field.is_some()) {
                let values = defaults.iter().map(|m| m.$field.ok_or_else(|| invalid("missing resolved metric"))).collect::<Result<Vec<_>,_>>()?;
                metrics.$field = Some(scalar(values)?);
            }
        )+ };
    }
    optional!(
        hhea_ascender,
        hhea_descender,
        hhea_line_gap,
        typo_ascender,
        typo_descender,
        typo_line_gap,
        win_ascent,
        win_descent,
        vhea_ascender,
        vhea_descender,
        vhea_line_gap,
        default_vertical_origin,
        default_vertical_advance
    );
    master.metrics = metrics;
    for vertical in [false, true] {
        let sources: Vec<_> = font
            .masters
            .iter()
            .map(|m| if vertical { &m.kerning_v } else { &m.kerning_h })
            .collect();
        let keys: BTreeSet<_> = sources.iter().flat_map(|k| k.0.keys().cloned()).collect();
        if keys.len().saturating_mul(sources.len()) > 8_000_000 {
            return Err(invalid("instance kerning work limit"));
        }
        let pairs = keys
            .into_iter()
            .map(|key| {
                let value = scalar(
                    sources
                        .iter()
                        .map(|k| k.0.get(&key).copied().unwrap_or(0.))
                        .collect(),
                )?;
                Ok((key, value))
            })
            .collect::<Result<_, Error>>()?;
        if vertical {
            master.kerning_v = Kerning(pairs);
        } else {
            master.kerning_h = Kerning(pairs);
        }
    }
    master.tt = Default::default();
    master.location.clear();
    for glyph in result.glyphs.values_mut() {
        let mut layer = interpolate_glyph(font, glyph, &location)
            .map_err(|e| invalid(format!("{}: {e}", glyph.name)))?;
        layer.kind = LayerKind::Master(master.id.clone());
        layer.tt_instructions = None;
        layer.tt_binding = None;
        layer.hints = Default::default();
        for point in layer.contours.iter_mut().flat_map(|c| &mut c.points) {
            point.tt_index = fc_model::TtIndex::Unnumbered;
        }
        glyph.layers = vec![layer];
        glyph.smart_axes.clear();
    }
    if let Some(instance) = instance {
        result.info.style_name = if instance.style_name.trim().is_empty() {
            instance.name.clone()
        } else {
            instance.style_name.clone()
        };
        if let Some(family) = &instance.family_name {
            result.info.family_name = family.clone();
        }
        master.name = result.info.style_name.clone();
        result
            .info
            .names
            .retain(|record| ![1, 2, 3, 4, 6, 16, 17, 18, 21, 22, 25].contains(&record.name_id));
        result.info.postscript_font_name = None;
        result.info.postscript_full_name = None;
        result.info.open_type_name_unique_id = None;
        result.info.open_type_name_preferred_family_name = None;
        result.info.open_type_name_preferred_subfamily_name = None;
        result.info.open_type_name_compatible_full_name = None;
        result.info.open_type_name_wws_family_name = None;
        result.info.open_type_name_wws_subfamily_name = None;
        instance.overrides.validate().map_err(invalid)?;
    }
    crate::instance_metrics::apply_style(&mut result, &mut master.metrics, &location);
    if let Some(instance) = instance {
        // Export-scope overrides win over both the font's own values and the
        // classes `apply_style` derives from the instance location.
        let overrides = &instance.overrides;
        if let Some(weight) = overrides.weight_class {
            result.info.open_type_os2_weight_class = Some(weight);
        }
        if let Some(width) = overrides.width_class {
            result.info.open_type_os2_width_class = Some(width);
        }
        if let Some(name) = &overrides.postscript_font_name {
            result.info.postscript_font_name = Some(name.clone());
        }
        if let Some(name) = &overrides.preferred_family_name {
            result.info.open_type_name_preferred_family_name = Some(name.clone());
        }
        if let Some(name) = &overrides.preferred_subfamily_name {
            result.info.open_type_name_preferred_subfamily_name = Some(name.clone());
        }
    }
    master.metrics.validate().map_err(invalid)?;
    result.masters = vec![master];
    result.axes.clear();
    result.virtual_masters.clear();
    result.instances.clear();
    Ok(result)
}
