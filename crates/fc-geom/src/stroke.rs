use std::ops::Range;

use fc_model::Contour;
use kurbo::{
    fit_to_bezpath, fit_to_cubic, stroke, BezPath, CurveFitSample, ParamCurve, ParamCurveDeriv,
    ParamCurveFit, PathEl, PathSeg, Point, Shape, Stroke, StrokeOpts, Vec2,
};

use crate::{
    from_bezpaths, to_bezpaths, Cap, CurveKind, Error, FallbackKind, Join,
    DEFAULT_BOOLEAN_TOLERANCE,
};

/// Shape of the pen used to expand a centerline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Nib {
    /// Circular pen whose diameter is [`StrokeOptions::width`].
    Round,
    /// Rotated rectangular calligraphic pen.
    Calligraphic {
        /// Pen width before rotation.
        width: f64,
        /// Pen height before rotation.
        height: f64,
        /// Counter-clockwise rotation in radians.
        angle: f64,
    },
    /// Axis-aligned square pen whose side is [`StrokeOptions::width`].
    Rect,
}

/// Options for expanding centerline contours into filled outlines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeOptions {
    /// Diameter of a round nib and side length of a rectangular nib.
    pub width: f64,
    /// Pen shape.
    pub nib: Nib,
    /// End-cap style.
    pub cap: Cap,
    /// Segment-join style.
    pub join: Join,
    /// Miter length limit as a multiple of stroke width.
    pub miter_limit: f64,
    /// Drop the inner boundary ring of a closed stroke.
    pub remove_internal: bool,
    /// Drop the outer boundary ring (or the sole ring of an open stroke).
    pub remove_external: bool,
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            width: 1.0,
            nib: Nib::Round,
            cap: Cap::Round,
            join: Join::Round,
            miter_limit: 4.0,
            remove_internal: false,
            remove_external: false,
        }
    }
}

/// Result of stroke expansion together with its geometry-quality status.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeOutcome {
    /// Expanded stroke contours.
    pub contours: Vec<Contour>,
    /// Fallback used to produce the contours.
    ///
    /// User interfaces should show a warning whenever this is not
    /// [`FallbackKind::None`].
    pub fallback: FallbackKind,
}

/// Expands centerline contours into closed filled outlines.
///
/// Round nibs use kurbo's offset-based stroker. `Rect` and `Calligraphic`
/// nibs trace both sides with the nib support function, fit translated source
/// spans back to cubic Béziers, and insert nib sweeps at caps and joins.
/// `flo_curves::stroke_path` is circular-only, so it cannot represent these two
/// nib shapes directly.
pub fn expand_stroke(
    contours: &[Contour],
    opts: &StrokeOptions,
    kind: CurveKind,
) -> Result<Vec<Contour>, Error> {
    validate_options(opts)?;
    expand_stroke_impl(contours, opts, kind, false).map(|outcome| outcome.contours)
}

/// Expands centerline contours, falling back to a flattened NonZero union when
/// the strict curve-preserving stroke-ring union fails its area oracle.
///
/// User interfaces should warn when the returned [`StrokeOutcome::fallback`]
/// is not [`FallbackKind::None`]. Invalid parameters are returned without
/// fallback.
pub fn expand_stroke_with_fallback(
    contours: &[Contour],
    opts: &StrokeOptions,
    kind: CurveKind,
) -> Result<StrokeOutcome, Error> {
    validate_options(opts)?;
    expand_stroke_impl(contours, opts, kind, true)
}

fn expand_stroke_impl(
    contours: &[Contour],
    opts: &StrokeOptions,
    kind: CurveKind,
    allow_fallback: bool,
) -> Result<StrokeOutcome, Error> {
    match opts.nib {
        Nib::Round => round_stroke(contours, opts, kind, allow_fallback),
        Nib::Rect => {
            let half = opts.width * 0.5;
            convex_nib_stroke(
                contours,
                rectangle_corners(half, half, 0.0),
                opts,
                kind,
                allow_fallback,
            )
        }
        Nib::Calligraphic {
            width,
            height,
            angle,
        } => convex_nib_stroke(
            contours,
            rectangle_corners(width * 0.5, height * 0.5, angle),
            opts,
            kind,
            allow_fallback,
        ),
    }
}

