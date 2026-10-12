//! Facing-edge stem detection with ink verification and conflict separation.

use kurbo::{
    BezPath, Line, ParamCurve, ParamCurveDeriv, ParamCurveExtrema, PathEl, PathSeg, Point, Shape,
};

use crate::Error;

/// Largest primary (always-active) stem set one axis may carry.
const MAX_PRIMARY: usize = 48;
/// Largest number of replacement stems one axis may carry.
const MAX_REPLACEMENT: usize = 16;

/// A detected stem together with the span over which its edges face each other.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Stem {
    /// Lower edge coordinate on the stem's axis.
    pub(super) start: f64,
    /// Stem width; negative values are ghost sentinels added later.
    pub(super) width: f64,
    /// Start of the overlap along the stem's direction.
    pub(super) from: f64,
    /// End of the overlap along the stem's direction.
    pub(super) to: f64,
}

impl Stem {
    /// Whether two stems on the same axis cannot be active at the same time.
    pub(super) fn conflicts(&self, other: &Self) -> bool {
        self.start < other.start + other.width + 0.5 && other.start < self.start + self.width + 0.5
    }

    /// Whether `position` lies inside this stem's closed coordinate interval.
    pub(super) fn covers(&self, position: f64) -> bool {
        position >= self.start - 0.5 && position <= self.start + self.width + 0.5
    }
}

/// One axis's primary stem set plus the stems that need hint replacement.
#[derive(Debug, Default)]
pub(super) struct Axis {
    /// Mutually compatible stems that stay active for the whole glyph.
    pub(super) primary: Vec<Stem>,
    /// Stems that conflict with a primary stem and need point-bound masks.
    /// Replacement stems never conflict with one another.
    pub(super) replacement: Vec<Stem>,
}

/// Flattens the outline into facing edges, including synthesized curve tangents.
pub(super) fn edges(path: &BezPath) -> Result<Vec<(Point, Point)>, Error> {
    let mut edges = Vec::new();
    let mut previous = Point::ZERO;
    let mut start = Point::ZERO;
    let mut too_complex = false;
    kurbo::flatten(path.elements().iter().copied(), 0.25, |element| {
        if edges.len() >= 16_384 {
            too_complex = true;
            return;
        }
        match element {
            PathEl::MoveTo(p) => {
                previous = p;
                start = p;
            }
            PathEl::LineTo(p) => {
                edges.push((previous, p));
                previous = p;
            }
            PathEl::ClosePath => {
                edges.push((previous, start));
                previous = start;
            }
            _ => {}
        }
    });
    if too_complex || edges.iter().any(|(a, b)| !a.is_finite() || !b.is_finite()) {
        return Err(Error::Invalid(
            "PS autohint outline is invalid or too complex".into(),
        ));
    }
    // Flattened chords miss tangents at round extrema. Exact curve extrema and
    // endpoint tangents supply the facing edges of bowls such as o and O.
    curve_tangents(path, &mut edges);
    Ok(edges)
}

