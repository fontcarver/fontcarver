//! Repertoire coverage for the glyph sidebar's present/total badges.

use fc_doc::DocId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Which Unicode grouping a coverage row or missing-member request refers to.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[ts(export)]
pub enum RepertoireKind {
    /// A broad category name such as `Letter`.
    Category,
    /// A `Category/Subcategory` pair such as `Letter/Uppercase`.
    SubCategory,
    /// An ISO 15924 script code such as `Latn`.
    Script,
    /// A Unicode block name such as `Cyrillic`.
    Block,
}

impl From<RepertoireKind> for fc_glyphdata::RepertoireKind {
    fn from(kind: RepertoireKind) -> Self {
        match kind {
            RepertoireKind::Category => Self::Category,
            RepertoireKind::SubCategory => Self::SubCategory,
            RepertoireKind::Script => Self::Script,
            RepertoireKind::Block => Self::Block,
        }
    }
}

/// One sidebar row's coverage of its Unicode repertoire.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CoverageRow {
    /// Repertoire name, using `Category/Subcategory` for a subcategory row.
    pub value: String,
    /// Assigned scalars of the repertoire that some glyph of the font encodes.
    pub present: u32,
    /// Assigned Unicode scalars the repertoire contains.
    pub total: u32,
    /// Glyphs classified into this repertoire by their first Unicode scalar.
    ///
    /// This is the number the matching sidebar filter selects; it differs from
    /// `present` when glyphs share an encoding or carry none.
    pub glyphs: u32,
}

/// Parameters for reading a document's repertoire coverage.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct InventoryCoverageParams {
    /// Open document identifier.
    pub doc: DocId,
}

/// Coverage of every repertoire the document's glyphs touch.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct InventoryCoverage {
    /// Broad categories in sidebar order.
    pub categories: Vec<CoverageRow>,
    /// Subcategories, keyed `Category/Subcategory`.
    pub sub_categories: Vec<CoverageRow>,
    /// ISO 15924 scripts.
    pub scripts: Vec<CoverageRow>,
    /// Unicode blocks.
    pub blocks: Vec<CoverageRow>,
    /// Total number of glyphs in the document.
    pub total_glyphs: u32,
}

/// Parameters for listing the members of one repertoire a document lacks.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct MissingMembersParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Repertoire grouping.
    pub kind: RepertoireKind,
    /// Repertoire name.
    pub value: String,
    /// Maximum number of scalars to return, up to 8,192.
    pub limit: u32,
}

/// The characters of a repertoire that the document does not encode.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct MissingMembers {
    /// Missing scalars in code-point order, truncated to the requested limit.
    pub scalars: Vec<u32>,
    /// Total number of missing scalars, which may exceed `scalars.len()`.
    pub total: u32,
    /// An "Add Glyphs" recipe for the returned scalars, using inclusive ranges.
    pub recipe: String,
}
