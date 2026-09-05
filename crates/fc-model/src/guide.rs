//! Global, master, and layer guide data.

use serde::{Deserialize, Serialize};

/// A guide in font-space coordinates.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Guide {
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
}
