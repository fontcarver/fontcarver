use std::{
    collections::HashMap,
    panic::{catch_unwind, AssertUnwindSafe},
};

use flo_curves::{
    bezier::path::{
        path_add, path_intersect, path_remove_interior_points, path_sub, SimpleBezierPath,
    },
    bezier::{find_self_intersection_point, path::path_intersects_path, BezierCurveFactory, Curve},
    geo::Coord2,
};
use fw_model::{Contour, PointKind};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use kurbo::{
    fit_to_bezpath, flatten, BezPath, CurveFitSample, Line, ParamCurve, ParamCurveFit, PathEl,
    PathSeg, Point, Shape, Vec2,
};

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
/// If no filled boundaries cross or overlap and their windings already match
/// [`correct_direction`], the complete input is returned byte-for-byte and in
/// the same order. Otherwise open contours keep their relative input slots,
/// boolean-generated closed contours occupy the closed slots in result order,
/// and any additional closed contours are appended.
pub fn remove_overlap(
    contours: &[Contour],
    kind: CurveKind,
    tol: Tolerance,
) -> Result<Vec<Contour>, Error> {
    validate_tolerance(tol)?;
    let closed: Vec<_> = contours
        .iter()
        .filter(|contour| contour.closed)
        .cloned()
        .collect();
    if closed.is_empty() {
        return Ok(contours.to_vec());
    }
    let clean_paths: Vec<_> = to_bezpaths(&closed)
        .iter()
        .map(|path| clean_path(path, tol))
        .filter(|path| path.segments().next().is_some())
        .collect();
    let original_paths = to_bezpaths(&closed);
    if !has_crossing_or_overlap(&original_paths) && windings_are_normalized(&closed) {
        if contours_match_kind(&closed, kind) {
            return Ok(contours.to_vec());
        }
        let converted = postprocess(&clean_paths, kind, tol);
        return Ok(interleave_open_contours(contours, converted));
    }
    let flo_paths: Vec<_> = clean_paths.iter().filter_map(kurbo_to_flo).collect();
    if flo_paths.is_empty() {
        return Ok(contours
            .iter()
            .filter(|contour| !contour.closed)
            .cloned()
            .collect());
    }

    let result = catch_unwind(AssertUnwindSafe(|| {
        path_remove_interior_points::<SimpleBezierPath, SimpleBezierPath>(&flo_paths, tol)
    }))
    .map_err(|_| boolean_failure("flo_curves panicked while removing overlap"))?;
    let mut result_paths: Vec<_> = result
        .iter()
        .map(flo_to_kurbo)
        .map(|path| clean_path(&path, tol))
        .collect();
    corrupt_flo_result_for_test(&mut result_paths);
    let mut output = postprocess(&result_paths, kind, tol);
    if let Some(outer_clockwise) = dominant_orientation(&closed) {
        output = correct_direction(&output, outer_clockwise);
    }
    validate_area(&clean_paths, &to_bezpaths(&output))?;
    restore_unchanged_contours(&mut output, &closed);
    Ok(interleave_open_contours(contours, output))
}

/// Whether [`remove_overlap_with_fallback`] used polygonal fallback geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallbackKind {
    /// The strict curve-preserving operation succeeded.
    None,
    /// The outline was flattened, unioned, and re-fitted to Bézier curves.
    Flattened,
}

/// Result of overlap removal together with its geometry-quality status.
#[derive(Clone, Debug, PartialEq)]
pub struct RemoveOverlapOutcome {
    /// Resulting contours.
    pub contours: Vec<Contour>,
    /// Fallback used to produce the contours.
    ///
    /// User interfaces should show a warning whenever this is not
    /// [`FallbackKind::None`].
    pub fallback: FallbackKind,
}

