//! Bounded geometric PostScript stem detection and master blue-zone inference.

use std::collections::BTreeMap;

use fc_model::{Font, GlyphName, MasterId, PsHints, PsPrivate, StemHint};
use kurbo::{
    BezPath, Line, ParamCurve, ParamCurveDeriv, ParamCurveExtrema, PathEl, PathSeg, Point, Shape,
};

use crate::Error;

/// Analyzes a captured selection without mutating any source or partially applying hints.
pub fn analyze(
    font: &Font,
    master: &MasterId,
    names: &[GlyphName],
) -> Result<(Vec<(GlyphName, PsHints)>, PsPrivate), Error> {
    if names.len() > 65_535 {
        return Err(Error::Invalid(
            "PS autohint selection exceeds glyph limit".into(),
        ));
    }
    let mut unique = std::collections::HashSet::new();
    let mut output = Vec::new();
    let mut points = 0usize;
    for name in names {
        if !unique.insert(name) {
            continue;
        }
        let layer = font
            .glyph(name)
            .and_then(|g| g.master_layer(master))
            .ok_or_else(|| Error::Invalid(format!("missing master layer for {name}")))?;
        if layer.cff_private_dict_index.unwrap_or(0) != 0 {
            return Err(Error::Invalid(
                "automatic PostScript analysis currently supports private dictionary 0 only; select glyphs assigned to dictionary 0 or edit this dictionary's hints manually".into(),
            ));
        }
        points =
            points.saturating_add(layer.contours.iter().map(|c| c.points.len()).sum::<usize>());
        if points > 8_000_000 {
            return Err(Error::Invalid(
                "PS autohint selection exceeds point budget".into(),
            ));
        }
        output.push((name.clone(), detect(font, name, master)?));
    }
    let hints: Vec<_> = output.iter().map(|(_, h)| h.clone()).collect();
    let private = infer_private(font, master, &hints)?;
    Ok((output, private))
}

/// Detects horizontal and vertical stems from facing outline edges.
///
/// Candidates must overlap and have filled ink between them. The highest-scoring
/// disjoint stems are retained, so the resulting single active hint set has no
/// conflicting stems. Master stem widths bias the geometric score when supplied.
pub fn detect(font: &Font, glyph: &GlyphName, master: &MasterId) -> Result<PsHints, Error> {
    let source = font
        .master(master)
        .ok_or_else(|| Error::Invalid("missing hint master".into()))?;
    let path =
        fc_model::flatten_layer(font, glyph, master).map_err(|e| Error::Invalid(e.to_string()))?;
    if path.elements().len() > 16_384
        || !path.is_finite()
        || path.elements().iter().any(|el| {
            let valid = |p: &Point| p.x.abs() <= 32767.0 && p.y.abs() <= 32767.0;
            match el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => !valid(p),
                PathEl::QuadTo(a, b) => !valid(a) || !valid(b),
                PathEl::CurveTo(a, b, c) => !valid(a) || !valid(b) || !valid(c),
                PathEl::ClosePath => false,
            }
        })
    {
        return Err(Error::Invalid(
            "PS autohint outline is invalid or exceeds the coordinate or analysis budget".into(),
        ));
    }
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
    curve_tangents(&path, &mut edges);
    let mut work = 4_000_000usize;
    let mut stems = |horizontal| {
        let preferred: Vec<_> = source
            .stems
            .iter()
            .filter(|s| s.horizontal == horizontal && s.width.is_finite() && s.width > 0.0)
            .map(|s| s.width)
            .collect();
        candidates(
            &path,
            &edges,
            horizontal,
            f64::from(font.info.units_per_em),
            &preferred,
            &mut work,
        )
    };
    Ok(PsHints {
        h_stems: stems(true)?,
        v_stems: stems(false)?,
        ..PsHints::default()
    })
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
struct Edge {
    position: f64,
    from: f64,
    to: f64,
    direction: bool,
}

