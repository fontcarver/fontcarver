use super::{invalid, Error};
use fc_model::{Bitmap, Contour, Point, PointKind};
use kurbo::{Point as GeometryPoint, Vec2};
use std::collections::BTreeMap;

// A quarter-pixel grid keeps diagonal bridges exact without another raster pass.
type Vertex = (i32, i32);

/// Corner treatment for reconstructing outlines from thresholded bitmap pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BitmapTraceMode {
    /// Preserve exact pixel edges and remove collinear boundary vertices.
    #[default]
    PixelEdges,
    /// Cut corners with straight segments, one-quarter pixel along each leg.
    ChamferedCorners,
    /// Round corners with tangent cubic arcs of approximately one-quarter pixel radius.
    RoundedCorners,
    /// Replace staircase turns with half-pixel diagonals; preserve rectangle corners.
    DiagonalSmoothing,
}

/// Bounded bitmap outline reconstruction settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BitmapTraceOptions {
    /// Treatment of corners after connected boundaries have been extracted.
    pub mode: BitmapTraceMode,
    /// Connect diagonally touching ink with a quarter-pixel bevel bridge.
    ///
    /// False uses four-connected ink and eight-connected background. True uses
    /// eight-connected ink and four-connected background, so closing a diagonal
    /// background passage can create a hole. Empty pixel centers stay empty.
    pub join_diagonals: bool,
}

/// Trace connected pixel boundaries into closed y-up contours, retaining holes.
///
/// Exact pixel-edge reconstruction with separate diagonal contacts. Outside
/// contours are clockwise in font coordinates and holes are counterclockwise.
pub fn trace_bitmap(
    bitmap: &Bitmap,
    ppem: u16,
    units_per_em: u16,
    threshold: u8,
) -> Result<Vec<Contour>, Error> {
    trace_bitmap_with_options(bitmap, ppem, units_per_em, threshold, Default::default())
}

/// Trace bitmap outlines with a corner treatment and separate diagonal contacts.
pub fn trace_bitmap_with_mode(
    bitmap: &Bitmap,
    ppem: u16,
    units_per_em: u16,
    threshold: u8,
    mode: BitmapTraceMode,
) -> Result<Vec<Contour>, Error> {
    trace_bitmap_with_options(
        bitmap,
        ppem,
        units_per_em,
        threshold,
        BitmapTraceOptions {
            mode,
            join_diagonals: false,
        },
    )
}

