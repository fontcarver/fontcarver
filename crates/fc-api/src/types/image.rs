//! Source PNG images and revision-checked raster tracing.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A persistent glyph layer in an open document.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ImageTarget {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Persistent glyph name.
    pub glyph: String,
    /// Persistent layer identifier.
    pub layer: fc_model::LayerId,
}

/// Saved placement plus the resolved display source, or a recoverable link error.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ImageState {
    /// Revision for the next edit.
    #[ts(type = "number")]
    pub generation: u64,
    /// Persistent source placement.
    pub image: Option<fc_model::Image>,
    /// Validated PNG bytes for rendering, omitted when the source cannot resolve.
    pub png: Option<Vec<u8>>,
    /// Decoded pixel width.
    pub width: Option<u32>,
    /// Decoded pixel height.
    pub height: Option<u32>,
    /// Missing or invalid source description, without failing the layer query.
    pub error: Option<String>,
    /// Absolute path the external link resolved to, for a relink prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_path: Option<String>,
}

/// Import a PNG from a path or embedded browser-upload bytes.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ImageImportParams {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Persistent glyph name.
    pub glyph: String,
    /// Persistent layer identifier.
    pub layer: fc_model::LayerId,
    /// Required document revision.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Local source path, absolute or relative to the saved document.
    pub path: Option<String>,
    /// PNG upload bytes; mutually exclusive with path.
    pub data: Option<Vec<u8>>,
    /// Logical file name for uploaded data.
    pub file_name: String,
    /// Store the bytes; false stores a relative link and requires a saved document.
    pub embed: bool,
}

/// Set a placement, lock state or remove a source image atomically.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ImageSetParams {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Persistent glyph name.
    pub glyph: String,
    /// Persistent layer identifier.
    pub layer: fc_model::LayerId,
    /// Required document revision.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Replacement placement; null removes it.
    pub image: Option<fc_model::Image>,
}

/// Preview or apply dark-ink polygon tracing while retaining the source image.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ImageTraceParams {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Persistent glyph name.
    pub glyph: String,
    /// Persistent layer identifier.
    pub layer: fc_model::LayerId,
    /// Required on apply, captured by the most recent preview.
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
    /// Required on apply; detects linked-file changes since the preview.
    #[serde(default)]
    pub expected_source_hash: Option<String>,
    /// Minimum alpha-composited dark ink coverage, 1..255.
    pub threshold: u8,
    /// Largest permitted deviation from the pixel edges, in source pixels, 0..16.
    pub tolerance: f64,
    /// Fit cubic curves; false simplifies to straight lines only.
    #[serde(default)]
    pub curves: bool,
    /// Turn angle in degrees at or above which a vertex stays a corner, 0..180.
    #[serde(default = "default_corner_angle")]
    pub corner_angle: f64,
    /// Low-pass radius applied before fitting, in source pixels, 0..8.
    #[serde(default)]
    pub smoothing: f64,
    /// Smallest retained ring area in square source pixels, 0..1048576.
    #[serde(default)]
    pub min_area: f64,
    /// Preview without mutating document state.
    pub dry_run: bool,
    /// Append traced contours; false replaces outlines and components.
    pub append: bool,
}

fn default_corner_angle() -> f64 {
    35.0
}

/// Proposed polygon geometry and optional applied change.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ImageTraceResult {
    /// SHA-256 of the resolved source bytes used for this preview.
    pub source_hash: String,
    /// Captured document revision before mutation.
    #[ts(type = "number")]
    pub generation: u64,
    /// Traced contours after placement, without existing appended contours.
    pub contours: Vec<fc_model::Contour>,
    /// SVG path suitable for a before-apply preview.
    pub path: String,
    /// Font-space bounds; zero for empty output.
    pub bounds: [f64; 4],
    /// On-curve nodes in the traced result.
    pub nodes: u32,
    /// Cubic segments in the traced result; zero for a straight-line trace.
    pub curve_segments: u32,
    /// Largest deviation from the pixel edges, in source pixels.
    pub max_error: f64,
    /// Rings the minimum-area filter removed.
    pub dropped: u32,
    /// Whether a ring kept its exact pixel edges because a fit changed topology.
    pub fallback: bool,
    /// Null for previews.
    pub change: Option<fc_doc::Change>,
}
