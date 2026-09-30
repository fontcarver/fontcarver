//! Opening a corner into an overlapping construction, and closing it again.
//!
//! An *open corner* is the professional way to keep a joint sharp while the
//! two strokes that form it are still separately editable: the corner node is
//! replaced by two nodes placed along the two adjacent tangents *past* the
//! corner, so the contour crosses itself in a small triangle. Removing the
//! overlap reproduces the original sharp corner exactly, and *reconnecting*
//! the two nodes — the inverse operation — puts the corner back at the
//! intersection of the same two tangents.
//!
//! Both operations are pure: they return a new contour and never touch the
//! input, so a command can fail without having changed the document.

use fc_model::{Contour, Point, PointKind};
use kurbo::Vec2;

/// Tangents this close to parallel do not define a corner.
const PARALLEL_EPSILON: f64 = 1e-9;

/// Why a corner cannot be opened or reconnected.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CornerError {
    /// The index does not identify a stored point.
    #[error("node {0} is outside the contour")]
    InvalidNode(usize),
    /// Off-curve control points have no corner to open.
    #[error("node {0} is not an on-curve point")]
    OffCurveNode(usize),
    /// A corner needs a segment on either side of the node.
    #[error("node {0} needs an adjacent segment on both sides")]
    UnsupportedNode(usize),
    /// Coincident neighbours or non-finite coordinates define no tangent.
    #[error("node {0} has a degenerate or non-finite tangent")]
    DegenerateNode(usize),
    /// A live smart corner already owns the join.
    #[error("node {0} carries a live smart corner; bake or remove it first")]
    SmartCorner(usize),
    /// The setback must be a positive, finite number of font units.
    #[error("the corner setback must be finite and positive")]
    InvalidDepth,
    /// The setback has to stay inside both adjacent segments.
    #[error("the setback is not shorter than an adjacent segment at node {0}")]
    DepthTooLarge(usize),
    /// Reconnecting needs the two ends of exactly one segment.
    #[error("nodes {0} and {1} are not the two ends of one segment of one contour")]
    NotOneSegment(usize, usize),
    /// Parallel tangents never meet.
    #[error("the tangents at nodes {0} and {1} are parallel, so they meet at no corner")]
    ParallelTangents(usize, usize),
    /// The tangents meet behind the first node or ahead of the second one.
    #[error("the tangents at nodes {0} and {1} do not meet in front of the open corner")]
    NoCornerAhead(usize, usize),
    /// The merged node would land on a neighbour and collapse the contour.
    #[error("reconnecting nodes {0} and {1} would collapse the contour onto a neighbour")]
    DegenerateResult(usize, usize),
}

/// One on-curve node together with the tangents of its two segments.
struct Corner {
    /// Previous on-curve index.
    previous: usize,
    /// Next on-curve index.
    next: usize,
    /// Unit direction the incoming segment arrives in.
    arrive: Vec2,
    /// Unit direction the outgoing segment leaves in.
    leave: Vec2,
}

fn at(contour: &Contour, index: usize) -> Vec2 {
    let point = &contour.points[index];
    Vec2::new(point.x, point.y)
}

fn finite(vector: Vec2) -> bool {
    vector.x.is_finite() && vector.y.is_finite()
}

/// The stored indexes of one cyclic step from `from`, exclusive of both ends.
fn controls_between(contour: &Contour, from: usize, to: usize) -> Vec<usize> {
    let len = contour.points.len();
    let mut found = Vec::new();
    let mut index = (from + 1) % len;
    while index != to {
        if contour.points[index].kind == PointKind::OffCurve {
            found.push(index);
        }
        index = (index + 1) % len;
    }
    found
}

/// The on-curve point before `node`, or `None` at the head of an open contour.
fn previous_on_curve(contour: &Contour, node: usize) -> Option<usize> {
    let len = contour.points.len();
    for step in 1..len {
        let index = if contour.closed {
            (node + len - step) % len
        } else {
            node.checked_sub(step)?
        };
        if contour.points[index].kind != PointKind::OffCurve {
            return Some(index);
        }
    }
    None
}

/// The on-curve point after `node`, or `None` at the tail of an open contour.
fn next_on_curve(contour: &Contour, node: usize) -> Option<usize> {
    let len = contour.points.len();
    for step in 1..len {
        let index = if contour.closed {
            (node + step) % len
        } else {
            let index = node + step;
            if index >= len {
                return None;
            }
            index
        };
        if contour.points[index].kind != PointKind::OffCurve {
            return Some(index);
        }
    }
    None
}

