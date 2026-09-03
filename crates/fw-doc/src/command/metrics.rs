use fw_model::{Font, GlyphName, LayerId, LayerKind, MasterId};
use kurbo::{Affine, Shape, Vec2};

use crate::Error;

use super::apply::{ensure_finite, layer_mut};

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
    let master = layer_master(font, glyph, layer)?;

    let bounds = if lsb.is_some() || rsb.is_some() {
        let flattened = fw_model::query::flatten_layer(font, glyph, &master)?;
        (!flattened.elements().is_empty()).then(|| flattened.bounding_box())
    } else {
        None
    };
    let current_width = layer_mut(font, glyph, layer)?.width;
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

    let target = layer_mut(font, glyph, layer)?;
    if delta != 0.0 {
        let translation = Affine::translate(Vec2::new(delta, 0.0));
        target.contours = fw_geom::transform(&target.contours, translation);
        for component in &mut target.components {
            component.transform = translation * component.transform;
        }
        for anchor in &mut target.anchors {
            anchor.x += delta;
        }
    }
    target.width = final_width;
    Ok(())
}

fn layer_master(font: &Font, glyph: &GlyphName, layer: &LayerId) -> Result<MasterId, Error> {
    let glyph_data = font
        .glyph(glyph)
        .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
    let layer_data = glyph_data.layer(layer).ok_or_else(|| Error::MissingLayer {
        glyph: glyph.clone(),
        layer: layer.clone(),
    })?;
    Ok(match &layer_data.kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. } => master.clone(),
        LayerKind::Backup { of, .. } => of.clone(),
        LayerKind::Custom(_) => font.default_master().id.clone(),
    })
}
