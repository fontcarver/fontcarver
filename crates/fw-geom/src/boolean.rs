use std::panic::{catch_unwind, AssertUnwindSafe};

use flo_curves::{
    bezier::path::{
        path_add, path_intersect, path_remove_interior_points, path_sub, SimpleBezierPath,
    },
    geo::Coord2,
};
use fw_model::{Contour, PointKind};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use kurbo::{flatten, BezPath, PathEl, PathSeg, Point, Shape};

use crate::{correct_direction, from_bezpaths, to_bezpaths, CurveKind, Error, Tolerance};

/// A two-input filled-outline boolean operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    /// Combine the filled regions of both inputs.
    Union,
    /// Keep only the region filled by both inputs.
    Intersect,
    /// Subtract the second input from the first.
    Exclude,
    /// Keep regions filled by exactly one input.
    Xor,
}

/// Removes overlapping interior edges from a set of contours.
///
/// Open contours are not part of the filled boolean and are returned unchanged.
pub fn remove_overlap(
    contours: &[Contour],
    kind: CurveKind,
    tol: Tolerance,
) -> Result<Vec<Contour>, Error> {
    validate_tolerance(tol)?;
    let (closed, open): (Vec<_>, Vec<_>) = contours.iter().cloned().partition(|c| c.closed);
    if closed.is_empty() {
        return Ok(open);
    }
    let clean_paths: Vec<_> = to_bezpaths(&closed)
        .iter()
        .map(|path| clean_path(path, tol))
        .filter(|path| path.segments().next().is_some())
        .collect();
    let original_paths = to_bezpaths(&closed);
    if clean_paths.len() == original_paths.len()
        && clean_paths
            .iter()
            .zip(&original_paths)
            .all(|(left, right)| same_path_segments(left, right))
        && !has_boundary_intersections(&clean_paths)
    {
        validate_area(&clean_paths, &clean_paths)?;
        if contours_match_kind(&closed, kind) {
            return Ok(contours.to_vec());
        }
        let mut converted = postprocess(&clean_paths, kind, tol);
        converted.extend(open);
        return Ok(converted);
    }
    let flo_paths: Vec<_> = clean_paths.iter().filter_map(kurbo_to_flo).collect();
    if flo_paths.is_empty() {
        return Ok(open);
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        path_remove_interior_points::<SimpleBezierPath, SimpleBezierPath>(&flo_paths, tol)
    }))
    .map_err(|_| boolean_failure("flo_curves panicked while removing overlap"))?;
    let result_paths: Vec<_> = result
        .iter()
        .map(flo_to_kurbo)
        .map(|path| clean_path(&path, tol))
        .collect();
    validate_area(&clean_paths, &result_paths)?;

    let mut output = postprocess(&result_paths, kind, tol);
    if let Some(outer_clockwise) = dominant_orientation(&closed) {
        output = correct_direction(&output, outer_clockwise);
    }
    restore_unchanged_contours(&mut output, &closed);
    output.extend(open);
    Ok(output)
}

fn contours_match_kind(contours: &[Contour], kind: CurveKind) -> bool {
    match kind {
        CurveKind::Cubic => contours
            .iter()
            .flat_map(|contour| &contour.points)
            .all(|point| point.kind != PointKind::QCurve),
        CurveKind::Quadratic { .. } => contours
            .iter()
            .flat_map(|contour| &contour.points)
            .all(|point| point.kind != PointKind::Curve),
    }
}

