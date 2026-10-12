//! Equal signed curvature at selected cubic Bézier joins.

use fc_model::{Contour, PointKind};
use kurbo::{Point, Vec2};

/// Why a selected join cannot be harmonized without moving its on-curve node.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum HarmonizeError {
    /// The index does not identify a stored point.
    #[error("node {0} is outside the contour")]
    InvalidNode(usize),
    /// Two ordinary cubic segments are required on either side.
    #[error("node {0} must join two cubic segments")]
    UnsupportedNode(usize),
    /// Zero-length handles or non-finite coordinates do not define a tangent.
    #[error("node {0} has a degenerate or non-finite tangent")]
    DegenerateNode(usize),
    /// The fixed outer controls cannot provide equal signed curvature.
    #[error("node {0} has incompatible curvature on its two sides")]
    IncompatibleCurvature(usize),
    /// Coupled neighboring joins could not meet the requested tolerance.
    #[error("the selected cubic joins did not converge to equal curvature")]
    NoConvergence,
}

struct Join {
    node: usize,
    before: usize,
    after: usize,
    outer_before: usize,
    outer_after: usize,
    center: Point,
    tangent: Vec2,
    total: f64,
}

/// Aligns selected cubic handles and equalizes their signed endpoint curvature.
///
/// On-curve positions and each selected node's combined adjacent-handle length
/// stay fixed. Neighboring selected joins are solved together, since changing a
/// control also changes the curvature at the other end of its segment. A
/// successful operation marks the selected nodes smooth without changing their
/// persistent super-smooth flags. Duplicate indexes are harmless. The operation
/// is atomic: unsupported, degenerate, inflection, or non-convergent joins leave
/// the contour unchanged. An empty selection does nothing.
///
/// At a cubic endpoint, curvature is `2 cross(tangent, outer_control - node) /
/// (3 handle_length²)`. Consequently the two handle lengths have the same ratio
/// as the square roots of the two signed cross products, when their signs agree.
pub fn harmonize_nodes(contour: &mut Contour, nodes: &[usize]) -> Result<(), HarmonizeError> {
    let mut nodes = nodes.to_vec();
    nodes.sort_unstable();
    nodes.dedup();
    if nodes.is_empty() {
        return Ok(());
    }
    let joins = nodes
        .into_iter()
        .map(|node| prepare_join(contour, node))
        .collect::<Result<Vec<_>, _>>()?;
    // Reapplying a retained constraint to an accepted shape must not introduce
    // floating-point drift on every preview frame or outline replacement.
    if joins.iter().all(|join| already_harmonized(contour, join)) {
        for join in &joins {
            contour.points[join.node].smooth = true;
        }
        return Ok(());
    }
    let mut result = contour.clone();
    for join in &joins {
        let before_length = (join.center - position(&result, join.before)).hypot();
        write_handles(&mut result, join, before_length);
    }
    for _ in 0..128 {
        for join in &joins {
            let length = balanced_length(&result, join)?;
            write_handles(&mut result, join, length);
        }
        let converged =
            joins
                .iter()
                .try_fold(true, |converged, join| -> Result<_, HarmonizeError> {
                    let actual = (join.center - position(&result, join.before)).hypot();
                    let expected = balanced_length(&result, join)?;
                    Ok(converged && (actual - expected).abs() <= join.total * 1e-12)
                })?;
        if converged {
            for join in &joins {
                result.points[join.node].smooth = true;
            }
            *contour = result;
            return Ok(());
        }
    }
    Err(HarmonizeError::NoConvergence)
}

fn already_harmonized(contour: &Contour, join: &Join) -> bool {
    let incoming = join.center - position(contour, join.before);
    let outgoing = position(contour, join.after) - join.center;
    let incoming_length = incoming.hypot();
    let incoming_unit = incoming / incoming_length;
    let outgoing_unit = outgoing / outgoing.hypot();
    incoming_unit.cross(outgoing_unit).abs() <= 1e-12
        && incoming_unit.dot(outgoing_unit) > 0.
        && balanced_length(contour, join)
            .is_ok_and(|expected| (incoming_length - expected).abs() <= join.total * 1e-12)
}

fn position(contour: &Contour, index: usize) -> Point {
    let point = &contour.points[index];
    Point::new(point.x, point.y)
}

fn prepare_join(contour: &Contour, node: usize) -> Result<Join, HarmonizeError> {
    let len = contour.points.len();
    let Some(point) = contour.points.get(node) else {
        return Err(HarmonizeError::InvalidNode(node));
    };
    if point.kind != PointKind::Curve
        || len < 6
        || (!contour.closed && (node < 3 || node + 3 >= len))
    {
        return Err(HarmonizeError::UnsupportedNode(node));
    }
    let before = (node + len - 1) % len;
    let outer_before = (node + len - 2) % len;
    let previous_node = (node + len - 3) % len;
    let after = (node + 1) % len;
    let outer_after = (node + 2) % len;
    let next_node = (node + 3) % len;
    if [before, outer_before, after, outer_after]
        .iter()
        .any(|&index| contour.points[index].kind != PointKind::OffCurve)
        || contour.points[previous_node].kind == PointKind::OffCurve
        || contour.points[next_node].kind != PointKind::Curve
    {
        return Err(HarmonizeError::UnsupportedNode(node));
    }
    let center = position(contour, node);
    let incoming = center - position(contour, before);
    let outgoing = position(contour, after) - center;
    let incoming_length = incoming.hypot();
    let outgoing_length = outgoing.hypot();
    let total = incoming_length + outgoing_length;
    if !total.is_finite() || incoming_length <= 0. || outgoing_length <= 0. {
        return Err(HarmonizeError::DegenerateNode(node));
    }
    let sum = incoming / incoming_length + outgoing / outgoing_length;
    if !sum.is_finite() || sum.hypot() <= 1e-12 {
        return Err(HarmonizeError::DegenerateNode(node));
    }
    Ok(Join {
        node,
        before,
        after,
        outer_before,
        outer_after,
        center,
        tangent: sum / sum.hypot(),
        total,
    })
}

fn balanced_length(contour: &Contour, join: &Join) -> Result<f64, HarmonizeError> {
    let before = position(contour, join.outer_before) - join.center;
    let after = position(contour, join.outer_after) - join.center;
    let incoming = join.tangent.cross(before);
    let outgoing = join.tangent.cross(after);
    if !incoming.is_finite() || !outgoing.is_finite() {
        return Err(HarmonizeError::DegenerateNode(join.node));
    }
    let tolerance = before.hypot().max(after.hypot()) * 1e-14;
    if incoming.abs() <= tolerance && outgoing.abs() <= tolerance {
        return Ok((join.center - position(contour, join.before)).hypot());
    }
    if incoming.abs() <= tolerance
        || outgoing.abs() <= tolerance
        || incoming.is_sign_positive() != outgoing.is_sign_positive()
    {
        return Err(HarmonizeError::IncompatibleCurvature(join.node));
    }
    let left = incoming.abs().sqrt();
    let right = outgoing.abs().sqrt();
    let length = join.total * (left / (left + right));
    if length <= 0. || length >= join.total || !length.is_finite() {
        return Err(HarmonizeError::DegenerateNode(join.node));
    }
    Ok(length)
}

fn write_handles(contour: &mut Contour, join: &Join, before_length: f64) {
    let before = join.center - join.tangent * before_length;
    let after = join.center + join.tangent * (join.total - before_length);
    contour.points[join.before].x = before.x;
    contour.points[join.before].y = before.y;
    contour.points[join.after].x = after.x;
    contour.points[join.after].y = after.y;
}