/// Reconstruct bitmap contours with corner and diagonal connectivity options.
///
/// Coverage at least `threshold` is ink. Coordinates retain the bitmap origin
/// and scale by `units_per_em / ppem`. Work is bounded by one million pixels,
/// 100,000 original boundary edges and 800,000 generated model points. This is
/// pixel-font reconstruction, not photographic curve fitting. Corner cuts on
/// shorter diagonal bridge edges are reduced to avoid overlapping treatments.
pub fn trace_bitmap_with_options(
    bitmap: &Bitmap,
    ppem: u16,
    units_per_em: u16,
    threshold: u8,
    options: BitmapTraceOptions,
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
    let saddle = |p: Vertex| {
        options.join_diagonals
            && ink(p.0 - 1, p.1 - 1) == ink(p.0, p.1)
            && ink(p.0, p.1 - 1) == ink(p.0 - 1, p.1)
            && ink(p.0 - 1, p.1 - 1) != ink(p.0, p.1 - 1)
    };
    for y in 0..i32::from(bitmap.height) {
        for x in 0..i32::from(bitmap.width) {
            if !ink(x, y) {
                continue;
            }
            // The y-up conversion produces clockwise outside contours.
            for (outside, a, b) in [
                (!ink(x, y - 1), (x, y), (x + 1, y)),
                (!ink(x + 1, y), (x + 1, y), (x + 1, y + 1)),
                (!ink(x, y + 1), (x + 1, y + 1), (x, y + 1)),
                (!ink(x - 1, y), (x, y + 1), (x, y)),
            ] {
                if outside {
                    edge_count += 1;
                    let direction = (b.0 - a.0, b.1 - a.1);
                    let from = if saddle(a) {
                        (a.0 * 4 + direction.0, a.1 * 4 + direction.1)
                    } else {
                        (a.0 * 4, a.1 * 4)
                    };
                    let to = if saddle(b) {
                        (b.0 * 4 - direction.0, b.1 * 4 - direction.1)
                    } else {
                        (b.0 * 4, b.1 * 4)
                    };
                    edges.entry(from).or_default().push(to);
                    if saddle(b) {
                        // Turn across the empty quadrant, adding a small diamond
                        // at the saddle. All bridge ports have one in/out edge.
                        let bridge = (b.0 * 4 + direction.1, b.1 * 4 - direction.0);
                        edges.entry(to).or_default().push(bridge);
                    }
                }
            }
            if edge_count > 100_000 {
                return Err(invalid("tracing contour work limit"));
            }
        }
    }
    let mut contours = Vec::new();
    // A saddle adds two edges and touches four original edges. Each original
    // edge touches at most two saddles, so total traced edges are at most 2E.
    let traced_edge_count: usize = edges.values().map(Vec::len).sum();
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
                    match cross.signum() {
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
            if vertices.len() > traced_edge_count {
                return Err(invalid("invalid pixel boundary cycle"));
            }
        }
        let vertices = remove_collinear(&vertices);
        let points = corner_points(&vertices, bitmap, scale, options.mode);
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

fn remove_collinear(vertices: &[Vertex]) -> Vec<Vertex> {
    let n = vertices.len();
    vertices
        .iter()
        .enumerate()
        .filter_map(|(i, &b)| {
            let a = vertices[(i + n - 1) % n];
            let c = vertices[(i + 1) % n];
            ((b.0 - a.0) * (c.1 - b.1) != (b.1 - a.1) * (c.0 - b.0)).then_some(b)
        })
        .collect()
}

fn corner_points(
    vertices: &[Vertex],
    bitmap: &Bitmap,
    scale: f64,
    mode: BitmapTraceMode,
) -> Vec<Point> {
    let n = vertices.len();
    let mut points = Vec::new();
    let position = |index: usize| {
        let p = vertices[index % n];
        GeometryPoint::new(f64::from(p.0) / 4.0, f64::from(p.1) / 4.0)
    };
    let mut emit = |p: GeometryPoint, kind: PointKind| {
        points.push(Point {
            x: (f64::from(bitmap.origin_x) + p.x) * scale,
            y: (f64::from(bitmap.origin_y) + f64::from(bitmap.height) - p.y) * scale,
            kind,
            ..Default::default()
        });
    };
    for i in 0..n {
        let a = position(i + n - 1);
        let b = position(i);
        let c = position(i + 1);
        let incoming = b - a;
        let outgoing = c - b;
        let cut = match mode {
            BitmapTraceMode::PixelEdges => 0.0,
            BitmapTraceMode::ChamferedCorners | BitmapTraceMode::RoundedCorners => 0.25_f64
                .min(incoming.hypot() * 0.25)
                .min(outgoing.hypot() * 0.25),
            BitmapTraceMode::DiagonalSmoothing => {
                let previous = a - position(i + n - 2);
                let next = position(i + 2) - c;
                if staircase_corner(previous, incoming, outgoing, next) {
                    0.5
                } else {
                    0.0
                }
            }
        };
        if cut == 0.0 {
            emit(b, PointKind::Line);
            continue;
        }
        let incoming = incoming.normalize();
        let outgoing = outgoing.normalize();
        let entry = b - incoming * cut;
        let exit = b + outgoing * cut;
        emit(entry, PointKind::Line);
        if mode == BitmapTraceMode::RoundedCorners {
            let angle = incoming.dot(outgoing).clamp(-1.0, 1.0).acos();
            let radius = cut / (angle / 2.0).tan();
            let handle = (4.0 / 3.0) * (angle / 4.0).tan() * radius;
            emit(entry + incoming * handle, PointKind::OffCurve);
            emit(exit - outgoing * handle, PointKind::OffCurve);
            emit(exit, PointKind::Curve);
        } else {
            emit(exit, PointKind::Line);
        }
    }
    if mode == BitmapTraceMode::DiagonalSmoothing {
        // Half-pixel cuts can meet at a segment midpoint; emit it only once.
        points.dedup_by(|a, b| a.x == b.x && a.y == b.y);
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
    }
    points
}

fn staircase_corner(previous: Vec2, incoming: Vec2, outgoing: Vec2, next: Vec2) -> bool {
    let orthogonal = |v: Vec2| v.x == 0.0 || v.y == 0.0;
    if !orthogonal(incoming) || !orthogonal(outgoing) {
        return false;
    }
    let turn = incoming.cross(outgoing).signum();
    (incoming.hypot() == 1.0 && orthogonal(previous) && previous.cross(incoming).signum() == -turn)
        || (outgoing.hypot() == 1.0 && orthogonal(next) && outgoing.cross(next).signum() == -turn)
}
