//! Editable OpenType feature source.

use serde::{Deserialize, Serialize};

/// Authoritative representation for OpenType layout data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeaturesSource {
    /// Editable Adobe feature syntax.
    #[default]
    Fea,
    /// Tables imported without a canonical feature source.
    ImportedTables,
}

/// A feature-file prefix or tagged feature block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureBlock {
    /// Feature tag or prefix name.
    #[serde(default)]
    pub tag: String,
    /// Feature syntax source.
    #[serde(default)]
    pub code: String,
    /// Whether the block is generated automatically.
    #[serde(default)]
    pub automatic: bool,
    /// Whether the block is disabled.
    #[serde(default)]
    pub disabled: bool,
    /// Free-form editor notes.
    #[serde(default)]
    pub notes: String,
}

/// A named feature-file glyph class.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureClass {
    /// Class name without the leading at-sign.
    #[serde(default)]
    pub name: String,
    /// Class contents in feature syntax.
    #[serde(default)]
    pub code: String,
    /// Whether the class is generated automatically.
    #[serde(default)]
    pub automatic: bool,
}

/// All editable feature source associated with a font.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Features {
    /// Prefix blocks emitted before classes and features.
    #[serde(default)]
    pub prefixes: Vec<FeatureBlock>,
    /// Named glyph classes.
    #[serde(default)]
    pub classes: Vec<FeatureClass>,
    /// Tagged feature blocks.
    #[serde(default)]
    pub features: Vec<FeatureBlock>,
    /// Which representation is authoritative.
    #[serde(default)]
    pub source_of_truth: FeaturesSource,
}
