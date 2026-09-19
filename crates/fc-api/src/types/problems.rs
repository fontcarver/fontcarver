//! Read-only, generation-bound document problem reports.

use fc_doc::DocId;
use fc_model::{GlyphId, GlyphName, LayerId, MasterId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Request a bounded document scan.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FindProblemsParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Maximum retained issues, from 1 to 2000; omitted means 200.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub limit: Option<usize>,
}

/// Diagnostic significance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ProblemSeverity {
    /// Invalid data or a compilation blocker.
    Error,
    /// Editable data that deserves attention.
    Warning,
    /// Informational feedback.
    Info,
}

/// One problem and its most precise available document location.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct ProblemIssue {
    /// Stable machine-readable diagnostic category.
    pub code: String,
    /// Diagnostic significance.
    pub severity: ProblemSeverity,
    /// Human-readable detail.
    pub message: String,
    /// Stable session glyph handle for navigation.
    pub glyph: Option<GlyphId>,
    /// Persistent glyph name for display and command-line reports.
    pub glyph_name: Option<GlyphName>,
    /// Associated master.
    pub master: Option<MasterId>,
    /// Persistent layer identifier.
    pub layer: Option<LayerId>,
    /// Color-layer index within the containing layer.
    pub color_layer: Option<usize>,
    /// Contour index within the ordinary or color layer.
    pub contour: Option<usize>,
    /// Point index within the contour.
    pub point: Option<usize>,
}

/// A report tied to the exact document generation inspected.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FindProblemsResult {
    /// Inspected document.
    pub doc: DocId,
    /// Mutation generation inspected; an edit invalidates the report.
    #[ts(type = "number")]
    pub generation: u64,
    /// Bounded deterministic diagnostics.
    pub issues: Vec<ProblemIssue>,
    /// Whether more diagnostics were detected than could be returned.
    pub truncated: bool,
    /// Detected count, including omitted issues; a lower bound if the compiler truncated.
    pub total: usize,
    /// Whether any detected issue is an error, including omitted diagnostics.
    pub has_errors: bool,
}
