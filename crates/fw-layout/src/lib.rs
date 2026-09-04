//! Layout and shaping services for FontWorkshop documents.
//!
//! P1 provides the shaping half: [`Shaper`] keeps one in-memory font per
//! document and master, compiled from the model by `fw-compile`, and shapes text
//! against it with HarfRust. Feature and layout-table editing arrive in a later
//! phase.

#![warn(missing_docs)]

mod error;
mod shaping;

pub use error::Error;
pub use shaping::{
    widths_for_text, FontKey, ModelWidths, ShapePlan, ShapeRequest, ShapedGlyph, ShapedRun, Shaper,
    COMPILE_FAILED, DEFAULT_DEBOUNCE,
};
