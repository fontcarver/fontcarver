//! Read-only text proofs at variable-font locations.

use std::collections::BTreeMap;

use fc_doc::DocId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Shape the complete interpolated font at a user-space location or named instance.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
pub struct VariableProofParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Text to shape, limited to 4096 UTF-8 bytes.
    pub text: String,
    /// User coordinates; omitted axes use their defaults.
    #[serde(default)]
    pub location: BTreeMap<String, f64>,
    /// Named instance index; its design coordinates take precedence over location.
    #[serde(default)]
    pub instance: Option<u32>,
    /// Feature tags to enable or disable.
    #[serde(default)]
    pub features: Vec<(String, bool)>,
    /// ISO 15924 script, or automatic detection.
    #[serde(default)]
    pub script: Option<String>,
    /// BCP 47 language, or the shaper default.
    #[serde(default)]
    pub language: Option<String>,
    /// Text direction, or automatic detection.
    #[serde(default)]
    pub direction: Option<String>,
}

/// One interpolated glyph positioned in font units.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
pub struct VariableProofGlyph {
    /// Source glyph name.
    pub name: Option<String>,
    /// Flattened SVG outline in font coordinates.
    pub path: String,
    /// Horizontal origin including shaping offset.
    pub x: f64,
    /// Vertical origin including shaping offset.
    pub y: f64,
    /// Shaped horizontal advance.
    pub x_advance: f64,
    /// Shaped vertical advance.
    pub y_advance: f64,
}

/// Exact geometry and shaping output for one immutable document snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
pub struct VariableProof {
    /// Snapshot revision used by this proof.
    #[ts(type = "number")]
    pub generation: u64,
    /// Resolved user-space coordinates, including named-instance coordinates.
    pub location: BTreeMap<String, f64>,
    /// Positioned outline glyphs in visual order.
    pub glyphs: Vec<VariableProofGlyph>,
    /// Font scale for rendering.
    pub units_per_em: u16,
    /// Non-fatal shaping diagnostics.
    pub warnings: Vec<String>,
}
