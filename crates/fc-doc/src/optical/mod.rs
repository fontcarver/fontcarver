//! Bounded optical metrics from resolved outline silhouettes.
mod prepare;
mod profile;

use fc_model::{GlyphName, MasterId};
pub use prepare::prepare;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The kind of optical edit to prepare.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum OpticalOperation {
    /// Adjust left and right sidebearings, preserving outline shape.
    Spacing,
    /// Set concrete pair exceptions for every ordered pair in the selection.
    Kerning,
}

/// Reasons why an optical proposal preserved a source or pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum OpticalSkipReason {
    /// Locked glyphs cannot be respaced or used for automatic pairs.
    Locked,
    /// Empty outlines and zero advances keep their spacing.
    EmptyOrMark,
    /// Linked metrics continue to be controlled by their formulas.
    MetricsKey,
    /// An existing effective rule is protected by the request.
    ExistingKerning,
    /// The two outlines have no sampled vertical overlap.
    NoOverlap,
}

/// A preserved layer or pair, identified without session handles.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct OpticalSkip {
    /// First glyph, or the glyph whose sidebearings were preserved.
    pub glyph: String,
    /// Second glyph for a pair, null for a layer.
    pub right: Option<String>,
    /// Display name of the source master.
    pub master: String,
    /// Stable translatable reason code.
    pub reason: OpticalSkipReason,
}

/// A reviewed layer or concrete pair update.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct OpticalExample {
    /// First glyph, or the glyph being respaced.
    pub glyph: String,
    /// Second glyph for kerning, null for sidebearings.
    pub right: Option<String>,
    /// Display name of the source master.
    pub master: String,
    /// LSB, RSB and width before spacing, or [effective kern, 0, 0].
    pub before: [f64; 3],
    /// LSB, RSB and width after spacing, or [proposed kern, 0, 0].
    pub after: [f64; 3],
    /// Resolved original outline, for the first glyph only, as SVG path data.
    pub outline: String,
    /// Resolved spacing outline after dependency-aware changes; null for kerning.
    pub after_outline: Option<String>,
    /// Bounds of the resolved spacing result; null for kerning.
    pub after_bounds: Option<[f64; 4]>,
    /// Bounds of that outline in Y-up font coordinates.
    pub bounds: [f64; 4],
}

/// One layer or pair a review can accept or reject on its own.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct OpticalProposalEntry {
    /// First glyph, or the glyph being respaced.
    pub glyph: String,
    /// Second glyph for a pair, null for sidebearings.
    pub right: Option<String>,
    /// Persistent master identifier, which is what an acceptance names.
    #[ts(type = "string")]
    pub master: MasterId,
    /// Display name of that master.
    pub master_name: String,
    /// LSB, RSB and width before spacing, or [effective kern, 0, 0].
    pub before: [f64; 3],
    /// The same values after the proposed change.
    pub after: [f64; 3],
}

/// One accepted proposal, named exactly as the proposal reported it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, TS)]
pub struct OpticalSelection {
    /// First glyph, or the glyph being respaced.
    pub glyph: String,
    /// Second glyph for a pair, null for sidebearings.
    pub right: Option<String>,
    /// Persistent master identifier.
    #[ts(type = "string")]
    pub master: MasterId,
}

impl OpticalSelection {
    fn of(entry: &OpticalProposalEntry) -> Self {
        Self {
            glyph: entry.glyph.clone(),
            right: entry.right.clone(),
            master: entry.master.clone(),
        }
    }
}

/// Options shared by pure preparation and the API.
#[derive(Clone, Debug)]
pub struct OpticalOptions {
    /// Persistent selected glyph names, at most 128 distinct names.
    pub glyphs: Vec<GlyphName>,
    /// One master, or all masters when absent.
    pub master: Option<MasterId>,
    /// Sidebearings or kerning.
    pub operation: OpticalOperation,
    /// Global density between 0.25 and 4 inclusive.
    pub density: f64,
    /// Replace effective existing rules when true.
    pub replace_existing: bool,
    /// Apply only these reviewed proposals; absent applies every proposal.
    ///
    /// A selection that the recomputed proposal no longer contains is an error, so
    /// a review of a stale document can never apply something it did not show.
    pub accepted: Option<Vec<OpticalSelection>>,
}

/// Prepared atomic commands and their review data.
pub struct OpticalProposal {
    /// Commands to apply at the captured source revision.
    pub commands: Vec<crate::Command>,
    /// Distinct selected glyph count.
    pub glyph_count: u32,
    /// Changed source layers or pairs, after any acceptance filter.
    pub changed_count: u32,
    /// Every skipped layer or pair.
    pub skipped: Vec<OpticalSkip>,
    /// First sixteen changed sources, with outlines for a visual review.
    pub examples: Vec<OpticalExample>,
    /// Every proposed change, for a review table that accepts or rejects rows.
    pub proposals: Vec<OpticalProposalEntry>,
}
