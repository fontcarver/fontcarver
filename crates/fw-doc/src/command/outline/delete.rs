use std::collections::{BTreeMap, BTreeSet, HashSet};

use fw_model::{Contour, Font, GlyphName, LayerId, Point, PointKind};
use kurbo::{CubicBez, Line, ParamCurve, PathSeg, Point as KurboPoint, QuadBez, Vec2};

use fw_geom::SpanFit;

use crate::Error;

use super::super::{
    apply::{layer_mut, remove_indices},
    CurveKind, DeleteMode, PointRef,
};
use super::{
    adjacent_indices, cyclic_between, layer_curve_kind, next_on_curve, off_curve, on_curve_indices,
    previous_on_curve, repair_kinds, validate_points, Touched,
};

/// Samples taken from every original segment of a joined span.
const SPAN_SAMPLES: u32 = 32;

/// Deletes stored points under one of the three deletion policies.
///
/// The layer's contour list is rebuilt once: every contour without a deleted
/// point is moved across untouched, and each contour that loses points is
/// replaced by its rewritten form, by nothing when it is emptied, or by the
/// open pieces [`DeleteMode::BreakContour`] leaves behind. The reported
/// [`Touched`] indexes address that rebuilt list.
pub(super) fn delete_points(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    points: &[PointRef],
    mode: DeleteMode,
) -> Result<Touched, Error> {
    let kind = layer_curve_kind(font, glyph, layer);
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, points)?;
    let mut grouped = BTreeMap::<usize, BTreeSet<usize>>::new();
    for point in points {
        grouped
            .entry(point.contour)
            .or_default()
            .insert(point.point);
    }
    let source = std::mem::take(&mut target.contours);
    let mut rebuilt = Vec::with_capacity(source.len());
    let mut touched = BTreeSet::new();
    for (index, contour) in source.into_iter().enumerate() {
        let Some(indices) = grouped.get(&index) else {
            rebuilt.push(contour);
            continue;
        };
        let replacements: Vec<Contour> = match mode {
            DeleteMode::KeepShape => keep_shape(&contour, indices, kind)?.into_iter().collect(),
            DeleteMode::Simple => simple(&contour, indices).into_iter().collect(),
            DeleteMode::BreakContour => break_at_points(&contour, indices)?,
        };
        for replacement in replacements {
            touched.insert(rebuilt.len());
            rebuilt.push(replacement);
        }
    }
    target.contours = rebuilt;
    Ok(Touched::Contours(touched))
}

/// Removes the listed points and their handles without re-fitting.
fn simple(contour: &Contour, delete: &BTreeSet<usize>) -> Option<Contour> {
    let mut remove = vec![false; contour.points.len()];
    for &index in delete {
        if let Some(flag) = remove.get_mut(index) {
            *flag = true;
        }
        if contour.points[index].kind != PointKind::OffCurve {
            for adjacent in adjacent_indices(contour, index) {
                if contour.points[adjacent].kind == PointKind::OffCurve {
                    if let Some(flag) = remove.get_mut(adjacent) {
                        *flag = true;
                    }
                }
            }
        }
    }
    finish(contour, &remove, &BTreeMap::new(), &BTreeMap::new())
}

