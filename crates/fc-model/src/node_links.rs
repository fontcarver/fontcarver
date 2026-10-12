//! Stable layer-local construction references and editable node constraints.

use std::collections::{BTreeMap, BTreeSet};

use kurbo::Point as KurboPoint;
use serde::{Deserialize, Serialize};

use crate::{Contour, Error, Layer};

/// Stable identity of a point within its containing layer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NodeRef {
    /// Persistent point identifier.
    pub id: String,
}

/// Coordinate constrained by a servant relationship.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NodeAxis {
    /// Horizontal coordinate.
    X,
    /// Vertical coordinate.
    Y,
}
impl NodeAxis {
    /// Reads the selected coordinate.
    pub fn coordinate(self, point: KurboPoint) -> f64 {
        match self {
            Self::X => point.x,
            Self::Y => point.y,
        }
    }
}

/// Neighboring handle that scales with its servant node's boundary span.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ServantHandle {
    /// Handle identity.
    pub node: NodeRef,
    /// Interpolation fraction, allowing values outside the boundary interval.
    pub ratio: f64,
}

/// An axis-interpolated node between two independently editable boundary nodes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NodeLink {
    /// Servant node.
    pub node: NodeRef,
    /// First interpolation boundary.
    pub first: NodeRef,
    /// Second interpolation boundary.
    pub second: NodeRef,
    /// Constrained coordinate.
    pub axis: NodeAxis,
    /// Fraction from first to second boundary.
    pub ratio: f64,
    /// Adjacent handle coordinates following the same interpolation span.
    #[serde(default)]
    pub handles: Vec<ServantHandle>,
}

/// One point's signed normal offset from its owning guide.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuidedNode {
    /// Stable point identity.
    pub node: NodeRef,
    /// Signed distance along the guide's left normal, in font units.
    pub offset: f64,
}

/// A layer guide that owns one or more outline point coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NodeLinkedGuide {
    /// Persistent identity of an ordinary layer guide.
    pub guide_id: String,
    /// Owned points and their preserved offsets.
    pub nodes: Vec<GuidedNode>,
}

type Coordinate = (String, NodeAxis);
type Constraints = BTreeMap<Coordinate, (String, String, f64)>;

fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidNodeLinks(message.into())
}

impl Layer {
    /// Finds a stable reference, rejecting missing or ambiguous point identities.
    pub fn node_index(&self, node: &NodeRef) -> Result<(usize, usize), Error> {
        let mut found = None;
        for (ci, contour) in self.contours.iter().enumerate() {
            for (pi, point) in contour.points.iter().enumerate() {
                if point.id.as_deref() == Some(node.id.as_str())
                    && found.replace((ci, pi)).is_some()
                {
                    return Err(invalid("duplicate point identity"));
                }
            }
        }
        found.ok_or_else(|| {
            invalid(format!(
                "missing point {}; unlink it before deleting it",
                node.id
            ))
        })
    }

    /// Assigns a stable identity to an existing stored point when needed.
    pub fn ensure_node_id(&mut self, contour: usize, point: usize) -> Result<NodeRef, Error> {
        let point = self
            .contours
            .get_mut(contour)
            .and_then(|c| c.points.get_mut(point))
            .ok_or_else(|| invalid("point index is outside the layer"))?;
        let id = point
            .id
            .get_or_insert_with(|| uuid::Uuid::new_v4().to_string())
            .clone();
        if id.is_empty() {
            return Err(invalid("point identity cannot be empty"));
        }
        Ok(NodeRef { id })
    }

    /// Resolves the current position of a stable node, including constraints.
    pub fn node_position(&self, node: &NodeRef) -> Result<KurboPoint, Error> {
        let (ci, pi) = self.node_index(node)?;
        if self.node_links.is_empty() && self.linked_guides.is_empty() {
            let point = &self.contours[ci].points[pi];
            return Ok(KurboPoint::new(point.x, point.y));
        }
        self.linked_positions()?
            .get(&node.id)
            .copied()
            .ok_or_else(|| invalid("missing node"))
    }

    /// Checks finite values, stable identities, coordinate ownership and cycles.
    pub fn validate_node_links(&self) -> Result<(), Error> {
        if self.node_links.is_empty() && self.linked_guides.is_empty() {
            return Ok(());
        }
        self.linked_positions().map(|_| ())
    }

    /// Returns a resolved contour snapshot without mutating source metadata.
    pub fn resolved_node_contours(&self) -> Result<Vec<Contour>, Error> {
        let mut layer = self.clone();
        layer.resolve_node_links()?;
        Ok(layer.contours)
    }

