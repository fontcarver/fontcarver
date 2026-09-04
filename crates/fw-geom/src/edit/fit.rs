//! Least-squares fitting of one segment onto a sampled outline span.

use kurbo::{
    cubics_to_quadratic_splines, CubicBez, Line, ParamCurveNearest, PathSeg, Point, QuadBez,
    QuadSpline, Vec2,
};

use crate::{CurveKind, DEFAULT_SIMPLIFY_TOLERANCE};

/// Maximum least-squares refits performed by [`fit_span`] for a cubic span.
const FIT_REFINEMENTS: usize = 16;

/// One model segment produced by [`fit_span`].
///
/// Each variant maps onto exactly one point-list segment: no handles for a
/// line, two handles and a cubic endpoint for [`SpanFit::Cubic`], and the
/// TrueType-style control run of [`SpanFit::Quadratic`] with its implied
/// on-curve midpoints.
#[derive(Clone, Debug, PartialEq)]
pub enum SpanFit {
    /// A straight line to the last sample.
    Line,
    /// One cubic with these two control points.
    Cubic(Point, Point),
    /// A quadratic chain with these control points and implied midpoints.
    Quadratic(Vec<Point>),
}

/// Fits one segment through a sampled span while honouring both end tangents.
///
/// `samples` is the polyline of the span to reproduce, in order; the fitted
/// segment starts at the first sample and ends at the last. `start_tangent`
/// points the way the segment leaves the first sample and `end_tangent` the way
/// it arrives at the last one. Both are normalised internally, and a degenerate
/// tangent falls back to the chord direction.
///
/// [`CurveKind::Cubic`] solves for the two handle lengths along those tangents
/// by least squares over a chord-length parameterisation, refining the
/// parameters against the fitted curve. [`CurveKind::Quadratic`] first tries the
/// single quadratic whose control point is where the two tangent lines meet and
/// keeps it when it stays within `max_err`; otherwise it converts the
/// least-squares cubic into a quadratic spline within the same tolerance, which
/// is still one point-list segment. Parallel or backward tangents, fewer than
/// three samples, non-finite coordinates, and any fit that would sit farther
/// from the samples than the plain chord all fall back to [`SpanFit::Line`], so
/// this function always returns a usable segment.
pub fn fit_span(
    samples: &[Point],
    start_tangent: Vec2,
    end_tangent: Vec2,
    kind: CurveKind,
) -> SpanFit {
    let clean: Vec<Point> = samples
        .iter()
        .copied()
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .collect();
    let (Some(&start), Some(&end)) = (clean.first(), clean.last()) else {
        return SpanFit::Line;
    };
    if clean.len() < 3 {
        return SpanFit::Line;
    }
    let chord = end - start;
    let outgoing = unit_or(start_tangent, chord);
    let incoming = unit_or(end_tangent, chord);
    let straight = span_deviation(&PathSeg::Line(Line::new(start, end)), &clean);
    let cubic = fit_cubic(&clean, start, end, outgoing, incoming);
    match kind {
        CurveKind::Cubic => match cubic {
            Some(cubic) if span_deviation(&PathSeg::Cubic(cubic), &clean) < straight => {
                SpanFit::Cubic(cubic.p1, cubic.p2)
            }
            _ => SpanFit::Line,
        },
        CurveKind::Quadratic { max_err } => {
            let tolerance = if max_err.is_finite() && max_err > 0.0 {
                max_err
            } else {
                DEFAULT_SIMPLIFY_TOLERANCE
            };
            let single = fit_quadratic(start, end, outgoing, incoming)
                .map(|quad| (span_deviation(&PathSeg::Quad(quad), &clean), quad));
            if let Some((error, quad)) = single {
                if error <= tolerance {
                    return SpanFit::Quadratic(vec![quad.p1]);
                }
            }
            let spline = cubic
                .and_then(|cubic| cubics_to_quadratic_splines(&[cubic], tolerance))
                .and_then(|splines| splines.into_iter().next())
                .map(|spline| (spline_deviation(&spline, &clean), spline));
            let spline_error = spline.as_ref().map_or(f64::INFINITY, |(error, _)| *error);
            let single_error = single.as_ref().map_or(f64::INFINITY, |(error, _)| *error);
            if spline_error <= single_error && spline_error < straight {
                if let Some((_, spline)) = spline {
                    let points = spline.points();
                    let controls = points
                        .get(1..points.len().saturating_sub(1))
                        .unwrap_or_default();
                    if !controls.is_empty() {
                        return SpanFit::Quadratic(controls.to_vec());
                    }
                }
            }
            match single {
                Some((error, quad)) if error < straight => SpanFit::Quadratic(vec![quad.p1]),
                _ => SpanFit::Line,
            }
        }
    }
}

