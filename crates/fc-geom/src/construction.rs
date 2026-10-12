//! Bounded affine glyph attachment and arc-length brush evaluation.
use std::collections::HashSet;

use fc_model::{ConstructionKind, Error, Layer, PointKind};
use kurbo::{Affine, BezPath, ParamCurve, ParamCurveArclen, PathSeg, Point, Vec2};

const MAX_STAMPS: usize = 1024;
const MAX_ELEMENTS: usize = 65_535;

fn invalid(message: &str) -> Error {
    Error::InvalidEffects(format!("construction: {message}"))
}

/// Place resolved source glyphs, retaining or hiding their editable carrier contours.
pub(crate) fn apply(
    layer: &Layer,
    input: BezPath,
    sources: &[BezPath],
    local_paths: &[BezPath],
) -> Result<BezPath, Error> {
    layer.validate_constructions()?;
    if layer.constructions.is_empty() {
        return Ok(input);
    }
    if sources.len() != layer.constructions.len() {
        return Err(invalid("source count does not match construction count"));
    }
    let mut hidden = HashSet::new();
    for entry in &layer.constructions {
        if !entry.keep_carrier {
            hidden.insert(layer.node_index(entry.kind.anchor())?.0);
        }
    }
    // Model traversal guarantees the local source contours precede components.
    let mut result = BezPath::new();
    let mut prefix = 0;
    for (index, path) in local_paths.iter().enumerate() {
        prefix += path.elements().len();
        if !hidden.contains(&index) {
            result.extend(path.iter());
        }
    }
    result.extend(input.iter().skip(prefix));
    let mut stamps = 0;
    for (entry, source) in layer.constructions.iter().zip(sources) {
        if source.is_empty() || !source.is_finite() || source.elements().len() > MAX_ELEMENTS {
            return Err(invalid(
                "source glyph must provide a nonempty bounded finite outline",
            ));
        }
        let (ci, pi) = layer.node_index(entry.kind.anchor())?;
        let contour = &layer.contours[ci];
        let point = &contour.points[pi];
        let origin = Point::new(point.x, point.y);
        match &entry.kind {
            ConstructionKind::Corner { scale, .. } | ConstructionKind::Cap { scale, .. } => {
                let backwards = matches!(entry.kind, ConstructionKind::Cap { .. }) && pi != 0;
                let mut direction = node_direction(contour, pi, backwards)?;
                if matches!(entry.kind, ConstructionKind::Cap { .. }) {
                    direction = -direction;
                }
                append(
                    &mut result,
                    source,
                    placement(origin, direction, *scale),
                    &mut stamps,
                )?;
            }
            ConstructionKind::Segment {
                end, source_length, ..
            } => {
                let end = layer.node_position(end)?;
                let direction = end - origin;
                if direction.hypot() < 1e-9 {
                    return Err(invalid("segment endpoints must occupy different positions"));
                }
                append(
                    &mut result,
                    source,
                    placement(origin, direction, direction.hypot() / source_length),
                    &mut stamps,
                )?;
            }
            ConstructionKind::Brush { spacing, scale, .. } => {
                let mut carrier = contour.clone();
                if carrier.closed {
                    carrier.points.rotate_left(pi);
                } else {
                    carrier.points = carrier.points[pi..].to_vec();
                    carrier.points[0].kind = PointKind::Move;
                }
                let path = carrier.try_to_bezpath()?;
                let segments: Vec<_> = path.segments().collect();
                let lengths: Vec<_> = segments.iter().map(|s| s.arclen(0.001)).collect();
                let total: f64 = lengths.iter().sum();
                if !total.is_finite() || total < 1e-9 {
                    return Err(invalid(
                        "brush carrier has no measurable length after its anchor",
                    ));
                }
                let count = if carrier.closed {
                    (total / spacing).ceil()
                } else {
                    (total / spacing).floor() + 1.
                };
                if count > MAX_STAMPS as f64 {
                    return Err(invalid("brush exceeds 1024 stamps; increase spacing"));
                }
                let mut segment_index = 0;
                let mut segment_start = 0.;
                for index in 0..count as usize {
                    let distance = index as f64 * spacing;
                    while segment_index + 1 < segments.len()
                        && segment_start + lengths[segment_index] <= distance
                    {
                        segment_start += lengths[segment_index];
                        segment_index += 1;
                    }
                    let segment = segments[segment_index];
                    let t = segment.inv_arclen((distance - segment_start).max(0.), 0.001);
                    let direction = tangent(segment, t)?;
                    append(
                        &mut result,
                        source,
                        placement(segment.eval(t), direction, *scale),
                        &mut stamps,
                    )?;
                }
            }
        }
    }
    check(&result)?;
    Ok(result)
}

fn placement(origin: Point, direction: Vec2, scale: f64) -> Affine {
    // Build coefficients directly to keep axis-aligned placement exact.
    let unit = direction / direction.hypot();
    Affine::new([
        unit.x * scale,
        unit.y * scale,
        -unit.y * scale,
        unit.x * scale,
        origin.x,
        origin.y,
    ])
}

fn append(
    result: &mut BezPath,
    source: &BezPath,
    transform: Affine,
    stamps: &mut usize,
) -> Result<(), Error> {
    *stamps += 1;
    if *stamps > MAX_STAMPS || result.elements().len() + source.elements().len() > MAX_ELEMENTS {
        return Err(invalid(
            "construction exceeds 1024 stamps or 65535 path elements",
        ));
    }
    let placed = transform * source;
    check(&placed)?;
    result.extend(placed);
    Ok(())
}

fn check(path: &BezPath) -> Result<(), Error> {
    let bounds = path.control_box();
    if !path.is_finite()
        || path.elements().len() > MAX_ELEMENTS
        || [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
            .iter()
            .any(|v| v.abs() > 32767.)
    {
        return Err(invalid(
            "geometry exceeds finite signed font coordinates or path budget",
        ));
    }
    Ok(())
}

fn node_direction(
    contour: &fc_model::Contour,
    point: usize,
    backwards: bool,
) -> Result<Vec2, Error> {
    let origin = &contour.points[point];
    for offset in 1..contour.points.len() {
        let index = if backwards {
            if !contour.closed && offset > point {
                break;
            }
            (point + contour.points.len() - offset) % contour.points.len()
        } else {
            if !contour.closed && point + offset >= contour.points.len() {
                break;
            }
            (point + offset) % contour.points.len()
        };
        let other = &contour.points[index];
        let direction = Vec2::new(other.x - origin.x, other.y - origin.y);
        if direction.is_finite() && direction.hypot() > 1e-9 {
            return Ok(direction);
        }
        if other.kind != PointKind::OffCurve {
            break;
        }
    }
    Err(invalid("attachment node has a degenerate tangent"))
}

fn tangent(segment: PathSeg, t: f64) -> Result<Vec2, Error> {
    use kurbo::ParamCurveDeriv;
    let derivative = match segment {
        PathSeg::Line(line) => line.p1 - line.p0,
        PathSeg::Quad(quad) => quad.deriv().eval(t).to_vec2(),
        PathSeg::Cubic(cubic) => cubic.deriv().eval(t).to_vec2(),
    };
    if derivative.hypot() > 1e-9 {
        return Ok(derivative);
    }
    let before = segment.eval((t - 0.0001).max(0.));
    let after = segment.eval((t + 0.0001).min(1.));
    let direction = after - before;
    if direction.hypot() > 1e-9 {
        Ok(direction)
    } else {
        Err(invalid("brush path has a degenerate tangent"))
    }
}
