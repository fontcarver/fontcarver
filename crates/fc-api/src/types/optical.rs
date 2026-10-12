//! Captured-selection optical spacing and automatic kerning.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Preview or apply outline-based spacing for a bounded captured selection.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct OpticalBatchParams {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Captured glyph handles; up to 128 distinct glyphs.
    pub glyphs: Vec<fc_model::GlyphId>,
    /// One master, or null for all masters.
    pub master: Option<fc_model::MasterId>,
    /// Sidebearings or pair kerning.
    pub operation: fc_doc::optical::OpticalOperation,
    /// Global density in [0.25, 4]; larger values tighten the spacing.
    pub density: f64,
    /// Permit replacing effective existing kerning with concrete pair exceptions.
    #[serde(default)]
    pub replace_existing: bool,
    /// Apply only these reviewed proposals; absent applies every proposal.
    #[serde(default)]
    pub accepted: Option<Vec<fc_doc::optical::OpticalSelection>>,
    /// Validate and preview without mutation.
    pub dry_run: bool,
    /// Required on apply, from the reviewed preview.
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
    /// User-visible undo label.
    pub label: String,
}

/// Outline metrics before and after the proposed edit.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct OpticalBatchResult {
    /// Captured document revision, before any mutation.
    #[ts(type = "number")]
    pub generation: u64,
    /// Distinct selected glyph count.
    pub glyph_count: u32,
    /// Changed master layers for spacing, or changed pairs for kerning, after the
    /// acceptance filter.
    pub changed_count: u32,
    /// Every proposed change, for a review table that accepts or rejects rows.
    pub proposals: Vec<fc_doc::optical::OpticalProposalEntry>,
    /// Skipped layers or pairs, with stable reason codes.
    pub skipped: Vec<fc_doc::optical::OpticalSkip>,
    /// First sixteen changed layers or pairs for review.
    pub examples: Vec<fc_doc::optical::OpticalExample>,
    /// Invalidation after successful apply; null during preview.
    pub change: Option<fc_doc::Change>,
}
