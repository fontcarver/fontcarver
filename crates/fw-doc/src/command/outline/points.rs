use std::collections::HashSet;

use fw_model::{Contour, Font, GlyphName, LayerId, PointKind};
use kurbo::{Affine, Point as KurboPoint};

use crate::Error;

use super::super::{
    apply::{checked_indices, ensure_finite, layer_mut},
    HandleMode, PointRef,
};
use super::{adjacent_indices, validate_points, Touched};

/// Translates the listed points, optionally carrying unselected handles.
///
/// [`HandleMode::PointsOnly`] moves exactly the listed stored points, on- or
/// off-curve. [`HandleMode::WithHandles`] additionally moves the unselected
/// off-curve neighbours of every listed on-curve node, so dragging a node
/// carries its handles; a listed off-curve point simply moves.
pub(super) fn move_points(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    requested: &[PointRef],
    delta: (f64, f64),
    handles: HandleMode,
) -> Result<Touched, Error> {
    ensure_finite(&[delta.0, delta.1], "point delta")?;
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, requested)?;
    let selected: HashSet<_> = requested.iter().copied().collect();
    let mut moved = selected.clone();
    if handles == HandleMode::WithHandles {
        for point_ref in &selected {
            let contour = &target.contours[point_ref.contour];
            if contour.points[point_ref.point].kind == PointKind::OffCurve {
                continue;
            }
            for adjacent in adjacent_indices(contour, point_ref.point) {
                let adjacent_ref = PointRef {
                    contour: point_ref.contour,
                    point: adjacent,
                };
                if contour.points[adjacent].kind == PointKind::OffCurve
                    && !selected.contains(&adjacent_ref)
                {
                    moved.insert(adjacent_ref);
                }
            }
        }
    }
    for point_ref in moved {
        let point = &mut target.contours[point_ref.contour].points[point_ref.point];
        point.x += delta.0;
        point.y += delta.1;
    }
    Ok(Touched::of_points(requested))
}

/// Sets one stored point's absolute position.
pub(super) fn set_position(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    point_ref: PointRef,
    x: f64,
    y: f64,
) -> Result<Touched, Error> {
    ensure_finite(&[x, y], "point position")?;
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, &[point_ref])?;
    let point = &mut target.contours[point_ref.contour].points[point_ref.point];
    point.x = x;
    point.y = y;
    Ok(Touched::one(point_ref.contour))
}

/// Sets the smooth flag of the listed nodes and optionally levels their handles.
pub(super) fn set_smooth(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    requested: &[PointRef],
    smooth: bool,
    level_handles: bool,
) -> Result<Touched, Error> {
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, requested)?;
    let mut points = requested
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    points.sort_by_key(|point| (point.contour, point.point));
    for point_ref in points {
        let contour = &mut target.contours[point_ref.contour];
        contour.points[point_ref.point].smooth = smooth;
        if level_handles {
            level_node_handles(contour, point_ref.point);
        }
    }
    Ok(Touched::of_points(requested))
}

fn level_node_handles(contour: &mut Contour, node: usize) {
    let adjacent = adjacent_indices(contour, node);
    if adjacent.len() != 2
        || adjacent
            .iter()
            .any(|&index| contour.points[index].kind != PointKind::OffCurve)
    {
        return;
    }
    let before = adjacent[0];
    let after = adjacent[1];
    let center = KurboPoint::new(contour.points[node].x, contour.points[node].y);
    let incoming = center - KurboPoint::new(contour.points[before].x, contour.points[before].y);
    let outgoing = KurboPoint::new(contour.points[after].x, contour.points[after].y) - center;
    let incoming_length = incoming.hypot();
    let outgoing_length = outgoing.hypot();
    if incoming_length <= f64::EPSILON || outgoing_length <= f64::EPSILON {
        return;
    }
    let sum = incoming / incoming_length + outgoing / outgoing_length;
    let tangent = if sum.hypot() <= f64::EPSILON {
        incoming / incoming_length
    } else {
        sum / sum.hypot()
    };
    let leveled_before = center - tangent * incoming_length;
    let leveled_after = center + tangent * outgoing_length;
    contour.points[before].x = leveled_before.x;
    contour.points[before].y = leveled_before.y;
    contour.points[after].x = leveled_after.x;
    contour.points[after].y = leveled_after.y;
}

/// Transforms explicit selections, or the whole layer when all lists are empty.
pub(super) fn transform_selection(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    points: &[PointRef],
    components: &[usize],
    anchors: &[usize],
    coefficients: [f64; 6],
) -> Result<Touched, Error> {
    ensure_finite(&coefficients, "affine transform")?;
    let affine = Affine::new(coefficients);
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, points)?;
    let components = checked_indices(components, target.components.len(), "component")?;
    let anchors = checked_indices(anchors, target.anchors.len(), "anchor")?;
    if points.is_empty() && components.is_empty() && anchors.is_empty() {
        target.contours = fw_geom::transform(&target.contours, affine);
        for component in &mut target.components {
            component.transform = affine * component.transform;
        }
        for anchor in &mut target.anchors {
            let transformed = affine * KurboPoint::new(anchor.x, anchor.y);
            anchor.x = transformed.x;
            anchor.y = transformed.y;
        }
        return Ok(Touched::Layer);
    }
    let transformed = fw_geom::transform(&target.contours, affine);
    for point in points.iter().copied().collect::<HashSet<_>>() {
        target.contours[point.contour].points[point.point].x =
            transformed[point.contour].points[point.point].x;
        target.contours[point.contour].points[point.point].y =
            transformed[point.contour].points[point.point].y;
    }
    for index in components {
        target.components[index].transform = affine * target.components[index].transform;
    }
    for index in anchors {
        let transformed =
            affine * KurboPoint::new(target.anchors[index].x, target.anchors[index].y);
        target.anchors[index].x = transformed.x;
        target.anchors[index].y = transformed.y;
    }
    Ok(Touched::of_points(points))
}
