//! Bounded, revision-checked transformations of a captured glyph selection.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Scale, shear, translate, then expand or contract a filled outline.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct StyleEffectOptions {
    /// Horizontal and vertical scale factors in [0.05, 4].
    pub scale: [f64; 2],
    /// Translation in font units.
    pub translate: [f64; 2],
    /// Oblique shear angle in degrees, within ±60.
    pub oblique: f64,
    /// X/Y boundary offsets: both zero, or same sign with magnitude [0.1, 500].
    pub weight: [f64; 2],
}
impl From<&StyleEffectOptions> for fc_geom::StyleOptions {
    fn from(value: &StyleEffectOptions) -> Self {
        Self {
            scale: value.scale,
            translate: value.translate,
            oblique: value.oblique,
            weight: value.weight,
        }
    }
}
/// Preview or apply baked outline effects to up to 512 selected glyphs.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct StyleBatchParams {
    /// Open document.
    pub doc: fc_doc::DocId,
    /// Captured session glyph handles.
    pub glyphs: Vec<fc_model::GlyphId>,
    /// One master, or null for all masters.
    pub master: Option<fc_model::MasterId>,
    /// Editable numeric effect settings.
    pub options: StyleEffectOptions,
    /// Scale horizontal advances by the X scale.
    pub scale_advance: bool,
    /// Center the final outline inside its advance; may combine with advance scaling.
    pub center: bool,
    /// Add a backup of each original resolved outline and its anchors/advances.
    pub backup: bool,
    /// Prepare a preview without mutating the document.
    pub dry_run: bool,
    /// Required on apply; obtained from the reviewed preview.
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
    /// Undo and backup display label.
    pub label: String,
}
/// One outline example in a batch preview, in Y-up font coordinates.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct StyleBatchExample {
    /// Persistent glyph name.
    pub glyph: String,
    /// Source master name.
    pub master: String,
    /// Original SVG path.
    pub before: String,
    /// Proposed SVG path.
    pub after: String,
    /// Original horizontal advance.
    pub before_width: f64,
    /// Proposed horizontal advance.
    pub after_width: f64,
    /// Union of before/after outline bounds, [x0, y0, x1, y1].
    pub bounds: [f64; 4],
}
/// Review summary or successful atomic transaction.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
pub struct StyleBatchResult {
    /// Source revision before any mutation.
    #[ts(type = "number")]
    pub generation: u64,
    /// Distinct glyphs included.
    pub glyph_count: u32,
    /// Master layers included.
    pub layer_count: u32,
    /// Layers whose components become explicit contours.
    pub decomposed_layers: u32,
    /// Layers requiring polygon union and curve refitting after a strict failure.
    pub approximated_layers: u32,
    /// Glyph/master labels for every layer using the refitting fallback.
    pub approximated_sources: Vec<String>,
    /// First twelve layers for visual review.
    pub examples: Vec<StyleBatchExample>,
    /// Invalidation after an apply; null for preview.
    pub change: Option<fc_doc::Change>,
}
