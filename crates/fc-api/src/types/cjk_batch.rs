//! Bounded selection-wide CJK spacing operations.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Preview or apply spacing to an explicit, captured glyph selection.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct CjkBatchParams {
    /// Open document identifier.
    pub doc: fc_doc::DocId,
    /// Captured session glyph handles, up to 65,535 distinct glyphs.
    pub glyphs: Vec<fc_model::GlyphId>,
    /// One master, or null for all masters.
    pub master: Option<fc_model::MasterId>,
    /// New horizontal advance; absent preserves the current advance.
    pub width: Option<f64>,
    /// Center the flattened outline within the resulting advance.
    pub center: bool,
    /// Replace vertical metrics; false leaves them untouched.
    pub vertical: bool,
    /// Vertical advance override; null restores the master default.
    pub advance: Option<f64>,
    /// Vertical origin override; null restores the master default.
    pub origin: Option<f64>,
    /// Validate and report without modifying the document.
    pub dry_run: bool,
    /// Required on apply, obtained from the preview.
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
    /// User-visible undo label.
    pub label: String,
}

/// A small example of the proposed metrics update.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct CjkBatchExample {
    /// Persistent glyph name.
    pub glyph: String,
    /// Source master name.
    pub master: String,
    /// Existing horizontal advance.
    pub before: f64,
    /// Proposed horizontal advance.
    pub after: f64,
}

/// Preview summary or successful atomic transaction result.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct CjkBatchResult {
    /// Revision used to prepare this operation, before any mutation.
    #[ts(type = "number")]
    pub generation: u64,
    /// Distinct selected glyph count.
    pub glyph_count: u32,
    /// Number of master layers included.
    pub layer_count: u32,
    /// First twelve affected layers for review.
    pub examples: Vec<CjkBatchExample>,
    /// Invalidation produced by applying, absent for a preview.
    pub change: Option<fc_doc::Change>,
}
