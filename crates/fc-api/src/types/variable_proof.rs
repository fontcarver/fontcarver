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

/// Compile one face of the live document for a self-contained proof sheet.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
pub struct ProofFontParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Master to compile, or the default master when absent.
    #[serde(default)]
    pub master: Option<fc_model::MasterId>,
    /// Named instance to interpolate instead of compiling a master.
    #[serde(default)]
    pub instance: Option<u32>,
}

/// One compiled WOFF 2 face, with what an `@font-face` rule needs to use it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
pub struct ProofFont {
    /// Snapshot revision this face was compiled from.
    #[ts(type = "number")]
    pub generation: u64,
    /// Standard base64 of the WOFF 2 bytes, without a data-URI prefix.
    pub woff2_base64: String,
    /// Family name the embedded font declares.
    pub family_name: String,
    /// Style name of the compiled master or instance.
    pub style_name: String,
    /// Font scale, so a sheet can relate point sizes to font units.
    pub units_per_em: u16,
    /// Non-fatal compilation diagnostics, already formatted for display.
    pub warnings: Vec<String>,
}