fn on_curve_node(contour: &Contour, node: usize) -> Result<(), CornerError> {
    let point = contour
        .points
        .get(node)
        .ok_or(CornerError::InvalidNode(node))?;
    if point.kind == PointKind::OffCurve {
        return Err(CornerError::OffCurveNode(node));
    }
    if !point.x.is_finite() || !point.y.is_finite() {
        return Err(CornerError::DegenerateNode(node));
    }
    if point.corner_radius != 0.0 {
        return Err(CornerError::SmartCorner(node));
    }
    Ok(())
}

fn corner_of(contour: &Contour, node: usize) -> Result<Corner, CornerError> {
    on_curve_node(contour, node)?;
    let previous = previous_on_curve(contour, node).ok_or(CornerError::UnsupportedNode(node))?;
    let next = next_on_curve(contour, node).ok_or(CornerError::UnsupportedNode(node))?;
    if previous == node || next == node || previous == next {
        return Err(CornerError::UnsupportedNode(node));
    }
    let incoming_controls = controls_between(contour, previous, node);
    let outgoing_controls = controls_between(contour, node, next);
    let here = at(contour, node);
    let before = incoming_controls
        .last()
        .map_or_else(|| at(contour, previous), |&index| at(contour, index));
    let after = outgoing_controls
        .first()
        .map_or_else(|| at(contour, next), |&index| at(contour, index));
    if !finite(here) || !finite(before) || !finite(after) {
        return Err(CornerError::DegenerateNode(node));
    }
    let arrive = here - before;
    let leave = after - here;
    if arrive.hypot() <= PARALLEL_EPSILON || leave.hypot() <= PARALLEL_EPSILON {
        return Err(CornerError::DegenerateNode(node));
    }
    Ok(Corner {
        previous,
        next,
        arrive: arrive.normalize(),
        leave: leave.normalize(),
    })
}

/// A fresh corner node at `position`, without inherited identity.
fn fresh(position: Vec2, kind: PointKind) -> Point {
    Point {
        x: position.x,
        y: position.y,
        kind,
        ..Point::default()
    }
}

/// Replaces one corner node with the two nodes of an open corner.
///
/// The node keeps its stored identity and its incoming segment's kind, moving
/// `depth` font units forward along the direction the incoming segment arrives
/// in. A second, fresh corner node is placed `depth` units back along the
/// direction the outgoing segment leaves in, and the straight segment between
/// the two closes the mouth. Both adjacent tangents are unchanged, so at a
/// concave (reflex) corner the construction crosses itself in a triangle whose
/// apex is the original corner, and `remove_overlap` reproduces that corner.
/// [`reconnect_nodes`] restores the node exactly at any corner.
///
/// Only a reflex corner can cross its own contour, so the triangle of an opened
/// *convex* corner lies outside the ink and survives overlap removal as an ear.
/// That is the caller's choice to make — the construction is still exactly
/// reversible — and it is what an open corner formed by two overlapping
/// contours relies on.
///
/// The operation is atomic. A node that is off-curve, out of range, at the end
/// of an open contour, degenerate, or carrying a live smart corner is rejected,
/// as is a setback that is not positive, not finite, or not shorter than both
/// adjacent chords.
pub fn open_corner(contour: &Contour, node: usize, depth: f64) -> Result<Contour, CornerError> {
    if !depth.is_finite() || depth <= 0.0 {
        return Err(CornerError::InvalidDepth);
    }
    let corner = corner_of(contour, node)?;
    let here = at(contour, node);
    let incoming_chord = (here - at(contour, corner.previous)).hypot();
    let outgoing_chord = (at(contour, corner.next) - here).hypot();
    if depth >= incoming_chord || depth >= outgoing_chord {
        return Err(CornerError::DepthTooLarge(node));
    }
    let mouth_start = here + corner.arrive * depth;
    let mouth_end = here - corner.leave * depth;
    if !finite(mouth_start) || !finite(mouth_end) {
        return Err(CornerError::DegenerateNode(node));
    }
    let mut points = Vec::with_capacity(contour.points.len() + 1);
    for (index, point) in contour.points.iter().enumerate() {
        if index != node {
            points.push(point.clone());
            continue;
        }
        let mut kept = point.clone();
        kept.x = mouth_start.x;
        kept.y = mouth_start.y;
        kept.smooth = false;
        kept.super_smooth = false;
        points.push(kept);
        points.push(fresh(mouth_end, PointKind::Line));
    }
    Ok(Contour {
        points,
        ..contour.clone()
    })
}