    /// Applies guide motion and axis interpolation atomically to stored points.
    pub fn resolve_node_links(&mut self) -> Result<(), Error> {
        if self.node_links.is_empty() && self.linked_guides.is_empty() {
            return Ok(());
        }
        let positions = self.linked_positions()?;
        for point in self.contours.iter_mut().flat_map(|c| &mut c.points) {
            if let Some(position) = point.id.as_ref().and_then(|id| positions.get(id)) {
                point.x = position.x;
                point.y = position.y;
            }
        }
        Ok(())
    }

    fn linked_positions(&self) -> Result<BTreeMap<String, KurboPoint>, Error> {
        let mut positions = BTreeMap::new();
        for point in self.contours.iter().flat_map(|c| &c.points) {
            if let Some(id) = &point.id {
                if id.is_empty()
                    || positions
                        .insert(id.clone(), KurboPoint::new(point.x, point.y))
                        .is_some()
                {
                    return Err(invalid(
                        "point identities must be nonempty and unique within the layer",
                    ));
                }
                if !point.x.is_finite() || !point.y.is_finite() {
                    return Err(invalid("node coordinates must be finite"));
                }
            }
        }
        let mut guided: BTreeMap<String, Vec<(f64, f64, f64)>> = BTreeMap::new();
        let mut guides = BTreeSet::new();
        for binding in &self.linked_guides {
            if !guides.insert(&binding.guide_id) {
                return Err(invalid("duplicate linked guide identity"));
            }
            let mut matches = self
                .guides
                .iter()
                .filter(|g| g.id.as_ref() == Some(&binding.guide_id));
            let guide = matches.next().ok_or_else(|| {
                invalid("missing linked guide; unlink its nodes before deleting it")
            })?;
            if binding.guide_id.is_empty() || matches.next().is_some() {
                return Err(invalid("guide identities must be nonempty and unique"));
            }
            if ![guide.x, guide.y, guide.angle]
                .iter()
                .all(|v| v.is_finite())
            {
                return Err(invalid("linked guide coordinates must be finite"));
            }
            let (sin, cos) = guide.angle.to_radians().sin_cos();
            let mut seen = BTreeSet::new();
            for node in &binding.nodes {
                if !node.offset.is_finite() || !seen.insert(&node.node.id) {
                    return Err(invalid(
                        "guided offsets must be finite and a guide cannot repeat a point",
                    ));
                }
                if !positions.contains_key(&node.node.id) {
                    return Err(invalid(
                        "missing guide-linked point; unlink it before deleting it",
                    ));
                }
                let equations = guided.entry(node.node.id.clone()).or_default();
                if equations.len() >= 2
                    || equations
                        .iter()
                        .any(|(nx, ny, _)| (nx * cos + ny * sin).abs() < 1e-9)
                {
                    return Err(invalid("a node can follow at most two nonparallel guides; unlink the parallel guide first"));
                }
                equations.push((-sin, cos, -guide.x * sin + guide.y * cos + node.offset));
            }
        }
        for (id, equations) in &guided {
            let point = positions
                .get_mut(id)
                .ok_or_else(|| invalid("missing guided point"))?;
            let (nx, ny, c) = equations[0];
            if let Some(&(mx, my, d)) = equations.get(1) {
                let determinant = nx * my - ny * mx;
                point.x = (c * my - ny * d) / determinant;
                point.y = (nx * d - c * mx) / determinant;
            } else {
                let delta = c - nx * point.x - ny * point.y;
                point.x += nx * delta;
                point.y += ny * delta;
            }
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(invalid("guide resolution overflowed"));
            }
        }

        let mut constraints = Constraints::new();
        for link in &self.node_links {
            if link.first == link.second {
                return Err(invalid("servant boundaries must be distinct"));
            }
            for (node, ratio) in std::iter::once((&link.node, link.ratio))
                .chain(link.handles.iter().map(|h| (&h.node, h.ratio)))
            {
                if !ratio.is_finite() || guided.contains_key(&node.id) {
                    return Err(invalid(
                        "servant ratio must be finite and a guided node cannot also be a servant",
                    ));
                }
                for id in [&node.id, &link.first.id, &link.second.id] {
                    if !positions.contains_key(id) {
                        return Err(invalid(format!(
                            "missing node {id}; unlink it before deleting it"
                        )));
                    }
                }
                if constraints
                    .insert(
                        (node.id.clone(), link.axis),
                        (link.first.id.clone(), link.second.id.clone(), ratio),
                    )
                    .is_some()
                {
                    return Err(invalid(
                        "each point coordinate can have only one servant owner",
                    ));
                }
            }
        }
        let mut values = BTreeMap::new();
        for key in constraints.keys() {
            resolve_coordinate(
                key,
                &constraints,
                &positions,
                &mut values,
                &mut BTreeSet::new(),
            )?;
        }
        for ((id, axis), value) in values {
            if let Some(point) = positions.get_mut(&id) {
                match axis {
                    NodeAxis::X => point.x = value,
                    NodeAxis::Y => point.y = value,
                }
            }
        }
        Ok(positions)
    }
}