/// Deletes points while preserving the surrounding outline shape.
///
/// On-curve nodes are grouped into maximal runs along the contour. Each run's
/// surviving neighbours receive exactly one new segment: a line when both
/// original neighbouring segments were straight, otherwise a segment fitted by
/// [`fw_geom::fit_span`] whose tangents match the original outgoing tangent at
/// the preceding node and incoming tangent at the following node. Deleting an
/// off-curve point turns its own segment into a line. Deleting every on-curve
/// node removes the contour, and deleting an endpoint of an open contour just
/// drops it.
fn keep_shape(
    contour: &Contour,
    delete: &BTreeSet<usize>,
    kind: CurveKind,
) -> Result<Option<Contour>, Error> {
    let len = contour.points.len();
    if len == 0 {
        return Ok(None);
    }
    let on_curves = on_curve_indices(contour);
    if on_curves.is_empty() {
        return Ok(simple(contour, delete));
    }
    let deleted: Vec<usize> = (0..on_curves.len())
        .filter(|&position| delete.contains(&on_curves[position]))
        .collect();
    if deleted.len() == on_curves.len() {
        return Ok(None);
    }
    if contour.closed && deleted.len() + 1 == on_curves.len() {
        return Ok(sole_survivor(contour, &on_curves, delete));
    }

    let mut remove = vec![false; len];
    let mut insert_after = BTreeMap::<usize, Vec<Point>>::new();
    let mut set_kind = BTreeMap::<usize, PointKind>::new();
    for (first, last) in runs(&deleted, on_curves.len(), contour.closed) {
        if !contour.closed && first == 0 {
            let Some(&boundary) = on_curves.get(last + 1) else {
                return Ok(None);
            };
            for flag in remove.iter_mut().take(boundary) {
                *flag = true;
            }
            set_kind.insert(boundary, PointKind::Move);
            continue;
        }
        if !contour.closed && last + 1 == on_curves.len() {
            let Some(&anchor) = first
                .checked_sub(1)
                .and_then(|before| on_curves.get(before))
            else {
                return Ok(None);
            };
            for flag in remove.iter_mut().skip(anchor + 1) {
                *flag = true;
            }
            continue;
        }
        let count = on_curves.len();
        let (Some(&previous), Some(&next)) = (
            on_curves.get((first + count - 1) % count),
            on_curves.get((last + 1) % count),
        ) else {
            return Ok(None);
        };
        for index in cyclic_between(len, previous, next) {
            if let Some(flag) = remove.get_mut(index) {
                *flag = true;
            }
        }
        let Some((&run_first, &run_last)) = on_curves.get(first).zip(on_curves.get(last)) else {
            return Ok(None);
        };
        let straight = cyclic_between(len, previous, run_first).is_empty()
            && cyclic_between(len, run_last, next).is_empty();
        if straight {
            set_kind.insert(next, PointKind::Line);
            continue;
        }
        let (controls, endpoint) = fit(contour, previous, next, kind);
        insert_after.insert(previous, controls);
        set_kind.insert(next, endpoint);
    }

    for &index in delete {
        if remove.get(index).copied().unwrap_or(true) {
            continue;
        }
        if contour.points[index].kind != PointKind::OffCurve {
            continue;
        }
        let previous = previous_on_curve(contour, index)
            .ok_or_else(|| Error::InvalidCommand("handle has no preceding node".to_owned()))?;
        let next = next_on_curve(contour, index)
            .ok_or_else(|| Error::InvalidCommand("handle has no following node".to_owned()))?;
        if remove.get(previous).copied().unwrap_or(true)
            || remove.get(next).copied().unwrap_or(true)
        {
            continue;
        }
        for candidate in cyclic_between(len, previous, next) {
            if contour.points[candidate].kind == PointKind::OffCurve {
                if let Some(flag) = remove.get_mut(candidate) {
                    *flag = true;
                }
            }
        }
        set_kind.insert(next, PointKind::Line);
    }

    Ok(finish(contour, &remove, &insert_after, &set_kind))
}

/// Keeps only the single surviving node of a closed contour.
fn sole_survivor(
    contour: &Contour,
    on_curves: &[usize],
    delete: &BTreeSet<usize>,
) -> Option<Contour> {
    let survivor = on_curves
        .iter()
        .copied()
        .find(|index| !delete.contains(index))?;
    let mut point = contour.points.get(survivor)?.clone();
    point.kind = PointKind::Line;
    let mut result = contour.clone();
    result.points = vec![point];
    Some(result)
}

/// Groups deleted on-curve positions into maximal runs along the contour.
///
/// A run of a closed contour may wrap, in which case its first position is
/// greater than its last.
fn runs(deleted: &[usize], count: usize, closed: bool) -> Vec<(usize, usize)> {
    let mut groups = Vec::<(usize, usize)>::new();
    for &position in deleted {
        match groups.last_mut() {
            Some((_, last)) if *last + 1 == position => *last = position,
            _ => groups.push((position, position)),
        }
    }
    if closed && groups.len() > 1 {
        let head = groups.first().copied();
        let tail = groups.last().copied();
        if let (Some((0, head_last)), Some((_, tail_last))) = (head, tail) {
            if tail_last + 1 == count {
                groups.remove(0);
                if let Some(last) = groups.last_mut() {
                    last.1 = head_last;
                }
            }
        }
    }
    groups
}

/// Fits one replacement segment for the span between two surviving nodes.
fn fit(
    contour: &Contour,
    previous: usize,
    next: usize,
    kind: CurveKind,
) -> (Vec<Point>, PointKind) {
    let segments = span_segments(contour, previous, next);
    let Some((first, last)) = segments.first().zip(segments.last()) else {
        return (Vec::new(), PointKind::Line);
    };
    let samples = sample_span(&segments);
    let fitted = fw_geom::fit_span(
        &samples,
        start_tangent(first),
        end_tangent(last),
        kind.geometry(),
    );
    match fitted {
        SpanFit::Line => (Vec::new(), PointKind::Line),
        SpanFit::Quadratic(controls) => (
            controls.into_iter().map(off_curve).collect(),
            PointKind::QCurve,
        ),
        SpanFit::Cubic(first, second) => {
            (vec![off_curve(first), off_curve(second)], PointKind::Curve)
        }
    }
}

