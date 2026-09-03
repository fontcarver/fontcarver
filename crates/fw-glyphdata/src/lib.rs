//! Glyph names, Unicode properties, and related data for FontWorkshop.

mod aglfn;
mod category;
mod names;
mod unicode;

pub use category::{category, sub_category, Category};
pub use names::{name_for_unicode, unicode_for_name};
pub use unicode::{
    all_block_names, all_scripts, block_name, cjk_charset_membership, is_cjk_ideograph, script,
    CjkCharsets,
};
