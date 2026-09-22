//! Conservative compatibility repair: contour order, direction and start points.
use crate::{
    canonical_order, canonical_start_points, compatible, correct_direction, reverse, Error,
};
use fc_model::{Contour, Layer};
use kurbo::Shape;

/// Canonicalize source ordering without adding, removing or moving geometry.
pub fn canonical_source(layer: &Layer) -> Layer {
    let mut result = layer.clone();
    result.contours =
        canonical_start_points(&canonical_order(&correct_direction(&layer.contours, true)));
    result.components.sort_by(|a, b| a.base.cmp(&b.base));
    clear_point_hints(&mut result);
    result
}

/// Align a source to a reference by reordering and rotating matching contours.
/// Geometry is preserved. Unmatched topology fails instead of inventing points.
pub fn repair_compatibility(reference: &Layer, source: &Layer) -> Result<Layer, Error> {
    if reference.contours.len() != source.contours.len() {
        return Err(Error::DesignSpace(
            "contour counts differ; repair topology manually".into(),
        ));
    }
    let mut result = source.clone();
    let mut remaining = source.contours.clone();
    let mut output = Vec::new();
    let mut budget = 2_000_000usize;
    let bounds = |layer: &Layer| {
        layer
            .contours
            .iter()
            .map(|c| c.to_bezpath().bounding_box())
            .reduce(|a, b| a.union(b))
            .unwrap_or(kurbo::Rect::ZERO)
    };
    let left = bounds(reference);
    let right = bounds(source);
    for contour in &reference.contours {
        let mut best: Option<(usize, Contour, f64)> = None;
        for (index, candidate) in remaining.iter().enumerate() {
            if candidate.closed != contour.closed || candidate.points.len() != contour.points.len()
            {
                continue;
            }
            for mut candidate in [candidate.clone(), reverse(candidate)] {
                let rotations = if candidate.closed {
                    candidate.points.len().max(1)
                } else {
                    1
                };
                for _ in 0..rotations {
                    budget = budget
                        .checked_sub(candidate.points.len().max(1))
                        .ok_or_else(|| {
                            Error::DesignSpace("compatibility repair exceeds work limit".into())
                        })?;
                    if candidate
                        .points
                        .iter()
                        .zip(&contour.points)
                        .all(|(a, b)| a.kind == b.kind)
                    {
                        let score = candidate
                            .points
                            .iter()
                            .zip(&contour.points)
                            .map(|(a, b)| {
                                let x = (a.x - right.x0) / right.width().max(1.0)
                                    - (b.x - left.x0) / left.width().max(1.0);
                                let y = (a.y - right.y0) / right.height().max(1.0)
                                    - (b.y - left.y0) / left.height().max(1.0);
                                x * x + y * y
                            })
                            .sum::<f64>();
                        if best.as_ref().is_none_or(|(_, _, old)| score < *old) {
                            best = Some((index, candidate.clone(), score));
                        }
                    }
                    if !candidate.points.is_empty() {
                        candidate.points.rotate_left(1);
                    }
                }
            }
        }
        let (index, contour, _) = best.ok_or_else(|| {
            Error::DesignSpace(
                "point counts or kinds cannot be aligned without changing geometry".into(),
            )
        })?;
        remaining.remove(index);
        output.push(contour);
    }
    result.contours = output;
    result.components.sort_by(|a, b| a.base.cmp(&b.base));
    compatible(reference, &result).map_err(Error::Incompatible)?;
    clear_point_hints(&mut result);
    Ok(result)
}

fn clear_point_hints(layer: &mut Layer) {
    layer.tt_instructions = None;
    layer.hints.hint_masks.clear();
    for point in layer
        .contours
        .iter_mut()
        .flat_map(|contour| &mut contour.points)
    {
        point.tt_index = fc_model::TtIndex::Unnumbered;
    }
}
