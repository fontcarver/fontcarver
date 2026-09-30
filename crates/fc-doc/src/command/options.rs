//! Serializable options shared by outline commands.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
    /// Re-fit the segments around every deleted node into one segment.
    #[default]
    KeepShape,
    /// Remove both adjacent segments and leave open contour pieces.
    BreakContour,
    /// Remove the listed points and their handles without re-fitting.
    Simple,
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
    pub(crate) fn geometry(self) -> fc_geom::CurveKind {
        match self {
            Self::Cubic => fc_geom::CurveKind::Cubic,
            Self::Quadratic { max_err } => fc_geom::CurveKind::Quadratic { max_err },
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
        let options = fc_geom::SimplifyOptions::default();
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
    pub(crate) fn geometry(self) -> fc_geom::SimplifyOptions {
        fc_geom::SimplifyOptions {
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
    pub(crate) fn geometry(self) -> fc_geom::StrokeOptions {
        fc_geom::StrokeOptions {
            width: self.width,
            nib: match self.nib {
                Nib::Round => fc_geom::Nib::Round,
                Nib::Calligraphic {
                    width,
                    height,
                    angle,
                } => fc_geom::Nib::Calligraphic {
                    width,
                    height,
                    angle,
                },
                Nib::Rect => fc_geom::Nib::Rect,
            },
            cap: match self.cap {
                StrokeCap::Butt => fc_geom::Cap::Butt,
                StrokeCap::Square => fc_geom::Cap::Square,
                StrokeCap::Round => fc_geom::Cap::Round,
            },
            join: match self.join {
                StrokeJoin::Bevel => fc_geom::Join::Bevel,
                StrokeJoin::Miter => fc_geom::Join::Miter,
                StrokeJoin::Round => fc_geom::Join::Round,
            },
            miter_limit: self.miter_limit,
            remove_internal: self.remove_internal,
            remove_external: self.remove_external,
        }
    }
}

/// A whole-layer outline operation delegated to `fc-geom`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum PathOp {
    /// Remove overlapping filled regions.
    RemoveOverlap,
    /// Normalize contour winding.
    CorrectDirection,
    /// Insert missing horizontal and vertical extrema.
    AddExtrema,
    /// Insert extrema even when they are very close to existing endpoints.
    ForceExtrema,
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
