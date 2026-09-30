use fc_model::{Font, GlyphName, LayerId, LayerKind, MasterId};
use kurbo::{Affine, Shape, Vec2};

use crate::Error;

use super::apply::{ensure_finite, layer_mut};

/// Resolve and synchronize the requested master layers as one command.
pub(super) fn update(
    font: &mut Font,
    glyph: &GlyphName,
    master: Option<&MasterId>,
) -> Result<(), Error> {
    let data = font
        .glyph(glyph)
        .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
    let layers = data
        .layers
        .iter()
        .filter_map(|layer| match &layer.kind {
            LayerKind::Master(id) if master.is_none_or(|master| master == id) => {
                Some((layer.id.clone(), id.clone()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if layers.is_empty() {
        return Err(Error::InvalidCommand(
            "glyph has no matching master layer".into(),
        ));
    }
    // Resolve every requested layer before mutating any of them.
    let mut updates = Vec::new();
    for (layer, master) in layers {
        let resolved = crate::metrics::resolve_layer(font, glyph, &master, &layer)?;
        if resolved.source == crate::MetricsSource::Explicit {
            continue;
        }
        let path = fc_geom::flatten_layer_by_id(font, glyph, &layer)?;
        let ink = if path.is_empty() {
            0.0
        } else {
            path.bounding_box().width()
        };
        if (resolved.lsb + ink + resolved.rsb - resolved.width).abs() > 1e-7 {
            return Err(Error::InvalidCommand(
                "metrics keys specify conflicting sidebearings and width".into(),
            ));
        }
        if path.is_empty() && resolved.lsb.abs() > 1e-7 {
            return Err(Error::InvalidCommand(
                "an empty layer cannot have a nonzero left sidebearing".into(),
            ));
        }
        if !(0.0..=65535.0).contains(&resolved.width) {
            return Err(Error::InvalidCommand(
                "resolved metrics width exceeds the OpenType range".into(),
            ));
        }
        updates.push((layer, resolved));
    }
    for (layer, resolved) in updates {
        set(
            font,
            glyph,
            &layer,
            None,
            Some(resolved.lsb),
            Some(resolved.rsb),
        )?;
    }
    Ok(())
}

pub(super) fn center(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    width: f64,
) -> Result<(), Error> {
    if !width.is_finite() || !(0.0..=65535.0).contains(&width) {
        return Err(Error::InvalidCommand(
            "horizontal advance exceeds the OpenType range".into(),
        ));
    }
    let path = fc_geom::flatten_layer_by_id(font, glyph, layer)?;
    if path.elements().is_empty() {
        return set(font, glyph, layer, Some(width), None, None);
    }
    let side = (width - path.bounding_box().width()) / 2.0;
    set(font, glyph, layer, None, Some(side), Some(side))
}

pub(super) fn set(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    width: Option<f64>,
    lsb: Option<f64>,
    rsb: Option<f64>,
) -> Result<(), Error> {
    let values = [width, lsb, rsb].into_iter().flatten().collect::<Vec<_>>();
    ensure_finite(&values, "metrics")?;
    let current_width = layer_mut(font, glyph, layer)?.width;
    // Sidebearings follow the bounds of the command's own layer, not the
    // master layer, so background and brace layers keep their own metrics.
    let bounds = if lsb.is_some() || rsb.is_some() {
        let flattened = fc_geom::flatten_layer_by_id(font, glyph, layer)?;
        (!flattened.elements().is_empty()).then(|| flattened.bounding_box())
    } else {
        None
    };
    let mut final_width = width.unwrap_or(current_width);
    let delta = match (lsb, bounds) {
        (Some(lsb), Some(bounds)) => {
            let delta = lsb - bounds.x0;
            final_width += delta;
            delta
        }
        (Some(lsb), None) => {
            final_width += lsb;
            0.0
        }
        (None, _) => 0.0,
    };
    if let Some(rsb) = rsb {
        final_width = bounds.map_or(rsb, |bounds| bounds.x1 + delta + rsb);
    }

    ensure_finite(&[final_width, delta], "resolved metrics")?;
    let target = layer_mut(font, glyph, layer)?;
    let mut proposed = target.clone();
    if delta != 0.0 {
        let translation = Affine::translate(Vec2::new(delta, 0.0));
        proposed.contours = fc_geom::transform(&proposed.contours, translation);
        for entry in &mut proposed.effects {
            if let fc_model::LayerEffectKind::Boolean { path, .. } = &mut entry.effect {
                *path = (translation * fc_model::validate_effect_mask(path)?).to_svg();
            }
        }
        proposed.validate_effects()?;
        for component in &mut proposed.components {
            component.transform = translation * component.transform;
        }
        for anchor in &mut proposed.anchors {
            anchor.x += delta;
        }
    }
    proposed.width = final_width;
    *target = proposed;
    Ok(())
}

/// Store optional vertical overrides without changing the outline.
pub(super) fn set_vertical(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    advance: Option<f64>,
    origin: Option<f64>,
) -> Result<(), Error> {
    if advance.is_some_and(|v| !v.is_finite() || !(0.0..=65535.0).contains(&v))
        || origin.is_some_and(|v| !v.is_finite() || !(-32768.0..=32767.0).contains(&v))
    {
        return Err(Error::InvalidCommand(
            "vertical metrics exceed the OpenType range".into(),
        ));
    }
    let layer = layer_mut(font, glyph, layer)?;
    layer.vertical_width = advance;
    layer.vertical_origin = origin;
    Ok(())
}
