use super::{invalid, Error};
use fc_model::{Bitmap, Contour, Point, PointKind};
use std::collections::BTreeMap;

type Vertex = (i32, i32);

/// Trace connected pixel boundaries into closed y-up contours, retaining holes.
///
/// This is exact pixel-edge tracing; photographic curve fitting belongs to P5.
pub fn trace_bitmap(
    bitmap: &Bitmap,
    ppem: u16,
    units_per_em: u16,
    threshold: u8,
) -> Result<Vec<Contour>, Error> {
    bitmap.validate()?;
    if ppem == 0 || units_per_em == 0 || threshold == 0 {
        return Err(invalid("invalid tracing scale or threshold"));
    }
    if usize::from(bitmap.width) * usize::from(bitmap.height) > 1_000_000 {
        return Err(invalid("tracing pixel work limit"));
    }
    let mut edges = BTreeMap::<Vertex, Vec<Vertex>>::new();
    let mut edge_count = 0usize;
    let ink = |x: i32, y: i32| {
        x >= 0
            && y >= 0
            && x < i32::from(bitmap.width)
            && y < i32::from(bitmap.height)
            && bitmap.coverage(x as u16, y as u16) >= threshold
    };
    for y in 0..i32::from(bitmap.height) {
        for x in 0..i32::from(bitmap.width) {
            if !ink(x, y) {
                continue;
            }
            // Clockwise in image coordinates; reversing y makes outside contours CCW.
            for (outside, a, b) in [
                (!ink(x, y - 1), (x, y), (x + 1, y)),
                (!ink(x + 1, y), (x + 1, y), (x + 1, y + 1)),
                (!ink(x, y + 1), (x + 1, y + 1), (x, y + 1)),
                (!ink(x - 1, y), (x, y + 1), (x, y)),
            ] {
                if outside {
                    edges.entry(a).or_default().push(b);
                    edge_count += 1;
                }
            }
            if edge_count > 100_000 {
                return Err(invalid("tracing contour work limit"));
            }
        }
    }
    let mut contours = Vec::new();
    let scale = f64::from(units_per_em) / f64::from(ppem);
    while let Some((&start, _)) = edges.first_key_value() {
        let mut current = start;
        let mut previous = (start.0 - 1, start.1);
        let mut vertices = Vec::new();
        loop {
            vertices.push(current);
            let options = edges
                .get_mut(&current)
                .ok_or_else(|| invalid("open pixel boundary"))?;
            let direction = (current.0 - previous.0, current.1 - previous.1);
            // At diagonal contacts, take the right turn to keep components separate.
            let index = options
                .iter()
                .enumerate()
                .max_by_key(|(_, v)| {
                    let next = (v.0 - current.0, v.1 - current.1);
                    let cross = direction.0 * next.1 - direction.1 * next.0;
                    match cross {
                        1 => 3,
                        0 => 2,
                        _ => 1,
                    }
                })
                .map(|(i, _)| i)
                .ok_or_else(|| invalid("empty pixel boundary"))?;
            let next = options.remove(index);
            if options.is_empty() {
                edges.remove(&current);
            }
            previous = current;
            current = next;
            if current == start {
                break;
            }
            if vertices.len() > edge_count {
                return Err(invalid("invalid pixel boundary cycle"));
            }
        }
        let n = vertices.len();
        let mut points = Vec::new();
        for i in 0..n {
            let a = vertices[(i + n - 1) % n];
            let b = vertices[i];
            let c = vertices[(i + 1) % n];
            if (b.0 - a.0) * (c.1 - b.1) == (b.1 - a.1) * (c.0 - b.0) {
                continue;
            }
            points.push(Point {
                x: (f64::from(bitmap.origin_x) + f64::from(b.0)) * scale,
                y: (f64::from(bitmap.origin_y) + f64::from(bitmap.height) - f64::from(b.1)) * scale,
                kind: PointKind::Line,
                ..Default::default()
            });
        }
        if points.len() >= 3 {
            contours.push(Contour {
                closed: true,
                points,
                ..Default::default()
            });
        }
    }
    Ok(contours)
}
