//! Native m/l/c splines. Quadratic SFD repeats its single control point.

use super::{error, finite, integer, number, MAX_ITEMS};
use crate::Error;
use fc_model::{Contour, Point, PointKind};
use kurbo::PathEl;

pub(super) fn read(lines: &[&str], quadratic: bool) -> Result<Vec<Contour>, Error> {
    let mut contours = Vec::new();
    let mut points = Vec::new();
    let mut count = 0usize;
    for line in lines {
        let words: Vec<_> = line.split_whitespace().collect();
        let op = words
            .iter()
            .position(|word| matches!(*word, "m" | "l" | "c"))
            .ok_or_else(|| error(format!("unsupported spline record {line:?}")))?;
        if words.len() < op + 2 || words[op + 2..].iter().any(|word| word.starts_with('x')) {
            return Err(error("spline hint masks are not supported"));
        }
        let flags = integer(words[op + 1].split(',').next().unwrap_or(""))?;
        let smooth = flags & 3 != 1;
        let coords = words[..op]
            .iter()
            .map(|value| number(value))
            .collect::<Result<Vec<_>, _>>()?;
        match (words[op], coords.as_slice()) {
            ("m", [x, y]) => {
                finish(&mut contours, &mut points);
                points.push(point(*x, *y, PointKind::Move, smooth));
            }
            ("l", [x, y]) if !points.is_empty() => {
                points.push(point(*x, *y, PointKind::Line, smooth))
            }
            ("c", [x1, y1, x2, y2, x, y]) if !points.is_empty() => {
                if quadratic && (x1 != x2 || y1 != y2) {
                    return Err(error("quadratic spline has unequal duplicated controls"));
                }
                points.push(point(*x1, *y1, PointKind::OffCurve, false));
                if !quadratic {
                    points.push(point(*x2, *y2, PointKind::OffCurve, false));
                }
                points.push(point(
                    *x,
                    *y,
                    if quadratic {
                        PointKind::QCurve
                    } else {
                        PointKind::Curve
                    },
                    smooth,
                ));
            }
            _ => return Err(error("malformed spline command or missing moveto")),
        }
        count += 1;
        if count > MAX_ITEMS {
            return Err(error("spline point limit exceeded"));
        }
    }
    finish(&mut contours, &mut points);
    Ok(contours)
}

fn point(x: f64, y: f64, kind: PointKind, smooth: bool) -> Point {
    Point {
        x,
        y,
        kind,
        smooth,
        ..Default::default()
    }
}

fn finish(contours: &mut Vec<Contour>, points: &mut Vec<Point>) {
    if points.is_empty() {
        return;
    }
    let closed = points.len() > 1
        && points
            .first()
            .zip(points.last())
            .is_some_and(|(a, b)| a.x == b.x && a.y == b.y);
    if closed {
        let end = points.pop();
        if let (Some(start), Some(end)) = (points.first_mut(), end) {
            start.kind = end.kind;
        }
    }
    contours.push(Contour {
        closed,
        points: std::mem::take(points),
        ..Default::default()
    });
}

pub(super) fn write(contours: &[Contour], quadratic: bool, out: &mut String) -> Result<(), Error> {
    if contours.is_empty() {
        return Ok(());
    }
    out.push_str("SplineSet\n");
    for contour in contours {
        for point in &contour.points {
            finite(point.x)?;
            finite(point.y)?;
        }
        let path = contour.try_to_bezpath().map_err(|e| error(e.to_string()))?;
        let mut start = None;
        let mut current = None;
        for el in path.elements() {
            match *el {
                PathEl::MoveTo(p) => {
                    start = Some(p);
                    current = Some(p);
                    out.push_str(&format!("{} {} m {}\n", p.x, p.y, flag(contour, p.x, p.y)));
                }
                PathEl::LineTo(p) => {
                    current = Some(p);
                    out.push_str(&format!(" {} {} l {}\n", p.x, p.y, flag(contour, p.x, p.y)));
                }
                PathEl::QuadTo(q, p) => {
                    let s = current.ok_or_else(|| error("quadratic without moveto"))?;
                    let (a, b) = if quadratic {
                        (q, q)
                    } else {
                        (s + (q - s) * (2.0 / 3.0), p + (q - p) * (2.0 / 3.0))
                    };
                    out.push_str(&format!(
                        " {} {} {} {} {} {} c {}\n",
                        a.x,
                        a.y,
                        b.x,
                        b.y,
                        p.x,
                        p.y,
                        flag(contour, p.x, p.y)
                    ));
                    current = Some(p);
                }
                PathEl::CurveTo(a, b, p) => {
                    if quadratic {
                        return Err(error("cubic curve in quadratic layer"));
                    }
                    out.push_str(&format!(
                        " {} {} {} {} {} {} c {}\n",
                        a.x,
                        a.y,
                        b.x,
                        b.y,
                        p.x,
                        p.y,
                        flag(contour, p.x, p.y)
                    ));
                    current = Some(p);
                }
                PathEl::ClosePath => {
                    if let Some(p) = start {
                        if current != Some(p) {
                            out.push_str(&format!(
                                " {} {} l {}\n",
                                p.x,
                                p.y,
                                flag(contour, p.x, p.y)
                            ));
                        }
                    }
                }
            }
        }
    }
    out.push_str("EndSplineSet\n");
    Ok(())
}

fn flag(contour: &Contour, x: f64, y: f64) -> u8 {
    u8::from(!contour.points.iter().any(|point| {
        point.x == x && point.y == y && point.smooth && point.kind != PointKind::OffCurve
    }))
}
