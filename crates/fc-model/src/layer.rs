//! Glyph layers, source images, hints, and color-outline data.

use kurbo::Affine;
use serde::{Deserialize, Serialize};

use crate::{Anchor, Component, Contour, DesignLocation, Guide, LayerId, MasterId, Tag};

/// Semantic role of a glyph layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerKind {
    /// Local smart-component source associated with a global master.
    Smart {
        /// Associated global master.
        master: MasterId,
        /// Local axis values; omitted axes use their defaults.
        #[serde(default)]
        location: std::collections::BTreeMap<String, f64>,
    },
    /// Main layer for a design-space master.
    Master(MasterId),
    /// Intermediate source at an explicit design location.
    Brace {
        /// Associated master.
        master: MasterId,
        /// Intermediate design-space location.
        #[serde(default, with = "crate::font::design_location_serde")]
        #[cfg_attr(feature = "ts", ts(type = "Record<string, number>"))]
        location: DesignLocation,
    },
    /// Conditional alternate layer active over one or more axis ranges.
    Bracket {
        /// Associated master.
        master: MasterId,
        /// Axis controlling the substitution.
        axis: Tag,
        /// Inclusive lower bound, when present.
        min: Option<f64>,
        /// Inclusive upper bound, when present.
        max: Option<f64>,
        /// Further axis ranges; the layer applies where every range holds.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<BracketCondition>>"))]
        extra: Vec<BracketCondition>,
    },
    /// Background layer associated with a master.
    Background(MasterId),
    /// Timestamped backup of a master layer.
    Backup {
        /// Master whose content was backed up.
        of: MasterId,
        /// Unix timestamp of the backup.
        time: i64,
    },
    /// Application- or format-defined layer kind.
    Custom(String),
}

/// One axis range of an alternate layer's condition set.
///
/// A condition set is a conjunction: the layer applies where every one of its
/// ranges holds, which is the OpenType `ConditionSet` model and the meaning of
/// the Glyphs `[80‹wght, 25‹wdth‹50]` notation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BracketCondition {
    /// Axis this range constrains.
    pub axis: Tag,
    /// Inclusive lower bound in design units, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<f64>,
    /// Inclusive upper bound in design units, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<f64>,
}

impl BracketCondition {
    /// Whether `value` design units fall inside this inclusive range.
    #[must_use]
    pub fn contains(&self, value: f64) -> bool {
        self.min.is_none_or(|min| value >= min) && self.max.is_none_or(|max| value <= max)
    }

    /// Whether the two ranges share at least one design coordinate.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min.unwrap_or(f64::NEG_INFINITY) <= other.max.unwrap_or(f64::INFINITY)
            && other.min.unwrap_or(f64::NEG_INFINITY) <= self.max.unwrap_or(f64::INFINITY)
    }
}

impl LayerKind {
    /// An alternate layer's complete condition set, in axis-tag order.
    ///
    /// The set is the canonical identity of a conditional region: two alternate
    /// layers belong to the same region exactly when their sets are equal, which
    /// is how repair, expansion and interpolation group them.
    #[must_use]
    pub fn bracket_conditions(&self) -> Option<Vec<BracketCondition>> {
        let Self::Bracket {
            axis,
            min,
            max,
            extra,
            ..
        } = self
        else {
            return None;
        };
        let mut conditions = Vec::with_capacity(extra.len() + 1);
        conditions.push(BracketCondition {
            axis: *axis,
            min: *min,
            max: *max,
        });
        conditions.extend(extra.iter().copied());
        conditions.sort_by_key(|condition| condition.axis);
        Some(conditions)
    }

