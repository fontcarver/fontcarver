//! Editable font-wide OpenType mathematical typesetting data.

use crate::GlyphName;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod constants;
mod validation;
pub use constants::MathConstants;
pub use validation::MAX_MATH_ITEMS;

/// Pixel corrections for consecutive PPEM sizes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathDevice {
    /// First PPEM size with a correction.
    #[serde(default)]
    pub start_size: u16,
    /// Signed pixel corrections, starting at `start_size`.
    #[serde(default)]
    pub deltas: Vec<i8>,
}

/// A font-unit value with optional pixel corrections.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathValue {
    /// Design-space distance in font units.
    #[serde(default)]
    pub value: f64,
    /// Optional size-specific pixel corrections.
    #[serde(default)]
    pub device: Option<MathDevice>,
}

impl From<f64> for MathValue {
    fn from(value: f64) -> Self {
        Self {
            value,
            device: None,
        }
    }
}

/// A piecewise constant mathematical kerning function.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathKern {
    /// Ascending baseline-relative transition heights.
    #[serde(default)]
    pub heights: Vec<MathValue>,
    /// Kerning values for each interval; length is heights plus one.
    #[serde(default)]
    pub values: Vec<MathValue>,
}

/// Optional kerning functions for the four corners of a glyph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathKerns {
    /// Kerning at the top right.
    #[serde(default)]
    pub top_right: Option<MathKern>,
    /// Kerning at the top left.
    #[serde(default)]
    pub top_left: Option<MathKern>,
    /// Kerning at the bottom right.
    #[serde(default)]
    pub bottom_right: Option<MathKern>,
    /// Kerning at the bottom left.
    #[serde(default)]
    pub bottom_left: Option<MathKern>,
}

impl MathKerns {
    /// Corner records in binary order.
    pub fn corners(&self) -> [&Option<MathKern>; 4] {
        [
            &self.top_right,
            &self.top_left,
            &self.bottom_right,
            &self.bottom_left,
        ]
    }
}

/// A ready-made glyph variant for one extension direction.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathVariant {
    /// Referenced glyph name.
    #[serde(default)]
    pub glyph: GlyphName,
    /// Measurement in the construction's extension direction.
    #[serde(default)]
    pub advance: f64,
}

/// One part of a stretchable glyph assembly.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathPart {
    /// Referenced glyph name.
    #[serde(default)]
    pub glyph: GlyphName,
    /// Connector length at the left or bottom end.
    #[serde(default)]
    pub start_connector: f64,
    /// Connector length at the right or top end.
    #[serde(default)]
    pub end_connector: f64,
    /// Full part advance in the extension direction.
    #[serde(default)]
    pub full_advance: f64,
    /// Whether the part may be omitted or repeated.
    #[serde(default)]
    pub extender: bool,
}

/// Parts arranged left to right or bottom to top.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathAssembly {
    /// Italic correction for the assembled shape.
    #[serde(default)]
    pub italic_correction: MathValue,
    /// Ordered parts in the extension direction.
    #[serde(default)]
    pub parts: Vec<MathPart>,
}

/// Ready-made variants and an optional extensible assembly.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathConstruction {
    /// Available sizes, in source order.
    #[serde(default)]
    pub variants: Vec<MathVariant>,
    /// Assembly used when ready-made sizes are insufficient.
    #[serde(default)]
    pub assembly: Option<MathAssembly>,
}

/// Mathematical placement and construction data for one glyph.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathGlyphInfo {
    /// Horizontal correction for a slanted glyph.
    #[serde(default)]
    pub italic_correction: Option<MathValue>,
    /// Horizontal attachment position for a top accent.
    #[serde(default)]
    pub top_accent: Option<MathValue>,
    /// Use the ink box for vertical positioning beside this shape.
    #[serde(default)]
    pub extended_shape: bool,
    /// Kerning functions for the four corners.
    #[serde(default)]
    pub kern: MathKerns,
    /// Construction that grows vertically.
    #[serde(default)]
    pub vertical: Option<MathConstruction>,
    /// Construction that grows horizontally.
    #[serde(default)]
    pub horizontal: Option<MathConstruction>,
}

/// Font-wide MATH 1.0 source data, independent of numeric glyph order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathTable {
    /// Optional global constants; absent in a partial source table.
    #[serde(default)]
    pub constants: Option<MathConstants>,
    /// Minimum overlap between adjacent assembly connectors.
    #[serde(default)]
    pub min_connector_overlap: f64,
    /// Placement and construction records keyed by persistent glyph name.
    #[serde(default)]
    pub glyphs: BTreeMap<GlyphName, MathGlyphInfo>,
}

impl MathTable {
    /// Rename a covered glyph and all variant/assembly references.
    pub fn rename_glyph(&mut self, from: &str, to: &GlyphName) {
        if let Some(info) = self.glyphs.remove(from) {
            self.glyphs.insert(to.clone(), info);
        }
        for info in self.glyphs.values_mut() {
            for construction in [&mut info.vertical, &mut info.horizontal]
                .into_iter()
                .flatten()
            {
                for variant in &mut construction.variants {
                    if variant.glyph.as_ref() == from {
                        variant.glyph = to.clone();
                    }
                }
                if let Some(assembly) = &mut construction.assembly {
                    for part in &mut assembly.parts {
                        if part.glyph.as_ref() == from {
                            part.glyph = to.clone();
                        }
                    }
                }
            }
        }
    }
}
