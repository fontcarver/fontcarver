//! Read-only queries over the document model.

mod brackets;
mod compatibility;
pub use brackets::{expand_brackets, BracketRule};
mod curves;
mod outline;
mod relationships;
mod unicode_index;
mod validation;
pub use unicode_index::UnicodeIndex;

pub use compatibility::{masters_compatible, Incompatibility};
pub use curves::{declared_curve_format, CurveFormat};
pub use outline::{flatten_layer, flatten_layer_by_id, flatten_smart_component, layer_bbox};
pub use relationships::{dependents, glyph_for_unicode};
pub use validation::{validate, Issue};
