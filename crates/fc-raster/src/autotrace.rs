//! Curve fitting for raster autotrace.
//!
//! The rings that [`crate::trace_bitmap`] returns are exact pixel boundaries:
//! unit steps along pixel edges. This module turns one such ring into a small
//! number of cubic segments and straight lines.
//!
//! The pipeline is the classical one.
//!
//! 1. The ring is optionally low-pass filtered.
//! 2. A digital-straightness sweep tiles the ring with its maximal straight runs.
//!    Those runs are the structural polygon: their directions are reliable where a
//!    Douglas–Peucker polygon of a pixel staircase would invent corners out of
//!    quantisation noise. The turn between two consecutive runs decides corner
//!    against smooth.
//! 3. A run long enough to be meant as a straight edge — which separates the side
//!    of a stem from the flat spot a curve leaves at an extreme — is trimmed until
//!    its chord is faithful and then kept as a line.
//! 4. The corner samples and the kept runs' ends cut the ring into pieces. A kept
//!    run is a line; every other piece is fitted with least-squares cubics
//!    ([`fc_geom::fit_span`]) that are split at their worst sample until the piece
//!    is inside the error tolerance.
//!
//! A pixel ring is a quantised sampling of the drawn edge. A one-bit raster of a
//! curve keeps a flat run of several pixels at every extreme, so no smooth curve
//! can follow such a ring closer than about one pixel however many nodes it is
//! given. The tolerance is therefore a target, not a guarantee: a piece is split
//! only while splitting actually reduces its deviation, and the deviation that
//! remains is reported so a review can see it. Raising `smoothing` lowers that
//! floor at the cost of detail.
//!
//! Every stage is bounded, the ring orientation is preserved, and a ring whose fit
//! cannot be represented keeps its exact pixel edges.

use fc_geom::{fit_span, CurveKind, SpanFit};
use fc_model::{Contour, Point as ModelPoint, PointKind};
use kurbo::{CubicBez, Line, ParamCurveNearest, PathSeg, Point, Vec2};

/// Largest number of ring samples a single contour may be fitted from.
const MAX_SAMPLES: usize = 200_000;
/// Largest number of smoothing passes, whatever radius is requested.
const MAX_SMOOTH_PASSES: usize = 16;
/// Largest number of fitted segments produced for one ring.
const MAX_SEGMENTS: usize = 4_096;
/// Deepest adaptive split of one piece.
const MAX_SPLIT_DEPTH: usize = 12;
/// Share of a piece's deviation a split must remove to be worth the extra nodes.
///
/// Without this, a piece whose deviation is the pixel ring's own quantisation is
/// split until every segment is a one-step line, because no curve can ever reach
/// the requested tolerance.
const SPLIT_GAIN: f64 = 0.8;
/// Sample steps the straightness sweep and run trimming may spend on one ring.
const STRAIGHT_BUDGET: usize = 4_000_000;
/// Deviation at or below which a run counts as straight, in source pixels.
///
/// A rasterized straight edge is a staircase of unit steps whose distance from
/// the ideal line stays below one pixel, so this separates a real straight run
/// from a curve.
const STRAIGHT_FLOOR: f64 = 0.75;
/// Factor on `sqrt(STRAIGHT_FLOOR * extent)` that a run must reach to be a line.
///
/// A circle of radius `r` has maximal straight runs of about `2 sqrt(2 r f)`
/// pixels, so a threshold that grows with the square root of the drawing's own
/// size keeps a circle curved at every scale while a stem of any size stays
/// straight.
const LINE_SCALE: f64 = 3.0;

/// Curve-fitting settings for one autotrace run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitOptions {
    /// Largest permitted deviation from the pixel ring, in source pixels.
    pub tolerance: f64,
    /// Turn angle in degrees at or above which a polygon vertex stays a corner.
    pub corner_angle: f64,
    /// Low-pass radius applied to the ring before fitting, in source pixels.
    pub smoothing: f64,
}

impl Default for FitOptions {
    fn default() -> Self {
        Self {
            tolerance: 1.0,
            corner_angle: 35.0,
            smoothing: 0.0,
        }
    }
}

/// What fitting one ring produced.
pub(crate) struct RingFit {
    /// The fitted contour, or the exact ring when fitting was refused.
    pub contour: Contour,
    /// Largest sampled deviation from the ring, in source pixels.
    pub error: f64,
    /// Whether the exact pixel edges were kept instead of a fit.
    pub fallback: bool,
}

