//! Source-node corner intent evaluated without changing the editable point list.
use fc_model::{Contour, PointKind};
use kurbo::{
    Arc, BezPath, ParamCurve, ParamCurveArclen, ParamCurveDeriv, PathSeg, Point, Shape, Vec2,
};

use crate::Error;

fn invalid(message: &str) -> Error {
    Error::SmartCorner(message.into())
}

struct Join {
    incoming: f64,
    outgoing: f64,
    path: BezPath,
}

/// Evaluate signed node radii as tangent circular fillets or triangular ink traps.
///
/// Positive radii fit a circular arc tangent to the adjacent line, quadratic or
/// cubic segments. Negative radii cut a V into a closed contour's filled side;
/// their magnitude controls the mouth's arc-length setback and the tip depth.
/// Oversized radii, colliding neighbors, ambiguous nodes and singular joins fail
/// explicitly. The source contour is never modified.
pub fn smart_corner_path(contour: &Contour) -> Result<BezPath, Error> {
    let source = contour
        .try_to_bezpath()
        .map_err(|e| invalid(&e.to_string()))?;
    evaluate(contour, &source, &source)
}

pub(crate) fn smart_corner_path_in_layer(
    contour: &Contour,
    fill: &BezPath,
) -> Result<BezPath, Error> {
    let source = contour
        .try_to_bezpath()
        .map_err(|e| invalid(&e.to_string()))?;
    evaluate(contour, &source, fill)
}

fn evaluate(contour: &Contour, source: &BezPath, fill: &BezPath) -> Result<BezPath, Error> {
    if !contour.points.iter().any(|p| p.corner_radius != 0.) {
        return Ok(source.clone());
    }
    if source.elements().len() > 4096 || !source.is_finite() {
        return Err(invalid(
            "source exceeds 4096 elements or has nonfinite coordinates",
        ));
    }
    let segments: Vec<_> = source.segments().collect();
    if segments.len() < 2 {
        return Err(invalid(
            "a corner needs two nondegenerate adjacent segments",
        ));
    }
    let mut vertices: Vec<_> = segments.iter().map(|s| s.start()).collect();
    if !contour.closed {
        vertices.push(segments[segments.len() - 1].end());
    }
    let mut joins: Vec<Option<Join>> = (0..vertices.len()).map(|_| None).collect();
    let area = source.area();
    let mut remaining = 1024;
    for point in contour.points.iter().filter(|p| p.corner_radius != 0.) {
        let radius = point.corner_radius;
        if remaining == 0 || !radius.is_finite() || radius.abs() > 8192. {
            return Err(invalid(
                "at most 1024 corners, with finite radius magnitude at most 8192",
            ));
        }
        remaining -= 1;
        if point.kind == PointKind::OffCurve {
            return Err(invalid("corner radius belongs to an on-curve node"));
        }
        let matches: Vec<_> = vertices
            .iter()
            .enumerate()
            .filter_map(|(i, p)| (*p == Point::new(point.x, point.y)).then_some(i))
            .collect();
        if matches.len() != 1 {
            return Err(invalid(
                "corner node is coincident or has no distinct path join",
            ));
        }
        let i = matches[0];
        if joins[i].is_some() || (!contour.closed && (i == 0 || i == segments.len())) {
            return Err(invalid("corner requires a distinct interior node"));
        }
        // Preserve intent through interpolation while avoiding sub-resolution
        // arcs whose endpoints cannot be distinguished reliably in font units.
        if radius.abs() < 1e-7 {
            continue;
        }
        let previous = (i + segments.len() - 1) % segments.len();
        joins[i] = Some(if radius > 0. {
            round_join(segments[previous], segments[i], radius)?
        } else {
            if !contour.closed || area.abs() < 1e-6 {
                return Err(invalid(
                    "ink traps require a closed contour with a defined winding",
                ));
            }
            trap_join(segments[previous], segments[i], -radius, fill)?
        });
    }
    let mut result = BezPath::new();
    for (i, segment) in segments.iter().enumerate() {
        let next = if contour.closed {
            (i + 1) % segments.len()
        } else {
            i + 1
        };
        let start = joins[i].as_ref().map_or(0., |j| j.outgoing);
        let end = joins[next].as_ref().map_or(1., |j| j.incoming);
        if end - start < 1e-7 {
            return Err(invalid(
                "neighboring corner radii overlap; reduce their radii",
            ));
        }
        let piece = segment.subsegment(start..end);
        if i == 0 {
            result.move_to(piece.start());
        }
        result.push(piece.as_path_el());
        if let Some(join) = &joins[next] {
            result.extend(join.path.iter().skip(1));
        }
    }
    if contour.closed {
        result.close_path();
    }
    if !result.is_finite() || result.elements().len() > 16_384 {
        return Err(invalid("corner output exceeds its geometry limit"));
    }
    Ok(result)
}

fn tangent(segment: PathSeg, t: f64) -> Result<Vec2, Error> {
    let v = match segment {
        PathSeg::Line(line) => line.p1 - line.p0,
        PathSeg::Quad(curve) => curve.deriv().eval(t).to_vec2(),
        PathSeg::Cubic(curve) => curve.deriv().eval(t).to_vec2(),
    };
    let length = v.hypot();
    if !length.is_finite() || length < 1e-9 {
        return Err(invalid("adjacent segment has a singular tangent"));
    }
    Ok(v / length)
}

