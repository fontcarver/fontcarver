//! Editable per-document glyph database overlays and inferred glyph metadata.
use fc_doc::DocId;
use fc_model::GlyphId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Inspect effective database metadata for one session glyph handle.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct GlyphDataStateParams {
    /// Open document.
    pub doc: DocId,
    /// Glyph to inspect.
    pub gid: GlyphId,
}

/// Resolved name, encoding, classification and construction suggestions.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct GlyphDataInfo {
    /// Persistent glyph name.
    pub name: String,
    /// Suggested Unicode scalar assignments.
    pub unicodes: Vec<u32>,
    /// Broad Unicode category.
    pub category: Option<String>,
    /// Finer classification.
    pub sub_category: Option<String>,
    /// Unicode script identifier.
    pub script: Option<String>,
    /// Suggested production name.
    pub production_name: Option<String>,
    /// Suggested anchor names, without invented coordinates.
    pub anchors: Vec<String>,
    /// Ordered component names for a composition recipe.
    pub composition: Vec<String>,
}

impl From<fc_glyphdata::GlyphInfo> for GlyphDataInfo {
    fn from(info: fc_glyphdata::GlyphInfo) -> Self {
        Self {
            name: info.name,
            unicodes: info.unicodes,
            category: info.category,
            sub_category: info.sub_category,
            script: info.script,
            production_name: info.production_name,
            anchors: info.anchors,
            composition: info.composition,
        }
    }
}

/// Database inspection and the current editable document overlay.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct GlyphDataState {
    /// Source document revision, required when applying changes.
    #[ts(type = "number")]
    pub generation: u64,
    /// Versioned JSON containing per-name field overrides.
    pub overlay_json: String,
    /// Resolved metadata for the selected glyph.
    pub info: GlyphDataInfo,
    /// Overlay metadata or glyph deviations apply to this glyph.
    pub custom: bool,
}

/// Replace the document's database overlay after validating the JSON file.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct SetGlyphDataOverlayParams {
    /// Open document.
    pub doc: DocId,
    /// Generation from the reviewed database state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// UTF-8 JSON overlay content; empty text removes all overrides.
    pub overlay_json: String,
    /// Undo label.
    pub label: String,
}

/// Refresh one captured selection from the effective database in one undo step.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct UpdateGlyphInfoParams {
    /// Open document.
    pub doc: DocId,
    /// Captured session handles, deduplicated before applying.
    pub glyphs: Vec<GlyphId>,
    /// Generation from the reviewed database state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Undo label.
    pub label: String,
}
