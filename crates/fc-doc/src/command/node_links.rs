//! Atomic authoring of guide ownership and axis-interpolated servant nodes.

use super::{apply::layer_mut, PointRef};
use crate::Error;
use fc_model::{
    Font, GlyphName, GuidedNode, Layer, LayerId, NodeAxis, NodeLink, NodeLinkedGuide, PointKind,
    ServantHandle,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Persistent node-construction operations on one editable glyph layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum NodeLinkCommand {
    /// Interpolate one node and its adjacent handles between two boundaries.
    SetServant {
        /// Glyph containing the points.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Editable source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Node to constrain.
        node: PointRef,
        /// First independent boundary node.
        first: PointRef,
        /// Second independent boundary node.
        second: PointRef,
        /// Axis to interpolate.
        #[ts(type = "NodeAxis")]
        axis: NodeAxis,
    },
    /// Remove one axis relationship, retaining its current resolved coordinates.
    RemoveServant {
        /// Glyph containing the points.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Editable source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Servant node.
        node: PointRef,
        /// Axis to release.
        #[ts(type = "NodeAxis")]
        axis: NodeAxis,
    },
    /// Attach selected nodes and their handles to an ordinary layer guide.
    LinkToGuide {
        /// Glyph containing the points.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Editable source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Existing layer guide index.
        guide_index: usize,
        /// Nodes whose normal offsets must follow the guide.
        points: Vec<PointRef>,
    },
    /// Release all points owned by one guide, preserving resolved positions.
    UnlinkGuide {
        /// Glyph containing the guide.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Editable source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Existing layer guide index.
        guide_index: usize,
    },
    /// Project selected nodes and translate their neighboring handles to a guide.
    ProjectToGuide {
        /// Glyph containing the points.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Editable source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Existing layer guide index.
        guide_index: usize,
        /// Nodes to project.
        points: Vec<PointRef>,
    },
    /// Assign persistent point identities before authoring other constructions.
    EnsureNodeIds {
        /// Glyph containing the points.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Editable source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Points receiving identities if absent.
        points: Vec<PointRef>,
    },
    /// Bake and remove every node relationship in this layer.
    Clear {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Editable source layer.
        #[ts(type = "string")]
        layer: LayerId,
    },
}

impl NodeLinkCommand {
    /// The layer guide this command would change, when it names one.
    fn guide_index(&self) -> Option<usize> {
        match self {
            Self::LinkToGuide { guide_index, .. }
            | Self::UnlinkGuide { guide_index, .. }
            | Self::ProjectToGuide { guide_index, .. } => Some(*guide_index),
            _ => None,
        }
    }

    pub(crate) fn target(&self) -> (&GlyphName, &LayerId) {
        match self {
            Self::SetServant { glyph, layer, .. }
            | Self::RemoveServant { glyph, layer, .. }
            | Self::LinkToGuide { glyph, layer, .. }
            | Self::UnlinkGuide { glyph, layer, .. }
            | Self::ProjectToGuide { glyph, layer, .. }
            | Self::EnsureNodeIds { glyph, layer, .. }
            | Self::Clear { glyph, layer } => (glyph, layer),
        }
    }
}

pub(super) fn apply(font: &mut Font, command: &NodeLinkCommand) -> Result<(), Error> {
    let (glyph, layer) = command.target();
    let target = layer_mut(font, glyph, layer)?;
    let mut candidate = target.clone();
    candidate.resolve_node_links()?;
    // A locked guide owns nothing new and releases nothing: like a locked image,
    // it has to be unlocked explicitly before anything may change it.
    if let Some(index) = command.guide_index() {
        let guide = candidate
            .guides
            .get(index)
            .ok_or_else(|| invalid("guide index is outside the layer"))?;
        if guide.locked {
            return Err(super::structure::locked_guide());
        }
    }
    match command {
        NodeLinkCommand::SetServant {
            node,
            first,
            second,
            axis,
            ..
        } => set_servant(&mut candidate, *node, *first, *second, *axis)?,
        NodeLinkCommand::RemoveServant { node, axis, .. } => {
            let id = candidate.ensure_node_id(node.contour, node.point)?;
            candidate
                .node_links
                .retain(|link| link.node != id || link.axis != *axis);
        }
        NodeLinkCommand::EnsureNodeIds { points, .. } => {
            for point in points {
                candidate.ensure_node_id(point.contour, point.point)?;
            }
        }
        NodeLinkCommand::LinkToGuide {
            guide_index,
            points,
            ..
        } => link_guide(&mut candidate, *guide_index, points)?,
        NodeLinkCommand::UnlinkGuide { guide_index, .. } => {
            let guide = candidate
                .guides
                .get(*guide_index)
                .ok_or_else(|| invalid("guide index is outside the layer"))?;
            if let Some(id) = &guide.id {
                candidate
                    .linked_guides
                    .retain(|binding| &binding.guide_id != id);
            }
        }
        NodeLinkCommand::ProjectToGuide {
            guide_index,
            points,
            ..
        } => project_guide(&mut candidate, *guide_index, points)?,
        NodeLinkCommand::Clear { .. } => {
            candidate.node_links.clear();
            candidate.linked_guides.clear();
        }
    }
    candidate.resolve_node_links()?;
    *target = candidate;
    Ok(())
}

fn invalid(message: &str) -> Error {
    Error::InvalidCommand(message.into())
}

fn point_position(layer: &Layer, point: PointRef) -> Result<kurbo::Point, Error> {
    let point = layer
        .contours
        .get(point.contour)
        .and_then(|c| c.points.get(point.point))
        .ok_or_else(|| invalid("point index is outside the layer"))?;
    if !point.x.is_finite() || !point.y.is_finite() {
        return Err(invalid("point coordinates must be finite"));
    }
    Ok(kurbo::Point::new(point.x, point.y))
}

fn neighboring_handles(layer: &Layer, node: PointRef) -> Result<Vec<PointRef>, Error> {
    point_position(layer, node)?;
    let contour = &layer.contours[node.contour];
    if contour.points[node.point].kind == PointKind::OffCurve {
        return Err(invalid("choose an on-curve node"));
    }
    let mut handles = Vec::new();
    if contour.points.len() < 2 {
        return Ok(handles);
    }
    let previous = if node.point > 0 {
        Some(node.point - 1)
    } else {
        contour.closed.then_some(contour.points.len() - 1)
    };
    let next = if node.point + 1 < contour.points.len() {
        Some(node.point + 1)
    } else {
        contour.closed.then_some(0)
    };
    for point in previous.into_iter().chain(next) {
        if contour.points[point].kind == PointKind::OffCurve {
            let point = PointRef {
                contour: node.contour,
                point,
            };
            if !handles.contains(&point) {
                handles.push(point);
            }
        }
    }
    Ok(handles)
}

fn set_servant(
    layer: &mut Layer,
    node: PointRef,
    first: PointRef,
    second: PointRef,
    axis: NodeAxis,
) -> Result<(), Error> {
    let handles = neighboring_handles(layer, node)?;
    neighboring_handles(layer, first)?;
    neighboring_handles(layer, second)?;
    if node == first || node == second || first == second {
        return Err(invalid(
            "servant and its two boundaries must be distinct nodes",
        ));
    }
    let a = axis.coordinate(point_position(layer, first)?);
    let b = axis.coordinate(point_position(layer, second)?);
    if (b - a).abs() < 1e-9 {
        return Err(invalid(
            "servant boundaries must have distinct coordinates on the selected axis",
        ));
    }
    let ratio = (axis.coordinate(point_position(layer, node)?) - a) / (b - a);
    let mut handle_links = Vec::new();
    for handle in handles {
        let ratio = (axis.coordinate(point_position(layer, handle)?) - a) / (b - a);
        handle_links.push(ServantHandle {
            node: layer.ensure_node_id(handle.contour, handle.point)?,
            ratio,
        });
    }
    let node = layer.ensure_node_id(node.contour, node.point)?;
    let first = layer.ensure_node_id(first.contour, first.point)?;
    let second = layer.ensure_node_id(second.contour, second.point)?;
    layer
        .node_links
        .retain(|link| link.node != node || link.axis != axis);
    layer.node_links.push(NodeLink {
        node,
        first,
        second,
        axis,
        ratio,
        handles: handle_links,
    });
    Ok(())
}

fn guide_geometry(layer: &Layer, index: usize) -> Result<(f64, f64, f64, f64), Error> {
    let guide = layer
        .guides
        .get(index)
        .ok_or_else(|| invalid("guide index is outside the layer"))?;
    if ![guide.x, guide.y, guide.angle]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(invalid("guide coordinates must be finite"));
    }
    let (sin, cos) = guide.angle.to_radians().sin_cos();
    Ok((guide.x, guide.y, sin, cos))
}

