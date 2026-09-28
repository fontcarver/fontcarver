use fc_model::{Contour, PointKind};
use kurbo::{
    simplify::{simplify_bezpath, SimplifyOptions as KurboSimplifyOptions},
    Affine, BezPath, Line, ParamCurve, ParamCurveExtrema, ParamCurveNearest, PathEl, PathSeg,
    Point, Rect, Shape,
};

use crate::{
    convert::path_to_contours, from_bezpaths, to_bezpaths, CurveKind, DEFAULT_SIMPLIFY_TOLERANCE,
};

mod fit;

pub use fit::{fit_span, SpanFit};

const ENDPOINT_EXTREMA_TOLERANCE: f64 = 0.01;

/// Controls FontForge-compatible policy around kurbo's simplification core.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimplifyOptions {
    /// Maximum permitted geometric error in font units.
    pub error: f64,
    /// Permit refitting without preserving endpoint slopes.
    pub ignore_slopes: bool,
    /// Permit removal of explicit extrema points.
    pub ignore_extrema: bool,
    /// Mark refitted curve endpoints as smooth.
    pub smooth_curves: bool,
    /// Convert curve segments that fit their chord within `error` to lines.
    pub force_lines: bool,
    /// Snap line endpoints whose x or y difference is at most [`Self::error`].
    pub nearly_hv_lines: bool,
    /// Rotate closed contours to begin at the leftmost, then lowest, on-curve point.
    pub start_at_extrema: bool,
}

impl Default for SimplifyOptions {
    fn default() -> Self {
        Self {
            error: DEFAULT_SIMPLIFY_TOLERANCE,
            ignore_slopes: false,
            ignore_extrema: false,
            smooth_curves: false,
            force_lines: false,
            nearly_hv_lines: false,
            start_at_extrema: false,
        }
    }
}

/// A closest point found on an outline segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathHit {
    /// Index of the containing contour.
    pub contour: usize,
    /// Index in `Contour::to_bezpath().segments()`.
    ///
    /// Closed contours start at their first on-curve point, and quadratic
    /// chains materialise implied midpoint segments. Use [`segment_to_points`]
    /// to recover the corresponding span in the model point list.
    pub segment: usize,
    /// Parametric position on the segment in the inclusive range `0..=1`.
    pub t: f64,
    /// Closest point in font-space coordinates.
    pub point: Point,
    /// Euclidean distance from the query point.
    pub distance: f64,
}

/// Simplifies contours using kurbo's curve fitter and FontForge-style flags.
///
/// A non-finite or non-positive error is rejected by returning the input
/// unchanged. When extrema are preserved, refitting may move an extremum by
/// at most the simplify error; extrema within `max(8 * error, 0.01)` of a
/// fitted endpoint are treated as already present.
pub fn simplify(contours: &[Contour], opts: &SimplifyOptions) -> Vec<Contour> {
    if !opts.error.is_finite() || opts.error <= 0.0 {
        tracing::warn!(
            simplify_error = opts.error,
            error = %crate::Error::InvalidParameter { parameter: "simplify error" },
            "simplify rejected an invalid parameter"
        );
        return contours.to_vec();
    }
    let error = opts.error;
    let mut result = Vec::with_capacity(contours.len());
    for contour in contours {
        let mut prepared = contour.clone();
        if opts.nearly_hv_lines {
            snap_nearly_hv_lines(&mut prepared, error);
        }
        let source = if opts.ignore_extrema {
            prepared
        } else {
            add_extrema(std::slice::from_ref(&prepared))
                .into_iter()
                .next()
                .unwrap_or(prepared)
        };
        let Some(path) = to_bezpaths(std::slice::from_ref(&source))
            .into_iter()
            .next()
        else {
            result.push(source);
            continue;
        };
        let core_options = if opts.ignore_slopes {
            KurboSimplifyOptions::default().angle_thresh(std::f64::consts::FRAC_PI_2)
        } else if opts.smooth_curves {
            KurboSimplifyOptions::default().angle_thresh(0.2)
        } else {
            KurboSimplifyOptions::default()
        };
        let mut simplified = simplify_bezpath(path.iter(), error, &core_options);
        if opts.force_lines {
            simplified = force_near_lines(&simplified, error);
        }
        let kind = if contour.is_quadratic() {
            CurveKind::Quadratic { max_err: error }
        } else {
            CurveKind::Cubic
        };
        let converted = from_bezpaths(&[simplified], kind);
        let converted = if opts.ignore_extrema {
            converted
        } else {
            add_extrema_with_tolerance(&converted, error.mul_add(8.0, 0.0).max(0.01))
        };
        let mut output = converted.into_iter().next().unwrap_or(source);
        restore_contour_metadata(&mut output, contour);
        restore_matching_point_metadata(&mut output, contour);
        if opts.smooth_curves {
            for point in &mut output.points {
                if matches!(point.kind, PointKind::Curve | PointKind::QCurve) {
                    point.smooth = true;
                }
            }
        }
        if opts.start_at_extrema {
            rotate_to_canonical_start(&mut output);
        }
        result.push(output);
    }
    result
}

