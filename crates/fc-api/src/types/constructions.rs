//! Preview and commit reusable glyph attachments on stable carrier nodes.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Preview, replace, append to or bake a layer's construction list.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct LayerConstructionsParams {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Glyph containing the carrier geometry.
    pub glyph: String,
    /// Persistent editable source layer.
    pub layer: fc_model::LayerId,
    /// Complete proposed list of existing attachments.
    pub constructions: Vec<fc_model::GlyphConstruction>,
    /// Optional new attachment; the backend assigns persistent identities atomically.
    #[serde(default)]
    pub add: Option<ConstructionCreate>,
    /// Calculate the proposed result without changing the document.
    pub dry_run: bool,
    /// Replace editable outlines with evaluated geometry and clear live operations.
    #[serde(default)]
    pub bake: bool,
    /// Required for writes; captured from the reviewed preview.
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
}

/// A new attachment expressed using the current editable point selection.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct ConstructionCreate {
    /// Reusable source glyph name.
    pub source: String,
    /// Placement mode and selected carrier nodes.
    pub kind: ConstructionCreateKind,
    /// Retain the carrier contour in the evaluated output.
    pub keep_carrier: bool,
}

/// Creation parameters whose point indices resolve to persistent node identities.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConstructionCreateKind {
    /// Place a source glyph at an interior carrier corner.
    Corner {
        /// Interior carrier point.
        at: fc_doc::PointRef,
        /// Uniform source scale.
        scale: f64,
    },
    /// Place a source glyph at an open carrier endpoint.
    Cap {
        /// Carrier endpoint.
        at: fc_doc::PointRef,
        /// Uniform source scale.
        scale: f64,
    },
    /// Map a source glyph onto one carrier segment.
    Segment {
        /// Start node of the carrier segment.
        start: fc_doc::PointRef,
        /// End node of the carrier segment.
        end: fc_doc::PointRef,
        /// Source x-axis distance mapped onto the carrier segment.
        source_length: f64,
    },
    /// Repeat a source glyph along the selected carrier contour.
    Brush {
        /// Node identifying the carrier contour.
        at: fc_doc::PointRef,
        /// Distance between placements in carrier units.
        spacing: f64,
        /// Uniform source scale.
        scale: f64,
    },
}
