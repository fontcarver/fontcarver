//! Bounded bitmap editor queries and optimistic transactions.
use fc_doc::{command::BitmapCommand, DocId};
use fc_model::{BdfValue, BitmapGlyph};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// A strike without its potentially large glyph collection.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapStrikeInfo {
    /// Pixels per em.
    pub ppem: u16,
    /// Bits per pixel.
    pub depth: u8,
    /// Pixel ascent.
    pub ascent: i16,
    /// Pixel descent.
    pub descent: i16,
    /// Format properties.
    pub properties: BTreeMap<String, BdfValue>,
    /// Number of encoded bitmaps.
    pub glyph_count: u32,
}
/// Strike inventory stamped with the document generation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapState {
    /// Document revision used by edits.
    #[ts(type = "number")]
    pub generation: u64,
    /// Available sizes and depths.
    pub strikes: Vec<BitmapStrikeInfo>,
}
/// Read one glyph from one strike.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapGlyphParams {
    /// Open document.
    pub doc: DocId,
    /// Pixels per em.
    pub ppem: u16,
    /// Bits per pixel.
    pub depth: u8,
    /// Persistent glyph name.
    pub glyph: String,
}
/// Pixel data for one editor, including flattened reference display.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapGlyphView {
    /// Document revision used by edits.
    #[ts(type = "number")]
    pub generation: u64,
    /// Saved bitmap, absent if the glyph has no image at this size.
    pub value: Option<BitmapGlyph>,
    /// Flattened rendering, retaining the glyph's own metrics.
    pub resolved: Option<BitmapGlyph>,
}
/// Apply only if the editor still refers to the current document revision.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapApplyParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the most recent query.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Atomic bitmap change.
    pub command: BitmapCommand,
    /// Undo label.
    pub label: String,
}

/// Grid-fitting policy used only while generating bitmap pixels.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS)]
pub enum BitmapHinting {
    /// Use the original model outlines without grid fitting.
    None,
    /// Interpret preserved embedded font hint programs.
    Embedded,
    /// Use skrifa's automatic grid fitting without writing hint programs.
    Auto,
}
/// Rasterize outlines into a new strike or update selected glyphs in an existing strike.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapRasterParams {
    /// Open document.
    pub doc: DocId,
    /// Expected revision.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Outline master; default master if absent.
    pub master: Option<fc_model::MasterId>,
    /// Pixels per em.
    pub ppem: u16,
    /// Coverage depth (1, 2, 4 or 8).
    pub depth: u8,
    /// Selected glyphs, or all glyphs if empty.
    pub glyphs: Vec<String>,
    /// Grid fitting; omitted means unhinted.
    #[serde(default)]
    #[ts(optional)]
    pub hinting: Option<BitmapHinting>,
    /// Monochrome coverage threshold, default 128.
    #[serde(default)]
    #[ts(optional)]
    pub threshold: Option<u8>,
}
/// Exact pixel-edge tracing into the selected outline layer.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, TS)]
pub enum BitmapTraceMode {
    /// Preserve exact square pixel edges.
    #[default]
    PixelEdges,
    /// Cut each corner by one quarter pixel.
    ChamferedCorners,
    /// Replace square corners with cubic quarter circles.
    RoundedCorners,
    /// Smooth staircase corners while retaining rectangular extents.
    DiagonalSmoothing,
}
/// Trace stored pixels into the selected outline layer.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapTraceParams {
    /// Open document.
    pub doc: DocId,
    /// Expected revision.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Strike size.
    pub ppem: u16,
    /// Strike depth.
    pub depth: u8,
    /// Glyph to trace.
    pub glyph: String,
    /// Destination master.
    pub master: Option<fc_model::MasterId>,
    /// Minimum coverage, 1..255.
    pub threshold: u8,
    /// Corner treatment; omitted preserves exact pixel boundaries.
    #[serde(default)]
    #[ts(optional)]
    pub mode: Option<BitmapTraceMode>,
    /// Connect pixels touching diagonally with a bounded quarter-pixel bridge.
    #[serde(default)]
    #[ts(optional)]
    pub join_diagonals: Option<bool>,
}
/// Bounded Unicode sample rendered at every strike size.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapPreviewParams {
    /// Open document.
    pub doc: DocId,
    /// At most 128 Unicode scalar values.
    pub text: String,
}
/// One waterfall line; absent entries denote missing glyphs.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapPreviewLine {
    /// Pixel size.
    pub ppem: u16,
    /// Pixel depth.
    pub depth: u8,
    /// Baseline ascent.
    pub ascent: i16,
    /// Baseline descent.
    pub descent: i16,
    /// Glyph images in text order; no shaping is applied to legacy bitmap fonts.
    pub glyphs: Vec<Option<BitmapGlyph>>,
}
