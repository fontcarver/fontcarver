//! Outline-editing, glyph-creation, and metrics API types.

use fc_doc::{DocId, ResolvedMetrics};
use fc_model::{Anchor, Component, Contour, GlyphId, Guide, LayerId, MasterId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{GlyphCell, GlyphFilter, GlyphSort};

/// Parameters for reading one editable layer.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LayerOutlineParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Stable session glyph handle.
    pub gid: GlyphId,
    /// Layer to read, or the default master layer when absent.
    #[serde(default)]
    pub layer: Option<LayerId>,
}

/// One component of a layer with its resolved outline.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ComponentView {
    /// Stored component reference.
    pub component: Component,
    /// Rounded SVG path of the transformed, fully flattened base outline.
    pub path: String,
    /// Exact transformed bounds as `[xmin, ymin, xmax, ymax]`.
    pub bbox: Option<[f64; 4]>,
}

/// Curve family declared by the font's preserved source or master data.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum LayerCurveFormat {
    /// TrueType quadratic outlines.
    Quadratic,
    /// PostScript cubic outlines.
    Cubic,
}

/// Everything the glyph editor needs to draw and edit one layer.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LayerOutline {
    /// Persistent layer identifier.
    pub layer: LayerId,
    /// Master the layer belongs to, or the default master.
    pub master: MasterId,
    /// Curve family declared by preserved model evidence, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub curve_format: Option<LayerCurveFormat>,
    /// Editable contours in stored order.
    pub contours: Vec<Contour>,
    /// Components with their resolved outlines.
    pub components: Vec<ComponentView>,
    /// Layer anchors.
    pub anchors: Vec<Anchor>,
    /// Layer guides.
    pub guides: Vec<Guide>,
    /// Stored horizontal advance width.
    pub width: f64,
    /// Left sidebearing measured from the flattened outline.
    pub lsb: f64,
    /// Right sidebearing measured from the flattened outline.
    pub rsb: f64,
    /// Exact flattened bounds as `[xmin, ymin, xmax, ymax]`.
    pub bbox: Option<[f64; 4]>,
    /// Metrics after resolving metrics keys.
    pub resolved_metrics: ResolvedMetrics,
}

/// Flattened component geometry used for ghost-node snapping.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FlattenedComponents {
    /// One rounded SVG path per component, in stored order.
    pub paths: Vec<String>,
    /// Every point of those paths as `[x, y]`, including off-curve points.
    pub points: Vec<[f64; 2]>,
}

/// Session handle reported for a glyph a dry run only previewed.
///
/// A previewed glyph does not exist in the document, so it has no handle. The
/// sentinel is deliberately outside the range any session allocates, so a caller
/// that forwards a preview cell to a glyph-scoped method gets a clean
/// "does not exist" error instead of acting on an unrelated glyph.
pub const PREVIEW_GLYPH_ID: GlyphId = GlyphId(u32::MAX);

/// Parameters for creating glyphs from a recipe.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AddGlyphsParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Recipe such as `A-Z uni4E00-uni4E1F a.sc f_i @charset:GB2312`.
    pub recipe: String,
    /// Master used for the returned cells, or the default master.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Preview the recipe instead of applying it.
    ///
    /// A dry run resolves the recipe against the document and reports exactly
    /// what a real call would create and skip, without touching the document or
    /// its undo stack. The previewed glyphs do not exist yet, so their cells
    /// carry [`PREVIEW_GLYPH_ID`](crate::PREVIEW_GLYPH_ID) instead of a session
    /// handle and no geometry; callers identify a preview by its name.
    #[serde(default)]
    pub dry_run: bool,
}

/// Result of one `glyphs.add` recipe.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct AddGlyphsResult {
    /// Cells of the glyphs that were created, in creation order.
    pub added: Vec<GlyphCell>,
    /// Recipe items that were skipped because they exist or are unknown.
    pub skipped: Vec<String>,
}

/// Parameters for listing every matching glyph handle.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GlyphIdsParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Master used for width sorting and component filtering.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Filter conditions.
    #[serde(default)]
    pub filter: GlyphFilter,
    /// Sort order.
    #[serde(default)]
    pub sort: GlyphSort,
}

/// Parameters for resolving one glyph's metrics keys.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct MetricsResolveParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Stable session glyph handle.
    pub gid: GlyphId,
    /// Master to resolve in, or the default master when absent.
    #[serde(default)]
    pub master: Option<MasterId>,
}