fn validate_options(opts: &StrokeOptions) -> Result<(), Error> {
    if !opts.width.is_finite() || opts.width <= 0.0 {
        return Err(Error::InvalidParameter {
            parameter: "stroke width",
        });
    }
    if !opts.miter_limit.is_finite() || opts.miter_limit <= 0.0 {
        return Err(Error::InvalidParameter {
            parameter: "miter limit",
        });
    }
    if let Nib::Calligraphic {
        width,
        height,
        angle,
    } = opts.nib
    {
        if !width.is_finite() || width <= 0.0 || !height.is_finite() || height <= 0.0 {
            return Err(Error::InvalidParameter {
                parameter: "calligraphic nib dimensions",
            });
        }
        if !angle.is_finite() {
            return Err(Error::InvalidParameter {
                parameter: "calligraphic nib angle",
            });
        }
    }
    Ok(())
}

fn round_stroke(
    contours: &[Contour],
    opts: &StrokeOptions,
    kind: CurveKind,
    allow_fallback: bool,
) -> Result<StrokeOutcome, Error> {
    if opts.remove_internal && opts.remove_external {
        return Ok(StrokeOutcome {
            contours: Vec::new(),
            fallback: FallbackKind::None,
        });
    }
    let style = Stroke::new(opts.width)
        .with_join(opts.join)
        .with_miter_limit(opts.miter_limit)
        .with_caps(opts.cap);
    let mut expanded = Vec::new();
    for (source, path) in contours.iter().zip(to_bezpaths(contours)) {
        let mut stroked = stroke(
            path.iter(),
            &style,
            &StrokeOpts::default(),
            DEFAULT_BOOLEAN_TOLERANCE,
        );
        // Kurbo returns round/square start caps to the first coordinate without
        // emitting ClosePath. Stroke output is filled geometry: explicitly close
        // it so booleans and outline writers include the resulting capsule.
        if !stroked.is_empty() && !matches!(stroked.elements().last(), Some(PathEl::ClosePath)) {
            stroked.close_path();
        }
        let rings = crate::boolean::normalize_self_intersecting_contours(from_bezpaths(
            &[stroked],
            CurveKind::Cubic,
        ))?;
        let rings = if source.closed {
            // Kurbo can split one smooth offset span at several implied
            // quadratic endpoints. Re-fit only tangent-continuous cubic runs,
            // with an error small relative to the nib diameter.
            compact_round_rings(&rings, opts.width * 0.025)
        } else {
            rings
        };
        expanded.extend(filter_stroke_rings(source, rings, opts));
    }
    union_stroke_rings(
        &expanded,
        kind,
        allow_fallback,
        contours.iter().any(|contour| contour.closed)
            && !opts.remove_internal
            && !opts.remove_external,
    )
}

fn compact_round_rings(rings: &[Contour], accuracy: f64) -> Vec<Contour> {
    let paths: Vec<_> = rings
        .iter()
        .map(|ring| compact_cubic_runs(&ring.to_bezpath(), accuracy))
        .collect();
    from_bezpaths(&paths, CurveKind::Cubic)
}

