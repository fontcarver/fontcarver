//! Saved, possibly invalid editor source and imported-layout conversion history.

use serde::{Deserialize, Serialize};

use crate::{
    Anchor, FeatureGenerationSettings, Features, GlyphName, Kerning, KerningGroups, LayerId,
    MasterId,
};

/// An editable source snapshot that has not replaced active layout.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureDraft {
    /// Ordered source blocks; syntax errors are valid draft data.
    #[serde(default)]
    pub source: Features,
    /// Candidate generation and writer policy, inactive until validation.
    #[serde(default)]
    pub settings: FeatureGenerationSettings,
    /// Revision assigned by the document transaction that saved this draft.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub revision: u64,
}

/// Horizontal pairs for one master captured for conversion undo/revert.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureMasterKerning {
    /// Stable master identity.
    pub master: MasterId,
    /// Complete horizontal pair data; vertical pairs remain independent.
    pub kerning: Kerning,
}

/// Redundant per-glyph group identities captured with group definitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureGlyphMembership {
    /// Existing glyph name.
    pub glyph: GlyphName,
    /// First-side group, if assigned.
    pub side1: Option<String>,
    /// Second-side group, if assigned.
    pub side2: Option<String>,
}

/// Outline-independent horizontal model data changed by a layout conversion.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureKerningState {
    /// Masters whose horizontal pairs are replaced.
    #[serde(default)]
    pub masters: Vec<FeatureMasterKerning>,
    /// Font-wide group definitions.
    #[serde(default)]
    pub groups: KerningGroups,
    /// Per-glyph redundant membership fields that are replaced.
    #[serde(default)]
    pub memberships: Vec<FeatureGlyphMembership>,
}

/// Anchors changed by conversion on one existing layer, without its outline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureAnchorState {
    /// Existing glyph name.
    pub glyph: GlyphName,
    /// Stable layer identity.
    pub layer: LayerId,
    /// Full anchor list for the affected layer.
    #[serde(default)]
    pub anchors: Vec<Anchor>,
}

/// Native-document backup of only the model data affected by conversion.
///
/// Raw original tables remain in `Font.extra`; this is not a duplicate binary
/// or a caller-supplied validation proof. Runtime conversion tokens are never
/// serialized here.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayoutConversion {
    /// Original imported-table authority and any saved source metadata.
    #[serde(default)]
    pub original_features: Features,
    /// Generation policy before conversion.
    #[serde(default)]
    pub original_settings: FeatureGenerationSettings,
    /// Exact original glyph order required to restore imported tables.
    #[serde(default)]
    pub original_glyph_order: Vec<GlyphName>,
    /// SHA-256 baseline table/provenance fingerprint, checked before revert.
    #[serde(default)]
    pub baseline_fingerprint: String,
    /// Original kerning model data, present only if a lift changed it.
    #[serde(default)]
    pub lifted_kerning: Option<FeatureKerningState>,
    /// Original anchor lists for layers changed by a successful lift.
    #[serde(default)]
    pub lifted_anchors: Vec<FeatureAnchorState>,
}