/// Rebuilds the geometric segments between two on-curve points.
fn span_segments(contour: &Contour, from: usize, to: usize) -> Vec<PathSeg> {
    let len = contour.points.len();
    let mut segments = Vec::new();
    let Some(origin) = contour.points.get(from).map(kurbo_point) else {
        return segments;
    };
    let mut start = origin;
    let mut controls = Vec::new();
    let mut cursor = (from + 1) % len;
    for _ in 0..len {
        let Some(point) = contour.points.get(cursor) else {
            break;
        };
        if point.kind == PointKind::OffCurve {
            controls.push(kurbo_point(point));
        } else {
            emit(&mut segments, &mut start, &controls, point);
            controls.clear();
            if cursor == to {
                break;
            }
        }
        cursor = (cursor + 1) % len;
    }
    segments
}

fn emit(segments: &mut Vec<PathSeg>, start: &mut KurboPoint, controls: &[KurboPoint], end: &Point) {
    let endpoint = kurbo_point(end);
    match (end.kind, controls) {
        (_, []) => segments.push(PathSeg::Line(Line::new(*start, endpoint))),
        (PointKind::Curve, [first, second]) => segments.push(PathSeg::Cubic(CubicBez::new(
            *start, *first, *second, endpoint,
        ))),
        _ => {
            for (index, control) in controls.iter().enumerate() {
                let segment_end = controls
                    .get(index + 1)
                    .map_or(endpoint, |next| midpoint(*control, *next));
                segments.push(PathSeg::Quad(QuadBez::new(*start, *control, segment_end)));
                *start = segment_end;
            }
        }
    }
    *start = endpoint;
}

fn sample_span(segments: &[PathSeg]) -> Vec<KurboPoint> {
    let mut samples = Vec::with_capacity(segments.len() * SPAN_SAMPLES as usize + 1);
    if let Some(first) = segments.first() {
        samples.push(first.start());
    }
    for segment in segments {
        for step in 1..=SPAN_SAMPLES {
            samples.push(segment.eval(f64::from(step) / f64::from(SPAN_SAMPLES)));
        }
    }
    samples
}

fn start_tangent(segment: &PathSeg) -> Vec2 {
    match segment {
        PathSeg::Line(line) => line.p1 - line.p0,
        PathSeg::Quad(quad) => longest(&[quad.p1 - quad.p0, quad.p2 - quad.p0]),
        PathSeg::Cubic(cubic) => longest(&[
            cubic.p1 - cubic.p0,
            cubic.p2 - cubic.p0,
            cubic.p3 - cubic.p0,
        ]),
    }
}

fn end_tangent(segment: &PathSeg) -> Vec2 {
    match segment {
        PathSeg::Line(line) => line.p1 - line.p0,
        PathSeg::Quad(quad) => longest(&[quad.p2 - quad.p1, quad.p2 - quad.p0]),
        PathSeg::Cubic(cubic) => longest(&[
            cubic.p3 - cubic.p2,
            cubic.p3 - cubic.p1,
            cubic.p3 - cubic.p0,
        ]),
    }
}

/// Returns the first non-degenerate candidate, or the last one.
fn longest(candidates: &[Vec2]) -> Vec2 {
    candidates
        .iter()
        .copied()
        .find(|candidate| candidate.hypot() > 1e-12)
        .or_else(|| candidates.last().copied())
        .unwrap_or(Vec2::new(1.0, 0.0))
}

fn kurbo_point(point: &Point) -> KurboPoint {
    KurboPoint::new(point.x, point.y)
}

fn midpoint(left: KurboPoint, right: KurboPoint) -> KurboPoint {
    KurboPoint::new((left.x + right.x) * 0.5, (left.y + right.y) * 0.5)
}

/// Rebuilds a contour from removal flags, inserted handles and new kinds.
fn finish(
    contour: &Contour,
    remove: &[bool],
    insert_after: &BTreeMap<usize, Vec<Point>>,
    set_kind: &BTreeMap<usize, PointKind>,
) -> Option<Contour> {
    let mut points = Vec::with_capacity(contour.points.len());
    for (index, point) in contour.points.iter().enumerate() {
        if remove.get(index).copied().unwrap_or(false) {
            continue;
        }
        let mut point = point.clone();
        if let Some(kind) = set_kind.get(&index) {
            point.kind = *kind;
        }
        points.push(point);
        if let Some(controls) = insert_after.get(&index) {
            points.extend(controls.iter().cloned());
        }
    }
    let mut result = contour.clone();
    result.points = points;
    repair_kinds(&mut result);
    (!result.points.is_empty()).then_some(result)
}