fn compact_cubic_runs(path: &BezPath, accuracy: f64) -> BezPath {
    let segments: Vec<_> = path.segments().collect();
    let Some(first) = segments.first() else {
        return path.clone();
    };
    let mut compacted = Vec::new();
    let mut start = 0;
    while start < segments.len() {
        if !matches!(segments[start], PathSeg::Cubic(_)) {
            compacted.push(segments[start]);
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < segments.len()
            && matches!(segments[end], PathSeg::Cubic(_))
            && smoothly_connected(segments[end - 1], segments[end])
        {
            end += 1;
        }
        compacted.extend(compact_cubic_run(&segments[start..end], accuracy));
        start = end;
    }
    let mut result = BezPath::new();
    result.move_to(first.start());
    for segment in compacted {
        result.push(segment.as_path_el());
    }
    if path
        .elements()
        .iter()
        .any(|element| matches!(element, PathEl::ClosePath))
    {
        result.close_path();
    }
    result
}

fn smoothly_connected(left: PathSeg, right: PathSeg) -> bool {
    let left_tangent = left.to_cubic().deriv().eval(1.0).to_vec2();
    let right_tangent = right.to_cubic().deriv().eval(0.0).to_vec2();
    left_tangent.dot(right_tangent) > 0.0
        && left_tangent.cross(right_tangent).abs()
            <= left_tangent.hypot() * right_tangent.hypot() * 0.01
}

fn compact_cubic_run(run: &[PathSeg], accuracy: f64) -> Vec<PathSeg> {
    let mut result = Vec::new();
    let mut start = 0;
    while start < run.len() {
        let mut fitted = None;
        for end in ((start + 2)..=run.len()).rev() {
            let source = CubicRunFit(&run[start..end]);
            if let Some((curve, _)) = fit_to_cubic(&source, 0.0..1.0, accuracy) {
                fitted = Some((end, PathSeg::Cubic(curve)));
                break;
            }
        }
        if let Some((end, curve)) = fitted {
            result.push(curve);
            start = end;
        } else {
            result.push(run[start]);
            start += 1;
        }
    }
    result
}

struct CubicRunFit<'a>(&'a [PathSeg]);

impl ParamCurveFit for CubicRunFit<'_> {
    fn sample_pt_tangent(&self, t: f64, _: f64) -> CurveFitSample {
        let (p, tangent) = self.sample_pt_deriv(t);
        CurveFitSample { p, tangent }
    }

    fn sample_pt_deriv(&self, t: f64) -> (Point, Vec2) {
        let scaled = t.clamp(0.0, 1.0) * self.0.len() as f64;
        let index = (scaled.floor() as usize).min(self.0.len() - 1);
        let local = if index + 1 == self.0.len() && t >= 1.0 {
            1.0
        } else {
            scaled - index as f64
        };
        let curve = self.0[index].to_cubic();
        (
            curve.eval(local),
            curve.deriv().eval(local).to_vec2() * self.0.len() as f64,
        )
    }

    fn break_cusp(&self, _: Range<f64>) -> Option<f64> {
        None
    }
}

fn convex_nib_stroke(
    contours: &[Contour],
    corners: [Vec2; 4],
    opts: &StrokeOptions,
    kind: CurveKind,
    allow_fallback: bool,
) -> Result<StrokeOutcome, Error> {
    if opts.remove_internal && opts.remove_external {
        return Ok(StrokeOutcome {
            contours: Vec::new(),
            fallback: FallbackKind::None,
        });
    }
    let mut expanded = Vec::new();
    for (source, path) in contours.iter().zip(to_bezpaths(contours)) {
        let paths = trace_convex_nib(&path, source.closed, &corners, opts);
        let rings = from_bezpaths(&paths, CurveKind::Cubic);
        let rings = if source.closed {
            crate::boolean::normalize_self_intersecting_contours(rings)?
        } else {
            rings
        };
        expanded.extend(filter_stroke_rings(source, rings, opts));
    }
    union_stroke_rings(
        &expanded,
        kind,
        allow_fallback,
        contours.iter().any(|contour| contour.closed)
            && !opts.remove_internal
            && !opts.remove_external,
    )
}

fn union_stroke_rings(
    rings: &[Contour],
    kind: CurveKind,
    allow_fallback: bool,
    preserve_holes: bool,
) -> Result<StrokeOutcome, Error> {
    let strict = if preserve_holes {
        crate::boolean::remove_stroke_overlap(rings, kind, DEFAULT_BOOLEAN_TOLERANCE)
    } else {
        crate::remove_overlap(rings, kind, DEFAULT_BOOLEAN_TOLERANCE)
    };
    match strict {
        Ok(contours) => Ok(StrokeOutcome {
            contours,
            fallback: FallbackKind::None,
        }),
        Err(Error::BooleanFailed { .. } | Error::BooleanMismatch { .. }) if allow_fallback => {
            crate::boolean::fallback_remove_overlap(rings, kind, DEFAULT_BOOLEAN_TOLERANCE).map(
                |contours| StrokeOutcome {
                    contours,
                    fallback: FallbackKind::Flattened,
                },
            )
        }
        Err(error) => Err(error),
    }
}

