//! Mutation commands and their application to the persistent model.

use std::collections::BTreeSet;

use fw_model::{
    Anchor, Component, Contour, FontInfo, Glyph, GlyphName, Guide, Layer, LayerId, LayerKind,
    Master, MasterId, MasterMetrics, MetricsKeys,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod apply;
mod glyphs;
mod layers;
mod metrics;
mod outline;
mod structure;

pub(crate) use apply::{apply_to_font, impact, Impact};

/// An index into a layer's stored contour and point arrays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct PointRef {
    /// Index in `Layer::contours`.
    pub contour: usize,
    /// Index in `Contour::points`, including off-curve points.
    pub point: usize,
}

/// Whether a node move also carries its unselected adjacent handles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum HandleMode {
    /// Move selected on-curves and their adjacent, unselected handles.
    #[default]
    WithHandles,
    /// Move exactly the listed stored points.
    PointsOnly,
}

/// Shape-preservation policy for point deletion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum DeleteMode {
    /// Re-fit the two neighboring segments into one segment.
    #[default]
    KeepShape,
    /// Remove both adjacent segments and leave open contour pieces.
    BreakContour,
}

/// Segment representation requested by a conversion command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum SegmentKind {
    /// A straight line.
    Line,
    /// A cubic curve.
    Curve,
}

/// Curve representation requested from a whole-path operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum CurveKind {
    /// Cubic Bézier contours.
    Cubic,
    /// Quadratic contours fitted within `max_err` font units.
    Quadratic {
        /// Maximum cubic-to-quadratic approximation error.
        max_err: f64,
    },
}

impl CurveKind {
    pub(crate) fn geometry(self) -> fw_geom::CurveKind {
        match self {
            Self::Cubic => fw_geom::CurveKind::Cubic,
            Self::Quadratic { max_err } => fw_geom::CurveKind::Quadratic { max_err },
        }
    }
}

/// Serializable simplification settings used by [`PathOp::Simplify`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct SimplifyOptions {
    /// Maximum permitted geometric error.
    pub error: f64,
    /// Permit refitting without preserving endpoint slopes.
    pub ignore_slopes: bool,
    /// Permit removal of explicit extrema.
    pub ignore_extrema: bool,
    /// Mark refitted curve endpoints smooth.
    pub smooth_curves: bool,
    /// Convert curves close to their chord into lines.
    pub force_lines: bool,
    /// Snap nearly horizontal or vertical line endpoints.
    pub nearly_hv_lines: bool,
    /// Rotate closed contours to begin at an extremum.
    pub start_at_extrema: bool,
}

impl Default for SimplifyOptions {
    fn default() -> Self {
        let options = fw_geom::SimplifyOptions::default();
        Self {
            error: options.error,
            ignore_slopes: options.ignore_slopes,
            ignore_extrema: options.ignore_extrema,
            smooth_curves: options.smooth_curves,
            force_lines: options.force_lines,
            nearly_hv_lines: options.nearly_hv_lines,
            start_at_extrema: options.start_at_extrema,
        }
    }
}

impl SimplifyOptions {
    pub(crate) fn geometry(self) -> fw_geom::SimplifyOptions {
        fw_geom::SimplifyOptions {
            error: self.error,
            ignore_slopes: self.ignore_slopes,
            ignore_extrema: self.ignore_extrema,
            smooth_curves: self.smooth_curves,
            force_lines: self.force_lines,
            nearly_hv_lines: self.nearly_hv_lines,
            start_at_extrema: self.start_at_extrema,
        }
    }
}

/// Serializable stroke nib shape.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum Nib {
    /// Circular pen.
    Round,
    /// Rotated rectangular calligraphic pen.
    Calligraphic {
        /// Pen width.
        width: f64,
        /// Pen height.
        height: f64,
        /// Counter-clockwise rotation in radians.
        angle: f64,
    },
    /// Axis-aligned square pen.
    Rect,
}

/// Serializable stroke cap style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum StrokeCap {
    /// Flat cap at the endpoint.
    Butt,
    /// Square cap extending beyond the endpoint.
    Square,
    /// Semicircular cap.
    Round,
}

/// Serializable stroke join style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum StrokeJoin {
    /// Beveled corner.
    Bevel,
    /// Mitered corner.
    Miter,
    /// Rounded corner.
    Round,
}