fn curve_tangents(path: &BezPath, edges: &mut Vec<(Point, Point)>) {
    for segment in path.segments() {
        if matches!(segment, PathSeg::Line(_)) {
            continue;
        }
        for t in std::iter::once(0.0).chain(segment.extrema()).chain([1.0]) {
            let tangent = match segment {
                PathSeg::Quad(curve) => curve.deriv().eval(t),
                PathSeg::Cubic(curve) => curve.deriv().eval(t),
                PathSeg::Line(_) => continue,
            };
            let point = segment.eval(t);
            let left = segment.eval((t - 0.04).max(0.0));
            let right = segment.eval((t + 0.04).min(1.0));
            if tangent.y.abs() < tangent.x.abs() * 0.01 {
                edges.push((Point::new(left.x, point.y), Point::new(right.x, point.y)));
            } else if tangent.x.abs() < tangent.y.abs() * 0.01 {
                edges.push((Point::new(point.x, left.y), Point::new(point.x, right.y)));
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Edge {
    pub(super) position: f64,
    pub(super) from: f64,
    pub(super) to: f64,
    pub(super) direction: bool,
}

/// Selects the edges that are flat enough and long enough to anchor a hint.
pub(super) fn flat_edges(lines: &[(Point, Point)], horizontal: bool, upem: f64) -> Vec<Edge> {
    lines
        .iter()
        .filter_map(|(a, b)| {
            let (along_a, along_b, cross_a, cross_b) = if horizontal {
                (a.x, b.x, a.y, b.y)
            } else {
                (a.y, b.y, a.x, b.x)
            };
            let length = (along_a - along_b).abs();
            (length >= upem / 1000.0 && (cross_a - cross_b).abs() <= length * 0.025).then_some(
                Edge {
                    position: (cross_a + cross_b) / 2.0,
                    from: along_a.min(along_b),
                    to: along_a.max(along_b),
                    direction: along_a < along_b,
                },
            )
        })
        .collect()
}

/// Detects one axis's stems, keeping conflicting candidates for hint replacement.
///
/// The highest scoring compatible candidates become the primary set. A candidate
/// that conflicts with the primary set is kept as a replacement stem when it is
/// compatible with every other replacement stem, so a glyph whose stems overlap
/// (a condensed `m`, an overlapping bowl and stem) keeps both hints instead of
/// losing the weaker one. `zones` raises the score of alignment-critical stems.
pub(super) fn axis(
    path: &BezPath,
    lines: &[(Point, Point)],
    horizontal: bool,
    upem: f64,
    preferred: &[f64],
    zones: &[(f64, f64)],
    work: &mut usize,
) -> Result<Axis, Error> {
    let edges = flat_edges(lines, horizontal, upem);
    let mut candidates = Vec::new();
    for (i, a) in edges.iter().enumerate() {
        for b in &edges[i + 1..] {
            spend(work, 1)?;
            let low = a.position.min(b.position);
            let width = (a.position - b.position).abs();
            let from = a.from.max(b.from);
            let to = a.to.min(b.to);
            if a.direction == b.direction
                || to - from < upem / 200.0
                || width < 1.0
                || width > upem / 3.0
            {
                continue;
            }
            let affinity = preferred
                .iter()
                .map(|value| (width - value).abs() / value)
                .fold(1.0, f64::min);
            // An edge inside an alignment zone must be hinted; prefer it when two
            // candidates otherwise score the same.
            let aligned = horizontal
                && [low, low + width].iter().any(|edge| {
                    zones
                        .iter()
                        .any(|(bottom, top)| *edge >= *bottom && *edge <= *top)
                });
            let score =
                (to - from) / width.sqrt() * (2.0 - affinity) * if aligned { 1.25 } else { 1.0 };
            if candidates.len() == 65_536 {
                return Err(Error::Invalid(
                    "PS autohint candidate limit exceeded".into(),
                ));
            }
            candidates.push((score, low, width, from, to));
        }
    }
    candidates.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.total_cmp(&b.2))
    });
    let mut result = Axis::default();
    for (_, start, width, from, to) in candidates {
        let stem = Stem {
            start,
            width,
            from,
            to,
        };
        let hits_primary = result.primary.iter().any(|other| other.conflicts(&stem));
        let hits_replacement = result
            .replacement
            .iter()
            .any(|other| other.conflicts(&stem));
        let target = if !hits_primary {
            (result.primary.len() < MAX_PRIMARY).then_some(true)
        } else if !hits_replacement
            // A replacement stem costs a mask at every point it governs, so it
            // must look like a stem rather than like the length of a crossbar or
            // the short chord beside a curve extremum: at least as long as wide.
            && to - from >= width
            && result.replacement.len() < MAX_REPLACEMENT
        {
            Some(false)
        } else {
            None
        };
        let Some(primary) = target else { continue };
        if !filled_sections(path, horizontal, start, width, from, to, work)? {
            continue;
        }
        if primary {
            result.primary.push(stem);
        } else {
            result.replacement.push(stem);
        }
    }
    result.primary.sort_by(|a, b| a.start.total_cmp(&b.start));
    result
        .replacement
        .sort_by(|a, b| a.start.total_cmp(&b.start));
    Ok(result)
}

// A single filled midpoint can belong to a crossbar inside an otherwise empty
// counter. Inspect three cross-sections, including every interval delimited by
// actual curve intersections. The inset tolerates the slight slope of accepted
// edges and curvature near synthesized extrema tangents. This remains a bounded
// geometric heuristic, not a proof that an entire stem rectangle is filled.
fn filled_sections(
    path: &BezPath,
    horizontal: bool,
    start: f64,
    width: f64,
    from: f64,
    to: f64,
    work: &mut usize,
) -> Result<bool, Error> {
    for fraction in [0.25, 0.5, 0.75] {
        let along = from + (to - from) * fraction;
        let point = |cross| {
            if horizontal {
                Point::new(along, cross)
            } else {
                Point::new(cross, along)
            }
        };
        let line = Line::new(point(start + width * 0.05), point(start + width * 0.95));
        let mut intersections = vec![0.0, 1.0];
        spend(work, path.elements().len())?;
        for segment in path.segments() {
            intersections.extend(segment.intersect_line(line).iter().map(|hit| hit.line_t));
        }
        intersections.sort_unstable_by(f64::total_cmp);
        // Coincident vertices and tangencies do not create a nonempty interval.
        intersections.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        for interval in intersections.windows(2) {
            spend(work, path.elements().len())?;
            if path.winding(line.eval((interval[0] + interval[1]) * 0.5)) == 0 {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Whether ink lies immediately on one side of a horizontal edge and not the other.
///
/// Returns `Some(true)` for a top edge (ink below) and `Some(false)` for a bottom
/// edge. An edge with ink on both sides or neither is not an outline boundary that
/// a ghost hint can describe.
pub(super) fn edge_side(path: &BezPath, edge: &Edge, upem: f64) -> Option<bool> {
    let inset = (upem / 2000.0).max(0.05);
    let along = (edge.from + edge.to) / 2.0;
    let below = path.winding(Point::new(along, edge.position - inset)) != 0;
    let above = path.winding(Point::new(along, edge.position + inset)) != 0;
    (below != above).then_some(below)
}

pub(super) fn spend(work: &mut usize, amount: usize) -> Result<(), Error> {
    *work = work
        .checked_sub(amount)
        .ok_or_else(|| Error::Invalid("PS autohint analysis work limit exceeded".into()))?;
    Ok(())
}