    /// Build an alternate-layer role from a complete condition set.
    ///
    /// # Errors
    ///
    /// Returns an error when the set is empty, repeats an axis, or contains a
    /// range with neither bound.
    pub fn from_bracket_conditions(
        master: MasterId,
        conditions: &[BracketCondition],
    ) -> Result<Self, crate::Error> {
        let invalid = |reason: &str| crate::Error::InvalidDesignSpace(reason.to_owned());
        let mut sorted = conditions.to_vec();
        sorted.sort_by_key(|condition| condition.axis);
        let Some((first, extra)) = sorted.split_first() else {
            return Err(invalid("an alternate layer needs at least one axis range"));
        };
        if sorted.windows(2).any(|pair| pair[0].axis == pair[1].axis) {
            return Err(invalid(
                "an alternate layer condition set names an axis twice",
            ));
        }
        if sorted
            .iter()
            .all(|condition| condition.min.is_none() && condition.max.is_none())
        {
            return Err(invalid(
                "an alternate layer condition set needs at least one bound",
            ));
        }
        Ok(Self::Bracket {
            master,
            axis: first.axis,
            min: first.min,
            max: first.max,
            extra: extra.to_vec(),
        })
    }
}

impl Default for LayerKind {
    fn default() -> Self {
        Self::Custom("default".to_owned())
    }
}

/// A source image placed behind an outline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Image {
    /// Prevent placement, replacement and removal until explicitly unlocked.
    #[serde(default)]
    pub locked: bool,
    /// Original or logical file name.
    #[serde(default)]
    pub file_name: String,
    /// Placement transform.
    #[serde(default = "default_affine")]
    #[cfg_attr(
        feature = "ts",
        ts(type = "[number, number, number, number, number, number]")
    )]
    pub transform: Affine,
    /// Optional serialized tint color.
    #[serde(default)]
    pub color: Option<String>,
    /// Optional embedded PNG bytes.
    #[serde(default)]
    pub data: Option<Vec<u8>>,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            locked: false,
            file_name: String::new(),
            transform: Affine::IDENTITY,
            color: None,
            data: None,
        }
    }
}

/// One horizontal or vertical PostScript stem hint.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StemHint {
    /// Stem start coordinate.
    #[serde(default)]
    pub start: f64,
    /// Stem width; `-20` and `-21` retain Type 1 ghost hints.
    #[serde(default)]
    pub width: f64,
    /// Active coordinate ranges.
    #[serde(default)]
    pub ranges: Vec<(f64, f64)>,
}

/// A diagonal PostScript stem hint.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DiagonalHint {
    /// Point on the left edge.
    #[serde(default)]
    pub left: (f64, f64),
    /// Point on the right edge.
    #[serde(default)]
    pub right: (f64, f64),
    /// Unit vector along the stem.
    #[serde(default)]
    pub unit: (f64, f64),
}

/// A PostScript hint mask at a contour point.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HintMask {
    /// Counter-control mask (`cntrmask`) rather than an active hint mask.
    #[serde(default)]
    pub counter: bool,
    /// Activate before the closing segment ending at the first on-curve point.
    #[serde(default)]
    pub closing: bool,
    /// Contour index.
    #[serde(default)]
    pub contour: usize,
    /// Point index within the contour.
    #[serde(default)]
    pub point: usize,
    /// Packed active-hint bits.
    #[serde(default)]
    pub mask: Vec<u8>,
}

/// PostScript hints attached to one layer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PsHints {
    /// Horizontal stem hints.
    #[serde(default)]
    pub h_stems: Vec<StemHint>,
    /// Vertical stem hints.
    #[serde(default)]
    pub v_stems: Vec<StemHint>,
    /// Diagonal stem hints.
    #[serde(default)]
    pub d_stems: Vec<DiagonalHint>,
    /// Whether the layer contains a flex hint.
    #[serde(default)]
    pub flex: bool,
    /// Hint substitution masks.
    #[serde(default)]
    pub hint_masks: Vec<HintMask>,
}

impl PsHints {
    /// Whether the layer contains no PostScript hint metadata.
    pub fn is_empty(&self) -> bool {
        self.h_stems.is_empty()
            && self.v_stems.is_empty()
            && self.d_stems.is_empty()
            && !self.flex
            && self.hint_masks.is_empty()
    }
}

/// One explicit color outline or paint entry.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ColorLayer {
    /// CPAL palette entry, when this is a simple solid layer.
    #[serde(default)]
    pub palette_index: Option<u16>,
    /// Color-layer contours.
    #[serde(default)]
    pub contours: Vec<Contour>,
    /// Color-layer components.
    #[serde(default)]
    pub components: Vec<Component>,
    /// Opaque COLR v1 paint graph data.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub paint: Option<serde_json::Value>,
}

