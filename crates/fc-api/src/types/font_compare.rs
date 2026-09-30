//! Read-only font/family comparisons and reference-outline overlays.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Compare selected masters of two open documents by persistent glyph name.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct FontCompareParams {
    /// Destination/current document.
    pub doc: fc_doc::DocId,
    /// Reference document, which may have been opened from a binary font.
    pub source_doc: fc_doc::DocId,
    /// Target master; omission uses its default master.
    #[serde(default)]
    pub master: Option<fc_model::MasterId>,
    /// Reference master; omission uses its default master.
    #[serde(default)]
    pub source_master: Option<fc_model::MasterId>,
    /// First row in the sorted, optionally filtered comparison.
    #[serde(default)]
    pub offset: u32,
    /// Maximum rows to return, within 1–512; defaults to 128.
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Omit equal rows from the page while retaining complete counts.
    #[serde(default)]
    pub differing_only: bool,
}

fn default_limit() -> u32 {
    128
}

/// Font-level compatibility, independent of internal document/master IDs.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct FontCompareCompatibility {
    /// Family names agree exactly.
    pub family_name: bool,
    /// Em square sizes agree.
    pub units_per_em: bool,
    /// Axis tags, bounds, defaults and coordinate mappings agree.
    pub axes: bool,
    /// Master names and design locations agree, independent of order.
    pub masters: bool,
    /// Persistent glyph-name inventories agree.
    pub glyph_set: bool,
    /// Selected-master global metrics agree.
    pub master_metrics: bool,
}

/// Counts over the complete compared glyph inventory, before filtering/paging.
#[derive(Clone, Debug, Default, Deserialize, Serialize, TS)]
pub struct FontCompareCounts {
    /// Glyphs with matching geometry, metrics, Unicode and topology.
    pub equal: u32,
    /// Glyphs present only in the reference.
    pub missing_target: u32,
    /// Glyphs present only in the target.
    pub missing_source: u32,
    /// Shared glyphs with different evaluated path commands or missing outlines.
    pub outlines: u32,
    /// Shared glyphs with different horizontal or vertical advances/origins.
    pub metrics: u32,
    /// Shared glyphs with different Unicode or variation-sequence sets.
    pub unicodes: u32,
    /// Shared glyphs whose selected layers have different interpolation topology.
    pub incompatible: u32,
}

/// High-level classification of a glyph comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FontCompareStatus {
    /// Every compared property matches.
    Equal,
    /// At least one compared property differs or cannot be evaluated.
    Different,
    /// The reference glyph has no target counterpart.
    MissingTarget,
    /// The target glyph has no reference counterpart.
    MissingSource,
}

/// One glyph comparison; session handles can be used for reviewed paste/merge.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct FontCompareGlyph {
    /// Persistent name shared by the two sides.
    pub name: String,
    /// Target session glyph handle, if present.
    pub target: Option<fc_model::GlyphId>,
    /// Reference session glyph handle, if present.
    pub source: Option<fc_model::GlyphId>,
    /// Summary status for filtering.
    pub status: FontCompareStatus,
    /// Evaluated drawing commands differ; IDs and point metadata are ignored.
    pub outline_changed: bool,
    /// Horizontal/vertical advances or vertical origins differ.
    pub metrics_changed: bool,
    /// Unicode or variation-selector assignments differ, ignoring order.
    pub unicode_changed: bool,
    /// Both selected layers have matching contour/component topology.
    pub compatible: bool,
    /// Target advance in its native font units.
    pub target_width: Option<f64>,
    /// Reference advance in its native font units.
    pub source_width: Option<f64>,
    /// Sorted target Unicode assignments.
    pub target_unicodes: Vec<u32>,
    /// Sorted reference Unicode assignments.
    pub source_unicodes: Vec<u32>,
    /// Missing-layer or geometry evaluation failures, identified by side.
    pub issues: Vec<String>,
}

/// A bounded comparison page and complete glyph counts.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct FontCompareResult {
    /// Captured target document generation.
    #[ts(type = "number")]
    pub generation: u64,
    /// Captured reference document generation.
    #[ts(type = "number")]
    pub source_generation: u64,
    /// Total rows after applying the differences-only filter.
    pub total: u32,
    /// Requested page offset.
    pub offset: u32,
    /// Requested page limit.
    pub limit: u32,
    /// Target family display name.
    pub doc_family: String,
    /// Reference family display name.
    pub source_family: String,
    /// Target em square.
    pub doc_upem: u16,
    /// Reference em square.
    pub source_upem: u16,
    /// Resolved target master ID.
    pub master: fc_model::MasterId,
    /// Resolved reference master ID.
    pub source_master: fc_model::MasterId,
    /// Font-level compatibility flags.
    pub compatibility: FontCompareCompatibility,
    /// Counts over every compared glyph, before filtering.
    pub counts: FontCompareCounts,
    /// Sorted glyph-name page.
    pub glyphs: Vec<FontCompareGlyph>,
}

/// Request a selected glyph's exact evaluated drawing from both documents.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct FontCompareGlyphParams {
    /// Current document.
    pub doc: fc_doc::DocId,
    /// Open reference document.
    pub source_doc: fc_doc::DocId,
    /// Target master, or default when omitted.
    #[serde(default)]
    pub master: Option<fc_model::MasterId>,
    /// Reference master, or default when omitted.
    #[serde(default)]
    pub source_master: Option<fc_model::MasterId>,
    /// Persistent glyph name.
    pub name: String,
    /// Optional reviewed target generation; reject stale comparison data.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
    /// Optional reviewed reference generation; reject stale comparison data.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_source_generation: Option<u64>,
}

/// One evaluated outline in its original Y-up font coordinate system.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct FontCompareOutline {
    /// Complete SVG path with component transforms and live effects applied.
    pub path: String,
    /// Native outline bounds [x0, y0, x1, y1], zero for an empty outline.
    pub bounds: [f64; 4],
    /// Horizontal advance.
    pub width: f64,
    /// Optional vertical advance.
    pub vertical_width: Option<f64>,
    /// Optional vertical origin.
    pub vertical_origin: Option<f64>,
    /// Native font units per em, for optional em-normalized overlays.
    pub upem: u16,
    /// Whether live-effect evaluation required curve refitting.
    pub approximated: bool,
}

/// Paired overlay drawings and their unscaled union bounds.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct FontCompareGlyphResult {
    /// Persistent glyph name.
    pub name: String,
    /// Current outline, absent when the glyph or selected master layer is absent.
    pub target: Option<FontCompareOutline>,
    /// Reference outline, absent when the glyph or selected master layer is absent.
    pub source: Option<FontCompareOutline>,
    /// Union of nonempty outlines in native coordinates, [x0, y0, x1, y1].
    pub bounds: [f64; 4],
}