/// Applies a filled-outline boolean operation to two contour sets.
///
/// Open contours do not define filled regions and are ignored.
pub fn boolean(
    a: &[Contour],
    b: &[Contour],
    op: BooleanOp,
    kind: CurveKind,
    tol: Tolerance,
) -> Result<Vec<Contour>, Error> {
    validate_tolerance(tol)?;
    let a_paths = closed_flo_paths(a, tol);
    let b_paths = closed_flo_paths(b, tol);
    match op {
        BooleanOp::Intersect if a_paths.is_empty() || b_paths.is_empty() => return Ok(Vec::new()),
        BooleanOp::Exclude if a_paths.is_empty() => return Ok(Vec::new()),
        BooleanOp::Union | BooleanOp::Xor if a_paths.is_empty() => {
            return remove_overlap(&closed_contours(b), kind, tol);
        }
        BooleanOp::Union | BooleanOp::Xor | BooleanOp::Exclude if b_paths.is_empty() => {
            return remove_overlap(&closed_contours(a), kind, tol);
        }
        BooleanOp::Union | BooleanOp::Intersect | BooleanOp::Exclude | BooleanOp::Xor => {}
    }
    let result = catch_unwind(AssertUnwindSafe(|| match op {
        BooleanOp::Union => path_add(&a_paths, &b_paths, tol),
        BooleanOp::Intersect => path_intersect(&a_paths, &b_paths, tol),
        BooleanOp::Exclude => path_sub(&a_paths, &b_paths, tol),
        BooleanOp::Xor => {
            let mut left: Vec<SimpleBezierPath> = path_sub(&a_paths, &b_paths, tol);
            let right: Vec<SimpleBezierPath> = path_sub(&b_paths, &a_paths, tol);
            left.extend(right);
            left
        }
    }))
    .map_err(|_| boolean_failure("flo_curves panicked during a boolean operation"))?;
    let paths: Vec<_> = result
        .iter()
        .map(flo_to_kurbo)
        .map(|path| clean_path(&path, tol))
        .collect();
    let mut output = postprocess(&paths, kind, tol);
    if let Some(outer_clockwise) = dominant_orientation(a).or_else(|| dominant_orientation(b)) {
        output = correct_direction(&output, outer_clockwise);
    }
    Ok(output)
}

fn closed_contours(contours: &[Contour]) -> Vec<Contour> {
    contours
        .iter()
        .filter(|contour| contour.closed)
        .cloned()
        .collect()
}

fn validate_tolerance(tol: f64) -> Result<(), Error> {
    if tol.is_finite() && tol > 0.0 {
        Ok(())
    } else {
        Err(Error::InvalidParameter {
            parameter: "tolerance",
        })
    }
}

fn boolean_failure(reason: &str) -> Error {
    tracing::warn!(reason, "curve boolean operation failed");
    Error::BooleanFailed {
        reason: reason.to_owned(),
    }
}

fn closed_flo_paths(contours: &[Contour], tol: f64) -> Vec<SimpleBezierPath> {
    to_bezpaths(contours)
        .iter()
        .zip(contours)
        .filter(|(_, contour)| contour.closed)
        .map(|(path, _)| clean_path(path, tol))
        .filter_map(|path| kurbo_to_flo(&path))
        .collect()
}

fn kurbo_to_flo(path: &BezPath) -> Option<SimpleBezierPath> {
    let mut start = None;
    let mut current = None;
    let mut segments = Vec::new();
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => {
                if start.is_none() {
                    start = Some(point);
                    current = Some(point);
                }
            }
            PathEl::LineTo(point) => {
                if let Some(from) = current {
                    let delta = point - from;
                    segments.push((
                        coord(from + delta / 3.0),
                        coord(from + delta * (2.0 / 3.0)),
                        coord(point),
                    ));
                    current = Some(point);
                }
            }
            PathEl::QuadTo(control, point) => {
                if let Some(from) = current {
                    let cubic = kurbo::QuadBez::new(from, control, point).raise();
                    segments.push((coord(cubic.p1), coord(cubic.p2), coord(cubic.p3)));
                    current = Some(point);
                }
            }
            PathEl::CurveTo(control1, control2, point) => {
                if current.is_some() {
                    segments.push((coord(control1), coord(control2), coord(point)));
                    current = Some(point);
                }
            }
            PathEl::ClosePath => {
                if let (Some(from), Some(first)) = (current, start) {
                    if from != first {
                        let delta = first - from;
                        segments.push((
                            coord(from + delta / 3.0),
                            coord(from + delta * (2.0 / 3.0)),
                            coord(first),
                        ));
                    }
                }
            }
        }
    }
    start
        .filter(|_| !segments.is_empty())
        .map(|point| (coord(point), segments))
}

fn flo_to_kurbo(path: &SimpleBezierPath) -> BezPath {
    let mut result = BezPath::new();
    result.move_to(point(path.0));
    for &(control1, control2, end) in &path.1 {
        result.curve_to(point(control1), point(control2), point(end));
    }
    result.close_path();
    result
}