/// Serializable stroke expansion settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct StrokeOptions {
    /// Round/rectangular nib width.
    pub width: f64,
    /// Pen shape.
    pub nib: Nib,
    /// Open-contour cap style.
    pub cap: StrokeCap,
    /// Segment join style.
    pub join: StrokeJoin,
    /// Miter length limit as a multiple of stroke width.
    pub miter_limit: f64,
    /// Remove the inner ring of a closed stroke.
    pub remove_internal: bool,
    /// Remove the outer or only ring.
    pub remove_external: bool,
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            width: 1.0,
            nib: Nib::Round,
            cap: StrokeCap::Round,
            join: StrokeJoin::Round,
            miter_limit: 4.0,
            remove_internal: false,
            remove_external: false,
        }
    }
}

impl StrokeOptions {
    pub(crate) fn geometry(self) -> fw_geom::StrokeOptions {
        fw_geom::StrokeOptions {
            width: self.width,
            nib: match self.nib {
                Nib::Round => fw_geom::Nib::Round,
                Nib::Calligraphic {
                    width,
                    height,
                    angle,
                } => fw_geom::Nib::Calligraphic {
                    width,
                    height,
                    angle,
                },
                Nib::Rect => fw_geom::Nib::Rect,
            },
            cap: match self.cap {
                StrokeCap::Butt => fw_geom::Cap::Butt,
                StrokeCap::Square => fw_geom::Cap::Square,
                StrokeCap::Round => fw_geom::Cap::Round,
            },
            join: match self.join {
                StrokeJoin::Bevel => fw_geom::Join::Bevel,
                StrokeJoin::Miter => fw_geom::Join::Miter,
                StrokeJoin::Round => fw_geom::Join::Round,
            },
            miter_limit: self.miter_limit,
            remove_internal: self.remove_internal,
            remove_external: self.remove_external,
        }
    }
}

/// A whole-layer outline operation delegated to `fw-geom`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum PathOp {
    /// Remove overlapping filled regions.
    RemoveOverlap,
    /// Normalize contour winding.
    CorrectDirection,
    /// Insert missing horizontal and vertical extrema.
    AddExtrema,
    /// Insert cubic inflection points.
    AddInflections,
    /// Simplify and re-fit outlines.
    Simplify(SimplifyOptions),
    /// Round coordinates to a grid of the given size.
    RoundToGrid(f64),
    /// Canonicalize closed-contour start points.
    CanonicalStart,
    /// Canonicalize contour starts and ordering.
    CanonicalOrder,
    /// Expand centerlines into stroked outlines.
    ExpandStroke(StrokeOptions),
    /// Convert all curves to the requested representation.
    ConvertCurves(CurveKind),
}

