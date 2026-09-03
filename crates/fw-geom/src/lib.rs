//! Geometry operations for FontWorkshop outlines.
//!
//! The crate uses kurbo paths as its working representation. Quadratic model
//! contours are degree-elevated exactly before cubic-only operations.

#![warn(missing_docs)]

mod boolean;
mod convert;
mod edit;
mod error;
mod interpolate;
mod stroke;

pub use boolean::{boolean, remove_overlap, BooleanOp};
pub use convert::{from_bezpaths, to_bezpaths, CurveKind};
pub use edit::{
    add_extrema, add_inflections, bbox, canonical_order, canonical_start_points, correct_direction,
    intersections_with_line, nearest_on_path, reverse, round_to_grid, segment_to_points, simplify,
    split_segment, transform, PathHit, SimplifyOptions,
};
pub use error::Error;
pub use interpolate::{compatible, interpolate, Incompatibility};
pub use kurbo::{Cap, Join};
pub use stroke::{expand_stroke, Nib, StrokeOptions};

/// A geometric tolerance measured in font units.
pub type Tolerance = f64;

/// Default tolerance for boolean operations, in font units.
pub const DEFAULT_BOOLEAN_TOLERANCE: Tolerance = 0.01;

/// Default tolerance for outline simplification, in font units.
pub const DEFAULT_SIMPLIFY_TOLERANCE: Tolerance = 0.1;