/// Editable content for one glyph layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Layer {
    /// Persistent layer identifier; deserialization mints a fresh ID when omitted.
    #[serde(default)]
    pub id: LayerId,
    /// Semantic layer role.
    pub kind: LayerKind,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Horizontal advance width.
    #[serde(default)]
    pub width: f64,
    /// Layer-local metrics formulas; missing fields inherit the glyph formulas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub metrics_keys: Option<crate::MetricsKeys>,
    /// Optional vertical advance width.
    #[serde(default)]
    pub vertical_width: Option<f64>,
    /// Optional vertical origin.
    #[serde(default)]
    pub vertical_origin: Option<f64>,
    /// Point-list contours.
    #[serde(default)]
    pub contours: Vec<Contour>,
    /// Pure component references.
    #[serde(default)]
    pub components: Vec<Component>,
    /// Ordered effects applied to resolved source geometry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::LayerEffect>>", optional))]
    pub effects: Vec<crate::LayerEffect>,
    /// Reusable glyph drawings attached to persistent carrier nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::GlyphConstruction>>", optional)
    )]
    pub constructions: Vec<crate::GlyphConstruction>,
    /// Named anchors.
    #[serde(default)]
    pub anchors: Vec<Anchor>,
    /// Layer-local guides.
    #[serde(default)]
    pub guides: Vec<Guide>,
    /// Axis-interpolated nodes and their neighboring handles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::NodeLink>>", optional))]
    pub node_links: Vec<crate::NodeLink>,
    /// Guide-owned node positions, stored as signed normal offsets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::NodeLinkedGuide>>", optional)
    )]
    pub linked_guides: Vec<crate::NodeLinkedGuide>,
    /// Optional source image.
    #[serde(default)]
    pub background_image: Option<Image>,
    /// PostScript hints.
    #[serde(default)]
    pub hints: PsHints,
    /// CFF1 CID Font DICT selection; absent or zero selects the master's `private_dict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cff_private_dict_index: Option<u8>,
    /// Raw per-glyph TrueType instructions.
    #[serde(default)]
    pub tt_instructions: Option<Vec<u8>>,
    /// Compiled source point identity; retained when an outline edit makes it stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tt_binding: Option<crate::TtInstructionBinding>,
    /// Optional color outlines or paint graphs.
    #[serde(default)]
    pub color_layers: Option<Vec<ColorLayer>>,
    /// Format-specific layer metadata.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub user_data: serde_json::Value,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            id: LayerId::new(),
            kind: LayerKind::default(),
            name: String::new(),
            width: 0.0,
            metrics_keys: None,
            vertical_width: None,
            vertical_origin: None,
            contours: Vec::new(),
            components: Vec::new(),
            effects: Vec::new(),
            constructions: Vec::new(),
            anchors: Vec::new(),
            guides: Vec::new(),
            node_links: Vec::new(),
            linked_guides: Vec::new(),
            background_image: None,
            hints: PsHints::default(),
            cff_private_dict_index: None,
            tt_instructions: None,
            tt_binding: None,
            color_layers: None,
            user_data: serde_json::Value::Null,
        }
    }
}

impl Layer {
    /// Creates an empty layer for a master.
    pub fn new_master(master: MasterId) -> Self {
        Self {
            kind: LayerKind::Master(master),
            ..Self::default()
        }
    }

    /// Returns whether the layer has no editable drawing, attachment, or hint content.
    pub fn is_empty(&self) -> bool {
        self.contours.is_empty()
            && self.components.is_empty()
            && self.effects.is_empty()
            && self.constructions.is_empty()
            && self.anchors.is_empty()
            && self.guides.is_empty()
            && self.node_links.is_empty()
            && self.linked_guides.is_empty()
            && self.background_image.is_none()
            && self.hints.is_empty()
            && self.cff_private_dict_index.unwrap_or(0) == 0
            && self
                .tt_instructions
                .as_ref()
                .is_none_or(|instructions| instructions.is_empty())
            && self
                .color_layers
                .as_ref()
                .is_none_or(|layers| layers.is_empty())
    }
}

fn default_affine() -> Affine {
    Affine::IDENTITY
}
