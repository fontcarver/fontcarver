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
    /// Names of `glyphs` that participate in export, in the same order.
    ///
    /// CID-keyed CFF needs a CID for exactly these, so the editor's
    /// incomplete-mapping notice counts them rather than the whole font: a
    /// non-exporting helper glyph never needs a CID.
    #[serde(default)]
    pub exported: Vec<String>,
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

/// A document whose radical library should be built.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct CjkRadicalsParams {
    /// Open document identifier.
    pub doc: fc_doc::DocId,
}

/// One ideograph that writes a radical, with the document glyph encoding it.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct CjkRadicalForm {
    /// Unicode scalar value of the ideograph.
    pub unicode: u32,
    /// Smallest recorded kTotalStrokes value, absent when the source has none.
    pub strokes: Option<u16>,
    /// Document glyph encoding this scalar, absent when the font lacks it.
    pub glyph: Option<String>,
}

/// One Kangxi radical as the radical library shows it for one document.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct CjkRadicalEntry {
    /// Kangxi radical number, from 1 through 214.
    pub number: u16,
    /// Character of the Kangxi Radicals block for this radical number.
    pub kangxi: u32,
    /// Document glyph encoding the block character, when the font has one.
    pub kangxi_glyph: Option<String>,
    /// Ideographs that write the radical, in ascending scalar order.
    pub forms: Vec<CjkRadicalForm>,
    /// Glyphs of this document whose encoding uses this radical.
    pub members: u32,
}

/// One glyph the document already uses as a component, with its use count.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct CjkComponentUse {
    /// Name of the base glyph.
    pub name: String,
    /// How many component references name it, across every layer.
    pub uses: u32,
    /// First Unicode scalar of the base glyph, when it is encoded.
    pub unicode: Option<u32>,
}

/// The radical library: pinned Unihan radicals plus this font's own components.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct CjkRadicalLibrary {
    /// Pinned Unihan database version the radicals come from.
    pub unihan_version: String,
    /// Every Kangxi radical, in radical order.
    pub radicals: Vec<CjkRadicalEntry>,
    /// Component bases, most used first, then by name; at most 2,000 entries.
    pub components: Vec<CjkComponentUse>,
    /// Whether the component list was truncated at that limit.
    pub components_truncated: bool,
}
