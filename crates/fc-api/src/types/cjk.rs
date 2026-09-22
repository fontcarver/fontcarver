//! CJK filtering and Unicode-property inspection.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Additional CJK filters; all configured conditions must match one encoding.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CjkFilter {
    /// Kangxi radical number, from 1 through 214.
    pub radical: Option<u16>,
    /// Inclusive total stroke range; every regional alternative is considered.
    pub strokes: Option<[u16; 2]>,
    /// Reading prefix, accepting tone-marked or unmarked Mandarin.
    pub reading: Option<String>,
    /// Reading language: mandarin, cantonese, japanese, korean or any.
    pub language: Option<String>,
    /// Any of gb2312, gb18030level1, big5, jisx0208, ksx1001 or iicore.
    #[serde(default)]
    pub charsets: Vec<String>,
}

/// A Unicode scalar whose Unihan properties should be displayed.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct UnihanParams {
    /// Unicode scalar value.
    pub unicode: u32,
}

/// Original Unicode properties; absent fields remain empty, never inferred.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct UnihanInfo {
    /// Pinned database version.
    pub version: String,
    /// Unicode scalar value.
    pub unicode: u32,
    /// kRSUnicode, including multiple values and simplified-radical indicators.
    pub radical_strokes: String,
    /// All kTotalStrokes alternatives.
    pub total_strokes: String,
    /// kMandarin with original tone marks.
    pub mandarin: String,
    /// kCantonese.
    pub cantonese: String,
    /// kJapaneseOn.
    pub japanese_on: String,
    /// kJapaneseKun.
    pub japanese_kun: String,
    /// kKorean.
    pub korean: String,
    /// kIICore priority and region indicators.
    pub iicore: String,
}

/// CID authoring state, with stable glyph names for mapping and ordering.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct CidState {
    /// Revision used by the editor's optimistic transaction.
    #[ts(type = "number")]
    pub generation: u64,
    /// Current CID identity and mapping, absent for name-keyed fonts.
    pub cid: Option<fc_model::CidInfo>,
    /// Complete persistent glyph order.
    pub glyphs: Vec<String>,
}

/// Atomic CID mapping and optional numeric glyph-order update.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct CidApplyParams {
    /// Open document identifier.
    pub doc: fc_doc::DocId,
    /// Revision from the authoring state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Candidate metadata, or null to switch back to name-keyed output.
    pub cid: Option<fc_model::CidInfo>,
    /// Move mapped glyphs into numeric CID order; retain unmapped glyph order.
    pub reorder: bool,
    /// User-visible undo label.
    pub label: String,
}
