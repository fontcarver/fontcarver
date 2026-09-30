//! Spacing-matrix and beam-measurement types for the Text tab.

use fc_doc::{DocId, ResolvedMetrics};
use fc_model::{LayerId, MasterId, MetricsKeys};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Largest number of glyph names one spacing request may carry.
pub const SPACING_MATRIX_LIMIT: usize = 256;

/// Select the glyphs of one shaped run in one master.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SpacingMatrixParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Master to measure, or the document's selected master when absent.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Glyph names in run order; at most [`SPACING_MATRIX_LIMIT`] of them.
    ///
    /// Repeats are answered once each, in the order given, so a caller can
    /// pass a de-duplicated run without tracking indices itself.
    pub glyphs: Vec<String>,
    /// Height in font units at which to measure each glyph's margins.
    ///
    /// Absent means no beam. A height that misses a glyph's outline leaves
    /// that row's beam fields null instead of failing the request.
    #[serde(default)]
    pub beam: Option<f64>,
}

/// One glyph's horizontal spacing, its formulas and its beam measurement.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SpacingRow {
    /// Glyph name exactly as requested.
    pub glyph: String,
    /// Master layer measured, absent when the glyph has no layer for the master.
    pub layer: Option<LayerId>,
    /// Stored horizontal advance.
    pub width: f64,
    /// Stored left sidebearing, null when the layer has no ink.
    pub lsb: Option<f64>,
    /// Stored right sidebearing, null when the layer has no ink.
    pub rsb: Option<f64>,
    /// Glyph-wide formulas, exactly as stored.
    pub glyph_keys: MetricsKeys,
    /// The measured layer's own formulas, when it overrides any field.
    pub layer_keys: Option<MetricsKeys>,
    /// Metrics after resolving the formulas, absent when resolution failed.
    pub resolved: Option<ResolvedMetrics>,
    /// Why resolution failed, for a cyclic or malformed formula.
    pub key_error: Option<String>,
    /// Whether resolved metrics differ from what the layer stores.
    pub out_of_sync: bool,
    /// Distance from the origin to the first ink crossing at the beam height.
    pub beam_left: Option<f64>,
    /// Distance from the last ink crossing to the advance at the beam height.
    pub beam_right: Option<f64>,
    /// Whether this glyph is missing from the document.
    pub missing: bool,
    /// Whether glyph editing is locked, so the row is read-only.
    pub locked: bool,
}

/// Spacing rows for one request, at one document generation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SpacingMatrix {
    /// Generation every row refers to.
    #[ts(type = "number")]
    pub generation: u64,
    /// Master the rows were measured in.
    pub master: MasterId,
    /// One row per requested glyph name, in request order.
    pub rows: Vec<SpacingRow>,
}