/// Fit one closed pixel ring with cubics and lines.
///
/// `ring` is the exact pixel boundary in source-pixel coordinates, in order and
/// without a repeated closing point. The returned contour is closed. A ring with
/// fewer than three points, a non-finite coordinate, more than [`MAX_SAMPLES`]
/// points, invalid options or a fit that cannot be represented keeps its exact
/// edges and reports `fallback`.
pub(crate) fn fit_ring(ring: &[Point], options: &FitOptions) -> RingFit {
    let refused = || RingFit {
        contour: exact_contour(ring),
        error: 0.0,
        fallback: true,
    };
    if ring.len() < 3
        || ring.len() > MAX_SAMPLES
        || ring.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
        || !options.tolerance.is_finite()
        || options.tolerance <= 0.0
        || !options.corner_angle.is_finite()
        || !options.smoothing.is_finite()
        || options.smoothing < 0.0
    {
        return refused();
    }
    let samples = smooth_ring(ring, options.smoothing);
    let mut budget = STRAIGHT_BUDGET;
    // The tiling depends on where it starts, so the first pass only locates the
    // sharpest turn and the second starts there, which keeps the seam off a curve.
    let scouted = straight_tiling(&samples, 0, &mut budget);
    if scouted.len() < 3 {
        return refused();
    }
    let span = turn_span(&samples, options.tolerance);
    let seed = scouted
        .iter()
        .copied()
        .max_by(|&a, &b| turn_at(&samples, a, span).total_cmp(&turn_at(&samples, b, span)))
        .unwrap_or(scouted[0]);
    let tiling = straight_tiling(&samples, seed, &mut budget);
    if tiling.len() < 3 {
        return refused();
    }
    let corners = corner_cuts(&samples, &tiling, options);
    let lines = line_runs(&samples, &tiling, &corners, options, &mut budget);
    let cuts = order_cuts(&samples, &corners, &lines);
    let Some(segments) = fit_pieces(&samples, &cuts, &lines, options) else {
        return refused();
    };
    let error = segments.iter().fold(0.0_f64, |worst, s| worst.max(s.error));
    match assemble(&samples, &segments) {
        Some(contour) => RingFit {
            contour,
            error,
            fallback: false,
        },
        None => refused(),
    }
}

/// One fitted piece of a ring, between two consecutive cuts.
struct Segment {
    /// Ring sample the segment starts at.
    start: usize,
    /// Ring sample the segment ends at, which may wrap.
    end: usize,
    /// Cubic control points, or `None` for a straight segment.
    controls: Option<(Point, Point)>,
    /// Largest sampled deviation of this segment from the ring.
    error: f64,
    /// Whether the node at `start` keeps tangent continuity.
    smooth_start: bool,
}

/// A corner of the ring and the two polygon directions that meet there.
struct Corner {
    sample: usize,
    incoming: Vec2,
    outgoing: Vec2,
}

/// A cut in the ring, where one piece ends and the next begins.
struct Cut {
    sample: usize,
    corner: bool,
    /// Tangent the arriving piece ends along.
    incoming: Vec2,
    /// Tangent the leaving piece starts along.
    outgoing: Vec2,
}

