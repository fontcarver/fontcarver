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
    /// Whether this glyph has project-specific database information.
    ///
    /// True when the document's overlay carries a record for the name or its
    /// base, or when the glyph's own stored metadata differs from what the
    /// database resolves. This asks about the *database*, so it stays true
    /// after a refresh from an overlay record; the grid's "custom glyph info"
    /// badge asks the narrower question of whether the *glyph* departs from the
    /// effective database, and goes out again after such a refresh.
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

/// Parameters for creating glyphs with the glyph database's compositions.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct GenerateGlyphsParams {
    /// Open document.
    pub doc: DocId,
    /// Recipe in the same grammar as `glyphs.add`.
    pub recipe: String,
    /// Master used for the returned cells, or the document default master.
    #[serde(default)]
    pub master: Option<fc_model::MasterId>,
    /// Preview the recipe without changing the document or its undo stack.
    #[serde(default)]
    pub dry_run: bool,
    /// Answer with counts alone, for a caller that only shows "how many?".
    ///
    /// A repertoire recipe resolves tens of thousands of glyphs, and building
    /// and sending a cell for each of them costs far more than the number a
    /// character-set row displays. With this set, `added` and `composed` stay
    /// empty and `added_count` carries the answer. It implies a dry run.
    #[serde(default)]
    pub count_only: bool,
}

/// How one created glyph was built from the database.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GeneratedGlyph {
    /// Name of the created glyph.
    pub name: String,
    /// Component base names used, in composition order; empty for a blank glyph.
    pub components: Vec<String>,
    /// Composition members the document lacks, which left the glyph blank.
    pub unavailable: Vec<String>,
}

/// Result of one database-driven generation run.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct GenerateGlyphsResult {
    /// Number of glyphs created, or that a dry run would create.
    ///
    /// Always reported, including for a count-only request, where it is the
    /// only answer.
    #[serde(default)]
    #[ts(as = "Option<u32>", optional)]
    pub added_count: u32,
    /// Cells of the created glyphs, in creation order; empty when counting.
    pub added: Vec<crate::GlyphCell>,
    /// Recipe items skipped because they exist or could not be read.
    pub skipped: Vec<String>,
    /// One entry per created glyph describing its composition outcome.
    pub composed: Vec<GeneratedGlyph>,
}
