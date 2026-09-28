use std::collections::BTreeSet;

use fc_model::{Contour, Font, GlyphName, LayerId, Point, PointKind};

use crate::Error;

use super::super::{
    apply::{checked_indices, ensure_finite, layer_mut, remove_indices},
    CurveKind, PathOp, PointRef, SegmentKind,
};
use super::{
    cyclic_between, index_error, layer_curve_kind, on_curve_indices, repair_kinds, validate_points,
    Touched,
};

/// Appends a contour to the layer.
pub(super) fn add_contour(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contour: &Contour,
) -> Result<Touched, Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    contours.push(contour.clone());
    Ok(Touched::one(contours.len().saturating_sub(1)))
}

/// Removes the listed contours.
pub(super) fn delete_contours(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contours: &[usize],
) -> Result<Touched, Error> {
    let target = &mut layer_mut(font, glyph, layer)?.contours;
    let indices = checked_indices(contours, target.len(), "contour")?;
    remove_indices(target, &indices);
    Ok(Touched::none())
}

/// Reverses the direction of the listed contours.
pub(super) fn reverse_contours(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contours: &[usize],
) -> Result<Touched, Error> {
    let target = &mut layer_mut(font, glyph, layer)?.contours;
    let indices = checked_indices(contours, target.len(), "contour")?;
    for &index in &indices {
        target[index] = fc_geom::reverse(&target[index]);
    }
    Ok(Touched::Contours(indices.into_iter().collect()))
}

/// Splits one geometric segment and inserts the resulting on-curve point.
pub(super) fn insert(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contour_index: usize,
    segment: usize,
    t: f64,
) -> Result<Touched, Error> {
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
    *contour = fc_geom::split_segment(contour, segment, t);
    Ok(Touched::one(contour_index))
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
) -> Result<Touched, Error> {
    let quadratic = matches!(
        layer_curve_kind(font, glyph, layer),
        CurveKind::Quadratic { .. }
    );
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
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
    let (start, end) = fc_geom::segment_to_points(contour, segment);
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
                return Ok(Touched::none());
            }
            (PointKind::Line, Vec::new())
        }
        SegmentKind::Curve if quadratic => {
            if controls.len() == 1 && endpoint_kind == PointKind::QCurve {
                return Ok(Touched::none());
            }
            let from = contour.points[start].clone();
            let to = contour.points[end].clone();
            (PointKind::QCurve, vec![point_between(&from, &to, 0.5)])
        }
        SegmentKind::Curve => {
            if controls.len() == 2 && endpoint_kind == PointKind::Curve {
                return Ok(Touched::none());
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
    Ok(Touched::one(contour_index))
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
) -> Result<Touched, Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    validate_points(contours, &[point])?;
    let contour = &mut contours[point.contour];
    if !contour.closed || contour.points[point.point].kind == PointKind::OffCurve {
        return Err(Error::InvalidCommand(
            "start point must be an on-curve point of a closed contour".to_owned(),
        ));
    }
    contour.points.rotate_left(point.point);
    Ok(Touched::one(point.contour))
}

/// Joins the end of one open contour to an endpoint of another.
pub(super) fn join(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    a: PointRef,
    b: PointRef,
) -> Result<Touched, Error> {
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
        second = fc_geom::reverse(&second);
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
    Ok(Touched::one(keep_index))
}

/// Opens a closed contour, or splits an open contour into two.
pub(super) fn split(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    point: PointRef,
) -> Result<Touched, Error> {
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
        return Ok(Touched::one(point.contour));
    }
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
    Ok(Touched::Contours(BTreeSet::from([
        point.contour,
        point.contour + 1,
    ])))
}

/// Applies one whole-layer geometry operation.
pub(super) fn path_op(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    op: &PathOp,
) -> Result<Touched, Error> {
    let layer_kind = layer_curve_kind(font, glyph, layer);
    let target = layer_mut(font, glyph, layer)?;
    if target.has_smart_corners()
        && matches!(
            op,
            PathOp::RemoveOverlap
                | PathOp::Simplify(_)
                | PathOp::ExpandStroke(_)
                | PathOp::ConvertCurves(_)
        )
    {
        return Err(Error::InvalidCommand(
            "bake or remove smart corners before this path operation".into(),
        ));
    }
    let contours = &target.contours;
    let result = match op {
        PathOp::RemoveOverlap => fc_geom::remove_overlap(
            contours,
            layer_kind.geometry(),
            fc_geom::DEFAULT_BOOLEAN_TOLERANCE,
        )
        .map_err(|error| Error::Geometry(error.to_string()))?,
        PathOp::CorrectDirection => fc_geom::correct_direction(contours, true),
        PathOp::AddExtrema => fc_geom::add_extrema(contours),
        PathOp::AddInflections => fc_geom::add_inflections(contours),
        PathOp::Simplify(options) => fc_geom::simplify(contours, &options.geometry()),
        PathOp::RoundToGrid(step) => {
            if !step.is_finite() || *step <= 0.0 {
                return Err(Error::InvalidCommand(
                    "grid step must be positive and finite".to_owned(),
                ));
            }
            fc_geom::round_to_grid(contours, *step)
        }
        PathOp::CanonicalStart => fc_geom::canonical_start_points(contours),
        PathOp::CanonicalOrder => fc_geom::canonical_order(contours),
        PathOp::ExpandStroke(options) => {
            fc_geom::expand_stroke(contours, &options.geometry(), layer_kind.geometry())
                .map_err(|error| Error::Geometry(error.to_string()))?
        }
        PathOp::ConvertCurves(kind) => {
            if matches!(kind, CurveKind::Quadratic { max_err } if !max_err.is_finite() || *max_err <= 0.0)
            {
                return Err(Error::InvalidCommand(
                    "quadratic conversion error must be positive and finite".to_owned(),
                ));
            }
            fc_geom::from_bezpaths(&fc_geom::to_bezpaths(contours), kind.geometry())
        }
    };
    target.contours = result;
    Ok(Touched::Layer)
}

fn same_position(a: Option<&Point>, b: Option<&Point>) -> bool {
    a.zip(b).is_some_and(|(a, b)| a.x == b.x && a.y == b.y)
}