fn link_guide(layer: &mut Layer, index: usize, points: &[PointRef]) -> Result<(), Error> {
    let (x, y, sin, cos) = guide_geometry(layer, index)?;
    if points.is_empty() {
        return Err(invalid("select at least one node to link"));
    }
    let mut owned = points.to_vec();
    for point in points {
        for handle in neighboring_handles(layer, *point)? {
            if !owned.contains(&handle) {
                owned.push(handle);
            }
        }
    }
    let mut nodes = Vec::new();
    for point in owned {
        let position = point_position(layer, point)?;
        let node = layer.ensure_node_id(point.contour, point.point)?;
        let offset = -(position.x - x) * sin + (position.y - y) * cos;
        if !nodes
            .iter()
            .any(|existing: &GuidedNode| existing.node == node)
        {
            nodes.push(GuidedNode { node, offset });
        }
    }
    let guide_id = layer.guides[index]
        .id
        .get_or_insert_with(|| fc_model::LayerId::new().to_string())
        .clone();
    if let Some(binding) = layer
        .linked_guides
        .iter_mut()
        .find(|binding| binding.guide_id == guide_id)
    {
        for node in nodes {
            if let Some(existing) = binding.nodes.iter_mut().find(|n| n.node == node.node) {
                *existing = node;
            } else {
                binding.nodes.push(node);
            }
        }
    } else {
        layer
            .linked_guides
            .push(NodeLinkedGuide { guide_id, nodes });
    }
    Ok(())
}

