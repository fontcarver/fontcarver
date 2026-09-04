use fw_model::{Contour, Font, GlyphName, LayerId, Point, PointKind};

use crate::Error;

use super::super::{
    apply::{checked_indices, ensure_finite, layer_mut, remove_indices},
    CurveKind, PathOp, PointRef, SegmentKind,
};
use super::{
    cyclic_between, index_error, inferred_kind, on_curve_indices, repair_kinds, validate_points,
};

/// Appends a contour to the layer.
pub(super) fn add_contour(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contour: &Contour,
) -> Result<(), Error> {
    layer_mut(font, glyph, layer)?
        .contours
        .push(contour.clone());
    Ok(())
}

/// Removes the listed contours.
pub(super) fn delete_contours(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contours: &[usize],
) -> Result<(), Error> {
    let target = &mut layer_mut(font, glyph, layer)?.contours;
    let indices = checked_indices(contours, target.len(), "contour")?;
    remove_indices(target, &indices);
    Ok(())
}

/// Reverses the direction of the listed contours.
pub(super) fn reverse_contours(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contours: &[usize],
) -> Result<(), Error> {
    let target = &mut layer_mut(font, glyph, layer)?.contours;
    let indices = checked_indices(contours, target.len(), "contour")?;
    for index in indices {
        target[index] = fw_geom::reverse(&target[index]);
    }
    Ok(())
}

/// Splits one geometric segment and inserts the resulting on-curve point.
pub(super) fn insert(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contour_index: usize,
    segment: usize,
    t: f64,
) -> Result<(), Error> {
    ensure_finite(&[t], "split parameter")?;
    if !(0.0..1.0).contains(&t) || t == 0.0 {
        return Err(Error::InvalidCommand(
            "split parameter must be inside 0..1".to_owned(),
        ));
    }
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    let len = contours.len();
    let contour = contours
        .get_mut(contour_index)
        .ok_or_else(|| index_error("contour", contour_index, len))?;
    let count = contour.to_bezpath().segments().count();
    if segment >= count {
        return Err(index_error("segment", segment, count));
    }
    *contour = fw_geom::split_segment(contour, segment, t);
    Ok(())
}

/// Converts one segment between a straight line and a curve.
///
/// The contour start point is never rotated: the segment's stored span is
/// rewritten in place. A curve gains two handles at one and two thirds of the
/// chord on a cubic layer and a single handle at the chord midpoint on a
/// quadratic layer, so the user starts from a straight, draggable segment.
pub(super) fn convert(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contour_index: usize,
    segment: usize,
    to: SegmentKind,
) -> Result<(), Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    let quadratic = matches!(inferred_kind(contours), CurveKind::Quadratic { .. });
    let len = contours.len();
    let contour = contours
        .get_mut(contour_index)
        .ok_or_else(|| index_error("contour", contour_index, len))?;
    if on_curve_indices(contour).len() < 2 {
        return Err(Error::InvalidCommand(
            "ConvertSegment requires a contour with at least two on-curve points".to_owned(),
        ));
    }
    let count = contour.to_bezpath().segments().count();
    if segment >= count {
        return Err(index_error("segment", segment, count));
    }
    let points = contour.points.len();
    let (start, end) = fw_geom::segment_to_points(contour, segment);
    if start >= points || end >= points || start == end {
        return Err(index_error("segment", segment, count));
    }
    let controls: Vec<usize> = cyclic_between(points, start, end)
        .into_iter()
        .filter(|&index| contour.points[index].kind == PointKind::OffCurve)
        .collect();
    let endpoint_kind = contour.points[end].kind;
    let (kind, inserted) = match to {
        SegmentKind::Line => {
            if controls.is_empty() {
                return Ok(());
            }
            (PointKind::Line, Vec::new())
        }
        SegmentKind::Curve if quadratic => {
            if controls.len() == 1 && endpoint_kind == PointKind::QCurve {
                return Ok(());
            }
            let from = contour.points[start].clone();
            let to = contour.points[end].clone();
            (PointKind::QCurve, vec![point_between(&from, &to, 0.5)])
        }
        SegmentKind::Curve => {
            if controls.len() == 2 && endpoint_kind == PointKind::Curve {
                return Ok(());
            }
            let from = contour.points[start].clone();
            let to = contour.points[end].clone();
            (
                PointKind::Curve,
                vec![
                    point_between(&from, &to, 1.0 / 3.0),
                    point_between(&from, &to, 2.0 / 3.0),
                ],
            )
        }
    };
    let mut rebuilt = Vec::with_capacity(points + inserted.len());
    for index in 0..points {
        if controls.contains(&index) {
            continue;
        }
        let mut point = contour.points[index].clone();
        if index == end {
            point.kind = kind;
        }
        rebuilt.push(point);
        if index == start {
            rebuilt.extend(inserted.iter().cloned());
        }
    }
    contour.points = rebuilt;
    repair_kinds(contour);
    Ok(())
}

fn point_between(a: &Point, b: &Point, t: f64) -> Point {
    Point {
        x: (b.x - a.x).mul_add(t, a.x),
        y: (b.y - a.y).mul_add(t, a.y),
        kind: PointKind::OffCurve,
        ..Point::default()
    }
}

