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
    /// Named sparse-source coordinates without physical master outlines.
    #[serde(default)]
    #[ts(optional)]
    pub virtual_masters: Option<Vec<fc_model::VirtualMaster>>,
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
/// One shape of one source: a stored contour or a component placement.
///
/// Shape order is the interpolation contract, so the compatibility view colours
/// and numbers shapes by this order and repairs swap positions in it.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct VariationShape {
    /// Index inside its own kind, matching the model's contour or component order.
    pub index: u32,
    /// Base glyph when this shape places a component.
    #[serde(default)]
    #[ts(optional)]
    pub component: Option<String>,
    /// Whether a contour shape is closed.
    pub closed: bool,
    /// This shape's own SVG path in font coordinates.
    pub path: String,
    /// The contour's first stored point, or a component's transformed origin.
    pub start: [f64; 2],
    /// Centre of the shape's bounding box, for the cross-source connector.
    pub center: [f64; 2],
    /// Ordered stored points; empty for a component.
    pub points: Vec<[f64; 2]>,
    /// `true` where the point at the same index is on-curve.
    pub on_curve: Vec<bool>,
}

/// How one shape compares across every shown source.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct VariationShapeStatus {
    /// Shape position in the reference source.
    pub index: u32,
    /// Base glyph when the shape places a component.
    #[serde(default)]
    #[ts(optional)]
    pub component: Option<String>,
    /// `compatible`, `angle` or `incompatible`.
    pub status: String,
    /// Per-segment status of the reference shape, in segment order.
    pub segments: Vec<String>,
    /// Largest direction difference on any segment, in degrees.
    pub max_angle: f64,
}

/// One source outline for comparison overlays.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct VariationSourcePreview {
    /// Master or supplemental layer name.
    pub name: String,
    /// Layer identifier, so a repair can name exactly this source.
    #[serde(default)]
    pub layer: String,
    /// SVG path in font coordinates.
    pub path: String,
    /// Shapes in shape order: stored contours first, then component placements.
    #[serde(default)]
    pub shapes: Vec<VariationShape>,
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
    /// Shape-order comparison of those overlays, in the reference's order.
    #[serde(default)]
    pub shape_status: Vec<VariationShapeStatus>,
    /// Source-specific reasons why interpolation could not proceed.
    pub issues: Vec<String>,
}

/// Change the font's units per em and optionally scale the drawing with it.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct RescaleUpmParams {
    /// Open document.
    pub doc: DocId,
    /// Revision the caller reviewed.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// New units per em, from 16 through 16384.
    pub units_per_em: u16,
    /// Scale outlines, component offsets, anchors and advances.
    #[serde(default)]
    pub scale_outlines: bool,
    /// Scale master metrics, alignment zones, stems and PostScript private values.
    #[serde(default)]
    pub scale_metrics: bool,
    /// Scale stored kerning values.
    #[serde(default)]
    pub scale_kerning: bool,
    /// Scale font, master and layer guides.
    #[serde(default)]
    pub scale_guides: bool,
    /// Undo label.
    pub label: String,
}

/// What a units-per-em change actually did.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct RescaleUpmResult {
    /// Scale factor applied to the selected groups.
    pub factor: f64,
    /// Number of glyphs rewritten.
    pub glyphs: u32,
    /// Number of source and background layers rewritten.
    pub layers: u32,
    /// Layers whose TrueType programs were cleared because they describe the old grid.
    pub cleared_programs: u32,
    /// Non-fatal notes the caller must show, such as stale preserved tables.
    pub warnings: Vec<String>,
    /// Invalidation of the single undo entry this produced.
    #[ts(type = "Change")]
    pub change: fc_doc::Change,
}

/// Per-font editor settings, stored with the project.
///
/// Glyphs keeps grid spacing, grid subdivision and the keyboard increments in
/// Font Info because they belong to the drawing, not to the person: a pixel
/// font needs a grid of two, a text face needs one. An absent field inherits
/// the application preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FontEditorSettings {
    /// Editing grid size in font units; zero disables the grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub grid_size: Option<f64>,
    /// Subdivisions per grid interval, from 1 through 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub grid_subdivision: Option<u32>,
    /// Arrow-key increments in font units: plain, shift, command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub nudge: Option<[f64; 3]>,
}

/// Stored per-font settings beside the values the editor should actually use.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EditorSettingsView {
    /// Overrides exactly as the project stores them.
    pub stored: FontEditorSettings,
    /// Grid size to use: the font's own value, or the preference.
    pub grid_size: f64,
    /// Grid subdivision to use.
    pub grid_subdivision: u32,
    /// Arrow-key increments to use.
    pub nudge: [f64; 3],
}

/// Replace one document's per-font editor settings.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SetEditorSettingsParams {
    /// Open document.
    pub doc: DocId,
    /// Complete replacement overrides; absent fields inherit the preference.
    pub settings: FontEditorSettings,
}
