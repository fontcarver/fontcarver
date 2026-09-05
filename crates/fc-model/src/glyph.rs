//! Glyph metadata and ordered layers.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{GlyphName, Layer, LayerId, LayerKind, MasterId};

/// Formula references for glyph metrics.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MetricsKeys {
    /// Left sidebearing formula.
    #[serde(default)]
    pub left: Option<String>,
    /// Right sidebearing formula.
    #[serde(default)]
    pub right: Option<String>,
    /// Horizontal width formula.
    #[serde(default)]
    pub width: Option<String>,
    /// Vertical origin formula.
    #[serde(default)]
    pub vertical_origin: Option<String>,
    /// Vertical width formula.
    #[serde(default)]
    pub vertical_width: Option<String>,
}

/// Persistent data for one glyph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Glyph {
    /// Persistent glyph name.
    #[serde(default)]
    pub name: GlyphName,
    /// Assigned Unicode scalar values.
    #[serde(default)]
    pub unicodes: Vec<u32>,
    /// Unicode variation sequences as `(base, variation selector)` pairs.
    #[serde(default)]
    pub variation_selectors: Vec<(u32, u32)>,
    /// Optional production glyph name.
    #[serde(default)]
    pub production_name: Option<String>,
    /// Optional broad glyph category.
    #[serde(default)]
    pub category: Option<String>,
    /// Optional glyph subcategory.
    #[serde(default)]
    pub sub_category: Option<String>,
    /// Optional script identifier.
    #[serde(default)]
    pub script: Option<String>,
    /// Whether the glyph participates in export.
    #[serde(default = "default_export")]
    pub export: bool,
    /// Optional application color-label index.
    #[serde(default)]
    pub color_label: Option<u8>,
    /// Searchable editor tags.
    #[serde(default)]
    pub tags: BTreeSet<String>,
    /// Free-form note.
    #[serde(default)]
    pub note: String,
    /// First-side kerning group name.
    #[serde(default)]
    pub kern_group_side1: Option<String>,
    /// Second-side kerning group name.
    #[serde(default)]
    pub kern_group_side2: Option<String>,
    /// Metrics formulas.
    #[serde(default)]
    pub metrics_keys: MetricsKeys,
    /// Master and supplemental layers in stable order.
    #[serde(default)]
    pub layers: Vec<Layer>,
    /// Whether editing is locked.
    #[serde(default)]
    pub locked: bool,
    /// Format-specific glyph metadata.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub user_data: serde_json::Value,
}

impl Default for Glyph {
    fn default() -> Self {
        Self {
            name: GlyphName::default(),
            unicodes: Vec::new(),
            variation_selectors: Vec::new(),
            production_name: None,
            category: None,
            sub_category: None,
            script: None,
            export: true,
            color_label: None,
            tags: BTreeSet::new(),
            note: String::new(),
            kern_group_side1: None,
            kern_group_side2: None,
            metrics_keys: MetricsKeys::default(),
            layers: Vec::new(),
            locked: false,
            user_data: serde_json::Value::Null,
        }
    }
}

impl Glyph {
    /// Creates an otherwise-empty exported glyph.
    pub fn new(name: impl Into<GlyphName>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Returns this glyph's main layer for a master.
    pub fn master_layer(&self, master: &MasterId) -> Option<&Layer> {
        self.layers.iter().find(|layer| {
            matches!(&layer.kind, LayerKind::Master(layer_master) if layer_master == master)
        })
    }

    /// Returns this glyph's mutable main layer for a master.
    pub fn master_layer_mut(&mut self, master: &MasterId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|layer| {
            matches!(&layer.kind, LayerKind::Master(layer_master) if layer_master == master)
        })
    }

    /// Returns a layer by its persistent identifier.
    pub fn layer(&self, id: &LayerId) -> Option<&Layer> {
        self.layers.iter().find(|layer| &layer.id == id)
    }
}

fn default_export() -> bool {
    true
}
