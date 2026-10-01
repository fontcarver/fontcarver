//! Glyph names, Unicode properties, and related data for FontCarver.

#![warn(missing_docs)]

mod aglfn;
mod category;
mod database;
mod names;
mod repertoire;
mod unicode;
mod unihan;

pub use unihan::{
    iicore_codepoints, kangxi_radical, kangxi_radicals, normalize_reading, unihan,
    unihan_codepoints, Radical, RadicalForm, RadicalStroke, UnihanRecord, KANGXI_RADICAL_COUNT,
    UNIHAN_VERSION,
};

pub use category::{category, category_name, is_assigned, sub_category, Category};
pub use database::{
    explicit_unicodes, parse_overlay, resolve_info, resolve_unicode, validate_overlay,
    GlyphDataOverlay, GlyphInfo, GlyphInfoOverride, BUNDLED_DATABASE_JSON,
};
pub use names::{name_for_unicode, unicode_for_name};
pub use repertoire::{repertoire_contains, repertoire_members, repertoire_total, RepertoireKind};
pub use unicode::{
    all_block_names, all_scripts, block_name, cjk_charset_codepoints, cjk_charset_membership,
    is_cjk_ideograph, script, CjkCharsets,
};
