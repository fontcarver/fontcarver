//! Document-query API types.

use fc_doc::DocId;
use fc_model::{FontInfo, Instance, Master};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Parameters identifying one open document.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DocParams {
    /// Open document identifier.
    pub doc: DocId,
}

/// One table from the imported font's sfnt directory.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct TableInfo {
    /// Four-character OpenType table tag.
    pub tag: String,
    /// Table length in bytes, or zero when the source bytes are unavailable.
    pub len: u32,
    /// Whether the table is retained verbatim for a preservation merge.
    pub preserved: bool,
}

/// Editable font metadata, complete master records, table inventory and export instances.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FontInfoView {
    /// Font-wide metadata.
    pub info: FontInfo,
    /// Design-space masters.
    pub masters: Vec<Master>,
    /// Imported sfnt table inventory, sorted by tag.
    pub tables: Vec<TableInfo>,
    /// Named export instances of the design space.
    #[serde(default)]
    pub instances: Vec<Instance>,
}

/// Counts used to populate the font-view filter sidebar.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SidebarCounts {
    /// Unicode category names and glyph counts.
    pub categories: Vec<(String, u32)>,
    /// Unicode category, subcategory and glyph counts for encoded glyphs.
    #[serde(default)]
    #[ts(as = "Option<Vec<(String, String, u32)>>", optional)]
    pub sub_categories: Vec<(String, String, u32)>,
    /// ISO 15924 script codes and glyph counts.
    pub scripts: Vec<(String, u32)>,
    /// Unicode block names and glyph counts.
    pub blocks: Vec<(String, u32)>,
    /// Number of glyphs without an assigned Unicode scalar.
    pub unencoded: u32,
    /// Total number of glyphs.
    pub total: u32,
    /// Color-label indexes and glyph counts.
    pub color_labels: Vec<(u8, u32)>,
    /// Sorted unique valid Unicode scalars, including secondary assignments.
    #[serde(default)]
    #[ts(as = "Option<Vec<u32>>", optional)]
    pub unicodes: Vec<u32>,
    /// Sorted unique editor tags used by glyphs in this document.
    #[serde(default)]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub tags: Vec<String>,
}
