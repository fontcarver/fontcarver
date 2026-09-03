//! Document-query API types.

use fw_doc::DocId;
use fw_model::{FontInfo, Master};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Parameters identifying one open document.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DocParams {
    /// Open document identifier.
    pub doc: DocId,
}

/// Editable font metadata and complete master records.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FontInfoView {
    /// Font-wide metadata.
    pub info: FontInfo,
    /// Design-space masters.
    pub masters: Vec<Master>,
}

/// Counts used to populate the font-view filter sidebar.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SidebarCounts {
    /// Unicode category names and glyph counts.
    pub categories: Vec<(String, u32)>,
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
}