/// One atomic, undoable document mutation.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum Command {
    /// Insert a new glyph at the end of glyph order.
    AddGlyph {
        /// Glyph to insert.
        #[ts(type = "Glyph")]
        glyph: Glyph,
    },
    /// Remove glyphs while preserving the order of the rest.
    RemoveGlyphs {
        /// Glyph names to remove.
        #[ts(type = "Array<string>")]
        names: Vec<GlyphName>,
    },
    /// Rename a glyph and all model references.
    RenameGlyph {
        /// Existing name.
        #[ts(type = "string")]
        from: GlyphName,
        /// Replacement name.
        #[ts(type = "string")]
        to: GlyphName,
    },
    /// Replace Unicode assignments.
    SetUnicodes {
        /// Glyph to update.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// New scalar values.
        unicodes: Vec<u32>,
    },
    /// Update selected glyph metadata.
    SetGlyphMeta {
        /// Glyph to update.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Export flag update.
        export: Option<bool>,
        /// Color-label update; inner `None` clears it.
        color_label: Option<Option<u8>>,
        /// Complete tag replacement.
        #[ts(type = "Array<string> | null")]
        tags: Option<BTreeSet<String>>,
        /// Note update.
        note: Option<String>,
    },
    /// Set a layer's horizontal advance width.
    SetWidth {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to update.
        #[ts(type = "string")]
        layer: LayerId,
        /// New width.
        width: f64,
    },
    /// Replace all editable outline content in a layer.
    ReplaceLayerOutline {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to update.
        #[ts(type = "string")]
        layer: LayerId,
        /// New contours.
        #[ts(type = "Array<Contour>")]
        contours: Vec<Contour>,
        /// New components.
        #[ts(type = "Array<Component>")]
        components: Vec<Component>,
        /// New anchors.
        #[ts(type = "Array<Anchor>")]
        anchors: Vec<Anchor>,
    },
    /// Replace font-wide metadata.
    SetFontInfo {
        /// New information.
        #[ts(type = "FontInfo")]
        info: Box<FontInfo>,
    },
    /// Replace one master's metrics.
    SetMasterMetrics {
        /// Master to update.
        #[ts(type = "string")]
        master: MasterId,
        /// Replacement metrics.
        #[ts(type = "MasterMetrics")]
        metrics: MasterMetrics,
    },
    /// Move stored outline points.
    MovePoints {
        /// Glyph containing the points.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing the points.
        #[ts(type = "string")]
        layer: LayerId,
        /// Explicit stored indexes.
        points: Vec<PointRef>,
        /// Translation.
        delta: (f64, f64),
        /// Handle movement policy.
        handles: HandleMode,
    },
    /// Set one point's absolute position.
    SetPointPosition {
        /// Glyph containing the point.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing it.
        #[ts(type = "string")]
        layer: LayerId,
        /// Stored point index.
        point: PointRef,
        /// Horizontal coordinate.
        x: f64,
        /// Vertical coordinate.
        y: f64,
    },
    /// Set node smoothness and optionally level handles.
    SetSmooth {
        /// Glyph containing nodes.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing nodes.
        #[ts(type = "string")]
        layer: LayerId,
        /// Stored node indexes.
        points: Vec<PointRef>,
        /// New flag.
        smooth: bool,
        /// Level two adjacent handles.
        level_handles: bool,
    },
    /// Split a geometric segment.
    InsertPointOnSegment {
        /// Glyph containing it.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing it.
        #[ts(type = "string")]
        layer: LayerId,
        /// Contour index.
        contour: usize,
        /// Geometric segment index.
        segment: usize,
        /// Parametric position.
        t: f64,
    },
    /// Delete stored points.
    DeletePoints {
        /// Glyph containing them.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing them.
        #[ts(type = "string")]
        layer: LayerId,
        /// Stored indexes.
        points: Vec<PointRef>,
        /// Deletion policy.
        mode: DeleteMode,
    },
    /// Convert one segment between line and cubic.
    ConvertSegment {
        /// Glyph containing it.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing it.
        #[ts(type = "string")]
        layer: LayerId,
        /// Contour index.
        contour: usize,
        /// Geometric segment index.
        segment: usize,
        /// Requested representation.
        to: SegmentKind,
    },
    /// Append a contour.
    AddContour {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Contour to append.
        #[ts(type = "Contour")]
        contour: Contour,
    },
    /// Delete contours.
    DeleteContours {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Contour indexes.
        contours: Vec<usize>,
    },
    /// Reverse contours.
    ReverseContours {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Contour indexes.
        contours: Vec<usize>,
    },
    /// Rotate a closed contour to begin at an on-curve point.
    SetStartPoint {
        /// Glyph containing it.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing it.
        #[ts(type = "string")]
        layer: LayerId,
        /// New start point.
        point: PointRef,
    },
    /// Join two open contours.
    JoinContours {
        /// Glyph containing them.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing them.
        #[ts(type = "string")]
        layer: LayerId,
        /// First contour end.
        a: PointRef,
        /// Second contour start or end.
        b: PointRef,
    },
    /// Open or split a contour.
    SplitContour {
        /// Glyph containing it.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing it.
        #[ts(type = "string")]
        layer: LayerId,
        /// Split node.
        point: PointRef,
    },
    /// Transform explicit selections, or the whole layer if all are empty.
    TransformSelection {
        /// Glyph containing the selection.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer containing it.
        #[ts(type = "string")]
        layer: LayerId,
        /// Stored point indexes.
        points: Vec<PointRef>,
        /// Component indexes.
        components: Vec<usize>,
        /// Anchor indexes.
        anchors: Vec<usize>,
        /// Kurbo affine coefficients.
        affine: [f64; 6],
    },
    /// Apply a whole-layer path operation.
    PathOp {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Geometry operation.
        op: PathOp,
    },
    /// Append a component.
    AddComponent {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Component to append.
        #[ts(type = "Component")]
        component: Component,
    },
    /// Replace a component.
    SetComponent {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Component index.
        index: usize,
        /// Replacement.
        #[ts(type = "Component")]
        component: Component,
    },
    /// Remove components.
    RemoveComponents {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Component indexes.
        indices: Vec<usize>,
    },
    /// Decompose components into contours.
    DecomposeComponents {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Component indexes.
        indices: Vec<usize>,
    },
    /// Append an anchor.
    AddAnchor {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Anchor to append.
        #[ts(type = "Anchor")]
        anchor: Anchor,
    },
    /// Replace an anchor.
    SetAnchor {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Anchor index.
        index: usize,
        /// Replacement.
        #[ts(type = "Anchor")]
        anchor: Anchor,
    },
    /// Remove anchors.
    RemoveAnchors {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Anchor indexes.
        indices: Vec<usize>,
    },
    /// Append a guide at glyph-layer, master, or font scope.
    AddGuide {
        /// Glyph scope, paired with `layer`.
        #[ts(type = "string | null")]
        glyph: Option<GlyphName>,
        /// Layer scope.
        #[ts(type = "string | null")]
        layer: Option<LayerId>,
        /// Master scope.
        #[ts(type = "string | null")]
        master: Option<MasterId>,
        /// Guide.
        #[ts(type = "Guide")]
        guide: Guide,
    },
    /// Replace a guide at glyph-layer, master, or font scope.
    SetGuide {
        /// Glyph scope.
        #[ts(type = "string | null")]
        glyph: Option<GlyphName>,
        /// Layer scope.
        #[ts(type = "string | null")]
        layer: Option<LayerId>,
        /// Master scope.
        #[ts(type = "string | null")]
        master: Option<MasterId>,
        /// Guide index.
        index: usize,
        /// Replacement.
        #[ts(type = "Guide")]
        guide: Guide,
    },
    /// Remove a guide at glyph-layer, master, or font scope.
    RemoveGuide {
        /// Glyph scope.
        #[ts(type = "string | null")]
        glyph: Option<GlyphName>,
        /// Layer scope.
        #[ts(type = "string | null")]
        layer: Option<LayerId>,
        /// Master scope.
        #[ts(type = "string | null")]
        master: Option<MasterId>,
        /// Guide index.
        index: usize,
    },
    /// Set width and sidebearings in that order.
    SetMetrics {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Optional width.
        width: Option<f64>,
        /// Optional LSB.
        lsb: Option<f64>,
        /// Optional RSB.
        rsb: Option<f64>,
    },
    /// Replace metrics formulas.
    SetMetricsKeys {
        /// Glyph to mutate.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Replacement formulas.
        #[ts(type = "MetricsKeys")]
        keys: MetricsKeys,
    },
    /// Add a non-master layer.
    AddLayer {
        /// Glyph receiving it.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to append.
        #[ts(type = "Layer")]
        layer: Layer,
    },
    /// Remove a non-master layer.
    RemoveLayer {
        /// Glyph containing it.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer identifier.
        #[ts(type = "string")]
        layer: LayerId,
    },
    /// Copy a layer to a background or backup role.
    CopyLayer {
        /// Glyph containing the layers.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Source layer.
        #[ts(type = "string")]
        from: LayerId,
        /// Destination role.
        #[ts(type = "LayerKind")]
        to: LayerKind,
    },
    /// Swap master and background drawing content.
    SwapWithBackground {
        /// Glyph containing the layers.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Master identifying both roles.
        #[ts(type = "string")]
        master: MasterId,
    },
    /// Clone a glyph with Unicode assignments cleared.
    DuplicateGlyph {
        /// Source glyph.
        #[ts(type = "string")]
        source: GlyphName,
        /// New glyph name.
        #[ts(type = "string")]
        name: GlyphName,
    },
    /// Replace persistent glyph order.
    SetGlyphOrder {
        /// Every current name exactly once.
        #[ts(type = "Array<string>")]
        order: Vec<GlyphName>,
    },
    /// Replace an existing master with the same identifier.
    SetMaster {
        /// Replacement master.
        #[ts(type = "Master")]
        master: Master,
    },
    /// Replace all font-global guides.
    SetFontGuides {
        /// Replacement guide list.
        #[ts(type = "Array<Guide>")]
        guides: Vec<Guide>,
    },
    /// Apply commands as one atomic undo entry.
    Batch(Vec<Command>),
}

/// Precise invalidation information produced by a command.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Change {
    /// Changed or transitively invalidated glyph handles.
    #[ts(type = "Array<number>")]
    pub glyphs: Vec<fw_model::GlyphId>,
    /// Whether font-wide metadata changed.
    pub font_info: bool,
    /// Whether master data changed.
    pub masters: bool,
    /// Whether glyph order or names changed.
    pub glyph_order: bool,
    /// Whether advance or sidebearing data changed.
    pub metrics: bool,
    /// Whether the layer list or roles changed.
    pub layers: bool,
    /// Whether glyph, master, or font guides changed.
    pub guides: bool,
}

impl Change {
    pub(crate) fn merge(&mut self, other: &Self) {
        for glyph in &other.glyphs {
            if !self.glyphs.contains(glyph) {
                self.glyphs.push(*glyph);
            }
        }
        self.glyphs.sort_by_key(|glyph| glyph.0);
        self.font_info |= other.font_info;
        self.masters |= other.masters;
        self.glyph_order |= other.glyph_order;
        self.metrics |= other.metrics;
        self.layers |= other.layers;
        self.guides |= other.guides;
    }
}
