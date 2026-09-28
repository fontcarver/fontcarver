//! Revision-checked editing of font-wide OpenType COLR/CPAL data.

use fc_doc::DocId;
use fc_model::{ColorTable, ColrLayer, MasterId, PaletteColor};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Where a document's COLR/CPAL data comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ColorStatus {
    /// The font has no COLR/CPAL data.
    Absent,
    /// Editable COLR/CPAL data owns the exported table.
    Editable,
    /// COLR/CPAL was explicitly deleted; exports omit it.
    Deleted,
    /// Only imported table bytes exist; they are exported under preservation rules.
    Imported,
}

/// COLR/CPAL editor state from one consistent document revision.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ColorState {
    /// Revision to use when applying edits.
    #[ts(type = "number")]
    pub generation: u64,
    /// Data ownership and availability.
    pub status: ColorStatus,
    /// Editable data when status is `Editable`.
    pub color: Option<ColorTable>,
    /// Why imported bytes cannot become editable, if they cannot.
    pub import_error: Option<String>,
    /// Scale of the editable font.
    pub units_per_em: u16,
}

/// Apply one or more COLR/CPAL edits as one undoable operation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ColorApplyParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the COLR/CPAL state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Replacement data, or null to delete both tables.
    pub color: Option<ColorTable>,
    /// Undo history label.
    pub label: String,
}

/// Convert imported COLR/CPAL bytes into editable data as one undoable operation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ColorDecodeParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the COLR/CPAL state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Undo history label.
    pub label: String,
}

/// Render saved or draft layers without changing the document.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ColorPreviewParams {
    /// Open document.
    pub doc: DocId,
    /// Base glyph whose advance defines the preview.
    pub glyph: String,
    /// Master to render, defaulting to the font's default master.
    #[serde(default)]
    #[ts(optional)]
    pub master: Option<MasterId>,
    /// Saved palette index, ignored if a draft palette is supplied.
    #[serde(default)]
    pub palette_index: u16,
    /// Optional draft layers in bottom-to-top paint order.
    pub layers: Option<Vec<ColrLayer>>,
    /// Optional draft palette.
    pub palette: Option<Vec<PaletteColor>>,
}

/// A painted outline in a color preview.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ColorPreviewLayer {
    /// Outline source glyph.
    pub glyph: String,
    /// SVG path in font coordinates (y up).
    pub path: String,
    /// Unpremultiplied color, or null to use the text foreground.
    pub color: Option<PaletteColor>,
    /// Ink bounds [left, bottom, right, top].
    pub bounds: [f64; 4],
}

/// Outline-based color preview using the current document geometry.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ColorPreview {
    /// Geometry revision.
    #[ts(type = "number")]
    pub generation: u64,
    /// Horizontal advance of the base glyph.
    pub width: f64,
    /// Font coordinate scale.
    pub units_per_em: u16,
    /// Layers in source-over paint order; nested COLR is not followed.
    pub layers: Vec<ColorPreviewLayer>,
}

/// Build a bounded draft COLRv1 font for the browser's color font renderer.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ColorGraphPreviewParams {
    /// Open document providing outline geometry.
    pub doc: DocId,
    /// Root glyph to preview at U+E000 in the result.
    pub glyph: String,
    /// Draft graph and palettes; never written to the document.
    pub color: ColorTable,
    /// Palette copied to the sole output palette.
    pub palette_index: u16,
}

/// Temporary static font containing only the requested color glyph and dependencies.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ColorGraphPreview {
    /// Geometry revision from which outlines were copied.
    #[ts(type = "number")]
    pub generation: u64,
    /// TrueType sfnt bytes for an ephemeral FontFace.
    pub bytes: Vec<u8>,
}