/// Maps a Bézier segment index back to its source model-point span.
///
/// The segment basis is exactly `contour.to_bezpath().segments()`. Closed
/// contours therefore begin at the first on-curve point rather than point
/// zero. Every segment materialised from one implied quadratic chain maps to
/// the same pair of explicit on-curve endpoints. For an all-off-curve contour,
/// the pair names adjacent controls around the implied-point ring. An invalid
/// segment maps to `(contour.points.len(), contour.points.len())`.
pub fn segment_to_points(contour: &Contour, segment: usize) -> (usize, usize) {
    let len = contour.points.len();
    if len == 0 {
        return (0, 0);
    }
    let on_curves: Vec<_> = contour
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind != PointKind::OffCurve)
        .map(|(index, _)| index)
        .collect();
    if on_curves.is_empty() {
        return if contour.closed && segment < len {
            (segment, (segment + 1) % len)
        } else {
            (len, len)
        };
    }

    let mut mappings = Vec::new();
    if contour.closed {
        let start = on_curves[0];
        let mut from = start;
        let mut controls = 0;
        for offset in 1..=len {
            let index = (start + offset) % len;
            if contour.points[index].kind == PointKind::OffCurve {
                controls += 1;
            } else {
                let count = model_segment_count(&contour.points[index], controls);
                mappings.extend(std::iter::repeat_n((from, index), count));
                from = index;
                controls = 0;
            }
        }
    } else {
        let mut from = on_curves[0];
        for &index in &on_curves[1..] {
            let controls = contour.points[from + 1..index]
                .iter()
                .filter(|point| point.kind == PointKind::OffCurve)
                .count();
            let count = model_segment_count(&contour.points[index], controls);
            mappings.extend(std::iter::repeat_n((from, index), count));
            from = index;
        }
    }
    mappings.get(segment).copied().unwrap_or((len, len))
}

fn model_segment_count(endpoint: &fc_model::Point, controls: usize) -> usize {
    if controls == 0 || (endpoint.kind == PointKind::Curve && controls == 2) {
        1
    } else {
        controls
    }
}

/// Inserts on-curve points at missing horizontal and vertical curve extrema.
///
/// Extrema within 0.01 font units of an existing endpoint are treated as that
/// endpoint, matching FontForge's tolerant `ForceEndPointExtrema` policy.
pub fn add_extrema(contours: &[Contour]) -> Vec<Contour> {
    add_extrema_with_tolerance(contours, ENDPOINT_EXTREMA_TOLERANCE)
}

fn add_extrema_with_tolerance(contours: &[Contour], endpoint_tolerance: f64) -> Vec<Contour> {
    contours
        .iter()
        .map(|contour| {
            split_contour_with(contour, |segment| {
                segment
                    .extrema()
                    .into_iter()
                    .filter(|&t| interior_split(segment, t, endpoint_tolerance))
                    .collect()
            })
        })
        .collect()
}

/// Inserts on-curve points at every cubic inflection that is not already an endpoint.
pub fn add_inflections(contours: &[Contour]) -> Vec<Contour> {
    contours
        .iter()
        .map(|contour| {
            split_contour_with(contour, |segment| match segment {
                PathSeg::Cubic(cubic) => cubic
                    .inflections()
                    .into_iter()
                    .filter(|&t| interior_split(segment, t, ENDPOINT_EXTREMA_TOLERANCE))
                    .collect(),
                PathSeg::Line(_) | PathSeg::Quad(_) => Vec::new(),
            })
        })
        .collect()
}

/// Rounds every on-curve point and handle to a grid while preserving point metadata.
///
/// A non-finite or non-positive step leaves the contours unchanged.
pub fn round_to_grid(contours: &[Contour], step: f64) -> Vec<Contour> {
    if !step.is_finite() || step <= 0.0 {
        return contours.to_vec();
    }
    contours
        .iter()
        .cloned()
        .map(|mut contour| {
            for point in &mut contour.points {
                point.x = round_coordinate(point.x, step);
                point.y = round_coordinate(point.y, step);
            }
            contour
        })
        .collect()
}

