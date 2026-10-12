//! Type 2 charstrings with optional stems, hint replacement and flex.

use std::collections::BTreeMap;

use kurbo::{BezPath, PathEl, Point, QuadBez, Rect, Shape};

use super::{encoding::number, WriteError};

pub(super) fn encode(
    path: &BezPath,
    width: f64,
    layer: Option<&fc_model::Layer>,
) -> Result<(Vec<u8>, Option<Rect>), WriteError> {
    let mut bytes = Vec::new();
    number(&mut bytes, width)?;
    let masks = if let Some(layer) = layer {
        super::hints::stems(&mut bytes, layer)?;
        super::hints::masks(layer)?
    } else {
        Default::default()
    };
    let flex = match layer {
        Some(layer) => fc_model::query::path_flex(&layer.hints, &layer.contours)
            .map_err(|error| WriteError(error.to_string()))?,
        None => BTreeMap::new(),
    };
    if !masks.is_empty()
        && !masks
            .get(&0)
            .is_some_and(|m| m.iter().any(|(counter, _)| !counter))
    {
        if let Some(layer) = layer {
            let count = layer.hints.h_stems.len() + layer.hints.v_stems.len();
            let mut initial = vec![255u8; count.div_ceil(8)];
            if !count.is_multiple_of(8) {
                if let Some(last) = initial.last_mut() {
                    *last <<= 8 - count % 8;
                }
            }
            bytes.push(19);
            bytes.extend(initial);
        }
    }
    let mut current = Point::ZERO;
    let mut source_current = Point::ZERO;
    let mut source_start = Point::ZERO;
    let mut encoded_path = BezPath::new();
    let mut open = false;
    let mut consumed = None;
    for (index, element) in path.elements().iter().enumerate() {
        if consumed == Some(index) {
            continue;
        }
        if let Some(masks) = masks.get(&index) {
            for (counter, mask) in masks {
                bytes.push(if *counter { 20 } else { 19 });
                bytes.extend_from_slice(mask);
            }
        }
        if let Some(placement) = flex.get(&index) {
            let second = path
                .elements()
                .get(placement.second)
                .ok_or_else(|| WriteError("flex hint runs past the outline".into()))?;
            let first = curve_points(*element, source_current)?;
            let second = curve_points(*second, first[2])?;
            emit_flex(
                &mut bytes,
                &mut current,
                &mut encoded_path,
                [first, second],
                placement.depth,
            )?;
            source_current = second[2];
            consumed = Some(placement.second);
            if bytes.len() >= 65535 {
                return Err(WriteError("charstring exceeds the 65,535-byte Type 2 limit; simplify the glyph before exporting".into()));
            }
            continue;
        }
        match *element {
            PathEl::MoveTo(point) => {
                if open {
                    return Err(WriteError(
                        "open contour cannot be exported as a filled outline".into(),
                    ));
                }
                encoded_path.move_to(delta(&mut bytes, &mut current, point)?);
                source_current = point;
                source_start = point;
                bytes.push(21); // rmoveto
                open = true;
            }
            PathEl::LineTo(point) => {
                encoded_path.line_to(delta(&mut bytes, &mut current, point)?);
                source_current = point;
                bytes.push(5); // rlineto
            }
            PathEl::QuadTo(control, end) => {
                let cubic = QuadBez::new(source_current, control, end).raise();
                let first = delta(&mut bytes, &mut current, cubic.p1)?;
                let second = delta(&mut bytes, &mut current, cubic.p2)?;
                let last = delta(&mut bytes, &mut current, cubic.p3)?;
                encoded_path.curve_to(first, second, last);
                source_current = end;
                bytes.push(8); // rrcurveto
            }
            PathEl::CurveTo(first, second, end) => {
                let first = delta(&mut bytes, &mut current, first)?;
                let second = delta(&mut bytes, &mut current, second)?;
                let last = delta(&mut bytes, &mut current, end)?;
                encoded_path.curve_to(first, second, last);
                source_current = end;
                bytes.push(8);
            }
            PathEl::ClosePath => {
                // Type 2 has no closepath operator. A closing-edge mask needs an
                // explicit endpoint so readers can retain its point association.
                if masks.contains_key(&index) && source_current != source_start {
                    encoded_path.line_to(delta(&mut bytes, &mut current, source_start)?);
                    bytes.push(5);
                }
                open = false;
                encoded_path.close_path();
                source_current = source_start;
            }
        }
        if bytes.len() >= 65535 {
            return Err(WriteError("charstring exceeds the 65,535-byte Type 2 limit; simplify the glyph before exporting".into()));
        }
    }
    if open {
        return Err(WriteError(
            "open contour cannot be exported as a filled outline".into(),
        ));
    }
    bytes.push(14); // endchar; also consumes width for an empty glyph
    let bounds = (!encoded_path.elements().is_empty()).then(|| encoded_path.bounding_box());
    Ok((bytes, bounds))
}