fn coord(point: Point) -> Coord2 {
    Coord2(point.x, point.y)
}

fn point(coord: Coord2) -> Point {
    Point::new(coord.0, coord.1)
}

fn clean_path(path: &BezPath, tol: f64) -> BezPath {
    let Some(first) = path.elements().iter().find_map(|element| match element {
        PathEl::MoveTo(point) => Some(*point),
        _ => None,
    }) else {
        return BezPath::new();
    };
    let closed = path
        .elements()
        .iter()
        .any(|element| matches!(element, PathEl::ClosePath));
    let tiny = tol.max(1e-9) * 0.01;
    let mut cleaned_segments = Vec::new();
    for segment in path.segments() {
        let cubic = segment.to_cubic();
        let all_tiny = (cubic.p3 - cubic.p0).hypot() <= tiny
            && (cubic.p1 - cubic.p0).hypot() <= tiny
            && (cubic.p2 - cubic.p0).hypot() <= tiny;
        if all_tiny {
            continue;
        }
        let line = kurbo::Line::new(cubic.p0, cubic.p3);
        let cleaned = if distance_to_line(cubic.p1, line) <= tiny
            && distance_to_line(cubic.p2, line) <= tiny
        {
            PathSeg::Line(line)
        } else {
            let control1 = if (cubic.p1 - cubic.p0).hypot() <= tiny {
                cubic.p0
            } else {
                cubic.p1
            };
            let control2 = if (cubic.p2 - cubic.p3).hypot() <= tiny {
                cubic.p3
            } else {
                cubic.p2
            };
            PathSeg::Cubic(kurbo::CubicBez::new(cubic.p0, control1, control2, cubic.p3))
        };
        if cleaned_segments
            .last()
            .is_some_and(|previous| segments_cancel(previous, &cleaned, tiny))
        {
            cleaned_segments.pop();
        } else {
            cleaned_segments.push(cleaned);
        }
    }
    let mut result = BezPath::new();
    result.move_to(first);
    for segment in cleaned_segments {
        result.push(segment.as_path_el());
    }
    if closed {
        result.close_path();
    }
    result
}

fn segments_cancel(left: &PathSeg, right: &PathSeg, tolerance: f64) -> bool {
    let left = left.to_cubic();
    let right = right.to_cubic();
    (left.p0 - right.p3).hypot() <= tolerance
        && (left.p1 - right.p2).hypot() <= tolerance
        && (left.p2 - right.p1).hypot() <= tolerance
        && (left.p3 - right.p0).hypot() <= tolerance
}

fn distance_to_line(point: Point, line: kurbo::Line) -> f64 {
    let vector = line.p1 - line.p0;
    let length = vector.hypot();
    if length <= f64::EPSILON {
        (point - line.p0).hypot()
    } else {
        ((point - line.p0).cross(vector) / length).abs()
    }
}

fn postprocess(paths: &[BezPath], kind: CurveKind, tol: f64) -> Vec<Contour> {
    let mut contours = from_bezpaths(paths, kind);
    for contour in &mut contours {
        merge_collinear_lines(contour, tol);
    }
    contours.retain(|contour| contour.points.len() >= if contour.closed { 3 } else { 2 });
    contours
}

fn merge_collinear_lines(contour: &mut Contour, tol: f64) {
    if contour.points.len() < 3 {
        return;
    }
    loop {
        let len = contour.points.len();
        let mut remove = None;
        for index in 0..len {
            if !contour.closed && (index == 0 || index + 1 == len) {
                continue;
            }
            let previous = if index == 0 { len - 1 } else { index - 1 };
            let next = (index + 1) % len;
            let incoming_line = contour.points[index].kind == PointKind::Line;
            let outgoing_line = contour.points[next].kind == PointKind::Line;
            if incoming_line
                && outgoing_line
                && collinear_forward(
                    &contour.points[previous],
                    &contour.points[index],
                    &contour.points[next],
                    tol,
                )
            {
                remove = Some(index);
                break;
            }
        }
        if let Some(index) = remove {
            contour.points.remove(index);
            if contour.points.len() < 3 {
                break;
            }
        } else {
            break;
        }
    }
}