/// Corrects contour winding using signed area and containment nesting parity.
pub fn correct_direction(contours: &[Contour], outer_clockwise: bool) -> Vec<Contour> {
    let polygons: Vec<_> = contours.iter().map(flatten_contour).collect();
    contours
        .iter()
        .enumerate()
        .map(|(index, contour)| {
            if !contour.closed || contour.points.len() < 2 {
                return contour.clone();
            }
            let Some(sample) = contour
                .points
                .iter()
                .find(|point| point.kind != PointKind::OffCurve)
                .map(|point| Point::new(point.x, point.y))
            else {
                return contour.clone();
            };
            let depth = polygons
                .iter()
                .enumerate()
                .filter(|(other, polygon)| *other != index && point_in_polygon(sample, polygon))
                .count();
            let desired_clockwise = if depth % 2 == 0 {
                outer_clockwise
            } else {
                !outer_clockwise
            };
            let area = contour.to_bezpath().area();
            if area.abs() <= 1e-12 {
                return contour.clone();
            }
            let clockwise = area < 0.0;
            if clockwise == desired_clockwise {
                contour.clone()
            } else {
                reverse(contour)
            }
        })
        .collect()
}

/// Rotates each closed contour to its leftmost, then lowest, on-curve point.
pub fn canonical_start_points(contours: &[Contour]) -> Vec<Contour> {
    contours
        .iter()
        .cloned()
        .map(|mut contour| {
            rotate_to_canonical_start(&mut contour);
            contour
        })
        .collect()
}

/// Produces deterministic contour starts and orders contours spatially.
pub fn canonical_order(contours: &[Contour]) -> Vec<Contour> {
    let mut result = canonical_start_points(contours);
    result.sort_by(|left, right| {
        let left_key = contour_sort_key(left);
        let right_key = contour_sort_key(right);
        left_key
            .0
            .total_cmp(&right_key.0)
            .then(left_key.1.total_cmp(&right_key.1))
            .then(right_key.2.total_cmp(&left_key.2))
            .then(left.points.len().cmp(&right.points.len()))
    });
    result
}

/// Reverses one contour while preserving its geometric segments.
pub fn reverse(contour: &Contour) -> Contour {
    let mut result = contour.clone();
    let on_curves: Vec<_> = result
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind != PointKind::OffCurve)
        .map(|(index, _)| index)
        .collect();
    if on_curves.is_empty() {
        result.points.reverse();
        return result;
    }
    let old_kinds: Vec<_> = on_curves
        .iter()
        .map(|&index| result.points[index].kind)
        .collect();
    if result.closed {
        for (position, &index) in on_curves.iter().enumerate() {
            result.points[index].kind = old_kinds[(position + 1) % old_kinds.len()];
        }
    } else {
        for (position, &index) in on_curves.iter().enumerate() {
            result.points[index].kind = old_kinds
                .get(position + 1)
                .copied()
                .unwrap_or(PointKind::Move);
        }
    }
    result.points.reverse();
    result
}

/// Applies an affine transform to points and handles while preserving metadata.
pub fn transform(contours: &[Contour], affine: Affine) -> Vec<Contour> {
    contours
        .iter()
        .cloned()
        .map(|mut contour| {
            for model_point in &mut contour.points {
                let transformed = affine * Point::new(model_point.x, model_point.y);
                model_point.x = transformed.x;
                model_point.y = transformed.y;
            }
            contour
        })
        .collect()
}

/// Finds the closest outline location no farther than `max_dist` font units.
pub fn nearest_on_path(contours: &[Contour], p: Point, max_dist: f64) -> Option<PathHit> {
    if max_dist.is_nan() || max_dist < 0.0 {
        return None;
    }
    let mut best: Option<PathHit> = None;
    for (contour_index, contour) in contours.iter().enumerate() {
        for (segment_index, segment) in contour.to_bezpath().segments().enumerate() {
            let nearest = segment.nearest(p, 1e-9);
            let distance = nearest.distance_sq.sqrt();
            if distance <= max_dist
                && best
                    .as_ref()
                    .is_none_or(|previous| distance < previous.distance)
            {
                best = Some(PathHit {
                    contour: contour_index,
                    segment: segment_index,
                    t: nearest.t,
                    point: segment.eval(nearest.t),
                    distance,
                });
            }
        }
    }
    best
}

