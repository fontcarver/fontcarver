//! Pure component references.

use std::collections::BTreeMap;

use kurbo::Affine;
use serde::{Deserialize, Serialize};

use crate::GlyphName;

/// A transformed reference to another glyph's outline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Component {
    /// Referenced base glyph.
    #[serde(default)]
    pub base: GlyphName,
    /// Affine transform applied to the resolved base outline.
    #[serde(default = "default_affine")]
    #[cfg_attr(
        feature = "ts",
        ts(type = "[number, number, number, number, number, number]")
    )]
    pub transform: Affine,
    /// Optional anchor used for automatic alignment.
    #[serde(default)]
    pub anchor: Option<String>,
    /// Whether automatic alignment is enabled.
    #[serde(default)]
    pub auto_align: bool,
    /// Whether the base glyph supplies metrics.
    #[serde(default)]
    pub use_my_metrics: bool,
    /// Whether transformed coordinates should be rounded.
    #[serde(default)]
    pub round_to_grid: bool,
    /// Smart-component axis values.
    #[serde(default)]
    pub smart: Option<BTreeMap<String, f64>>,
    /// Base and parent point indexes used for point matching.
    #[serde(default)]
    pub point_match: Option<(u16, u16)>,
}

fn default_affine() -> Affine {
    Affine::IDENTITY
}

impl Default for Component {
    fn default() -> Self {
        Self {
            base: GlyphName::default(),
            transform: Affine::IDENTITY,
            anchor: None,
            auto_align: false,
            use_my_metrics: false,
            round_to_grid: false,
            smart: None,
            point_match: None,
        }
    }
}
