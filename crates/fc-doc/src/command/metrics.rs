use fc_model::{Font, GlyphName, LayerId};
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
    let current_width = layer_mut(font, glyph, layer)?.width;
    // Sidebearings follow the bounds of the command's own layer, not the
    // master layer, so background and brace layers keep their own metrics.
    let bounds = if lsb.is_some() || rsb.is_some() {
        let flattened = fc_model::query::flatten_layer_by_id(font, glyph, layer)?;
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

    let target = layer_mut(font, glyph, layer)?;
    if delta != 0.0 {
        let translation = Affine::translate(Vec2::new(delta, 0.0));
        target.contours = fc_geom::transform(&target.contours, translation);
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
