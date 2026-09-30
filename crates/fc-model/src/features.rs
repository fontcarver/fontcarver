//! Editable OpenType feature source.

use std::{collections::HashSet, fmt};

use serde::{Deserialize, Deserializer, Serialize};

mod assembly;
mod generators;
mod numbers;
mod predicates;
mod preprocess;

pub use assembly::{AssembledFea, FeaSourceLocation, FeaSourceMapping, FeatureSourceKind};
pub use generators::{
    aalt_feature_order, automatic_class_registry, family_kind, family_names, generate_family,
    GeneratedFamily, GeneratedItemKind, GeneratedRules, GenerationIssue, GENERATED_NAME_PREFIX,
};
pub use preprocess::FeatureTokenError;

/// Persistent identity of a prefix, glyph class or feature block.
///
/// Clone preserves identity for document snapshots. Creating a new source item
/// uses a fresh ID; tags, names, text and source ordering do not define identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureSourceId(pub String);

impl Default for FeatureSourceId {
    fn default() -> Self {
        Self(uuid::Uuid::new_v4().simple().to_string())
    }
}

impl fmt::Display for FeatureSourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

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
    /// Stable source item identity, generated for documents saved without IDs.
    #[serde(default)]
    pub id: FeatureSourceId,
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
    /// Stable source item identity, generated for documents saved without IDs.
    #[serde(default)]
    pub id: FeatureSourceId,
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
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

impl Features {
    /// Assemble the effective FEA used by compilation and source queries.
    ///
    /// Imported-table authority emits no FEA. Disabled prefixes/features are
    /// omitted; `automatic` is metadata and does not suppress stored source.
    /// This preserves the existing source spelling without parsing or loading
    /// includes. Callers must validate the result before interpreting it.
    pub fn to_fea(&self) -> String {
        self.assemble_fea().text
    }
}

impl<'de> Deserialize<'de> for Features {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct StoredFeatures {
            prefixes: Vec<FeatureBlock>,
            classes: Vec<FeatureClass>,
            features: Vec<FeatureBlock>,
            source_of_truth: FeaturesSource,
        }

        let stored = StoredFeatures::deserialize(deserializer)?;
        let mut result = Self {
            prefixes: stored.prefixes,
            classes: stored.classes,
            features: stored.features,
            source_of_truth: stored.source_of_truth,
        };
        // Before P2b the UFO importer stored an entire file in this one synthetic
        // feature block. Restrict migration to its exact shape so moving it to
        // prefixes cannot reorder source relative to user-authored blocks.
        if result.source_of_truth == FeaturesSource::Fea
            && result.prefixes.is_empty()
            && result.classes.is_empty()
            && matches!(result.features.as_slice(), [block]
                if block.tag == "fea" && !block.automatic && block.notes.is_empty())
        {
            result.prefixes = std::mem::take(&mut result.features);
        }

        // Preserve the first occurrence of every stored identity. Empty and
        // duplicate IDs must not make diagnostics refer to a different item.
        let mut seen = HashSet::new();
        for id in result
            .prefixes
            .iter_mut()
            .map(|block| &mut block.id)
            .chain(result.classes.iter_mut().map(|class| &mut class.id))
            .chain(result.features.iter_mut().map(|block| &mut block.id))
        {
            while id.0.is_empty() || !seen.insert(id.clone()) {
                *id = FeatureSourceId::default();
            }
        }
        Ok(result)
    }
}
