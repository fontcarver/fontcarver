//! Mutation commands and their application to the persistent model.

use std::collections::BTreeSet;

use fc_model::{
    Anchor, Component, Contour, FontInfo, Glyph, GlyphName, Guide, Layer, LayerId, LayerKind,
    Master, MasterId, MasterMetrics, MetricsKeys,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod apply;
mod bitmap;
mod cid;
pub use bitmap::BitmapCommand;
mod designspace;
pub use designspace::DesignSpaceCommand;
mod features;
mod glyphs;
mod hints;
pub use hints::HintCommand;
mod kerning;
mod layers;
mod math;
pub use math::MathCommand;
mod metrics;
mod options;
mod outline;
mod structure;

pub(crate) use apply::{apply_to_font, impact, Impact};
pub(crate) use features::{bump_revision as bump_feature_revision, is_draft};
pub(crate) use hints::invalidate_stale_ps_hints;
pub(crate) use kerning::is_kerning;

pub use options::{
    CurveKind, DeleteMode, HandleMode, Nib, PathOp, PointRef, SegmentKind, SimplifyOptions,
    StrokeCap, StrokeJoin, StrokeOptions,
};

/// One atomic, undoable document mutation.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum Command {
    /// Edit hints or infer stems and zones without changing outline geometry.
    Hints(HintCommand),
    /// Set or disable CID metadata; incomplete mappings may be authored but cannot export.
    SetCidInfo {
        /// Registry, ordering, supplement and numeric mapping, or null to disable.
        #[ts(type = "CidInfo | null")]
        cid: Option<fc_model::CidInfo>,
    },
    /// Edit axes, masters and named instances as one validated operation.
    DesignSpace(DesignSpaceCommand),
    /// Edit bitmap pixels and strikes independently of outline data.
    Bitmap(BitmapCommand),
    /// Edit font-wide mathematical typesetting data.
    Math(MathCommand),
    /// Save source text without compiling or changing effective layout authority.
    SetFeatureDraft {
        /// Current persistent feature editor revision.
        #[ts(type = "number")]
        expected_revision: u64,
        /// Candidate FEA source; imported-table authority is rejected.
        #[ts(type = "Features")]
        source: fc_model::Features,
        /// Candidate settings, inactive until successful application.
        #[ts(type = "FeatureGenerationSettings")]
        settings: fc_model::FeatureGenerationSettings,
    },
    /// Remove the saved draft while preserving effective source.
    DiscardFeatureDraft {
        /// Current persistent feature editor revision.
        #[ts(type = "number")]
        expected_revision: u64,
    },
    /// Set or remove one horizontal kerning record in a master.
    SetHorizontalKerning {
        /// Persistent master identifier.
        #[ts(type = "string")]
        master: MasterId,
        /// First glyph or first-side group.
        #[ts(type = "KernSide")]
        left: fc_model::KernSide,
        /// Second glyph or second-side group.
        #[ts(type = "KernSide")]
        right: fc_model::KernSide,
        /// Explicit adjustment, including zero; null removes the record.
        value: Option<f64>,
    },
    /// Set or remove one vertical kerning record in a master.
    SetVerticalKerning {
        /// Persistent master identifier.
        #[ts(type = "string")]
        master: MasterId,
        /// First glyph or first-side group.
        #[ts(type = "KernSide")]
        left: fc_model::KernSide,
        /// Second glyph or second-side group.
        #[ts(type = "KernSide")]
        right: fc_model::KernSide,
        /// Explicit adjustment, including zero; null removes the record.
        value: Option<f64>,
    },
    /// Create a kerning group, rejecting an existing name atomically.
    CreateKerningGroup {
        /// Logical pair position.
        side: crate::kerning::KerningGroupSide,
        /// Canonical UFO group name.
        name: String,
        /// Initial members, without duplicates or membership conflicts.
        #[ts(type = "Array<string>")]
        members: Vec<GlyphName>,
    },
    /// Create a kerning group or replace its members.
    SetKerningGroup {
        /// Logical pair position.
        side: crate::kerning::KerningGroupSide,
        /// Canonical UFO group name.
        name: String,
        /// Complete member list, without duplicates or membership conflicts.
        #[ts(type = "Array<string>")]
        members: Vec<GlyphName>,
    },
    /// Move one glyph to a group on a given side, or remove its membership.
    AssignKerningGroup {
        /// Logical pair position.
        side: crate::kerning::KerningGroupSide,
        /// Existing glyph.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Existing canonical group, or null to unassign.
        group: Option<String>,
    },
    /// Rename a group and all horizontal pair references atomically.
    RenameKerningGroup {
        /// Logical pair position.
        side: crate::kerning::KerningGroupSide,
        /// Existing canonical name.
        from: String,
        /// New canonical name.
        to: String,
    },
    /// Remove a group, optionally removing its horizontal pair references.
    RemoveKerningGroup {
        /// Logical pair position.
        side: crate::kerning::KerningGroupSide,
        /// Existing canonical name.
        name: String,
        /// Explicitly authorize removal of all horizontal pairs using the group.
        remove_pairs: bool,
    },
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
    /// Set a layer's vertical advance and origin; null restores master defaults.
    SetVerticalMetrics {
        /// Glyph to update.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Editable source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Vertical advance, between zero and 65535 font units.
        advance: Option<f64>,
        /// Vertical origin in signed font units.
        origin: Option<f64>,
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
    /// Center the resolved outline within an advance, using current component geometry.
    CenterMetrics {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to mutate.
        #[ts(type = "string")]
        layer: LayerId,
        /// Final horizontal advance.
        width: f64,
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
    /// Bitmap pixels, references or strike metadata changed.
    #[serde(default)]
    pub bitmaps: bool,
    /// Font-wide MATH data changed.
    #[serde(default)]
    pub math: bool,
    /// Whether feature editor source, settings or conversion metadata changed.
    #[serde(default)]
    pub feature_source: bool,
    /// Whether layout rules or kerning membership changed without outline edits.
    #[serde(default)]
    pub layout: bool,
    /// Changed or transitively invalidated glyph handles.
    #[ts(type = "Array<number>")]
    pub glyphs: Vec<fc_model::GlyphId>,
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
    /// Whether this mutation changes any existing compile input.
    ///
    /// Draft-only mutations are deliberately excluded; legacy mutation causes
    /// retain their conservative invalidation behavior, including guides.
    pub fn affects_compile(&self) -> bool {
        self.layout
            || !self.glyphs.is_empty()
            || self.font_info
            || self.masters
            || self.glyph_order
            || self.metrics
            || self.layers
            || self.guides
    }

    pub(crate) fn merge(&mut self, other: &Self) {
        self.bitmaps |= other.bitmaps;
        self.layout |= other.layout;
        self.feature_source |= other.feature_source;
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
