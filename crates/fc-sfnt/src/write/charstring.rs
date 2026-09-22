//! Type 2 charstrings with optional stems and point-bound hint replacement.

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
    for (index, element) in path.elements().iter().enumerate() {
        if let Some(masks) = masks.get(&index) {
            for (counter, mask) in masks {
                bytes.push(if *counter { 20 } else { 19 });
                bytes.extend_from_slice(mask);
            }
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

fn delta(bytes: &mut Vec<u8>, current: &mut Point, next: Point) -> Result<Point, WriteError> {
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
    number(bytes, next.x - current.x)?;
    number(bytes, next.y - current.y)?;
    *current = next;
    Ok(next)
}