/// The two controls and the endpoint of one curve segment, raising quadratics.
fn curve_points(element: PathEl, from: Point) -> Result<[Point; 3], WriteError> {
    match element {
        PathEl::CurveTo(a, b, c) => Ok([a, b, c]),
        PathEl::QuadTo(control, end) => {
            let cubic = QuadBez::new(from, control, end).raise();
            Ok([cubic.p1, cubic.p2, cubic.p3])
        }
        _ => Err(WriteError(
            "a flex hint must cover two curve segments".into(),
        )),
    }
}

/// Writes one flex hint as `flex1` where the shape allows it, otherwise `flex`.
///
/// `flex1` (12 37) omits the final coordinate that returns to the starting x or y
/// and implies a flex height of 50, so it is both smaller and the form Adobe's
/// own tools prefer; every other pair is written as `flex` (12 35) with its
/// explicit height. Type 2 specification, "Hint Operators".
fn emit_flex(
    bytes: &mut Vec<u8>,
    current: &mut Point,
    encoded_path: &mut BezPath,
    curves: [[Point; 3]; 2],
    depth: f64,
) -> Result<(), WriteError> {
    let start = *current;
    let mut points = Vec::with_capacity(6);
    for curve in curves {
        for point in curve {
            points.push(quantize(point)?);
        }
    }
    let mut deltas = Vec::with_capacity(6);
    let mut previous = start;
    for point in &points {
        deltas.push((point.x - previous.x, point.y - previous.y));
        previous = *point;
    }
    let end = points[5];
    let total: (f64, f64) = deltas[..5]
        .iter()
        .fold((0.0, 0.0), |sum, d| (sum.0 + d.0, sum.1 + d.1));
    let horizontal = total.0.abs() > total.1.abs();
    let compact = depth == 50.0
        && if horizontal {
            end.y == start.y
        } else {
            end.x == start.x
        };
    if compact {
        for (dx, dy) in &deltas[..5] {
            number(bytes, *dx)?;
            number(bytes, *dy)?;
        }
        number(bytes, if horizontal { deltas[5].0 } else { deltas[5].1 })?;
        bytes.extend([12, 37]); // flex1
    } else {
        for (dx, dy) in &deltas {
            number(bytes, *dx)?;
            number(bytes, *dy)?;
        }
        number(bytes, depth)?;
        bytes.extend([12, 35]); // flex
    }
    encoded_path.curve_to(points[0], points[1], points[2]);
    encoded_path.curve_to(points[3], points[4], points[5]);
    *current = end;
    Ok(())
}

fn quantize(next: Point) -> Result<Point, WriteError> {
    if !next.x.is_finite() || !next.y.is_finite() {
        return Err(WriteError("non-finite outline coordinate".into()));
    }
    let next = Point::new(
        (next.x * 65536.0).round() / 65536.0,
        (next.y * 65536.0).round() / 65536.0,
    );
    if [next.x, next.y]
        .iter()
        .any(|value| !(-32768.0..=32767.99998474121).contains(value))
    {
        return Err(WriteError(
            "absolute outline coordinate exceeds Type 2 signed 16.16 limits".into(),
        ));
    }
    Ok(next)
}

fn delta(bytes: &mut Vec<u8>, current: &mut Point, next: Point) -> Result<Point, WriteError> {
    let next = quantize(next)?;
    number(bytes, next.x - current.x)?;
    number(bytes, next.y - current.y)?;
    *current = next;
    Ok(next)
}