fn project_guide(layer: &mut Layer, index: usize, points: &[PointRef]) -> Result<(), Error> {
    let (x, y, sin, cos) = guide_geometry(layer, index)?;
    let mut translations = std::collections::BTreeMap::new();
    for point in points {
        let position = point_position(layer, *point)?;
        let normal = -(position.x - x) * sin + (position.y - y) * cos;
        for target in std::iter::once(*point).chain(neighboring_handles(layer, *point)?) {
            translations
                .entry((target.contour, target.point))
                .or_insert((normal * sin, -normal * cos));
        }
    }
    let mut projected = std::collections::BTreeMap::new();
    for ((contour, point), (dx, dy)) in translations {
        let target = &mut layer.contours[contour].points[point];
        if let Some(id) = &target.id {
            if layer.node_links.iter().any(|link| {
                link.node.id == *id || link.handles.iter().any(|handle| handle.node.id == *id)
            }) {
                return Err(invalid(
                    "release servant relationships before projecting their nodes or handles",
                ));
            }
        }
        target.x += dx;
        target.y += dy;
        if !target.x.is_finite() || !target.y.is_finite() {
            return Err(invalid("guide projection overflowed"));
        }
        if let Some(id) = &target.id {
            projected.insert(id.clone(), (target.x, target.y));
        }
    }
    // Explicit projection changes the existing guide offsets as well, so a
    // subsequent Snap + Link does not restore the old normal distance.
    for binding in &mut layer.linked_guides {
        let guide = layer
            .guides
            .iter()
            .find(|guide| guide.id.as_ref() == Some(&binding.guide_id))
            .ok_or_else(|| invalid("linked guide is missing"))?;
        let (sin, cos) = guide.angle.to_radians().sin_cos();
        for node in &mut binding.nodes {
            if let Some((x, y)) = projected.get(&node.node.id) {
                node.offset = -(x - guide.x) * sin + (y - guide.y) * cos;
            }
        }
    }
    Ok(())
}