/// Rotates a closed contour to begin at the requested on-curve point.
pub(super) fn set_start(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    point: PointRef,
) -> Result<(), Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    validate_points(contours, &[point])?;
    let contour = &mut contours[point.contour];
    if !contour.closed || contour.points[point.point].kind == PointKind::OffCurve {
        return Err(Error::InvalidCommand(
            "start point must be an on-curve point of a closed contour".to_owned(),
        ));
    }
    contour.points.rotate_left(point.point);
    Ok(())
}

/// Joins the end of one open contour to an endpoint of another.
pub(super) fn join(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    a: PointRef,
    b: PointRef,
) -> Result<(), Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    validate_points(contours, &[a, b])?;
    if a.contour == b.contour {
        return Err(Error::InvalidCommand(
            "JoinContours requires two contours".to_owned(),
        ));
    }
    let first = &contours[a.contour];
    let second = &contours[b.contour];
    if first.closed || second.closed || a.point + 1 != first.points.len() {
        return Err(Error::InvalidCommand(
            "a must be the end of an open contour".to_owned(),
        ));
    }
    let mut second = second.clone();
    if b.point + 1 == second.points.len() {
        second = fw_geom::reverse(&second);
    } else if b.point != 0 {
        return Err(Error::InvalidCommand(
            "b must be an endpoint of an open contour".to_owned(),
        ));
    }
    if let Some(start) = second.points.first_mut() {
        start.kind = PointKind::Line;
    }
    let second_index = b.contour;
    let mut first = contours[a.contour].clone();
    if same_position(first.points.last(), second.points.first()) {
        second.points.remove(0);
    }
    first.points.extend(second.points);
    let keep_index = if second_index < a.contour {
        a.contour - 1
    } else {
        a.contour
    };
    contours.remove(second_index);
    contours[keep_index] = first;
    Ok(())
}

/// Opens a closed contour, or splits an open contour into two.
pub(super) fn split(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    point: PointRef,
) -> Result<(), Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    validate_points(contours, &[point])?;
    let contour = &mut contours[point.contour];
    if contour.points[point.point].kind == PointKind::OffCurve {
        return Err(Error::InvalidCommand(
            "split point must be on-curve".to_owned(),
        ));
    }
    if contour.closed {
        contour.points.rotate_left(point.point);
        contour.closed = false;
        while contour
            .points
            .last()
            .is_some_and(|point| point.kind == PointKind::OffCurve)
        {
            contour.points.pop();
        }
        repair_kinds(contour);
    } else {
        if point.point == 0 || point.point + 1 == contour.points.len() {
            return Err(Error::InvalidCommand(
                "open contour split must be an interior node".to_owned(),
            ));
        }
        let mut right = contour.clone();
        right.points = right.points[point.point..].to_vec();
        contour.points.truncate(point.point + 1);
        repair_kinds(contour);
        repair_kinds(&mut right);
        contours.insert(point.contour + 1, right);
    }
    Ok(())
}

/// Applies one whole-layer geometry operation.
pub(super) fn path_op(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    op: &PathOp,
) -> Result<(), Error> {
    let target = layer_mut(font, glyph, layer)?;
    let contours = &target.contours;
    let result = match op {
        PathOp::RemoveOverlap => fw_geom::remove_overlap(
            contours,
            inferred_kind(contours).geometry(),
            fw_geom::DEFAULT_BOOLEAN_TOLERANCE,
        )
        .map_err(|error| Error::Geometry(error.to_string()))?,
        PathOp::CorrectDirection => fw_geom::correct_direction(contours, true),
        PathOp::AddExtrema => fw_geom::add_extrema(contours),
        PathOp::AddInflections => fw_geom::add_inflections(contours),
        PathOp::Simplify(options) => fw_geom::simplify(contours, &options.geometry()),
        PathOp::RoundToGrid(step) => {
            if !step.is_finite() || *step <= 0.0 {
                return Err(Error::InvalidCommand(
                    "grid step must be positive and finite".to_owned(),
                ));
            }
            fw_geom::round_to_grid(contours, *step)
        }
        PathOp::CanonicalStart => fw_geom::canonical_start_points(contours),
        PathOp::CanonicalOrder => fw_geom::canonical_order(contours),
        PathOp::ExpandStroke(options) => fw_geom::expand_stroke(
            contours,
            &options.geometry(),
            inferred_kind(contours).geometry(),
        )
        .map_err(|error| Error::Geometry(error.to_string()))?,
        PathOp::ConvertCurves(kind) => {
            if matches!(kind, CurveKind::Quadratic { max_err } if !max_err.is_finite() || *max_err <= 0.0)
            {
                return Err(Error::InvalidCommand(
                    "quadratic conversion error must be positive and finite".to_owned(),
                ));
            }
            fw_geom::from_bezpaths(&fw_geom::to_bezpaths(contours), kind.geometry())
        }
    };
    target.contours = result;
    Ok(())
}

fn same_position(a: Option<&Point>, b: Option<&Point>) -> bool {
    a.zip(b).is_some_and(|(a, b)| a.x == b.x && a.y == b.y)
}
