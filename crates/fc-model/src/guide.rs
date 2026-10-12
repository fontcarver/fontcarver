//! Global, master, and layer guide data.

use serde::{Deserialize, Serialize};

/// What a guide's stored horizontal position is measured from.
///
/// Anchoring lets a guide follow the glyph rather than the canvas: a guide
/// anchored to a sidebearing keeps its distance to that edge of the ink when
/// the outline or the advance moves, which is what makes it usable as a
/// "stay 20 units inside the left edge" reference across a whole family.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GuideAnchor {
    /// Absolute font-space x, which is how every imported guide arrives.
    #[default]
    Origin,
    /// Offset from the left edge of the layer's ink (its left sidebearing).
    Lsb,
    /// Offset from the right edge of the layer's ink (its right sidebearing).
    Rsb,
}

/// A guide in font-space coordinates.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Guide {
    /// Optional stable identity for layer-local node relationships.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    /// Horizontal origin in font units.
    #[serde(default)]
    pub x: f64,
    /// Vertical origin in font units.
    #[serde(default)]
    pub y: f64,
    /// Counter-clockwise angle in degrees.
    #[serde(default)]
    pub angle: f64,
    /// Optional display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Whether editing is locked.
    #[serde(default)]
    pub locked: bool,
    /// Optional serialized color.
    #[serde(default)]
    pub color: Option<String>,
    /// What `x` is measured from; `Origin` keeps it absolute.
    #[serde(default)]
    pub anchor: GuideAnchor,
}