fn normal(tangent: Vec2) -> Vec2 {
    Vec2::new(-tangent.y, tangent.x)
}

fn offset(segment: PathSeg, t: f64, radius: f64) -> Result<Point, Error> {
    Ok(segment.eval(t) + normal(tangent(segment, t)?) * radius)
}

fn round_join(a: PathSeg, b: PathSeg, radius: f64) -> Result<Join, Error> {
    let ta = tangent(a, 1.)?;
    let tb = tangent(b, 0.)?;
    let turn = ta.cross(tb).atan2(ta.dot(tb));
    if turn.abs() < 1e-6 || turn.abs() > std::f64::consts::PI - 1e-5 {
        return Err(invalid(
            "straight and reversing joins do not define a corner",
        ));
    }
    let setback = radius * (turn.abs() * 0.5).tan();
    let tolerance = (radius * 1e-5).clamp(1e-12, 1e-7);
    let length_a = a.arclen(tolerance);
    let length_b = b.arclen(tolerance);
    if setback >= length_a || setback >= length_b {
        return Err(invalid(
            "radius exceeds its adjacent segments; reduce the radius",
        ));
    }
    let mut t = a.inv_arclen(length_a - setback, tolerance);
    let mut u = b.inv_arclen(setback, tolerance);
    let signed_radius = radius * turn.signum();
    let difference = |t, u| -> Result<Vec2, Error> {
        Ok(offset(a, t, signed_radius)? - offset(b, u, signed_radius)?)
    };
    let mut converged = false;
    for _ in 0..40 {
        let delta = difference(t, u)?;
        if delta.hypot() < tolerance {
            converged = true;
            break;
        }
        let lo_t = (t - 1e-5).max(0.);
        let hi_t = (t + 1e-5).min(1.);
        let lo_u = (u - 1e-5).max(0.);
        let hi_u = (u + 1e-5).min(1.);
        let dt = (difference(hi_t, u)? - difference(lo_t, u)?) / (hi_t - lo_t);
        let du = (difference(t, hi_u)? - difference(t, lo_u)?) / (hi_u - lo_u);
        let determinant = dt.cross(du);
        if determinant.abs() < 1e-12 {
            break;
        }
        let step_t = -delta.cross(du) / determinant;
        let step_u = -dt.cross(delta) / determinant;
        let mut improved = false;
        let mut scale = 1.;
        for _ in 0..16 {
            let next_t = t + step_t * scale;
            let next_u = u + step_u * scale;
            if next_t > 0.
                && next_t < 1.
                && next_u > 0.
                && next_u < 1.
                && difference(next_t, next_u)?.hypot2() < delta.hypot2()
            {
                t = next_t;
                u = next_u;
                improved = true;
                break;
            }
            scale *= 0.5;
        }
        if !improved {
            break;
        }
    }
    if !converged {
        return Err(invalid(
            "no bounded tangent fillet found; reduce the radius or reshape the join",
        ));
    }
    let center = offset(a, t, signed_radius)?;
    let start = a.eval(t) - center;
    let end = b.eval(u) - center;
    let sweep = start.cross(end).atan2(start.dot(end));
    if sweep * turn <= 0. || sweep.abs() > std::f64::consts::PI - 1e-5 {
        return Err(invalid("fillet would reverse the source join"));
    }
    let arc = Arc::new(
        center,
        Vec2::new(radius, radius),
        start.y.atan2(start.x),
        sweep,
        0.,
    );
    Ok(Join {
        incoming: t,
        outgoing: u,
        path: arc.to_path((radius * 1e-4).min(0.001)),
    })
}

fn trap_join(a: PathSeg, b: PathSeg, depth: f64, fill: &BezPath) -> Result<Join, Error> {
    let tolerance = (depth * 1e-5).clamp(1e-12, 1e-5);
    let length_a = a.arclen(tolerance);
    let length_b = b.arclen(tolerance);
    if depth >= length_a || depth >= length_b {
        return Err(invalid(
            "trap exceeds its adjacent segments; reduce the radius",
        ));
    }
    let direction = normal(tangent(a, 1.)?) + normal(tangent(b, 0.)?);
    if direction.hypot() < 1e-6 {
        return Err(invalid("reversing joins cannot define an ink trap"));
    }
    let direction = direction / direction.hypot();
    let probe = (depth * 0.01)
        .min(length_a * 0.001)
        .min(length_b * 0.001)
        .clamp(1e-5, 0.1);
    let left_filled = fill.winding(a.end() + direction * probe) != 0;
    let right_filled = fill.winding(a.end() - direction * probe) != 0;
    if left_filled == right_filled {
        return Err(invalid(
            "ink trap has no unambiguous filled side at this join",
        ));
    }
    let direction = if left_filled { direction } else { -direction };
    let t = a.inv_arclen(length_a - depth, tolerance);
    let u = b.inv_arclen(depth, tolerance);
    let mut path = BezPath::new();
    path.move_to(a.eval(t));
    path.line_to(a.end() + direction * depth);
    path.line_to(b.eval(u));
    Ok(Join {
        incoming: t,
        outgoing: u,
        path,
    })
}