fn resolve_coordinate(
    key: &Coordinate,
    constraints: &Constraints,
    positions: &BTreeMap<String, KurboPoint>,
    values: &mut BTreeMap<Coordinate, f64>,
    visiting: &mut BTreeSet<Coordinate>,
) -> Result<f64, Error> {
    if let Some(value) = values.get(key) {
        return Ok(*value);
    }
    let Some((first, second, ratio)) = constraints.get(key) else {
        return positions
            .get(&key.0)
            .map(|p| key.1.coordinate(*p))
            .ok_or_else(|| invalid("missing boundary node"));
    };
    if visiting.len() >= 128 || !visiting.insert(key.clone()) {
        return Err(invalid("servant cycle or dependency depth above 128"));
    }
    let a = resolve_coordinate(
        &(first.clone(), key.1),
        constraints,
        positions,
        values,
        visiting,
    )?;
    let b = resolve_coordinate(
        &(second.clone(), key.1),
        constraints,
        positions,
        values,
        visiting,
    )?;
    let value = a + (b - a) * ratio;
    if !value.is_finite() {
        return Err(invalid("servant resolution overflowed"));
    }
    visiting.remove(key);
    values.insert(key.clone(), value);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Contour, Guide, Point};

    fn layer() -> Layer {
        Layer {
            contours: vec![Contour {
                points: vec![
                    Point {
                        id: Some("a".into()),
                        x: 0.,
                        ..Point::default()
                    },
                    Point {
                        id: Some("b".into()),
                        x: 50.,
                        ..Point::default()
                    },
                    Point {
                        id: Some("c".into()),
                        x: 100.,
                        ..Point::default()
                    },
                ],
                ..Contour::default()
            }],
            ..Layer::default()
        }
    }
    fn node(id: &str) -> NodeRef {
        NodeRef { id: id.into() }
    }
    fn link() -> NodeLink {
        NodeLink {
            node: node("b"),
            first: node("a"),
            second: node("c"),
            axis: NodeAxis::X,
            ratio: 0.5,
            handles: vec![],
        }
    }
    #[test]
    fn interpolation_survives_reordering() {
        let mut layer = layer();
        layer.node_links.push(link());
        layer.contours[0].points[2].x = 200.;
        layer.contours[0].points.reverse();
        assert_eq!(layer.node_position(&node("b")).unwrap().x, 100.);
        layer.resolve_node_links().unwrap();
        assert_eq!(layer.contours[0].points[1].x, 100.);
    }
    #[test]
    fn rejects_cycles_dangling_and_nonfinite() {
        let mut layer = layer();
        layer.node_links.push(link());
        layer.node_links.push(NodeLink {
            node: node("a"),
            first: node("b"),
            second: node("c"),
            axis: NodeAxis::X,
            ratio: 0.5,
            handles: vec![],
        });
        assert!(layer.validate_node_links().is_err());
        layer.node_links.pop();
        layer.node_links[0].first = node("missing");
        assert!(layer.validate_node_links().is_err());
        layer.node_links[0].first = node("a");
        layer.node_links[0].ratio = f64::INFINITY;
        assert!(layer.validate_node_links().is_err());
    }
    #[test]
    fn guide_owns_offset_and_moves_its_nodes() {
        let mut layer = layer();
        layer.guides.push(Guide {
            id: Some("g".into()),
            y: 30.,
            ..Guide::default()
        });
        layer.linked_guides.push(NodeLinkedGuide {
            guide_id: "g".into(),
            nodes: vec![GuidedNode {
                node: node("b"),
                offset: 10.,
            }],
        });
        layer.resolve_node_links().unwrap();
        assert_eq!(layer.contours[0].points[1].y, 40.);
        layer.guides[0].y = 50.;
        layer.resolve_node_links().unwrap();
        assert_eq!(layer.contours[0].points[1].y, 60.);
        layer.guides.clear();
        assert!(layer.validate_node_links().is_err());
    }
}
