//! Layout and shaping services for FontCarver documents.
//!
//! P1 provides the shaping half: [`Shaper`] keeps one in-memory font per
//! document and master, compiled from the model by `fc-compile`, and shapes text
//! against it with HarfRust. Feature and layout-table editing arrive in a later
//! phase.

#![warn(missing_docs)]

pub mod conversion;
mod error;
mod import;
mod shaping;

pub use error::Error;
pub use import::{
    equivalent_layout, LayoutBlocker, LayoutImport, LayoutLookup, LayoutRule, LayoutTableSummary,
};
pub use shaping::{
    widths_for_text, widths_for_text_indexed, FontKey, ModelWidths, ShapePlan, ShapeRequest,
    ShapedGlyph, ShapedRun, Shaper, COMPILE_FAILED, DEFAULT_DEBOUNCE,
};