fn candidates(
    path: &BezPath,
    lines: &[(Point, Point)],
    horizontal: bool,
    upem: f64,
    preferred: &[f64],
    work: &mut usize,
) -> Result<Vec<StemHint>, Error> {
    let edges: Vec<_> = lines
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
        .collect();
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
            let score = (to - from) / width.sqrt() * (2.0 - affinity);
            if candidates.len() == 65_536 {
                return Err(Error::Invalid(
                    "PS autohint candidate limit exceeded".into(),
                ));
            }
            candidates.push((score, low, width, from, to));
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut result: Vec<StemHint> = Vec::new();
    for (_, start, width, from, to) in candidates {
        if result
            .iter()
            .any(|s| start < s.start + s.width + 0.5 && start + width + 0.5 > s.start)
        {
            continue;
        }
        if !filled_sections(path, horizontal, start, width, from, to, work)? {
            continue;
        }
        result.push(StemHint {
            start,
            width,
            ranges: Vec::new(),
        });
        if result.len() == 48 {
            break;
        }
    }
    result.sort_by(|a, b| a.start.total_cmp(&b.start));
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

fn spend(work: &mut usize, amount: usize) -> Result<(), Error> {
    *work = work
        .checked_sub(amount)
        .ok_or_else(|| Error::Invalid("PS autohint analysis work limit exceeded".into()))?;
    Ok(())
}

/// Infers missing private values from authored zones, master metrics and stem modes.
/// Existing private values remain authoritative and are validated on return.
pub fn infer_private(
    font: &Font,
    master: &MasterId,
    hints: &[PsHints],
) -> Result<PsPrivate, Error> {
    let source = font
        .master(master)
        .ok_or_else(|| Error::Invalid("missing hint master".into()))?;
    let mut private = source.private_dict.clone();
    let overshoot = (f64::from(font.info.units_per_em) / 100.0).max(1.0);
    if private.blue_values.is_empty() {
        let mut zones: Vec<(f64, f64)> = if source.alignment_zones.is_empty() {
            vec![
                (-overshoot, 0.0),
                (source.metrics.x_height, source.metrics.x_height + overshoot),
                (
                    source.metrics.cap_height,
                    source.metrics.cap_height + overshoot,
                ),
            ]
        } else {
            source
                .alignment_zones
                .iter()
                .filter(|z| z.size >= 0.0 || z.position == 0.0)
                .map(|z| {
                    (
                        z.position.min(z.position + z.size),
                        z.position.max(z.position + z.size),
                    )
                })
                .collect()
        };
        zones.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut end = f64::NEG_INFINITY;
        for (low, high) in zones {
            if low <= end {
                continue;
            }
            private.blue_values.extend([low, high]);
            end = high;
            if private.blue_values.len() == 14 {
                break;
            }
        }
    }
    if private.other_blues.is_empty() {
        if source.alignment_zones.is_empty() && source.metrics.descender < -overshoot {
            private.other_blues = vec![
                source.metrics.descender - overshoot,
                source.metrics.descender,
            ];
        } else {
            let mut zones: Vec<_> = source
                .alignment_zones
                .iter()
                .filter(|z| z.size < 0.0 && z.position != 0.0)
                .collect();
            zones.sort_by(|a, b| a.position.total_cmp(&b.position));
            for zone in zones.iter().take(5) {
                private
                    .other_blues
                    .extend([zone.position + zone.size, zone.position]);
            }
        }
    }
    for horizontal in [true, false] {
        let mut widths = BTreeMap::<i32, usize>::new();
        for hint in hints {
            for stem in if horizontal {
                &hint.h_stems
            } else {
                &hint.v_stems
            } {
                if stem.width > 0.0 {
                    *widths.entry(stem.width.round() as i32).or_default() += 1;
                }
            }
        }
        for stem in source.stems.iter().filter(|s| s.horizontal == horizontal) {
            if !stem.width.is_finite() || stem.width <= 0.0 || stem.width > 32767.0 {
                return Err(Error::Invalid("invalid master stem width".into()));
            }
            *widths.entry(stem.width.round() as i32).or_default() += hints.len().max(1);
        }
        let mut ranked: Vec<_> = widths.into_iter().collect();
        ranked.sort_by_key(|(width, count)| (std::cmp::Reverse(*count), *width));
        let (standard, snaps) = if horizontal {
            (&mut private.std_hw, &mut private.stem_snap_h)
        } else {
            (&mut private.std_vw, &mut private.stem_snap_v)
        };
        if standard.is_none() {
            *standard = ranked.first().map(|(width, _)| f64::from(*width));
        }
        if snaps.is_empty() {
            *snaps = ranked
                .iter()
                .take(12)
                .map(|(width, _)| f64::from(*width))
                .collect();
            snaps.sort_by(f64::total_cmp);
        }
    }
    private
        .validate()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    Ok(private)
}