/// Removes the handles of one segment and turns it into a straight line.
fn delete_handle_as_line(contour: &mut Contour, index: usize) -> Result<(), Error> {
    let previous = previous_on_curve(contour, index)
        .ok_or_else(|| Error::InvalidCommand("handle has no preceding node".to_owned()))?;
    let next = next_on_curve(contour, index)
        .ok_or_else(|| Error::InvalidCommand("handle has no following node".to_owned()))?;
    let mut remove = cyclic_between(contour.points.len(), previous, next);
    remove.retain(|&candidate| contour.points[candidate].kind == PointKind::OffCurve);
    remove.sort_unstable();
    remove_indices(&mut contour.points, &remove);
    let adjusted_next = next - remove.iter().filter(|&&removed| removed < next).count();
    if let Some(endpoint) = contour.points.get_mut(adjusted_next) {
        endpoint.kind = PointKind::Line;
    }
    Ok(())
}

/// Removes the listed points and both adjacent segments, opening the contour.
///
/// Returns the open pieces that replace the contour, in order.
fn break_at_points(source: &Contour, indices: &BTreeSet<usize>) -> Result<Vec<Contour>, Error> {
    let mut contour = source.clone();
    if indices.is_empty() {
        return Ok(vec![contour]);
    }
    let selected_on_curves = indices
        .iter()
        .copied()
        .filter(|&index| contour.points[index].kind != PointKind::OffCurve)
        .collect::<Vec<_>>();
    if selected_on_curves.is_empty() {
        for &index in indices.iter().rev() {
            delete_handle_as_line(&mut contour, index)?;
        }
        repair_kinds(&mut contour);
        return Ok(vec![contour]);
    }

    let len = contour.points.len();
    let mut keep = vec![true; len];
    let mut force_line = HashSet::new();
    for &index in indices {
        if contour.points[index].kind == PointKind::OffCurve {
            let previous = previous_on_curve(&contour, index)
                .ok_or_else(|| Error::InvalidCommand("handle has no preceding node".to_owned()))?;
            let next = next_on_curve(&contour, index)
                .ok_or_else(|| Error::InvalidCommand("handle has no following node".to_owned()))?;
            for candidate in cyclic_between(len, previous, next) {
                if contour.points[candidate].kind == PointKind::OffCurve {
                    keep[candidate] = false;
                }
            }
            force_line.insert(next);
        }
    }
    for &index in &selected_on_curves {
        keep[index] = false;
        if contour.closed || index > 0 {
            let mut before = if index == 0 { len - 1 } else { index - 1 };
            while contour.points[before].kind == PointKind::OffCurve {
                keep[before] = false;
                if before == 0 {
                    if !contour.closed {
                        break;
                    }
                    before = len - 1;
                } else {
                    before -= 1;
                }
            }
        }
        if contour.closed || index + 1 < len {
            let mut after = (index + 1) % len;
            while contour.points[after].kind == PointKind::OffCurve {
                keep[after] = false;
                after = (after + 1) % len;
                if !contour.closed && after == 0 {
                    break;
                }
            }
        }
    }

    let order = if contour.closed {
        let start = keep
            .iter()
            .position(|kept| !kept)
            .map_or(0, |index| (index + 1) % len);
        (0..len)
            .map(|offset| (start + offset) % len)
            .collect::<Vec<_>>()
    } else {
        (0..len).collect()
    };
    let mut groups = Vec::<Vec<(usize, Point)>>::new();
    let mut current = Vec::new();
    for index in order {
        if keep[index] {
            current.push((index, contour.points[index].clone()));
        } else if !current.is_empty() {
            groups.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }

    let mut pieces = Vec::new();
    let mut metadata_available = true;
    for mut group in groups {
        while group
            .first()
            .is_some_and(|(_, point)| point.kind == PointKind::OffCurve)
        {
            group.remove(0);
        }
        while group
            .last()
            .is_some_and(|(_, point)| point.kind == PointKind::OffCurve)
        {
            group.pop();
        }
        if group.is_empty() {
            continue;
        }
        for (source_index, point) in &mut group {
            if force_line.contains(source_index) {
                point.kind = PointKind::Line;
            }
        }
        let points = group
            .into_iter()
            .map(|(_, point)| point)
            .collect::<Vec<_>>();
        let mut piece = Contour {
            closed: false,
            points,
            name: if metadata_available {
                contour.name.clone()
            } else {
                None
            },
            id: if metadata_available {
                contour.id.clone()
            } else {
                None
            },
        };
        repair_kinds(&mut piece);
        if piece.points.is_empty() {
            continue;
        }
        pieces.push(piece);
        metadata_available = false;
    }
    Ok(pieces)
}