fn collinear_forward(
    a: &fw_model::Point,
    b: &fw_model::Point,
    c: &fw_model::Point,
    tol: f64,
) -> bool {
    let ab = kurbo::Vec2::new(b.x - a.x, b.y - a.y);
    let bc = kurbo::Vec2::new(c.x - b.x, c.y - b.y);
    let scale = ab.hypot().max(bc.hypot()).max(1.0);
    ab.cross(bc).abs() <= tol * scale && ab.dot(bc) >= 0.0
}

fn validate_area(input: &[BezPath], output: &[BezPath]) -> Result<(), Error> {
    let input_flat = flatten_paths(input, 0.05);
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    let oracle = catch_unwind(AssertUnwindSafe(|| {
        input_flat.overlay(&empty, OverlayRule::Subject, FillRule::NonZero)
    }))
    .map_err(|_| boolean_failure("i_overlay panicked while validating area"))?;
    let area_a: f64 = oracle
        .iter()
        .map(|shape| {
            shape
                .iter()
                .map(|polygon| signed_polygon_area(polygon))
                .sum::<f64>()
                .abs()
        })
        .sum();
    let output_flat = flatten_paths(output, 0.05);
    let output_shapes = catch_unwind(AssertUnwindSafe(|| {
        output_flat.overlay(&empty, OverlayRule::Subject, FillRule::EvenOdd)
    }))
    .map_err(|_| boolean_failure("i_overlay panicked while validating result area"))?;
    let area_b: f64 = output_shapes
        .iter()
        .map(|shape| {
            shape
                .iter()
                .map(|polygon| signed_polygon_area(polygon))
                .sum::<f64>()
                .abs()
        })
        .sum();
    let scale = area_a.abs().max(area_b.abs());
    if (area_a - area_b).abs() > scale * 0.01 && scale > 1e-9 {
        tracing::warn!(
            area_a,
            area_b,
            "boolean result disagrees with polygon area oracle"
        );
        Err(Error::BooleanMismatch { area_a, area_b })
    } else {
        Ok(())
    }
}

fn flatten_paths(paths: &[BezPath], tolerance: f64) -> Vec<Vec<[f64; 2]>> {
    paths
        .iter()
        .filter_map(|path| {
            let mut polygon = Vec::new();
            flatten(path.iter(), tolerance, |element| {
                if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                    polygon.push([point.x, point.y]);
                }
            });
            (polygon.len() >= 3).then_some(polygon)
        })
        .collect()
}

fn signed_polygon_area(polygon: &[[f64; 2]]) -> f64 {
    if polygon.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..polygon.len() {
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        area += current[0] * next[1] - next[0] * current[1];
    }
    area * 0.5
}

fn dominant_orientation(contours: &[Contour]) -> Option<bool> {
    contours
        .iter()
        .filter(|contour| contour.closed)
        .max_by(|left, right| {
            left.to_bezpath()
                .area()
                .abs()
                .total_cmp(&right.to_bezpath().area().abs())
        })
        .map(|contour| contour.to_bezpath().area() < 0.0)
}

fn restore_unchanged_contours(output: &mut [Contour], input: &[Contour]) {
    let mut used = vec![false; input.len()];
    for contour in output {
        if let Some((index, original)) = input
            .iter()
            .enumerate()
            .find(|(index, original)| !used[*index] && same_contour_geometry(contour, original))
        {
            used[index] = true;
            *contour = original.clone();
        }
    }
}

fn same_contour_geometry(left: &Contour, right: &Contour) -> bool {
    if left.closed != right.closed {
        return false;
    }
    if left.points.len() == right.points.len() {
        if left.points.is_empty() {
            return true;
        }
        let matches_at_offset = |offset: usize| {
            left.points.iter().enumerate().all(|(index, left_point)| {
                let right_point = &right.points[(index + offset) % right.points.len()];
                left_point.kind == right_point.kind
                    && (left_point.x - right_point.x).abs() <= 1e-9
                    && (left_point.y - right_point.y).abs() <= 1e-9
            })
        };
        if (left.closed && (0..right.points.len()).any(matches_at_offset))
            || (!left.closed && matches_at_offset(0))
        {
            return true;
        }
    }
    same_path_geometry(left, right)
}

