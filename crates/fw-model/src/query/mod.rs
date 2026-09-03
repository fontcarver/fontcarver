//! Read-only queries over the document model.

mod compatibility;
mod outline;
mod relationships;
mod validation;

pub use compatibility::{masters_compatible, Incompatibility};
pub use outline::{flatten_layer, layer_bbox};
pub use relationships::{dependents, glyph_for_unicode};
pub use validation::{validate, Issue};
