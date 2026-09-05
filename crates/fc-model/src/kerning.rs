//! Pair kerning and kerning-group data.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::GlyphName;

/// One side of a kerning pair.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum KernSide {
    /// A concrete glyph.
    Glyph(GlyphName),
    /// A named kerning group.
    Group(String),
}

impl Default for KernSide {
    fn default() -> Self {
        Self::Glyph(GlyphName::default())
    }
}

/// Serializable representation of one kerning pair.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KernPair {
    /// Left side of the pair.
    #[serde(default)]
    pub left: KernSide,
    /// Right side of the pair.
    #[serde(default)]
    pub right: KernSide,
    /// Adjustment in font units.
    #[serde(default)]
    pub value: f64,
}

/// Kerning pairs keyed by their left and right sides.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Kerning(
    #[cfg_attr(feature = "ts", ts(type = "KernPair[]"))] pub BTreeMap<(KernSide, KernSide), f64>,
);

impl Serialize for Kerning {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0
            .iter()
            .map(|((left, right), value)| KernPair {
                left: left.clone(),
                right: right.clone(),
                value: *value,
            })
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Kerning {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let pairs = Vec::<KernPair>::deserialize(deserializer)?;
        Ok(Self(
            pairs
                .into_iter()
                .map(|pair| ((pair.left, pair.right), pair.value))
                .collect(),
        ))
    }
}

/// Named kerning groups for each pair side.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KerningGroups {
    /// First-side group members.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, string[]>"))]
    pub side1: BTreeMap<String, BTreeSet<GlyphName>>,
    /// Second-side group members.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, string[]>"))]
    pub side2: BTreeMap<String, BTreeSet<GlyphName>>,
}