/// Merges the two nodes of an open corner back into one corner node.
///
/// `a` and `b` must be the two on-curve ends of one segment of one contour, in
/// path order. The merged node sits where the incoming tangent at `a` meets the
/// outgoing tangent at `b`, keeps `a`'s stored identity and segment kind, and
/// the segment between the two nodes is dropped together with its controls. For
/// a corner produced by [`open_corner`] this restores the original node exactly.
///
/// The operation is atomic. Nodes that are not one segment apart, tangents that
/// are parallel, and an intersection that lies behind `a` or ahead of `b` are
/// all rejected without changing the contour.
pub fn reconnect_nodes(contour: &Contour, a: usize, b: usize) -> Result<Contour, CornerError> {
    on_curve_node(contour, a)?;
    on_curve_node(contour, b)?;
    if a == b {
        return Err(CornerError::NotOneSegment(a, b));
    }
    if next_on_curve(contour, a) != Some(b) {
        return Err(CornerError::NotOneSegment(a, b));
    }
    let before = previous_on_curve(contour, a).ok_or(CornerError::UnsupportedNode(a))?;
    let after = next_on_curve(contour, b).ok_or(CornerError::UnsupportedNode(b))?;
    if before == b || after == a || before == a || after == b {
        return Err(CornerError::NotOneSegment(a, b));
    }
    let incoming_controls = controls_between(contour, before, a);
    let outgoing_controls = controls_between(contour, b, after);
    let start = at(contour, a);
    let end = at(contour, b);
    let arrive_from = incoming_controls
        .last()
        .map_or_else(|| at(contour, before), |&index| at(contour, index));
    let leave_to = outgoing_controls
        .first()
        .map_or_else(|| at(contour, after), |&index| at(contour, index));
    if !finite(start) || !finite(end) || !finite(arrive_from) || !finite(leave_to) {
        return Err(CornerError::DegenerateNode(a));
    }
    let arrive = start - arrive_from;
    let leave = leave_to - end;
    if arrive.hypot() <= PARALLEL_EPSILON {
        return Err(CornerError::DegenerateNode(a));
    }
    if leave.hypot() <= PARALLEL_EPSILON {
        return Err(CornerError::DegenerateNode(b));
    }
    let arrive = arrive.normalize();
    let leave = leave.normalize();
    let cross = arrive.cross(leave);
    if cross.abs() <= PARALLEL_EPSILON {
        return Err(CornerError::ParallelTangents(a, b));
    }
    let gap = end - start;
    // The two parameters of the meeting point: along `arrive` measured from
    // `a`, and along `leave` measured from `b`. An open corner puts the corner
    // behind `a` and ahead of `b`, which is the only topology that reconnects.
    let behind_a = gap.cross(leave) / cross;
    let ahead_of_b = gap.cross(arrive) / cross;
    if !behind_a.is_finite() || !ahead_of_b.is_finite() {
        return Err(CornerError::ParallelTangents(a, b));
    }
    if behind_a > 0.0 || ahead_of_b < 0.0 {
        return Err(CornerError::NoCornerAhead(a, b));
    }
    let meeting = start + arrive * behind_a;
    if !finite(meeting) {
        return Err(CornerError::ParallelTangents(a, b));
    }
    if (meeting - at(contour, before)).hypot() <= PARALLEL_EPSILON
        || (meeting - at(contour, after)).hypot() <= PARALLEL_EPSILON
    {
        return Err(CornerError::DegenerateResult(a, b));
    }
    // The mouth's own controls and one of its two ends go away. The merged node
    // stays where the closing seam keeps a closed contour's stored start point.
    let dropped: Vec<usize> = controls_between(contour, a, b);
    let merge_at = if b == 0 { b } else { a };
    let removed = if b == 0 { a } else { b };
    let mut points = Vec::with_capacity(contour.points.len());
    for (index, point) in contour.points.iter().enumerate() {
        if index == removed || dropped.contains(&index) {
            continue;
        }
        if index == merge_at {
            let mut merged = contour.points[a].clone();
            merged.x = meeting.x;
            merged.y = meeting.y;
            merged.smooth = false;
            merged.super_smooth = false;
            points.push(merged);
            continue;
        }
        points.push(point.clone());
    }
    Ok(Contour {
        points,
        ..contour.clone()
    })
}
