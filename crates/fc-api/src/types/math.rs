//! Revision-checked editing of font-wide OpenType MATH data.

use fc_doc::{command::MathCommand, DocId};
use fc_model::{MathAssemblyLayout, MathTable};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Where a document's MATH data comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum MathStatus {
    /// The font has no MATH data.
    Absent,
    /// Editable MATH data owns the exported table.
    Editable,
    /// MATH was explicitly deleted; exports omit it.
    Deleted,
    /// Only imported table bytes exist; they are exported under preservation rules.
    Imported,
}

/// MATH editor state from one consistent document revision.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MathState {
    /// Revision to use when applying edits.
    #[ts(type = "number")]
    pub generation: u64,
    /// Data ownership and availability.
    pub status: MathStatus,
    /// Editable data when status is `Editable`.
    pub math: Option<MathTable>,
    /// Why imported bytes cannot become editable, if they cannot.
    pub import_error: Option<String>,
    /// Scale of the editable font.
    pub units_per_em: u16,
}

/// Apply one or more MATH edits as one undoable operation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MathApplyParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the MATH state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Operations to apply atomically.
    pub commands: Vec<MathCommand>,
    /// Undo history label.
    pub label: String,
}

/// Convert imported MATH bytes into editable data as one undoable operation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MathDecodeParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the MATH state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Undo history label.
    pub label: String,
}

/// Request a size preview for one glyph's construction.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MathPreviewParams {
    /// Open document.
    pub doc: DocId,
    /// Glyph whose construction is previewed.
    pub glyph: String,
    /// Vertical construction when true, horizontal otherwise.
    pub vertical: bool,
    /// Requested extent in font units.
    pub target: f64,
    /// Unapplied construction data to preview instead of the saved record.
    pub construction: Option<fc_model::MathConstruction>,
    /// Unapplied minimum connector overlap; the saved value when absent.
    #[ts(optional)]
    pub min_connector_overlap: Option<f64>,
}

/// One glyph outline used by a preview.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MathPreviewGlyph {
    /// Glyph name.
    pub name: String,
    /// Default-master outline as SVG path data in font units, y up.
    pub path: String,
    /// Horizontal advance.
    pub width: f64,
    /// Ink bounds as `[x_min, y_min, x_max, y_max]`; zero for an empty outline.
    pub bounds: [f64; 4],
}

/// A ready-made variant or an assembly chosen for the requested size.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct MathPreview {
    /// Document revision used for the preview.
    #[ts(type = "number")]
    pub generation: u64,
    /// First variant whose measurement reaches the request, if any.
    pub variant: Option<String>,
    /// Assembly arrangement when no variant is large enough.
    pub assembly: Option<MathAssemblyLayout>,
    /// Largest variant when neither a variant nor an assembly reaches the request.
    pub fallback: Option<String>,
    /// Outlines for every glyph named by the result.
    pub glyphs: Vec<MathPreviewGlyph>,
}