fn rectangle_corners(half_width: f64, half_height: f64, angle: f64) -> [Vec2; 4] {
    let (sin, cos) = angle.sin_cos();
    let rotate = |x: f64, y: f64| Vec2::new(x * cos - y * sin, x * sin + y * cos);
    [
        rotate(-half_width, -half_height),
        rotate(half_width, -half_height),
        rotate(half_width, half_height),
        rotate(-half_width, half_height),
    ]
}

fn filter_stroke_rings(
    source: &Contour,
    mut rings: Vec<Contour>,
    opts: &StrokeOptions,
) -> Vec<Contour> {
    if rings.is_empty() {
        return rings;
    }
    if !source.closed || rings.len() == 1 {
        return if opts.remove_external {
            Vec::new()
        } else {
            rings
        };
    }
    let external = rings
        .iter()
        .enumerate()
        .max_by(|left, right| {
            left.1
                .to_bezpath()
                .area()
                .abs()
                .total_cmp(&right.1.to_bezpath().area().abs())
        })
        .map(|(index, _)| index);
    rings
        .drain(..)
        .enumerate()
        .filter(|(index, _)| {
            let is_external = Some(*index) == external;
            (!is_external || !opts.remove_external) && (is_external || !opts.remove_internal)
        })
        .map(|(_, contour)| contour)
        .collect()
}

fn trace_convex_nib(
    path: &BezPath,
    closed: bool,
    corners: &[Vec2; 4],
    opts: &StrokeOptions,
) -> Vec<BezPath> {
    let segments: Vec<_> = path
        .segments()
        .filter(|segment| (segment.end() - segment.start()).hypot() > 1e-9)
        .collect();
    if segments.is_empty() {
        return Vec::new();
    }
    let left = trace_side(&segments, 1.0, corners, opts, closed);
    let right = trace_side(&segments, -1.0, corners, opts, closed);
    if closed {
        let (external, internal) = if left.area().abs() >= right.area().abs() {
            (left, right)
        } else {
            (right, left)
        };
        return vec![external, internal.reverse_subpaths()];
    }

    let mut outline = left;
    let end = segments.last().map(PathSeg::end).unwrap_or(Point::ORIGIN);
    let end_tangent = tangent_at(*segments.last().unwrap_or(&segments[0]), 1.0);
    let left_end = outline
        .elements()
        .iter()
        .rev()
        .find_map(PathEl::end_point)
        .unwrap_or(end);
    let reversed_right = right.reverse_subpaths();
    let right_end = reversed_right
        .elements()
        .first()
        .and_then(PathEl::end_point)
        .unwrap_or(end);
    append_cap(
        &mut outline,
        end,
        left_end,
        right_end,
        end_tangent,
        false,
        corners,
        opts.cap,
    );
    append_path(&mut outline, &reversed_right);
    let start = segments[0].start();
    let start_tangent = tangent_at(segments[0], 0.0);
    let right_start = outline
        .elements()
        .iter()
        .rev()
        .find_map(PathEl::end_point)
        .unwrap_or(start);
    let left_start = outline
        .elements()
        .first()
        .and_then(PathEl::end_point)
        .unwrap_or(start);
    append_cap(
        &mut outline,
        start,
        right_start,
        left_start,
        start_tangent,
        true,
        corners,
        opts.cap,
    );
    outline.close_path();
    vec![outline]
}