/// Removes overlap, falling back to a flattened NonZero union when the strict
/// curve boolean fails or disagrees with the polygon area oracle.
///
/// The fallback union is re-fitted to cubic Bézier segments with accuracy
/// `tol * 10`. User interfaces should warn when the returned `fallback` is not
/// [`FallbackKind::None`]. Invalid parameters are returned without fallback.
pub fn remove_overlap_with_fallback(
    contours: &[Contour],
    kind: CurveKind,
    tol: Tolerance,
) -> Result<RemoveOverlapOutcome, Error> {
    match remove_overlap(contours, kind, tol) {
        Ok(contours) => Ok(RemoveOverlapOutcome {
            contours,
            fallback: FallbackKind::None,
        }),
        Err(Error::BooleanFailed { .. } | Error::BooleanMismatch { .. }) => {
            fallback_remove_overlap(contours, kind, tol).map(|contours| RemoveOverlapOutcome {
                contours,
                fallback: FallbackKind::Flattened,
            })
        }
        Err(error) => Err(error),
    }
}

fn interleave_open_contours(input: &[Contour], closed_output: Vec<Contour>) -> Vec<Contour> {
    let mut closed = closed_output.into_iter();
    let mut output = Vec::new();
    for contour in input {
        if contour.closed {
            if let Some(result) = closed.next() {
                output.push(result);
            }
        } else {
            output.push(contour.clone());
        }
    }
    output.extend(closed);
    output
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

/// Cleans one path containing exactly one subpath.
fn clean_path(path: &BezPath, tol: f64) -> BezPath {
    debug_assert!(
        path.elements()
            .iter()
            .filter(|element| matches!(element, PathEl::MoveTo(_)))
            .count()
            <= 1,
        "clean_path requires a single-subpath BezPath"
    );
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
        let end = if closed && (cubic.p3 - first).hypot() <= tiny {
            first
        } else {
            cubic.p3
        };
        let all_tiny = (cubic.p3 - cubic.p0).hypot() <= tiny
            && (cubic.p1 - cubic.p0).hypot() <= tiny
            && (cubic.p2 - cubic.p0).hypot() <= tiny;
        if all_tiny {
            continue;
        }
        let logical_start = cleaned_segments.last().map(PathSeg::end).unwrap_or(first);
        let line = kurbo::Line::new(logical_start, end);
        let cleaned = if distance_to_line(cubic.p1, line) <= tiny
            && distance_to_line(cubic.p2, line) <= tiny
        {
            PathSeg::Line(line)
        } else {
            let control1 = if (cubic.p1 - cubic.p0).hypot() <= tiny {
                logical_start
            } else {
                cubic.p1
            };
            let control2 = if (cubic.p2 - cubic.p3).hypot() <= tiny {
                end
            } else {
                cubic.p2
            };
            PathSeg::Cubic(kurbo::CubicBez::new(logical_start, control1, control2, end))
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
        output_flat.overlay(&empty, OverlayRule::Subject, FillRule::NonZero)
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

fn windings_are_normalized(contours: &[Contour]) -> bool {
    let Some(outer_clockwise) = dominant_orientation(contours) else {
        return true;
    };
    let corrected = correct_direction(contours, outer_clockwise);
    contours.iter().zip(&corrected).all(|(original, fixed)| {
        let original_area = original.to_bezpath().area();
        let fixed_area = fixed.to_bezpath().area();
        original_area.abs() <= 1e-12
            || fixed_area.abs() <= 1e-12
            || original_area.signum() == fixed_area.signum()
    })
}

pub(crate) fn normalize_self_intersecting_contours(
    contours: Vec<Contour>,
) -> Result<Vec<Contour>, Error> {
    const INTERSECTION_ACCURACY: f64 = 0.001;
    let mut normalized = Vec::new();
    for contour in &contours {
        let path = contour.to_bezpath();
        let split = catch_unwind(AssertUnwindSafe(|| {
            split_self_intersecting_ring(&path, INTERSECTION_ACCURACY)
        }))
        .map_err(|_| {
            boolean_failure("curve intersection panicked while normalizing stroke ring")
        })?;
        let Some(loops) = split else {
            normalized.extend(postprocess(
                &[clean_path(&path, INTERSECTION_ACCURACY)],
                CurveKind::Cubic,
                INTERSECTION_ACCURACY,
            ));
            continue;
        };
        normalized.extend(postprocess(&loops, CurveKind::Cubic, INTERSECTION_ACCURACY));
    }
    if let Some(clockwise) = dominant_orientation(&contours) {
        normalized = correct_direction(&normalized, clockwise);
    }
    Ok(normalized)
}

fn split_self_intersecting_ring(path: &BezPath, accuracy: f64) -> Option<Vec<BezPath>> {
    let segments: Vec<_> = path.segments().collect();
    if segments.is_empty() {
        return None;
    }
    let flo_path = kurbo_to_flo(path)?;
    let mut cuts = vec![vec![0.0, 1.0]; segments.len()];
    let mut found = false;
    for ((left, left_t), (right, right_t)) in path_intersects_path(&flo_path, &flo_path, accuracy) {
        if left >= right || are_only_shared_endpoint(left, left_t, right, right_t, segments.len()) {
            continue;
        }
        found = true;
        add_cut(&mut cuts[left], left_t);
        add_cut(&mut cuts[right], right_t);
    }
    for (index, segment) in segments.iter().enumerate() {
        let cubic = segment.to_cubic();
        let curve = Curve::from_points(
            coord(cubic.p0),
            (coord(cubic.p1), coord(cubic.p2)),
            coord(cubic.p3),
        );
        if let Some((first, second)) = find_self_intersection_point(&curve, accuracy) {
            if add_cut(&mut cuts[index], first) | add_cut(&mut cuts[index], second) {
                found = true;
            }
        }
    }
    if !found {
        return None;
    }

    let mut pieces = Vec::new();
    for (segment, segment_cuts) in segments.into_iter().zip(&mut cuts) {
        segment_cuts.sort_by(f64::total_cmp);
        segment_cuts.dedup_by(|left, right| (*left - *right).abs() <= 1e-7);
        for range in segment_cuts.windows(2) {
            if range[1] - range[0] <= 1e-8 {
                continue;
            }
            let piece = segment.subsegment(range[0]..range[1]);
            if (piece.end() - piece.start()).hypot() > accuracy * 0.01 {
                pieces.push(piece);
            }
        }
    }
    let original_area = path.area();
    let mut loops = directed_piece_loops(&pieces, accuracy);
    loops.retain(|candidate| {
        let area = candidate.area();
        area.abs() > accuracy * accuracy && area * original_area > 0.0
    });
    let largest_area = loops
        .iter()
        .map(|candidate| candidate.area().abs())
        .fold(0.0, f64::max);
    // Intersection solvers can leave microscopic same-winding slivers at a
    // multiple crossing. They are not stable stroke rings and otherwise make
    // the downstream curve boolean fail.
    loops.retain(|candidate| candidate.area().abs() >= largest_area * 0.001);
    Some(loops)
}

fn are_only_shared_endpoint(
    left: usize,
    left_t: f64,
    right: usize,
    right_t: f64,
    count: usize,
) -> bool {
    const PARAM_EPSILON: f64 = 1e-6;
    (right == left + 1 && (left_t - 1.0).abs() <= PARAM_EPSILON && right_t.abs() <= PARAM_EPSILON)
        || (left == 0
            && right + 1 == count
            && left_t.abs() <= PARAM_EPSILON
            && (right_t - 1.0).abs() <= PARAM_EPSILON)
}

fn add_cut(cuts: &mut Vec<f64>, value: f64) -> bool {
    const PARAM_EPSILON: f64 = 1e-7;
    if !value.is_finite() || !(-PARAM_EPSILON..=1.0 + PARAM_EPSILON).contains(&value) {
        return false;
    }
    let value = value.clamp(0.0, 1.0);
    if cuts
        .iter()
        .any(|existing| (*existing - value).abs() <= PARAM_EPSILON)
    {
        false
    } else {
        cuts.push(value);
        true
    }
}

fn directed_piece_loops(pieces: &[PathSeg], accuracy: f64) -> Vec<BezPath> {
    if pieces.is_empty() {
        return Vec::new();
    }
    let mut node_points = Vec::<Point>::new();
    let mut piece_nodes = Vec::with_capacity(pieces.len());
    for piece in pieces {
        let start = nearby_node(&mut node_points, piece.start(), accuracy);
        let end = nearby_node(&mut node_points, piece.end(), accuracy);
        piece_nodes.push((start, end));
    }

    let mut stack_nodes = vec![piece_nodes[0].0];
    let mut stack_edges = Vec::new();
    let mut positions = HashMap::from([(piece_nodes[0].0, 0_usize)]);
    let mut loops = Vec::new();
    for (piece, &(start, end)) in pieces.iter().zip(&piece_nodes) {
        if stack_nodes.last().copied() != Some(start) {
            stack_nodes.push(start);
            positions.insert(start, stack_nodes.len() - 1);
        }
        stack_edges.push(*piece);
        if let Some(&position) = positions.get(&end) {
            let loop_edges = stack_edges.split_off(position);
            stack_nodes.truncate(position + 1);
            positions.clear();
            positions.extend(
                stack_nodes
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, node)| (node, index)),
            );
            if let Some(path) = closed_path_from_pieces(&loop_edges) {
                loops.push(path);
            }
        } else {
            stack_nodes.push(end);
            positions.insert(end, stack_nodes.len() - 1);
        }
    }
    loops
}

fn nearby_node(nodes: &mut Vec<Point>, point: Point, accuracy: f64) -> usize {
    if let Some(index) = nodes
        .iter()
        .position(|candidate| (*candidate - point).hypot() <= accuracy * 2.0)
    {
        index
    } else {
        nodes.push(point);
        nodes.len() - 1
    }
}

fn closed_path_from_pieces(pieces: &[PathSeg]) -> Option<BezPath> {
    let first = pieces.first()?;
    let mut path = BezPath::new();
    path.move_to(first.start());
    for piece in pieces {
        path.push(piece.as_path_el());
    }
    path.close_path();
    Some(path)
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

fn fallback_remove_overlap(
    contours: &[Contour],
    kind: CurveKind,
    tol: f64,
) -> Result<Vec<Contour>, Error> {
    validate_tolerance(tol)?;
    let closed: Vec<_> = contours
        .iter()
        .filter(|contour| contour.closed)
        .cloned()
        .collect();
    if closed.is_empty() {
        return Ok(contours.to_vec());
    }
    let polygons = flatten_paths(&to_bezpaths(&closed), tol);
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    let shapes = catch_unwind(AssertUnwindSafe(|| {
        polygons.overlay(&empty, OverlayRule::Union, FillRule::NonZero)
    }))
    .map_err(|_| boolean_failure("i_overlay panicked during fallback overlap removal"))?;
    let fitted_paths: Vec<_> = shapes
        .iter()
        .flat_map(|shape| shape.iter())
        .filter_map(|polygon| fit_polygon_to_cubics(polygon, tol * 10.0))
        .collect();
    let mut output = from_bezpaths(&fitted_paths, kind);
    output.retain(|contour| contour.points.len() >= 3);
    if let Some(outer_clockwise) = dominant_orientation(&closed) {
        output = correct_direction(&output, outer_clockwise);
    }
    Ok(interleave_open_contours(contours, output))
}

fn fit_polygon_to_cubics(polygon: &[[f64; 2]], accuracy: f64) -> Option<BezPath> {
    let edges = polygon_edges(polygon);
    let first = edges.first()?.0;
    let mut result = BezPath::new();
    result.move_to((first[0], first[1]));
    for &(start, end) in &edges {
        let fitted = fit_to_bezpath(
            &LineFit(Line::new(
                Point::new(start[0], start[1]),
                Point::new(end[0], end[1]),
            )),
            accuracy,
        );
        for element in fitted.elements() {
            if !matches!(element, PathEl::MoveTo(_)) {
                result.push(*element);
            }
        }
    }
    result.close_path();
    Some(result)
}

struct LineFit(Line);

impl ParamCurveFit for LineFit {
    fn sample_pt_tangent(&self, t: f64, _: f64) -> CurveFitSample {
        let (p, tangent) = self.sample_pt_deriv(t);
        CurveFitSample { p, tangent }
    }

    fn sample_pt_deriv(&self, t: f64) -> (Point, Vec2) {
        (self.0.p0.lerp(self.0.p1, t), self.0.p1 - self.0.p0)
    }

    fn break_cusp(&self, _: std::ops::Range<f64>) -> Option<f64> {
        None
    }
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

/// Uses an x-axis sweep and y-bbox rejection, making the common case
/// `O(E log E + K)` for `E` edges and `K` overlapping bbox candidates.
fn has_crossing_or_overlap(paths: &[BezPath]) -> bool {
    const FLATTEN_ACCURACY: f64 = 0.01;
    const TOUCH_TOLERANCE: f64 = 0.001;
    let polygons = flatten_paths(paths, FLATTEN_ACCURACY);
    let mut edges = Vec::new();
    for (path, polygon) in polygons.iter().enumerate() {
        let polygon_edges = polygon_edges(polygon);
        let count = polygon_edges.len();
        edges.extend(
            polygon_edges
                .into_iter()
                .enumerate()
                .map(|(index, segment)| FlatEdge::new(segment, path, index, count)),
        );
    }
    edges.sort_by(|left, right| left.min_x.total_cmp(&right.min_x));
    for left in 0..edges.len() {
        for right in left + 1..edges.len() {
            if edges[right].min_x > edges[left].max_x + TOUCH_TOLERANCE {
                break;
            }
            if edges[left].max_y + TOUCH_TOLERANCE < edges[right].min_y
                || edges[right].max_y + TOUCH_TOLERANCE < edges[left].min_y
                || edges[left].adjacent_to(&edges[right])
            {
                continue;
            }
            if flat_edges_cross_or_overlap(
                edges[left].segment,
                edges[right].segment,
                TOUCH_TOLERANCE,
            ) {
                return true;
            }
        }
    }
    false
}

#[derive(Clone, Copy)]
struct FlatEdge {
    segment: ([f64; 2], [f64; 2]),
    path: usize,
    index: usize,
    count: usize,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl FlatEdge {
    fn new(segment: ([f64; 2], [f64; 2]), path: usize, index: usize, count: usize) -> Self {
        Self {
            segment,
            path,
            index,
            count,
            min_x: segment.0[0].min(segment.1[0]),
            max_x: segment.0[0].max(segment.1[0]),
            min_y: segment.0[1].min(segment.1[1]),
            max_y: segment.0[1].max(segment.1[1]),
        }
    }

    fn adjacent_to(&self, other: &Self) -> bool {
        self.path == other.path
            && (self.index.abs_diff(other.index) == 1
                || (self.index == 0 && other.index + 1 == self.count)
                || (other.index == 0 && self.index + 1 == self.count))
    }
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

fn flat_edges_cross_or_overlap(
    left: ([f64; 2], [f64; 2]),
    right: ([f64; 2], [f64; 2]),
    touch_tolerance: f64,
) -> bool {
    let left_line = Line::new(
        Point::new(left.0[0], left.0[1]),
        Point::new(left.1[0], left.1[1]),
    );
    let right_line = Line::new(
        Point::new(right.0[0], right.0[1]),
        Point::new(right.1[0], right.1[1]),
    );
    if collinear_overlap(left_line, right_line, touch_tolerance) {
        return true;
    }
    PathSeg::Line(left_line)
        .intersect_line(right_line)
        .iter()
        .any(|intersection| {
            let point = left_line.eval(intersection.segment_t);
            [left_line.p0, left_line.p1, right_line.p0, right_line.p1]
                .iter()
                .all(|endpoint| (*endpoint - point).hypot() > touch_tolerance)
        })
}

fn collinear_overlap(left: Line, right: Line, touch_tolerance: f64) -> bool {
    const COLLINEAR_EPSILON: f64 = 1e-8;
    let left_vector = left.p1 - left.p0;
    let right_vector = right.p1 - right.p0;
    let left_length = left_vector.hypot();
    let right_length = right_vector.hypot();
    if left_length <= COLLINEAR_EPSILON || right_length <= COLLINEAR_EPSILON {
        return false;
    }
    if left_vector.cross(right_vector).abs() > COLLINEAR_EPSILON * left_length * right_length
        || distance_to_line(right.p0, left) > COLLINEAR_EPSILON
        || distance_to_line(right.p1, left) > COLLINEAR_EPSILON
    {
        return false;
    }
    let axis = left_vector / left_length;
    let left_range = [0.0, left_length];
    let right_projection = [
        (right.p0 - left.p0).dot(axis),
        (right.p1 - left.p0).dot(axis),
    ];
    let overlap_start = left_range[0].max(right_projection[0].min(right_projection[1]));
    let overlap_end = left_range[1].min(right_projection[0].max(right_projection[1]));
    overlap_end - overlap_start > touch_tolerance
}

#[cfg(test)]
thread_local! {
    static CORRUPT_FLO_RESULT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn corrupt_flo_result_for_test(paths: &mut Vec<BezPath>) {
    CORRUPT_FLO_RESULT.with(|enabled| {
        if enabled.get() {
            paths.clear();
        }
    });
}

#[cfg(not(test))]
fn corrupt_flo_result_for_test(_: &mut Vec<BezPath>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use fw_model::{Point as ModelPoint, TtIndex};

    fn model_point(x: f64, y: f64) -> ModelPoint {
        ModelPoint {
            x,
            y,
            kind: PointKind::Line,
            smooth: false,
            name: None,
            tt_index: TtIndex::Unnumbered,
        }
    }

    fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
        Contour {
            closed: true,
            points: vec![
                model_point(x0, y0),
                model_point(x1, y0),
                model_point(x1, y1),
                model_point(x0, y1),
            ],
            name: None,
            id: None,
        }
    }

    #[test]
    fn area_oracle_rejects_a_corrupted_flo_result() {
        CORRUPT_FLO_RESULT.with(|enabled| enabled.set(true));
        let result = remove_overlap(
            &[
                rectangle(0.0, 0.0, 100.0, 100.0),
                rectangle(50.0, 0.0, 150.0, 100.0),
            ],
            CurveKind::Cubic,
            0.01,
        );
        CORRUPT_FLO_RESULT.with(|enabled| enabled.set(false));
        assert!(matches!(result, Err(Error::BooleanMismatch { .. })));
    }

    #[test]
    fn fallback_recovers_from_a_corrupted_flo_result_and_reports_it() {
        CORRUPT_FLO_RESULT.with(|enabled| enabled.set(true));
        let outcome = remove_overlap_with_fallback(
            &[
                rectangle(0.0, 0.0, 100.0, 100.0),
                rectangle(50.0, 0.0, 150.0, 100.0),
            ],
            CurveKind::Cubic,
            0.01,
        );
        CORRUPT_FLO_RESULT.with(|enabled| enabled.set(false));
        let outcome = outcome.expect("polygon fallback");
        assert_eq!(outcome.fallback, FallbackKind::Flattened);
        assert!(outcome
            .contours
            .iter()
            .flat_map(|contour| &contour.points)
            .any(|point| point.kind == PointKind::OffCurve));
    }

    #[test]
    fn cleanup_removes_tiny_segments_repairs_controls_and_cancels_backtracks() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((0.000_001, 0.0));
        path.curve_to((0.000_001, 0.0), (80.0, 20.0), (100.0, 0.0));
        path.line_to((150.0, 0.0));
        path.line_to((100.0, 0.0));
        let cleaned = clean_path(&path, 0.01);
        let segments: Vec<_> = cleaned.segments().collect();
        assert_eq!(segments.len(), 1);
        let cubic = segments[0].to_cubic();
        assert_eq!(cubic.p0, Point::new(0.0, 0.0));
        assert!((cubic.p1 - cubic.p0).hypot() <= 0.000_001);
        assert_eq!(cubic.p2, Point::new(80.0, 20.0));
        assert_eq!(cubic.p3, Point::new(100.0, 0.0));
    }

    #[test]
    fn a_normalized_stroke_ring_uses_the_fast_path_predicate() {
        let source = rectangle(0.0, 0.0, 200.0, 200.0).to_bezpath();
        let stroked = kurbo::stroke(
            source.iter(),
            &kurbo::Stroke::new(20.0).with_join(kurbo::Join::Miter),
            &kurbo::StrokeOpts::default(),
            0.01,
        );
        let contours =
            normalize_self_intersecting_contours(from_bezpaths(&[stroked], CurveKind::Cubic))
                .expect("normalize stroke rings");
        let originals = to_bezpaths(&contours);
        let intersects = has_crossing_or_overlap(&originals);
        let normalized = windings_are_normalized(&contours);
        assert!(
            !intersects && normalized,
            "intersects={intersects}, normalized={normalized}"
        );
    }
}