fn same_path_geometry(left: &Contour, right: &Contour) -> bool {
    let left_segments: Vec<_> = left
        .to_bezpath()
        .segments()
        .map(|segment| segment.to_cubic())
        .collect();
    let right_segments: Vec<_> = right
        .to_bezpath()
        .segments()
        .map(|segment| segment.to_cubic())
        .collect();
    if left_segments.len() != right_segments.len() {
        return false;
    }
    if left_segments.is_empty() {
        return true;
    }
    let matches_at_offset = |offset: usize| {
        left_segments.iter().enumerate().all(|(index, left)| {
            let right = right_segments[(index + offset) % right_segments.len()];
            points_near(left.p0, right.p0)
                && points_near(left.p1, right.p1)
                && points_near(left.p2, right.p2)
                && points_near(left.p3, right.p3)
        })
    };
    if left.closed {
        (0..right_segments.len()).any(matches_at_offset)
    } else {
        matches_at_offset(0)
    }
}

fn points_near(left: Point, right: Point) -> bool {
    (left - right).hypot() <= 1e-7
}

fn same_path_segments(left: &BezPath, right: &BezPath) -> bool {
    let left: Vec<_> = left.segments().map(|segment| segment.to_cubic()).collect();
    let right: Vec<_> = right.segments().map(|segment| segment.to_cubic()).collect();
    left.len() == right.len()
        && left.iter().zip(&right).all(|(left, right)| {
            points_near(left.p0, right.p0)
                && points_near(left.p1, right.p1)
                && points_near(left.p2, right.p2)
                && points_near(left.p3, right.p3)
        })
}

fn has_boundary_intersections(paths: &[BezPath]) -> bool {
    let polygons = flatten_paths(paths, 0.05);
    for (polygon_index, polygon) in polygons.iter().enumerate() {
        let edges = polygon_edges(polygon);
        for left in 0..edges.len() {
            for right in left + 1..edges.len() {
                let adjacent = right == left + 1 || (left == 0 && right + 1 == edges.len());
                if !adjacent && line_segments_intersect(edges[left], edges[right]) {
                    return true;
                }
            }
        }
        for other in &polygons[polygon_index + 1..] {
            let other_edges = polygon_edges(other);
            if edges.iter().any(|&left| {
                other_edges
                    .iter()
                    .any(|&right| line_segments_intersect(left, right))
            }) {
                return true;
            }
        }
    }
    false
}

fn polygon_edges(polygon: &[[f64; 2]]) -> Vec<([f64; 2], [f64; 2])> {
    if polygon.len() < 2 {
        return Vec::new();
    }
    let mut edges: Vec<_> = polygon.windows(2).map(|edge| (edge[0], edge[1])).collect();
    if polygon.first() != polygon.last() {
        edges.push((polygon[polygon.len() - 1], polygon[0]));
    }
    edges
}

fn line_segments_intersect(left: ([f64; 2], [f64; 2]), right: ([f64; 2], [f64; 2])) -> bool {
    const EPSILON: f64 = 1e-8;
    let orientation = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let on_segment = |a: [f64; 2], b: [f64; 2], p: [f64; 2]| {
        p[0] >= a[0].min(b[0]) - EPSILON
            && p[0] <= a[0].max(b[0]) + EPSILON
            && p[1] >= a[1].min(b[1]) - EPSILON
            && p[1] <= a[1].max(b[1]) + EPSILON
    };
    let o1 = orientation(left.0, left.1, right.0);
    let o2 = orientation(left.0, left.1, right.1);
    let o3 = orientation(right.0, right.1, left.0);
    let o4 = orientation(right.0, right.1, left.1);
    (o1 * o2 < -EPSILON && o3 * o4 < -EPSILON)
        || (o1.abs() <= EPSILON && on_segment(left.0, left.1, right.0))
        || (o2.abs() <= EPSILON && on_segment(left.0, left.1, right.1))
        || (o3.abs() <= EPSILON && on_segment(right.0, right.1, left.0))
        || (o4.abs() <= EPSILON && on_segment(right.0, right.1, left.1))
}