/// Splits one segment with de Casteljau subdivision and inserts its on-curve point.
///
/// An out-of-range segment or a parameter outside the open interval `0..1`
/// leaves the contour unchanged.
pub fn split_segment(contour: &Contour, segment: usize, t: f64) -> Contour {
    if !t.is_finite() || t <= 0.0 || t >= 1.0 {
        return contour.clone();
    }
    split_contour_with_index(contour, segment, t)
}

/// Intersects every outline segment with a finite line segment.
///
/// Each tuple contains the point, contour index, segment index, and path-segment
/// parameter. Duplicate vertex hits are coalesced.
pub fn intersections_with_line(
    contours: &[Contour],
    line: Line,
) -> Vec<(Point, usize, usize, f64)> {
    let mut result = Vec::new();
    for (contour_index, contour) in contours.iter().enumerate() {
        for (segment_index, segment) in contour.to_bezpath().segments().enumerate() {
            for intersection in segment.intersect_line(line) {
                if (-1e-9..=1.0 + 1e-9).contains(&intersection.line_t)
                    && (-1e-9..=1.0 + 1e-9).contains(&intersection.segment_t)
                {
                    let t = intersection.segment_t.clamp(0.0, 1.0);
                    let point = segment.eval(t);
                    let duplicate = result.iter().any(
                        |(existing, existing_contour, _, _): &(Point, usize, usize, f64)| {
                            *existing_contour == contour_index
                                && (*existing - point).hypot() <= 1e-8
                        },
                    );
                    if !duplicate {
                        result.push((point, contour_index, segment_index, t));
                    }
                }
            }
        }
    }
    result.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then(left.2.cmp(&right.2))
            .then(left.3.total_cmp(&right.3))
    });
    result
}

/// Returns the tight union bounding box of all non-empty contours.
pub fn bbox(contours: &[Contour]) -> Option<Rect> {
    let mut bounds = None;
    for contour in contours {
        let path = contour.to_bezpath();
        let mut had_segment = false;
        for segment in path.segments() {
            had_segment = true;
            let segment_bounds = ParamCurveExtrema::bounding_box(&segment);
            bounds = Some(bounds.map_or(segment_bounds, |current: Rect| {
                current.union(segment_bounds)
            }));
        }
        if !had_segment {
            if let Some(point) = contour.points.first() {
                let point_bounds = Rect::new(point.x, point.y, point.x, point.y);
                bounds = Some(bounds.map_or(point_bounds, |current| current.union(point_bounds)));
            }
        }
    }
    bounds
}

fn split_contour_with(
    contour: &Contour,
    mut parameters: impl FnMut(&PathSeg) -> Vec<f64>,
) -> Contour {
    let path = contour.to_bezpath();
    let Some(first) = path.segments().next().map(|segment| segment.start()) else {
        return contour.clone();
    };
    let mut result = BezPath::new();
    result.move_to(first);
    let mut changed = false;
    for segment in path.segments() {
        let mut splits = parameters(&segment);
        splits.sort_by(f64::total_cmp);
        splits.dedup_by(|left, right| (*left - *right).abs() <= 1e-9);
        changed |= !splits.is_empty();
        append_split_segments(&mut result, segment, &splits);
    }
    if contour.closed {
        result.close_path();
    }
    if changed {
        contour_from_changed_path(&result, contour)
    } else {
        contour.clone()
    }
}

fn split_contour_with_index(contour: &Contour, target: usize, t: f64) -> Contour {
    let path = contour.to_bezpath();
    let Some(first) = path.segments().next().map(|segment| segment.start()) else {
        return contour.clone();
    };
    let mut result = BezPath::new();
    result.move_to(first);
    let mut found = false;
    for (index, segment) in path.segments().enumerate() {
        if index == target {
            append_split_segments(&mut result, segment, &[t]);
            found = true;
        } else {
            result.push(segment.as_path_el());
        }
    }
    if contour.closed {
        result.close_path();
    }
    if found {
        contour_from_changed_path(&result, contour)
    } else {
        contour.clone()
    }
}

fn append_split_segments(path: &mut BezPath, segment: PathSeg, splits: &[f64]) {
    let mut previous = 0.0;
    for &t in splits {
        if t > previous && t < 1.0 {
            path.push(segment.subsegment(previous..t).as_path_el());
            previous = t;
        }
    }
    path.push(segment.subsegment(previous..1.0).as_path_el());
}

fn interior_split(segment: &PathSeg, t: f64, tolerance: f64) -> bool {
    t > 1e-9
        && t < 1.0 - 1e-9
        && (segment.eval(t) - segment.start()).hypot() > tolerance
        && (segment.eval(t) - segment.end()).hypot() > tolerance
}

