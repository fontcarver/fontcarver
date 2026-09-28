//! Revision-checked editing and baking of one layer's persistent effects.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Preview, replace or bake the effect stack on a captured source layer.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct LayerEffectsParams {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Glyph whose source is being edited.
    pub glyph: String,
    /// Persistent source layer identifier.
    pub layer: fc_model::LayerId,
    /// Complete proposed stack in evaluation order.
    pub effects: Vec<fc_model::LayerEffect>,
    /// Only calculate the proposed result.
    pub dry_run: bool,
    /// Replace source geometry by the evaluated result and clear the stack.
    #[serde(default)]
    pub bake: bool,
    /// Required for writes; obtained from the reviewed preview.
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
}

/// Evaluated draft and optional committed transaction.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct LayerEffectsResult {
    /// Document revision used for evaluation, before any mutation.
    #[ts(type = "number")]
    pub generation: u64,
    /// Before/after drawings in one shared coordinate frame.
    pub example: crate::StyleBatchExample,
    /// Source layers requiring polygon curve refitting, including components.
    pub approximated_layers: Vec<fc_model::LayerId>,
    /// Invalidation after applying or baking; null for preview.
    pub change: Option<fc_doc::Change>,
}

/// Capture an evaluated mask as an independent SVG path in source coordinates.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct EffectMaskParams {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Glyph to capture.
    pub glyph: String,
    /// Source layer, or the default master when omitted.
    pub layer: Option<fc_model::LayerId>,
}