fn trace_side(
    segments: &[PathSeg],
    side: f64,
    corners: &[Vec2; 4],
    opts: &StrokeOptions,
    closed: bool,
) -> BezPath {
    let traces: Vec<_> = segments
        .iter()
        .map(|segment| trace_segment(*segment, side, corners))
        .collect();
    let mut output = traces[0].path.clone();
    for pair in traces.windows(2) {
        append_join(
            &mut output,
            pair[0].end - pair[0].end_offset,
            &pair[0],
            &pair[1],
            side,
            corners,
            opts,
        );
        append_path(&mut output, &pair[1].path);
    }
    if closed {
        let last = traces.len() - 1;
        append_join(
            &mut output,
            segments[last].end(),
            &traces[last],
            &traces[0],
            side,
            corners,
            opts,
        );
        output.close_path();
    }
    output
}

struct SegmentTrace {
    path: BezPath,
    start: Point,
    end: Point,
    start_offset: Vec2,
    end_offset: Vec2,
    start_tangent: Vec2,
    end_tangent: Vec2,
}

fn trace_segment(segment: PathSeg, side: f64, corners: &[Vec2; 4]) -> SegmentTrace {
    let intervals = support_intervals(segment, side, corners);
    let mut path = BezPath::new();
    let mut start = segment.start();
    let mut end = segment.end();
    let mut start_offset = Vec2::ZERO;
    let mut end_offset = Vec2::ZERO;
    for (index, interval) in intervals.iter().enumerate() {
        let translated = TranslatedSegment {
            segment,
            range: interval.range.clone(),
            offset: interval.offset,
        };
        let fitted = fit_to_bezpath(&translated, DEFAULT_BOOLEAN_TOLERANCE);
        if index == 0 {
            start = translated.sample_pt_deriv(0.0).0;
            start_offset = interval.offset;
            path.extend(&fitted);
        } else {
            let next = translated.sample_pt_deriv(0.0).0;
            path.line_to(next);
            append_path(&mut path, &fitted);
        }
        end = translated.sample_pt_deriv(1.0).0;
        end_offset = interval.offset;
    }
    SegmentTrace {
        path,
        start,
        end,
        start_offset,
        end_offset,
        start_tangent: tangent_at(segment, 0.0),
        end_tangent: tangent_at(segment, 1.0),
    }
}

struct SupportInterval {
    range: Range<f64>,
    offset: Vec2,
}