fn contour_from_changed_path(path: &BezPath, original: &Contour) -> Contour {
    let mut result = path_to_contours(path)
        .into_iter()
        .next()
        .unwrap_or_else(|| original.clone());
    restore_contour_metadata(&mut result, original);
    restore_matching_point_metadata(&mut result, original);
    result
}

fn restore_contour_metadata(result: &mut Contour, original: &Contour) {
    result.name.clone_from(&original.name);
    result.id.clone_from(&original.id);
}

fn restore_matching_point_metadata(result: &mut Contour, original: &Contour) {
    for point in &mut result.points {
        if point.kind == PointKind::OffCurve {
            continue;
        }
        if let Some(source) = original.points.iter().find(|source| {
            source.kind != PointKind::OffCurve
                && (source.x - point.x).abs() <= 1e-9
                && (source.y - point.y).abs() <= 1e-9
        }) {
            point.smooth = source.smooth;
            point.corner_radius = source.corner_radius;
            point.name.clone_from(&source.name);
            point.tt_index = source.tt_index;
        }
    }
}

fn round_coordinate(coordinate: f64, step: f64) -> f64 {
    if coordinate.is_finite() {
        (coordinate / step).round() * step
    } else {
        coordinate
    }
}

fn rotate_to_canonical_start(contour: &mut Contour) {
    if !contour.closed {
        return;
    }
    let start = contour
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind != PointKind::OffCurve)
        .min_by(|(_, left), (_, right)| left.x.total_cmp(&right.x).then(left.y.total_cmp(&right.y)))
        .map(|(index, _)| index);
    if let Some(index) = start {
        contour.points.rotate_left(index);
    }
}

fn contour_sort_key(contour: &Contour) -> (f64, f64, f64) {
    let min_x = contour
        .points
        .iter()
        .map(|point| point.x)
        .min_by(f64::total_cmp)
        .unwrap_or(f64::INFINITY);
    let min_y = contour
        .points
        .iter()
        .map(|point| point.y)
        .min_by(f64::total_cmp)
        .unwrap_or(f64::INFINITY);
    (min_x, min_y, contour.to_bezpath().area().abs())
}

fn flatten_contour(contour: &Contour) -> Vec<Point> {
    let mut points = Vec::new();
    kurbo::flatten(contour.to_bezpath().iter(), 0.05, |element| {
        if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
            points.push(point);
        }
    });
    points
}

fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let a = polygon[current];
        let b = polygon[previous];
        if (a.y > point.y) != (b.y > point.y) {
            let crossing = (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x;
            if point.x < crossing {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn snap_nearly_hv_lines(contour: &mut Contour, tolerance: f64) {
    let len = contour.points.len();
    if len < 2 {
        return;
    }
    for index in 0..len {
        if contour.points[index].kind != PointKind::Line {
            continue;
        }
        let previous = if index == 0 {
            if !contour.closed {
                continue;
            }
            len - 1
        } else {
            index - 1
        };
        if contour.points[previous].kind == PointKind::OffCurve {
            continue;
        }
        if (contour.points[index].x - contour.points[previous].x).abs() <= tolerance {
            contour.points[index].x = contour.points[previous].x;
        }
        if (contour.points[index].y - contour.points[previous].y).abs() <= tolerance {
            contour.points[index].y = contour.points[previous].y;
        }
    }
}

fn force_near_lines(path: &BezPath, tolerance: f64) -> BezPath {
    let Some(first) = path.segments().next().map(|segment| segment.start()) else {
        return path.clone();
    };
    let closed = path
        .elements()
        .iter()
        .any(|element| matches!(element, PathEl::ClosePath));
    let mut result = BezPath::new();
    result.move_to(first);
    for segment in path.segments() {
        let cubic = segment.to_cubic();
        let chord = Line::new(cubic.p0, cubic.p3);
        if distance_to_line(cubic.p1, chord) <= tolerance
            && distance_to_line(cubic.p2, chord) <= tolerance
        {
            result.line_to(cubic.p3);
        } else {
            result.push(segment.as_path_el());
        }
    }
    if closed {
        result.close_path();
    }
    result
}

fn distance_to_line(point: Point, line: Line) -> f64 {
    let vector = line.p1 - line.p0;
    let length = vector.hypot();
    if length <= f64::EPSILON {
        (point - line.p0).hypot()
    } else {
        ((point - line.p0).cross(vector) / length).abs()
    }
}