fn exact_contour(ring: &[Point]) -> Contour {
    Contour {
        closed: true,
        points: ring
            .iter()
            .map(|p| ModelPoint {
                x: p.x,
                y: p.y,
                kind: PointKind::Line,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

/// Low-pass filter a closed ring with repeated `(1, 2, 1) / 4` passes.
///
/// The radius is in source pixels and the pixel ring has unit steps, so two
/// passes per pixel of radius give a visually proportional amount of smoothing.
fn smooth_ring(ring: &[Point], smoothing: f64) -> Vec<Point> {
    let passes = ((smoothing.max(0.0) * 2.0).round() as usize).min(MAX_SMOOTH_PASSES);
    let mut current = ring.to_vec();
    for _ in 0..passes {
        let n = current.len();
        let next: Vec<Point> = (0..n)
            .map(|i| {
                let previous = current[(i + n - 1) % n].to_vec2();
                let point = current[i].to_vec2();
                let following = current[(i + 1) % n].to_vec2();
                ((previous + point * 2.0 + following) * 0.25).to_point()
            })
            .collect();
        current = next;
    }
    current
}

/// Tile a closed ring with its maximal straight runs, starting at `seed`.
///
/// The returned indices are the run starts in ring order; run `i` spans from
/// vertex `i` to vertex `i + 1`, cyclically. The last run closes back onto the
/// seed, so the seam should be a real corner.
fn straight_tiling(ring: &[Point], seed: usize, budget: &mut usize) -> Vec<usize> {
    let n = ring.len();
    let mut vertices = vec![seed];
    let mut index = seed;
    let mut consumed = 0usize;
    while consumed < n && *budget > 0 && vertices.len() < MAX_SEGMENTS {
        let end = extend_run(ring, index, budget);
        let step = (end + n - index) % n;
        if step == 0 {
            break;
        }
        consumed += step;
        index = end;
        if consumed >= n {
            break;
        }
        vertices.push(index);
    }
    vertices
}

/// Turn angle in radians at one ring sample, measured over `span` pixels each way.
///
/// The chords of the straight runs that meet at a vertex are not a reliable
/// measure: the staircase at an extreme leaves single-step runs whose direction is
/// axis aligned, which would read as a right-angle corner. Reading the direction
/// over a fixed path distance on each side keeps a real corner sharp and a curve
/// smooth.
fn turn_at(ring: &[Point], index: usize, span: f64) -> f64 {
    let n = ring.len();
    let reach = |forward: bool| -> Option<Vec2> {
        let mut travelled = 0.0;
        let mut current = index;
        for _ in 0..n {
            let next = if forward {
                (current + 1) % n
            } else {
                (current + n - 1) % n
            };
            travelled += ring[next].distance(ring[current]);
            current = next;
            if travelled >= span {
                break;
            }
        }
        let offset = if forward {
            ring[current] - ring[index]
        } else {
            ring[index] - ring[current]
        };
        (offset.hypot() > f64::EPSILON).then_some(offset)
    };
    match (reach(false), reach(true)) {
        (Some(incoming), Some(outgoing)) => incoming
            .normalize()
            .dot(outgoing.normalize())
            .clamp(-1.0, 1.0)
            .acos(),
        _ => 0.0,
    }
}

/// Path distance on each side of a vertex that its turn angle is measured over.
///
/// The window is a small share of the drawing's own size. A fixed window reads
/// the staircase instead of the shape once the drawing is more than a few hundred
/// pixels across, which invents corners all over a smooth curve.
fn turn_span(ring: &[Point], tolerance: f64) -> f64 {
    let extent = ring_extent(ring);
    (0.02 * extent)
        .max(2.0 + 2.0 * tolerance)
        .max(3.0)
        .min((0.25 * extent).max(3.0))
}

/// Largest side of the ring's bounding box, in source pixels.
fn ring_extent(ring: &[Point]) -> f64 {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for point in ring {
        x0 = x0.min(point.x);
        y0 = y0.min(point.y);
        x1 = x1.max(point.x);
        y1 = y1.max(point.y);
    }
    (x1 - x0).max(y1 - y0).max(0.0)
}

/// Tiling vertices whose local turn reaches `corner_angle`, with their directions.
fn corner_cuts(ring: &[Point], tiling: &[usize], options: &FitOptions) -> Vec<Corner> {
    let n = ring.len();
    let threshold = options.corner_angle.clamp(0.0, 180.0).to_radians();
    let span = turn_span(ring, options.tolerance);
    tiling
        .iter()
        .copied()
        .filter(|&sample| turn_at(ring, sample, span) >= threshold)
        .map(|sample| {
            // Local chords, not whole-run chords, so a corner between two curves
            // keeps the direction each side actually arrives and leaves along.
            let mut before = sample;
            let mut after = sample;
            let mut travelled = 0.0;
            while travelled < span {
                let next = (before + n - 1) % n;
                travelled += ring[next].distance(ring[before]);
                before = next;
                if before == sample {
                    break;
                }
            }
            travelled = 0.0;
            while travelled < span {
                let next = (after + 1) % n;
                travelled += ring[next].distance(ring[after]);
                after = next;
                if after == sample {
                    break;
                }
            }
            Corner {
                sample,
                incoming: ring[sample] - ring[before],
                outgoing: ring[after] - ring[sample],
            }
        })
        .collect()
}

/// Smallest chord length, in source pixels, at which a straight run is a line.
fn min_line_length(ring: &[Point], tolerance: f64) -> f64 {
    let extent = ring_extent(ring);
    (3.0_f64)
        .max(2.0 * tolerance)
        .max(LINE_SCALE * (STRAIGHT_FLOOR * extent).sqrt())
}

/// Tiling runs long and faithful enough to be drawn as straight lines.
///
/// A run is kept when its chord reaches [`min_line_length`], it still does after
/// being trimmed so its own chord stays within the fitting tolerance, and it holds
/// no corner.
fn line_runs(
    ring: &[Point],
    tiling: &[usize],
    corners: &[Corner],
    options: &FitOptions,
    budget: &mut usize,
) -> Vec<(usize, usize)> {
    let n = ring.len();
    let m = tiling.len();
    let min_length = min_line_length(ring, options.tolerance);
    let limit = options.tolerance.max(STRAIGHT_FLOOR);
    let mut kept: Vec<(usize, usize)> = Vec::new();
    for k in 0..m {
        let (start, end) = (tiling[k], tiling[(k + 1) % m]);
        if ring[end].distance(ring[start]) < min_length {
            continue;
        }
        let Some((a, b)) = trim_run(ring, start, end, limit, budget) else {
            continue;
        };
        if ring[b].distance(ring[a]) >= min_length
            && !corners
                .iter()
                .any(|corner| strictly_inside(n, a, b, corner.sample))
        {
            kept.push((a, b));
        }
    }
    kept
}

/// Whether `sample` lies strictly between `a` and `b` in ring order.
fn strictly_inside(n: usize, a: usize, b: usize, sample: usize) -> bool {
    let span = (b + n - a) % n;
    let offset = (sample + n - a) % n;
    offset > 0 && offset < span
}

/// Farthest sample reachable from `start` while one straight line fits the run.
///
/// The admissible directions of that line form an angular interval that can only
/// shrink, so the test costs constant time per sample.
fn extend_run(ring: &[Point], start: usize, budget: &mut usize) -> usize {
    let n = ring.len();
    let mut end = start;
    let mut low = f64::NEG_INFINITY;
    let mut high = f64::INFINITY;
    let mut reference: Option<f64> = None;
    for _ in 0..n {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        let next = (end + 1) % n;
        let offset = ring[next] - ring[start];
        let distance = offset.hypot();
        if distance > STRAIGHT_FLOOR {
            let half = (STRAIGHT_FLOOR / distance).clamp(-1.0, 1.0).asin();
            let mut center = offset.y.atan2(offset.x);
            match reference {
                None => reference = Some(center),
                Some(anchor) => {
                    while center - anchor > std::f64::consts::PI {
                        center -= std::f64::consts::TAU;
                    }
                    while center - anchor < -std::f64::consts::PI {
                        center += std::f64::consts::TAU;
                    }
                }
            }
            let (candidate_low, candidate_high) = (low.max(center - half), high.min(center + half));
            if candidate_low > candidate_high {
                break;
            }
            low = candidate_low;
            high = candidate_high;
        }
        end = next;
        if end == start {
            break;
        }
    }
    end
}

/// Shrink a run until its own chord is within `limit` of every sample it covers.
fn trim_run(
    ring: &[Point],
    start: usize,
    end: usize,
    limit: f64,
    budget: &mut usize,
) -> Option<(usize, usize)> {
    let n = ring.len();
    let (mut a, mut b) = (start, end);
    for _ in 0..n {
        let span = (b + n - a) % n;
        if span < 1 || *budget == 0 {
            return None;
        }
        let chord = PathSeg::Line(Line::new(ring[a], ring[b]));
        let mut worst = (0.0_f64, 0usize);
        for offset in 1..span {
            let index = (a + offset) % n;
            let distance = chord.nearest(ring[index], 1e-6).distance_sq.max(0.0).sqrt();
            if distance > worst.0 {
                worst = (distance, offset);
            }
        }
        *budget = budget.saturating_sub(span);
        if worst.0 <= limit {
            return Some((a, b));
        }
        // The curved part sits at one end; drop the sample on that side.
        if worst.1 * 2 <= span {
            a = (a + 1) % n;
        } else {
            b = (b + n - 1) % n;
        }
    }
    None
}

/// Cuts in ring order: every corner plus both ends of every kept line run.
fn order_cuts(ring: &[Point], corners: &[Corner], lines: &[(usize, usize)]) -> Vec<Cut> {
    let mut samples: Vec<usize> = corners.iter().map(|corner| corner.sample).collect();
    for &(a, b) in lines {
        samples.push(a);
        samples.push(b);
    }
    samples.sort_unstable();
    samples.dedup();
    let count = samples.len();
    samples
        .iter()
        .enumerate()
        .map(|(index, &sample)| {
            let corner = corners.iter().find(|corner| corner.sample == sample);
            let before = lines.iter().find(|&&(_, b)| b == sample);
            let after = lines.iter().find(|&&(a, _)| a == sample);
            let (incoming, outgoing) = if let Some(corner) = corner {
                (corner.incoming, corner.outgoing)
            } else {
                // A straight run fixes the tangent of the curve continuing from
                // it, so the join between a side and a corner arc has no kink.
                let shared = if let Some(&(a, b)) = before {
                    ring[b] - ring[a]
                } else if let Some(&(a, b)) = after {
                    ring[b] - ring[a]
                } else {
                    let previous = samples[(index + count - 1) % count];
                    let next = samples[(index + 1) % count];
                    let incoming = ring[sample] - ring[previous];
                    let outgoing = ring[next] - ring[sample];
                    if incoming.hypot() > f64::EPSILON && outgoing.hypot() > f64::EPSILON {
                        incoming.normalize() + outgoing.normalize()
                    } else {
                        outgoing
                    }
                };
                (shared, shared)
            };
            Cut {
                sample,
                corner: corner.is_some(),
                incoming,
                outgoing,
            }
        })
        .collect()
}

/// Fit every piece between consecutive cuts, in ring order.
fn fit_pieces(
    ring: &[Point],
    cuts: &[Cut],
    lines: &[(usize, usize)],
    options: &FitOptions,
) -> Option<Vec<Segment>> {
    let mut segments = Vec::new();
    if cuts.is_empty() {
        // A ring with no corner and no straight run is one closed piece, seamed
        // at its first sample with the tangent of the chord through its ends.
        let samples = piece_samples(ring, 0, 0);
        let points: Vec<Point> = samples.iter().map(|&index| ring[index]).collect();
        let tangent = local_tangent(&points, 0);
        segments = fit_piece(ring, &samples, tangent, tangent, true, options, 0).0;
        return (!segments.is_empty()).then_some(segments);
    }
    for (index, cut) in cuts.iter().enumerate() {
        let next = &cuts[(index + 1) % cuts.len()];
        let samples = piece_samples(ring, cut.sample, next.sample);
        if samples.len() < 2 {
            return None;
        }
        let straight = lines
            .iter()
            .any(|&(a, b)| a == cut.sample && b == next.sample);
        if straight {
            let points: Vec<Point> = samples.iter().map(|&i| ring[i]).collect();
            let error = deviation(
                &PathSeg::Line(Line::new(points[0], points[points.len() - 1])),
                &points,
            );
            segments.push(Segment {
                start: cut.sample,
                end: next.sample,
                controls: None,
                error,
                smooth_start: !cut.corner,
            });
        } else {
            segments.extend(
                fit_piece(
                    ring,
                    &samples,
                    cut.outgoing,
                    next.incoming,
                    !cut.corner,
                    options,
                    0,
                )
                .0,
            );
        }
        if segments.len() > MAX_SEGMENTS {
            return None;
        }
    }
    (!segments.is_empty()).then_some(segments)
}

/// Ring sample indices from `start` to `end` inclusive, wrapping once.
///
/// `start == end` describes the whole ring, so the seam sample appears at both
/// ends and the piece closes.
fn piece_samples(ring: &[Point], start: usize, end: usize) -> Vec<usize> {
    let n = ring.len();
    let mut samples = vec![start];
    let mut index = start;
    loop {
        index = (index + 1) % n;
        samples.push(index);
        if index == end || samples.len() > n {
            break;
        }
    }
    samples
}

/// Fit one piece, splitting at its worst sample while splitting still helps.
///
/// Returns the segments of the piece in order, together with their largest
/// deviation. A split is kept only when it removes at least [`SPLIT_GAIN`] of the
/// single segment's deviation, so quantisation noise cannot drive the node count.
#[allow(clippy::too_many_arguments)]
fn fit_piece(
    ring: &[Point],
    samples: &[usize],
    start_tangent: Vec2,
    end_tangent: Vec2,
    smooth_start: bool,
    options: &FitOptions,
    depth: usize,
) -> (Vec<Segment>, f64) {
    let (Some(&start), Some(&end)) = (samples.first(), samples.last()) else {
        return (Vec::new(), 0.0);
    };
    if samples.len() < 2 {
        return (Vec::new(), 0.0);
    }
    let points: Vec<Point> = samples.iter().map(|&index| ring[index]).collect();
    let chord = PathSeg::Line(Line::new(points[0], points[points.len() - 1]));
    let straight = deviation(&chord, &points);
    // A piece that close to its chord is drawn faithfully by the simpler line.
    if straight <= options.tolerance.max(STRAIGHT_FLOOR) {
        return (
            vec![Segment {
                start,
                end,
                controls: None,
                error: straight,
                smooth_start,
            }],
            straight,
        );
    }
    let (controls, error) = match fit_span(&points, start_tangent, end_tangent, CurveKind::Cubic) {
        SpanFit::Cubic(c1, c2) => {
            let curve = CubicBez::new(points[0], c1, c2, points[points.len() - 1]);
            (Some((c1, c2)), deviation(&PathSeg::Cubic(curve), &points))
        }
        _ => (None, straight),
    };
    let single = vec![Segment {
        start,
        end,
        controls,
        error,
        smooth_start,
    }];
    let segment = match controls {
        Some((c1, c2)) => {
            PathSeg::Cubic(CubicBez::new(points[0], c1, c2, points[points.len() - 1]))
        }
        None => chord,
    };
    let split = worst_sample(&segment, &points);
    let divisible = points.len() >= 5 && split >= 2 && split + 2 < points.len();
    if error <= options.tolerance || depth >= MAX_SPLIT_DEPTH || !divisible {
        return (single, error);
    }
    let tangent = local_tangent(&points, split);
    let (mut left, left_error) = fit_piece(
        ring,
        &samples[..=split],
        start_tangent,
        tangent,
        smooth_start,
        options,
        depth + 1,
    );
    let (right, right_error) = fit_piece(
        ring,
        &samples[split..],
        tangent,
        end_tangent,
        true,
        options,
        depth + 1,
    );
    let divided = left_error.max(right_error);
    if left.is_empty() || right.is_empty() || divided > error * SPLIT_GAIN {
        return (single, error);
    }
    left.extend(right);
    (left, divided)
}

/// Direction of the ring at one sample, measured over a window of samples.
///
/// A pixel ring steps along pixel edges, so the chord between two neighbouring
/// samples is axis aligned and says nothing about the real tangent. The window is a
/// share of the piece being fitted, so the direction is read over a scale that is
/// proportional to the curve it has to start or end.
fn local_tangent(points: &[Point], index: usize) -> Vec2 {
    let window = (points.len() / 20).clamp(2, points.len() / 4 + 1);
    let last = points.len().saturating_sub(1);
    for reach in (1..=window).rev() {
        let before = index.saturating_sub(reach);
        let after = (index + reach).min(last);
        if before == after {
            continue;
        }
        // A closed piece repeats its seam, so read across it rather than through.
        let (before, after) = if index == 0 && points.first() == points.last() {
            (last.saturating_sub(reach), reach.min(last))
        } else {
            (before, after)
        };
        let direction = points[after] - points[before];
        if direction.hypot() > f64::EPSILON {
            return direction;
        }
    }
    points[last] - points[0]
}

/// Largest distance from a fitted segment to the samples it should reproduce.
fn deviation(segment: &PathSeg, samples: &[Point]) -> f64 {
    samples.iter().fold(0.0_f64, |worst, &point| {
        worst.max(segment.nearest(point, 1e-6).distance_sq.max(0.0).sqrt())
    })
}

/// Index of the sample farthest from the fitted segment.
fn worst_sample(segment: &PathSeg, samples: &[Point]) -> usize {
    let mut worst = (f64::NEG_INFINITY, 0usize);
    for (index, &point) in samples.iter().enumerate() {
        let distance = segment.nearest(point, 1e-6).distance_sq;
        if distance > worst.0 {
            worst = (distance, index);
        }
    }
    worst.1
}

/// Build the closed point list of a fitted ring.
///
/// Each on-curve point carries the role of the segment arriving at it, which is
/// what the model's closed-contour reader expects, so the two control points of
/// the closing segment trail the last on-curve point.
fn assemble(ring: &[Point], segments: &[Segment]) -> Option<Contour> {
    let last = segments.last()?;
    let first = ring.get(segments.first()?.start)?;
    let mut points = Vec::with_capacity(segments.len() * 3);
    points.push(ModelPoint {
        x: first.x,
        y: first.y,
        kind: if last.controls.is_some() {
            PointKind::Curve
        } else {
            PointKind::Line
        },
        smooth: segments.first()?.smooth_start,
        ..Default::default()
    });
    for (index, segment) in segments.iter().enumerate() {
        if let Some((c1, c2)) = segment.controls {
            for control in [c1, c2] {
                if !control.x.is_finite() || !control.y.is_finite() {
                    return None;
                }
                points.push(ModelPoint {
                    x: control.x,
                    y: control.y,
                    kind: PointKind::OffCurve,
                    ..Default::default()
                });
            }
        }
        if index + 1 == segments.len() {
            break;
        }
        let end = ring.get(segment.end)?;
        points.push(ModelPoint {
            x: end.x,
            y: end.y,
            kind: if segment.controls.is_some() {
                PointKind::Curve
            } else {
                PointKind::Line
            },
            smooth: segments[index + 1].smooth_start,
            ..Default::default()
        });
    }
    let contour = Contour {
        closed: true,
        points,
        ..Default::default()
    };
    // A point list the model cannot read back is never offered to a document.
    contour.try_to_bezpath().ok()?;
    Some(contour)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Circle, Shape};

    /// Pixel-edge ring of a filled region, as `trace_bitmap` would produce it.
    fn ring_of_ink(size: usize, ink: impl Fn(f64, f64) -> bool) -> Vec<Point> {
        let mut bitmap = fc_model::Bitmap::new(size as u16, size as u16, 1).expect("bitmap");
        for y in 0..size {
            for x in 0..size {
                if ink(x as f64 + 0.5, y as f64 + 0.5) {
                    bitmap.set_pixel(x as u16, y as u16, 1).expect("pixel");
                }
            }
        }
        let traced = crate::trace_bitmap(&bitmap, 1, 1, 1).expect("trace");
        assert_eq!(traced.len(), 1, "fixture must produce one ring");
        traced[0]
            .points
            .iter()
            .map(|p| Point::new(p.x, p.y))
            .collect()
    }

    /// Pixel-edge ring of a filled shape, as `trace_bitmap` would produce it.
    fn ring_of(shape: &impl Shape, size: usize) -> Vec<Point> {
        let path = shape.to_path(0.05);
        ring_of_ink(size, |x, y| path.winding(Point::new(x, y)) != 0)
    }

    /// Pixel-edge ring of an analytic disc, which needs no path winding test.
    fn disc_ring(size: usize, centre: f64, radius: f64) -> Vec<Point> {
        ring_of_ink(size, |x, y| (x - centre).hypot(y - centre) <= radius)
    }

    fn nodes(fit: &RingFit) -> usize {
        fit.contour
            .points
            .iter()
            .filter(|p| p.kind != PointKind::OffCurve)
            .count()
    }

    fn handles(fit: &RingFit) -> usize {
        fit.contour
            .points
            .iter()
            .filter(|p| p.kind == PointKind::OffCurve)
            .count()
    }

    #[test]
    fn a_rasterized_square_keeps_four_corners_and_four_lines() {
        let ring = ring_of(&kurbo::Rect::new(20., 20., 80., 80.), 100);
        let fit = fit_ring(&ring, &FitOptions::default());
        assert!(!fit.fallback);
        assert_eq!(handles(&fit), 0, "a square must not gain handles");
        assert_eq!(nodes(&fit), 4);
        assert!(fit.error <= 0.75, "square error {}", fit.error);
    }

    #[test]
    fn a_rasterized_circle_is_fitted_with_few_curves_inside_the_tolerance() {
        let size = 200;
        let radius = 80.;
        let ring = ring_of(&Circle::new((100., 100.), radius), size);
        let fit = fit_ring(&ring, &FitOptions::default());
        assert!(!fit.fallback);
        let count = nodes(&fit);
        assert!(
            (4..=12).contains(&count),
            "circle fitted with {count} nodes from {} ring points",
            ring.len()
        );
        assert!(count * 8 < ring.len(), "fit did not reduce the point count");
        assert!(
            fit.contour
                .points
                .iter()
                .filter(|p| p.kind != PointKind::OffCurve)
                .all(|p| p.smooth),
            "a ring without corners must have only smooth nodes"
        );
        // Measure the fit against the analytic circle, allowing the half pixel
        // the pixel ring itself sits away from it plus the fitting tolerance.
        let path = fit.contour.try_to_bezpath().expect("fitted contour");
        let mut worst = 0.0_f64;
        for step in 0..720 {
            let angle = step as f64 * std::f64::consts::TAU / 720.;
            let point = Point::new(100. + radius * angle.cos(), 100. + radius * angle.sin());
            // The y-up conversion of the traced ring mirrors the fixture.
            let mirrored = Point::new(point.x, size as f64 - point.y);
            let nearest = path
                .segments()
                .map(|segment| segment.nearest(mirrored, 1e-6).distance_sq)
                .fold(f64::INFINITY, f64::min);
            worst = worst.max(nearest.sqrt());
        }
        assert!(worst < 2.0, "fitted circle deviates {worst} from analytic");
    }

    #[test]
    fn a_high_resolution_circle_is_fitted_to_a_fraction_of_a_pixel() {
        let size = 1000;
        let radius = 400.;
        let ring = disc_ring(size, 500., radius);
        let started = std::time::Instant::now();
        let fit = fit_ring(&ring, &FitOptions::default());
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
        assert!(!fit.fallback);
        let count = nodes(&fit);
        assert!(
            (4..=32).contains(&count),
            "circle fitted with {count} nodes from {} ring points",
            ring.len()
        );
        assert!(
            count * 40 < ring.len(),
            "fit did not reduce the point count"
        );
        let path = fit.contour.try_to_bezpath().expect("fitted contour");
        let mut worst = 0.0_f64;
        for step in 0..2000 {
            let angle = step as f64 * std::f64::consts::TAU / 2000.;
            let point = Point::new(500. + radius * angle.cos(), 500. + radius * angle.sin());
            let mirrored = Point::new(point.x, size as f64 - point.y);
            let nearest = path
                .segments()
                .map(|segment| segment.nearest(mirrored, 1e-6).distance_sq)
                .fold(f64::INFINITY, f64::min);
            worst = worst.max(nearest.sqrt());
        }
        assert!(
            worst < 1.5,
            "a clean high-resolution circle deviates {worst} from analytic"
        );
    }

    #[test]
    fn a_rounded_rectangle_keeps_its_straight_sides_straight() {
        let ring = ring_of(&kurbo::RoundedRect::new(20., 20., 180., 180., 30.), 200);
        let fit = fit_ring(&ring, &FitOptions::default());
        assert!(!fit.fallback);
        let lines = fit
            .contour
            .points
            .iter()
            .filter(|p| p.kind == PointKind::Line)
            .count();
        assert!(lines >= 4, "expected four straight sides, found {lines}");
        assert!(handles(&fit) >= 8, "expected four fitted corner arcs");
        assert!(fit.error <= 1.5, "rounded rect error {}", fit.error);
    }

    #[test]
    fn corner_angle_zero_keeps_every_structural_vertex_as_a_corner() {
        let ring = ring_of(&Circle::new((100., 100.), 80.), 200);
        let fit = fit_ring(
            &ring,
            &FitOptions {
                corner_angle: 0.0,
                ..FitOptions::default()
            },
        );
        assert!(!fit.fallback);
        assert_eq!(
            handles(&fit),
            0,
            "every vertex is a corner, so no curve may be fitted"
        );
        assert!(nodes(&fit) > 12, "the polygon must keep its vertices");
    }

    #[test]
    fn a_looser_tolerance_produces_fewer_nodes() {
        let ring = ring_of(&Circle::new((150., 150.), 120.), 300);
        let counts: Vec<usize> = [0.5, 2.0, 8.0]
            .into_iter()
            .map(|tolerance| {
                nodes(&fit_ring(
                    &ring,
                    &FitOptions {
                        tolerance,
                        ..FitOptions::default()
                    },
                ))
            })
            .collect();
        assert!(
            counts[0] >= counts[1] && counts[1] >= counts[2],
            "node counts {counts:?} must not grow with the tolerance"
        );
    }

    #[test]
    fn a_stem_with_round_ends_keeps_both_sides_straight() {
        let mut shape = kurbo::BezPath::new();
        shape.move_to((80., 40.));
        shape.curve_to((120., 40.), (140., 60.), (140., 100.));
        shape.line_to((140., 200.));
        shape.curve_to((140., 240.), (120., 260.), (80., 260.));
        shape.line_to((80., 40.));
        shape.close_path();
        let ring = ring_of(&shape, 300);
        let fit = fit_ring(&ring, &FitOptions::default());
        assert!(!fit.fallback);
        let straight = fit
            .contour
            .points
            .iter()
            .filter(|p| p.kind == PointKind::Line)
            .count();
        assert!(
            straight >= 2,
            "stem sides must stay straight, got {straight}"
        );
        assert!(fit.error <= 1.5, "stem error {}", fit.error);
    }

    #[test]
    fn smoothing_lowers_the_quantisation_floor_of_a_one_bit_raster() {
        let ring = disc_ring(600, 300., 240.);
        let rough = fit_ring(&ring, &FitOptions::default());
        let smoothed = fit_ring(
            &ring,
            &FitOptions {
                smoothing: 2.0,
                ..FitOptions::default()
            },
        );
        assert!(!rough.fallback && !smoothed.fallback);
        assert!(
            smoothed.error < rough.error,
            "smoothing {} did not lower the floor {}",
            smoothed.error,
            rough.error
        );
        assert!(
            nodes(&smoothed) <= nodes(&rough),
            "smoothing {} must not need more nodes than {}",
            nodes(&smoothed),
            nodes(&rough)
        );
    }

    #[test]
    fn degenerate_and_invalid_inputs_keep_their_exact_edges() {
        for ring in [
            vec![],
            vec![Point::new(0., 0.), Point::new(1., 0.)],
            vec![
                Point::new(f64::NAN, 0.),
                Point::new(1., 0.),
                Point::new(1., 1.),
            ],
        ] {
            let fit = fit_ring(&ring, &FitOptions::default());
            assert!(fit.fallback);
            assert_eq!(fit.contour.points.len(), ring.len());
        }
        let ring = ring_of(&kurbo::Rect::new(10., 10., 40., 40.), 50);
        for options in [
            FitOptions {
                tolerance: 0.0,
                ..FitOptions::default()
            },
            FitOptions {
                tolerance: f64::NAN,
                ..FitOptions::default()
            },
            FitOptions {
                smoothing: f64::INFINITY,
                ..FitOptions::default()
            },
            FitOptions {
                smoothing: -1.0,
                ..FitOptions::default()
            },
            FitOptions {
                corner_angle: f64::NAN,
                ..FitOptions::default()
            },
        ] {
            assert!(fit_ring(&ring, &options).fallback, "{options:?}");
        }
    }

    #[test]
    fn smoothing_keeps_a_closed_ring() {
        let ring = ring_of(&Circle::new((60., 60.), 40.), 120);
        let rough = fit_ring(&ring, &FitOptions::default());
        let smoothed = fit_ring(
            &ring,
            &FitOptions {
                smoothing: 2.0,
                ..FitOptions::default()
            },
        );
        assert!(!rough.fallback && !smoothed.fallback);
        assert!(smoothed.contour.closed);
        assert!(smoothed.contour.points.len() >= 3);
        assert!(smoothed.error <= rough.error.max(1.0) + 1.0);
    }
}
