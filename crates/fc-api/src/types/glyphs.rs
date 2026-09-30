//! Glyph-grid and glyph-detail API types.

use fc_doc::DocId;
use fc_model::{Glyph, GlyphId, LayerId, MasterId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// All glyph-grid filter conditions.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphFilter {
    /// Optional Unihan and CJK repertoire filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub cjk: Option<super::CjkFilter>,
    /// Name or Unicode query.
    #[serde(default)]
    pub query: Option<String>,
    /// Searchable metadata to inspect; omitted searches all supported fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub query_scope: Option<super::GlyphQueryScope>,
    /// Whether textual search distinguishes uppercase and lowercase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub case_sensitive: Option<bool>,
    /// Whether the search query is a bounded regular expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub regex: Option<bool>,
    /// Boolean smart-filter DSL intersected with the other filter conditions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub smart_expression: Option<String>,
    /// Exact case-sensitive glyph-name set; an empty set matches no glyphs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub glyph_names: Option<Vec<String>>,
    /// Allowed Unicode categories.
    #[serde(default)]
    pub categories: Vec<String>,
    /// Allowed ISO 15924 scripts.
    #[serde(default)]
    pub scripts: Vec<String>,
    /// Allowed Unicode blocks.
    #[serde(default)]
    pub blocks: Vec<String>,
    /// Whether only unencoded glyphs are included.
    #[serde(default)]
    pub unencoded_only: bool,
    /// Allowed color-label indexes.
    #[serde(default)]
    pub color_labels: Vec<u8>,
    /// Allowed editor tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Inclusive allowed Unicode ranges.
    #[serde(default)]
    pub unicode_ranges: Vec<[u32; 2]>,
    /// Optional component-presence condition.
    #[serde(default)]
    pub has_components: Option<bool>,
    /// Whether only glyphs changed in this session are included.
    #[serde(default)]
    pub changed_only: bool,
}

/// Glyph-grid ordering mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum GlyphSort {
    /// Persistent font glyph order.
    #[default]
    Order,
    /// First Unicode scalar, with unencoded glyphs last.
    Unicode,
    /// Case-sensitive glyph name.
    Name,
    /// Horizontal advance width.
    Width,
    /// First Unihan radical and residual stroke count; unknown values last.
    Radical,
    /// First recorded total stroke count; unknown values last.
    Strokes,
    /// Mandarin reading; unknown values last.
    Reading,
    /// Numeric CID order; unmapped glyphs last.
    Cid,
    /// Effective broad category, then subcategory, then glyph order.
    Category,
    /// Effective ISO 15924 script, then glyph order.
    Script,
    /// Glyphs needing attention first: warning badges, then the other badges.
    Status,
}

/// Parameters for a filtered, sorted page of glyph cells.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphListParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Master to render, or the document default master.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Filter conditions.
    #[serde(default)]
    pub filter: GlyphFilter,
    /// Sort order.
    #[serde(default)]
    pub sort: GlyphSort,
    /// Number of matching cells to skip.
    pub offset: u32,
    /// Maximum number of cells to return, up to 2,000.
    pub limit: u32,
    /// Whether each returned cell includes its SVG path.
    pub include_paths: bool,
}

/// One glyph-grid cell.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct GlyphCell {
    /// Stable session glyph handle.
    pub gid: GlyphId,
    /// Persistent glyph name.
    pub name: String,
    /// Assigned Unicode scalar values.
    pub unicodes: Vec<u32>,
    /// Explicit or Unicode-derived broad glyph category.
    pub category: String,
    /// Explicit or Unicode-derived glyph subcategory.
    pub sub_category: String,
    /// Explicit or Unicode-derived ISO 15924 script code.
    pub script: String,
    /// Horizontal advance width in the selected master.
    pub width: f64,
    /// Effective vertical advance, including master defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub vertical_advance: Option<f64>,
    /// Effective vertical origin, including master defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub vertical_origin: Option<f64>,
    /// Exact flattened outline bounds as `[xmin, ymin, xmax, ymax]`.
    pub bbox: Option<[f64; 4]>,
    /// Rounded component-flattened SVG path data when requested.
    pub path: Option<String>,
    /// Optional application color-label index.
    pub color_label: Option<u8>,
    /// Independent color label of the selected master layer, when it has one.
    ///
    /// Stored at `Layer.user_data["com.fontcarver.colorLabel"]` and separate
    /// from `color_label`, so the grid can show both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub layer_color_label: Option<u8>,
    /// Whether the glyph participates in export.
    pub export: bool,
    /// Whether the selected layer contains components.
    pub has_components: bool,
    /// Whether the selected layer has no editable content.
    pub is_empty: bool,
    /// Whether selected-master horizontal formulas differ from stored metrics or fail to resolve.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub metrics_out_of_sync: bool,
    /// Whether a glyph note or a per-layer editor annotation is present.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub has_annotations: bool,
    /// Whether explicit glyph-info overrides or format-specific glyph metadata exist.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub has_custom_glyph_info: bool,
    /// Whether a required master layer is missing or cannot interpolate.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub incompatible_masters: bool,
    /// Whether persistent glyph editing is locked.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub locked: bool,
    /// Whether the glyph stands in for a component base another document had.
    ///
    /// Set by a cross-document transfer, which tags the stand-in
    /// `missing-component-base` and marks it non-exporting.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub placeholder: bool,
    /// Glyph-specific invalidation generation.
    #[ts(type = "number")]
    pub generation: u64,
}

/// A page of glyph-grid cells and its unpaged match count.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct GlyphListResult {
    /// Total number of matching glyphs.
    pub total: u32,
    /// Requested page of cells.
    pub items: Vec<GlyphCell>,
}

/// Parameters for locating a glyph in a filtered, sorted grid listing.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphIndexParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Stable session glyph handle to locate.
    pub gid: GlyphId,
    /// Master used for width sorting and component filtering.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Filter conditions.
    #[serde(default)]
    pub filter: GlyphFilter,
    /// Sort order.
    #[serde(default)]
    pub sort: GlyphSort,
}

/// Parameters for fetching one glyph-grid cell.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphCellParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Stable session glyph handle.
    pub gid: GlyphId,
    /// Master to render.
    pub master: MasterId,
}

/// One flattened layer path in a glyph detail response.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LayerPath {
    /// Persistent layer identifier.
    pub layer: LayerId,
    /// Rounded, component-flattened SVG path data.
    pub flattened: String,
}

/// Complete persistent glyph data plus flattened layer paths.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct GlyphDetail {
    /// Persistent glyph data.
    pub glyph: Glyph,
    /// Flattened paths in layer order.
    pub paths: Vec<LayerPath>,
}

/// Parameters for fetching complete glyph detail.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphDetailParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Stable session glyph handle.
    pub gid: GlyphId,
}

/// Parameters for a compact glyph search.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphSearchParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Name or Unicode query.
    pub query: String,
    /// Maximum number of matches.
    pub limit: u32,
}
