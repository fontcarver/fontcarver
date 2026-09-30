//! Persistent, named lists of exact glyph names for the sidebar.

use fc_doc::DocId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A user-maintained glyph list stored with its font document.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphList {
    /// Stable identifier using ASCII letters, digits, underscores, or hyphens.
    pub id: String,
    /// Human-readable list name, normalized by trimming surrounding whitespace.
    pub name: String,
    /// Ordered exact glyph names, deduplicated while preserving first occurrence.
    pub glyphs: Vec<String>,
}

/// A stored list together with its current document coverage.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphListView {
    /// Stable list identifier.
    pub id: String,
    /// Human-readable list name.
    pub name: String,
    /// All requested exact glyph names in list order.
    pub glyphs: Vec<String>,
    /// Names currently present in the document, in list order.
    pub present: Vec<String>,
    /// Names absent from the document, in list order.
    pub missing: Vec<String>,
}

/// The document's named glyph lists and their live coverage.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphListsResult {
    /// Ordered named lists.
    pub lists: Vec<GlyphListView>,
}

/// Replace a document's complete collection of named glyph lists atomically.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SetGlyphListsParams {
    /// Open document identifier.
    pub doc: DocId,
    /// New complete ordered collection, with unique identifiers.
    pub lists: Vec<GlyphList>,
}
