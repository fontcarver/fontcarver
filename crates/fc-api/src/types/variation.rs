//! Editable design spaces and interpolation preview contracts.
use fc_doc::{command::DesignSpaceCommand, DocId};
use fc_model::{Axis, Instance, Master};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// Current design-space metadata without glyph outline payloads.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct VariationState {
    /// Optimistic concurrency revision.
    #[ts(type = "number")]
    pub generation: u64,
    /// Ordered variation axes, including user-to-design mappings.
    pub axes: Vec<Axis>,
    /// Full editable master records.
    pub masters: Vec<Master>,
    /// Named export instances.
    pub instances: Vec<Instance>,
    /// First available glyph for an initial preview.
    pub first_glyph: Option<String>,
}
/// Apply a design-space transaction against its original document revision.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct VariationApplyParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the metadata query.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Complete atomic metadata update or compatibility repair.
    pub command: DesignSpaceCommand,
    /// Undo label.
    pub label: String,
}
/// Request one glyph at a position in user coordinates.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct VariationPreviewParams {
    /// Open document.
    pub doc: DocId,
    /// Persistent glyph name.
    pub glyph: String,
    /// User-space axis values, with omitted values taking axis defaults.
    pub location: BTreeMap<String, f64>,
}
/// One source outline for comparison overlays.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct VariationSourcePreview {
    /// Master or supplemental layer name.
    pub name: String,
    /// SVG path in font coordinates.
    pub path: String,
}
/// Interpolated outline, metrics and actionable compatibility diagnostics.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct VariationPreview {
    /// Revision used for this preview.
    #[ts(type = "number")]
    pub generation: u64,
    /// SVG path in font coordinates; empty when incompatible.
    pub path: String,
    /// Horizontal advance.
    pub width: f64,
    /// Vertical advance, when modeled.
    pub vertical_width: Option<f64>,
    /// Vertical origin, when modeled.
    pub vertical_origin: Option<f64>,
    /// Units per em, used to choose the preview scale.
    pub units_per_em: u16,
    /// Source overlays for correspondence inspection.
    pub sources: Vec<VariationSourcePreview>,
    /// Source-specific reasons why interpolation could not proceed.
    pub issues: Vec<String>,
}