fn support_intervals(segment: PathSeg, side: f64, corners: &[Vec2; 4]) -> Vec<SupportInterval> {
    const STEPS: usize = 64;
    let feature_at = |t| {
        let tangent = tangent_at(segment, t);
        support_feature(corners, Vec2::new(-tangent.y * side, tangent.x * side))
    };
    let (mut feature, mut offset) = feature_at(0.0);
    let mut start = 0.0;
    let mut result = Vec::new();
    for step in 1..=STEPS {
        let t = step as f64 / STEPS as f64;
        let (next_feature, next_offset) = feature_at(t);
        if next_feature != feature {
            let mut low = (step - 1) as f64 / STEPS as f64;
            let mut high = t;
            for _ in 0..40 {
                let middle = (low + high) * 0.5;
                if feature_at(middle).0 == feature {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            let boundary = (low + high) * 0.5;
            if boundary > start + 1e-9 {
                result.push(SupportInterval {
                    range: start..boundary,
                    offset,
                });
            }
            start = boundary;
            feature = next_feature;
            offset = next_offset;
        }
    }
    if start < 1.0 - 1e-9 {
        result.push(SupportInterval {
            range: start..1.0,
            offset,
        });
    }
    if result.is_empty() {
        result.push(SupportInterval {
            range: 0.0..1.0,
            offset,
        });
    }
    result
}

fn support_feature(corners: &[Vec2; 4], direction: Vec2) -> (u8, Vec2) {
    let scale = direction.hypot().max(1.0);
    let max_projection = corners
        .iter()
        .map(|corner| corner.dot(direction))
        .fold(f64::NEG_INFINITY, f64::max);
    let mut feature = 0_u8;
    let mut sum = Vec2::ZERO;
    let mut count = 0.0;
    for (index, corner) in corners.iter().enumerate() {
        if (corner.dot(direction) - max_projection).abs() <= scale * 1e-10 {
            feature |= 1 << index;
            sum += *corner;
            count += 1.0;
        }
    }
    (feature, sum / count)
}

fn tangent_at(segment: PathSeg, t: f64) -> Vec2 {
    let cubic = segment.to_cubic();
    let derivative = cubic.deriv();
    let mut tangent = derivative.eval(t.clamp(0.0, 1.0)).to_vec2();
    if tangent.hypot2() <= 1e-18 {
        let delta = 1e-5;
        let before = cubic.eval((t - delta).max(0.0));
        let after = cubic.eval((t + delta).min(1.0));
        tangent = after - before;
    }
    if tangent.hypot2() <= 1e-18 {
        tangent = cubic.p3 - cubic.p0;
    }
    tangent
}

struct TranslatedSegment {
    segment: PathSeg,
    range: Range<f64>,
    offset: Vec2,
}

impl ParamCurveFit for TranslatedSegment {
    fn sample_pt_tangent(&self, t: f64, _: f64) -> CurveFitSample {
        let (p, tangent) = self.sample_pt_deriv(t);
        CurveFitSample { p, tangent }
    }

    fn sample_pt_deriv(&self, t: f64) -> (Point, Vec2) {
        let span = self.range.end - self.range.start;
        let source_t = self.range.start + t * span;
        (
            self.segment.eval(source_t) + self.offset,
            tangent_at(self.segment, source_t) * span,
        )
    }

    fn break_cusp(&self, _: Range<f64>) -> Option<f64> {
        None
    }
}

fn append_path(output: &mut BezPath, path: &BezPath) {
    for element in path.elements() {
        if !matches!(element, PathEl::MoveTo(_)) {
            output.push(*element);
        }
    }
}

fn append_join(
    output: &mut BezPath,
    center: Point,
    incoming: &SegmentTrace,
    outgoing: &SegmentTrace,
    side: f64,
    corners: &[Vec2; 4],
    opts: &StrokeOptions,
) {
    if (incoming.end - outgoing.start).hypot() <= 1e-9 {
        return;
    }
    let outside = incoming.end_tangent.cross(outgoing.start_tangent) * side < 0.0;
    match opts.join {
        kurbo::Join::Bevel if outside => output.line_to(outgoing.start),
        kurbo::Join::Miter if outside => {
            let miter = line_intersection(
                incoming.end,
                incoming.end_tangent,
                outgoing.start,
                outgoing.start_tangent,
            );
            let limit = corners
                .iter()
                .map(|corner| corner.hypot())
                .fold(0.0, f64::max)
                * opts.miter_limit;
            if let Some(point) = miter.filter(|point| (*point - center).hypot() <= limit) {
                output.line_to(point);
            }
            output.line_to(outgoing.start);
        }
        kurbo::Join::Round if outside => {
            let direction = (incoming.end_offset + outgoing.start_offset).normalize();
            append_nib_sweep(
                output,
                center,
                incoming.end_offset,
                outgoing.start_offset,
                direction,
                corners,
            );
        }
        kurbo::Join::Bevel | kurbo::Join::Miter | kurbo::Join::Round => {
            output.line_to(outgoing.start);
        }
    }
}

fn line_intersection(p: Point, d: Vec2, q: Point, e: Vec2) -> Option<Point> {
    let denominator = d.cross(e);
    if denominator.abs() <= 1e-12 {
        None
    } else {
        Some(p + d * ((q - p).cross(e) / denominator))
    }
}

#[allow(clippy::too_many_arguments)]
fn append_cap(
    output: &mut BezPath,
    center: Point,
    from: Point,
    to: Point,
    tangent: Vec2,
    start: bool,
    corners: &[Vec2; 4],
    cap: kurbo::Cap,
) {
    let direction = tangent.normalize() * if start { -1.0 } else { 1.0 };
    match cap {
        kurbo::Cap::Butt => output.line_to(to),
        kurbo::Cap::Square => {
            let extension = corners
                .iter()
                .map(|corner| corner.dot(direction).abs())
                .fold(0.0, f64::max);
            output.line_to(from + direction * extension);
            output.line_to(to + direction * extension);
            output.line_to(to);
        }
        kurbo::Cap::Round => append_nib_sweep(
            output,
            center,
            from - center,
            to - center,
            direction,
            corners,
        ),
    }
}

fn append_nib_sweep(
    output: &mut BezPath,
    center: Point,
    from: Vec2,
    to: Vec2,
    outward: Vec2,
    corners: &[Vec2; 4],
) {
    let from_angle = from.y.atan2(from.x);
    let to_angle = to.y.atan2(to.x);
    let mut candidates: Vec<_> = corners
        .iter()
        .copied()
        .filter(|corner| corner.dot(outward) > 0.0)
        .collect();
    candidates.sort_by(|left, right| {
        angular_progress(from_angle, left.y.atan2(left.x), outward).total_cmp(&angular_progress(
            from_angle,
            right.y.atan2(right.x),
            outward,
        ))
    });
    for corner in candidates {
        let progress = angular_progress(from_angle, corner.y.atan2(corner.x), outward);
        let total = angular_progress(from_angle, to_angle, outward);
        if progress > 1e-9 && progress < total - 1e-9 {
            output.line_to(center + corner);
        }
    }
    output.line_to(center + to);
}

fn angular_progress(from: f64, angle: f64, outward: Vec2) -> f64 {
    let ccw = (angle - from).rem_euclid(std::f64::consts::TAU);
    let direction_angle = outward.y.atan2(outward.x);
    let direction_ccw = (direction_angle - from).rem_euclid(std::f64::consts::TAU);
    if direction_ccw <= std::f64::consts::PI {
        ccw
    } else {
        (-angle + from).rem_euclid(std::f64::consts::TAU)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dejavu_two_forced_stroke_fallback_is_compact_and_within_one_percent() {
        let path = fc_fixtures::path("DejaVuSans.ttf");
        if !path.is_file() {
            eprintln!("skipping test: DejaVuSans.ttf is unavailable");
            return;
        }
        let read =
            fc_sfnt::read_path(&path, &fc_sfnt::ReadOptions::default()).expect("read DejaVu Sans");
        let font = &read.fonts[0];
        let contours = &font
            .glyph("two")
            .and_then(|glyph| glyph.master_layer(&font.default_master().id))
            .expect("two master layer")
            .contours;
        let opts = StrokeOptions {
            width: 20.0,
            nib: Nib::Round,
            cap: Cap::Round,
            join: Join::Round,
            miter_limit: 4.0,
            remove_internal: false,
            remove_external: false,
        };
        let strict = expand_stroke(contours, &opts, CurveKind::Cubic).expect("strict stroke two");
        crate::boolean::set_corrupt_flo_result_for_test(true);
        let outcome = expand_stroke_with_fallback(contours, &opts, CurveKind::Cubic);
        crate::boolean::set_corrupt_flo_result_for_test(false);
        let outcome = outcome.expect("forced stroke fallback");
        assert_eq!(outcome.fallback, FallbackKind::Flattened);
        let points = outcome
            .contours
            .iter()
            .map(|contour| contour.points.len())
            .sum::<usize>();
        assert!(points <= 300, "fallback has {points} points");
        let strict_points = strict
            .iter()
            .map(|contour| contour.points.len())
            .sum::<usize>();
        assert!(
            points <= strict_points * 2,
            "strict has {strict_points} points; fallback has {points}"
        );
        let strict_area = strict
            .iter()
            .map(|contour| contour.to_bezpath().area())
            .sum::<f64>()
            .abs();
        let fallback_area = outcome
            .contours
            .iter()
            .map(|contour| contour.to_bezpath().area())
            .sum::<f64>()
            .abs();
        assert!(
            (strict_area - fallback_area).abs() / strict_area < 0.01,
            "strict area={strict_area}, fallback area={fallback_area}"
        );
    }
}
