//! Embedded bitmap strikes and composite bitmap references.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::GlyphName;

mod ops;
pub use ops::{MAX_BITMAP_BYTES, MAX_BITMAP_SIZE, MAX_FONT_BITMAP_BYTES};

/// A value from a BDF properties dictionary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BdfValue {
    /// Signed integer value.
    Int(i64),
    /// String value.
    Str(String),
}

impl Default for BdfValue {
    fn default() -> Self {
        Self::Int(0)
    }
}

/// Packed bitmap pixels for one glyph.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Bitmap {
    /// Horizontal bitmap origin relative to the glyph origin.
    #[serde(default)]
    pub origin_x: i16,
    /// Vertical bitmap origin relative to the glyph origin.
    #[serde(default)]
    pub origin_y: i16,
    /// Width in pixels.
    #[serde(default)]
    pub width: u16,
    /// Height in pixels.
    #[serde(default)]
    pub height: u16,
    /// Bits per pixel.
    #[serde(default)]
    pub depth: u8,
    /// MSB-first, row-major packed pixels.
    #[serde(default)]
    pub rows: Vec<u8>,
}

/// A translated reference to another bitmap glyph.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BitmapRef {
    /// Referenced base glyph.
    #[serde(default)]
    pub base: GlyphName,
    /// Horizontal pixel offset.
    #[serde(default)]
    pub dx: i16,
    /// Vertical pixel offset.
    #[serde(default)]
    pub dy: i16,
}

/// Bitmap data and metrics for one glyph in a strike.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BitmapGlyph {
    /// Packed pixel data.
    #[serde(default)]
    pub bitmap: Bitmap,
    /// Horizontal pixel advance.
    #[serde(default)]
    pub advance: i16,
    /// Optional vertical pixel advance.
    #[serde(default)]
    pub vertical_advance: Option<i16>,
    /// Vertical writing origin relative to the horizontal origin, in pixels.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vertical_origin: Option<(i16, i16)>,
    /// Composite bitmap references.
    #[serde(default)]
    pub refs: Vec<BitmapRef>,
}

/// A complete bitmap strike at one size and depth.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BitmapStrike {
    /// Pixels per em.
    #[serde(default)]
    pub ppem: u16,
    /// Bits per pixel.
    #[serde(default)]
    pub depth: u8,
    /// Pixel ascender.
    #[serde(default)]
    pub ascent: i16,
    /// Pixel descender magnitude.
    #[serde(default)]
    pub descent: i16,
    /// BDF-compatible strike properties.
    #[serde(default)]
    pub properties: BTreeMap<String, BdfValue>,
    /// Bitmap glyphs keyed by persistent glyph name.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, BitmapGlyph>"))]
    pub glyphs: BTreeMap<GlyphName, BitmapGlyph>,
}
