//! Reusable glyph attachments on persistent carrier nodes.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{Error, GlyphName, Layer, NodeRef, PointKind};

/// One reusable glyph placement or repeated brush along an editable carrier.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GlyphConstruction {
    /// Stable identity within its layer.
    pub id: String,
    /// Glyph supplying the drawing, in the carrier layer's master.
    pub source: GlyphName,
    /// Placement and explicit geometric parameters.
    pub kind: ConstructionKind,
    /// Include the referenced carrier contour in evaluated output.
    #[serde(default)]
    pub keep_carrier: bool,
}

/// Affine construction semantics; source coordinates use an origin of `(0,0)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ConstructionKind {
    /// Attach the source origin at an interior node, rotating +x to its outgoing tangent.
    Corner {
        /// Stable interior on-curve node.
        at: NodeRef,
        /// Uniform source scale.
        scale: f64,
    },
    /// Attach at an open endpoint, rotating +x outward from the carrier.
    Cap {
        /// Stable open endpoint.
        at: NodeRef,
        /// Uniform source scale.
        scale: f64,
    },
    /// Map a straight source baseline to the chord between two carrier nodes.
    Segment {
        /// Source origin destination.
        start: NodeRef,
        /// Source baseline endpoint destination on the same contour.
        end: NodeRef,
        /// Positive source baseline length; transverse scale matches longitudinal scale.
        source_length: f64,
    },
    /// Repeat source origins by arc length from a node through the contour's end.
    /// A closed carrier makes one complete circuit without duplicating the first stamp.
    Brush {
        /// Starting on-curve node and carrier selector.
        at: NodeRef,
        /// Distance between stamp origins in font units.
        spacing: f64,
        /// Uniform scale applied to every stamp.
        scale: f64,
    },
}

impl ConstructionKind {
    /// First attachment, which identifies the carrier contour.
    pub fn anchor(&self) -> &NodeRef {
        match self {
            Self::Corner { at, .. } | Self::Cap { at, .. } | Self::Brush { at, .. } => at,
            Self::Segment { start, .. } => start,
        }
    }
}

impl Layer {
    /// Check construction identities, parameters and carrier topology.
    pub fn validate_constructions(&self) -> Result<(), Error> {
        let invalid = |s: &str| Error::InvalidEffects(format!("construction: {s}"));
        if self.constructions.len() > 128 {
            return Err(invalid("at most 128 constructions per layer"));
        }
        let mut ids = HashSet::new();
        for entry in &self.constructions {
            if entry.id.is_empty() || entry.id.len() > 256 || !ids.insert(&entry.id) {
                return Err(invalid(
                    "construction identifiers must be nonempty and unique",
                ));
            }
            if entry.source.is_empty() {
                return Err(invalid("source glyph name is empty"));
            }
            let (ci, pi) = self.node_index(entry.kind.anchor())?;
            let contour = &self.contours[ci];
            if contour.points[pi].kind == PointKind::OffCurve {
                return Err(invalid("attachments require on-curve nodes"));
            }
            let scale = match &entry.kind {
                ConstructionKind::Corner { scale, .. } => {
                    if !contour.closed && (pi == 0 || pi + 1 == contour.points.len()) {
                        return Err(invalid("corner requires an interior node"));
                    }
                    *scale
                }
                ConstructionKind::Cap { scale, .. } => {
                    if contour.closed || (pi != 0 && pi + 1 != contour.points.len()) {
                        return Err(invalid("cap requires an open contour endpoint"));
                    }
                    *scale
                }
                ConstructionKind::Segment {
                    end, source_length, ..
                } => {
                    let (end_ci, end_pi) = self.node_index(end)?;
                    if end_ci != ci
                        || end_pi == pi
                        || self.contours[end_ci].points[end_pi].kind == PointKind::OffCurve
                    {
                        return Err(invalid(
                            "segment requires distinct on-curve nodes on one contour",
                        ));
                    }
                    if !source_length.is_finite() || !(0.01..=32767.).contains(source_length) {
                        return Err(invalid(
                            "source baseline length must be between 0.01 and 32767",
                        ));
                    }
                    1.
                }
                ConstructionKind::Brush { spacing, scale, .. } => {
                    if !spacing.is_finite() || !(0.1..=32767.).contains(spacing) {
                        return Err(invalid("brush spacing must be between 0.1 and 32767"));
                    }
                    *scale
                }
            };
            if !scale.is_finite() || !(0.001..=1000.).contains(&scale) {
                return Err(invalid("scale must be between 0.001 and 1000"));
            }
        }
        Ok(())
    }
}
