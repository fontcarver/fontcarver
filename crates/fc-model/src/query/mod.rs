//! Read-only queries over the document model.

mod brackets;
mod compatibility;
pub use brackets::{expand_brackets, BracketRule};
mod curves;
mod hint_masks;
pub use hint_masks::{path_hint_masks, PathHintMasks};
mod outline;
mod relationships;
mod unicode_index;
mod validation;
pub use unicode_index::UnicodeIndex;

pub use compatibility::{masters_compatible, Incompatibility};
pub use curves::{declared_curve_format, CurveFormat};
pub use outline::{
    flatten_layer, flatten_layer_by_id, flatten_layer_by_id_with, flatten_layer_by_id_with_sources,
    flatten_layer_with, flatten_layer_with_sources, flatten_smart_component,
    flatten_smart_component_with, flatten_smart_component_with_sources, layer_bbox,
    LayerPathFilter, LayerSourceFilter,
};
pub use relationships::{dependents, glyph_for_unicode};
pub use validation::{validate, Issue};
