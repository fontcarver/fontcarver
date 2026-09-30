//! Serializable request and response types for every API method.

mod automation;
pub use automation::*;

mod bitmap;
mod image;
pub use image::*;
mod bitmap_parity;
pub use bitmap_parity::*;
mod commands;
pub use bitmap::*;
mod editing;
mod events;
mod export;
mod features;
mod font;
mod glyph_lists;
mod glyphs;
pub use glyph_lists::*;
mod smart_filters;
pub use smart_filters::*;
mod hints;
mod tt_hints;
pub use tt_hints::*;
mod color;
mod svg;
pub use svg::*;
mod kerning;
mod layout;
mod layout_conversion;
pub use color::*;
mod math;
pub use math::*;
mod problems;
mod session;
mod shaping;
mod variable_proof;
mod variation;
pub use hints::*;
pub use variable_proof::*;
pub use variation::*;

pub use commands::*;
pub use editing::*;
pub use events::*;
pub use export::*;
pub use features::*;
pub use font::*;
pub use glyphs::*;
pub use kerning::*;
pub use layout::*;
pub use layout_conversion::*;
pub use problems::*;
pub use session::*;
pub use shaping::*;

mod bitmap_config;
pub use bitmap_config::*;

mod cjk;
pub use cjk::*;

mod cjk_batch;
pub use cjk_batch::*;
mod style;
pub use style::*;

mod effects;
pub use effects::*;

mod constructions;
pub use constructions::*;

mod cross_paste;
pub use cross_paste::*;

mod glyph_data;
pub use glyph_data::*;

mod font_compare;
pub use font_compare::*;

mod optical;
pub use optical::*;
