//! Bounded read-only inspection of preserved imported layout tables.

use fc_doc::DocId;
use fc_model::{FeaturesSource, GlyphId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Parameters for a generation-checked imported lookup page.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutInspectParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Required for continuation pages; null starts a fresh inspection.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub generation: Option<u64>,
    /// Number of lookup records to skip.
    pub offset: u32,
    /// Maximum returned records, between one and 200.
    pub limit: u32,
}

/// Parameters for a bounded page of rendered lookup rules.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutRulesParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Table tag, normally GSUB or GPOS.
    pub table: String,
    /// Lookup index within the selected table.
    pub lookup: u16,
    /// Required for continuation pages; null starts a fresh page.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub generation: Option<u64>,
    /// Number of rule records to skip.
    pub offset: u32,
    /// Maximum returned records, between one and 200.
    pub limit: u32,
}

/// Preserved table and its parsed lookup count.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutTableInfo {
    /// Four-byte table tag.
    pub tag: String,
    /// Original table length in bytes.
    pub byte_length: u32,
    /// Number of lookups in the table.
    pub lookup_count: u32,
}

/// Explicit coverage reason preventing conversion of the complete layout.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutConversionBlocker {
    /// Table owning the unsupported or invalid content.
    pub table: String,
    /// Lookup index when applicable.
    pub lookup: Option<u16>,
    /// Located explanation; inspection never silently drops this content.
    pub reason: String,
}

/// One lookup in original execution order.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutLookupInfo {
    /// Owning GSUB or GPOS table.
    pub table: String,
    /// Original table-local lookup index.
    pub index: u16,
    /// OpenType lookup kind.
    pub kind: u16,
    /// Original lookup flags.
    pub flags: u16,
    /// Bounded indices of referenced lookups, preserving order.
    pub referenced_lookups: Vec<u16>,
    /// Whether additional lookup references were omitted.
    pub references_truncated: bool,
    /// Total rendered rule records, without glyph-pair Cartesian expansion.
    pub rule_count: u32,
}

/// Imported layout inventory and one bounded lookup page.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutInspection {
    /// Open document identifier.
    pub doc: DocId,
    /// Document generation inspected.
    #[ts(type = "number")]
    pub generation: u64,
    /// Effective compile-input revision inspected.
    #[ts(type = "number")]
    pub compile_revision: u64,
    /// Feature draft/source revision inspected.
    #[ts(type = "number")]
    pub draft_revision: u64,
    /// Current authoritative representation, independent of coverage.
    pub authority: FeaturesSource,
    /// Native conversion backup exists for a checked original-layout revert.
    pub original_layout_available: bool,
    /// Hash of preserved layout and glyph provenance.
    pub fingerprint: String,
    /// Bounded set of relevant OpenType and competing tables.
    pub tables: Vec<LayoutTableInfo>,
    /// First 200 explicit conversion blockers.
    pub blockers: Vec<LayoutConversionBlocker>,
    /// Whether additional blockers were omitted.
    pub blockers_truncated: bool,
    /// Coverage permits preparing a conversion; application still requires proof.
    pub can_convert: bool,
    /// Total lookup records in original table order.
    pub total: u32,
    /// Requested lookup page.
    pub items: Vec<LayoutLookupInfo>,
    /// Offset for the next page, if more records exist.
    pub next_offset: Option<u32>,
}

/// Stable glyph identity for rendering a bounded rule preview.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutRuleGlyph {
    /// Current glyph name from the original layout mapping.
    pub name: String,
    /// Session handle, absent when the referenced glyph is missing.
    pub gid: Option<GlyphId>,
}

/// One bounded read-only rendering of an imported rule.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutRuleInfo {
    /// At most 4096 characters of source-like rule text.
    pub text: String,
    /// At most 64 referenced glyph previews.
    pub glyphs: Vec<LayoutRuleGlyph>,
    /// Whether source text or glyph previews were truncated.
    pub truncated: bool,
}

/// Bounded rules from one original lookup.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutRules {
    /// Open document identifier.
    pub doc: DocId,
    /// Document generation inspected.
    #[ts(type = "number")]
    pub generation: u64,
    /// Effective compile-input revision inspected.
    #[ts(type = "number")]
    pub compile_revision: u64,
    /// Feature draft/source revision inspected.
    #[ts(type = "number")]
    pub draft_revision: u64,
    /// Hash of original layout and glyph provenance.
    pub fingerprint: String,
    /// Requested table tag.
    pub table: String,
    /// Requested original lookup index.
    pub lookup: u16,
    /// Total rendered rules in the selected lookup.
    pub total: u32,
    /// Requested rule page.
    pub items: Vec<LayoutRuleInfo>,
    /// Offset for the next page, if more records exist.
    pub next_offset: Option<u32>,
}

/// A bounded glyph-reference page within one large imported rule or class.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutRuleGlyphsParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Original layout table.
    pub table: String,
    /// Original lookup index.
    pub lookup: u16,
    /// Original rule index, independent of rule page boundaries.
    pub rule: u32,
    /// Required for continuation pages.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub generation: Option<u64>,
    /// Number of referenced glyphs to skip.
    pub offset: u32,
    /// Number of references to return, between one and 200.
    pub limit: u32,
}

/// A bounded page of glyph references, without expanding class pairs.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutRuleGlyphs {
    /// Open document identifier.
    pub doc: DocId,
    /// Document generation inspected.
    #[ts(type = "number")]
    pub generation: u64,
    /// Effective compile-input revision inspected.
    #[ts(type = "number")]
    pub compile_revision: u64,
    /// Feature draft/source revision inspected.
    #[ts(type = "number")]
    pub draft_revision: u64,
    /// Hash of original layout and glyph provenance.
    pub fingerprint: String,
    /// Requested table.
    pub table: String,
    /// Requested original lookup.
    pub lookup: u16,
    /// Requested original rule.
    pub rule: u32,
    /// Total glyph references in this bounded rule.
    pub total: u32,
    /// Requested glyph reference page.
    pub items: Vec<LayoutRuleGlyph>,
    /// Offset for the next page, if any.
    pub next_offset: Option<u32>,
}