fn unit_or(tangent: Vec2, fallback: Vec2) -> Vec2 {
    for candidate in [tangent, fallback] {
        let length = candidate.hypot();
        if length.is_finite() && length > 1e-12 {
            return candidate / length;
        }
    }
    Vec2::new(1.0, 0.0)
}

fn fit_quadratic(start: Point, end: Point, outgoing: Vec2, incoming: Vec2) -> Option<QuadBez> {
    let chord = end - start;
    let determinant = outgoing.cross(incoming);
    if determinant.abs() <= 1e-12 {
        return None;
    }
    let along = chord.cross(incoming) / determinant;
    let back = -chord.cross(outgoing) / determinant;
    if !along.is_finite() || !back.is_finite() || along <= 0.0 || back <= 0.0 {
        return None;
    }
    Some(QuadBez::new(start, start + outgoing * along, end))
}

fn fit_cubic(
    samples: &[Point],
    start: Point,
    end: Point,
    outgoing: Vec2,
    incoming: Vec2,
) -> Option<CubicBez> {
    let heuristic = (end - start).hypot() / 3.0;
    if !heuristic.is_finite() || heuristic <= 0.0 {
        return None;
    }
    let goal = (heuristic * 3e-7).max(1e-9);
    let mut parameters = chord_parameters(samples);
    let mut best: Option<(f64, CubicBez)> = None;
    for _ in 0..FIT_REFINEMENTS {
        let candidate = solve_cubic_handles(
            samples,
            &parameters,
            start,
            end,
            outgoing,
            incoming,
            heuristic,
        );
        let error = span_deviation(&PathSeg::Cubic(candidate), samples);
        let improved = best
            .as_ref()
            .is_none_or(|(best_error, _)| error < *best_error);
        if improved {
            best = Some((error, candidate));
        }
        if error <= goal || !improved {
            break;
        }
        parameters = samples
            .iter()
            .map(|&sample| PathSeg::Cubic(candidate).nearest(sample, 1e-9).t)
            .collect();
    }
    best.map(|(_, segment)| segment)
}

fn solve_cubic_handles(
    samples: &[Point],
    parameters: &[f64],
    start: Point,
    end: Point,
    outgoing: Vec2,
    incoming: Vec2,
    heuristic: f64,
) -> CubicBez {
    let mut normal = [0.0_f64; 3];
    let mut projection = [0.0_f64; 2];
    for (sample, &t) in samples.iter().zip(parameters.iter()) {
        let inverse = 1.0 - t;
        let basis = [
            inverse * inverse * inverse,
            3.0 * t * inverse * inverse,
            3.0 * t * t * inverse,
            t * t * t,
        ];
        let first = outgoing * basis[1];
        let second = -incoming * basis[2];
        let residual = sample.to_vec2()
            - start.to_vec2() * (basis[0] + basis[1])
            - end.to_vec2() * (basis[2] + basis[3]);
        normal[0] += first.dot(first);
        normal[1] += first.dot(second);
        normal[2] += second.dot(second);
        projection[0] += first.dot(residual);
        projection[1] += second.dot(residual);
    }
    let determinant = normal[0].mul_add(normal[2], -(normal[1] * normal[1]));
    let (mut along, mut back) = if determinant.abs() > 1e-12 {
        (
            projection[0].mul_add(normal[2], -(projection[1] * normal[1])) / determinant,
            normal[0].mul_add(projection[1], -(normal[1] * projection[0])) / determinant,
        )
    } else {
        (heuristic, heuristic)
    };
    if !along.is_finite() || along <= 1e-9 {
        along = heuristic;
    }
    if !back.is_finite() || back <= 1e-9 {
        back = heuristic;
    }
    CubicBez::new(start, start + outgoing * along, end - incoming * back, end)
}

fn spline_deviation(spline: &QuadSpline, samples: &[Point]) -> f64 {
    let quads: Vec<PathSeg> = spline.to_quads().map(PathSeg::Quad).collect();
    samples
        .iter()
        .map(|&sample| {
            quads
                .iter()
                .map(|quad| quad.nearest(sample, 1e-6).distance_sq)
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0_f64, f64::max)
        .sqrt()
}

fn chord_parameters(samples: &[Point]) -> Vec<f64> {
    let mut lengths = Vec::with_capacity(samples.len());
    let mut total = 0.0;
    lengths.push(0.0);
    for window in samples.windows(2) {
        total += (window[1] - window[0]).hypot();
        lengths.push(total);
    }
    if total <= 1e-12 {
        let last = samples.len().saturating_sub(1).max(1) as f64;
        return (0..samples.len())
            .map(|index| index as f64 / last)
            .collect();
    }
    lengths.iter().map(|length| length / total).collect()
}

fn span_deviation(segment: &PathSeg, samples: &[Point]) -> f64 {
    samples
        .iter()
        .map(|&sample| segment.nearest(sample, 1e-6).distance_sq)
        .fold(0.0_f64, f64::max)
        .sqrt()
}
