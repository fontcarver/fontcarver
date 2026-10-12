//! Resolve local smart parameters before global interpolation or font compilation.
use crate::Error;
use fc_model::{Contour, Font, Layer, LayerKind};

/// Replace direct smart-component uses with their evaluated source-master geometry.
pub fn resolve_smart_layer(font: &Font, layer: &Layer) -> Result<Layer, Error> {
    let master = match &layer.kind {
        LayerKind::Master(id)
        | LayerKind::Background(id)
        | LayerKind::Brace { master: id, .. }
        | LayerKind::Bracket { master: id, .. }
        | LayerKind::Smart { master: id, .. }
        | LayerKind::Backup { of: id, .. } => id,
        LayerKind::Custom(_) => &font.default_master().id,
    };
    let mut result = layer.clone();
    let mut remaining = Vec::new();
    let mut budget = 1_000_000usize;
    for component in &layer.components {
        let base = font
            .glyph(&component.base)
            .ok_or_else(|| Error::DesignSpace(format!("missing component {}", component.base)))?;
        if base.smart_axes.is_empty() && component.smart.as_ref().is_none_or(|v| v.is_empty()) {
            remaining.push(component.clone());
            continue;
        }
        let path = crate::flatten_smart_component(
            font,
            component.base.as_ref(),
            master,
            component.smart.as_ref().unwrap_or(&Default::default()),
        )
        .map_err(|e| Error::DesignSpace(e.to_string()))?;
        budget = budget
            .checked_sub(path.elements().len())
            .ok_or_else(|| Error::DesignSpace("smart decomposition work limit".into()))?;
        let transformed = component.transform * path;
        if !transformed.is_finite() {
            return Err(Error::DesignSpace(
                "smart component transform overflows outline coordinates".into(),
            ));
        }
        result.contours.extend(Contour::from_bezpath(&transformed));
        result.tt_instructions = None;
        result.hints = Default::default();
    }
    result.components = remaining;
    Ok(result)
}

/// Prepare export source layers using the original local axes, including nested uses.
/// Non-export drawing layers and local sources are omitted from the compilation copy.
pub fn resolve_smart_font(font: &Font) -> Result<Font, Error> {
    let mut result = font.clone();
    for glyph in result.glyphs.values_mut() {
        glyph.layers.retain(|layer| {
            matches!(
                layer.kind,
                LayerKind::Master(_) | LayerKind::Brace { .. } | LayerKind::Bracket { .. }
            )
        });
        for layer in &mut glyph.layers {
            *layer = resolve_smart_layer(font, layer)?;
        }
        glyph.smart_axes.clear();
    }
    Ok(result)
}
