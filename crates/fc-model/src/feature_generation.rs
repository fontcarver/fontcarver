//! Explicit opt-in feature generation and source/model writer ownership.

use serde::{Deserialize, Serialize};

use crate::{FeatureSourceId, GlyphName};

/// One family of materialized name or mapping based feature source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeatureGenerator {
    /// `.sc` substitutions in `smcp`.
    SmallCaps,
    /// `.ss01` through `.ss20` substitutions.
    StylisticSets,
    /// `.case` substitutions.
    CaseForms,
    /// `.vert` substitutions in `vert` and `vrt2`.
    VerticalForms,
    /// Unambiguous underscore names in `liga`.
    Ligatures,
    /// Explicit script/language substitutions in `locl`.
    Localized,
    /// Explicit multi-glyph substitutions in `dlig`.
    DiscretionaryLigatures,
}

/// Whether live model data contributes kerning and anchor rules to compilation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureWriterPolicy {
    /// Permit the existing model horizontal kerning writer.
    pub kerning: bool,
    /// Permit the existing model mark/cursive anchor writers.
    pub anchors: bool,
}

impl Default for FeatureWriterPolicy {
    fn default() -> Self {
        Self {
            kerning: true,
            anchors: true,
        }
    }
}

/// An explicit single-glyph substitution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureSubstitution {
    /// Existing input glyph.
    pub from: GlyphName,
    /// Existing replacement glyph.
    pub to: GlyphName,
}

/// Explicit localized substitutions for one OpenType script and language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureLocalizedMapping {
    /// Four-character OpenType script tag, such as `latn`.
    pub script: String,
    /// OpenType language tag, such as `TRK` or `dflt`.
    pub language: String,
    /// Explicit substitutions; names are never inferred from language suffixes.
    pub substitutions: Vec<FeatureSubstitution>,
}

/// One explicitly configured discretionary ligature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureLigatureMapping {
    /// Four-character OpenType script tag; old mappings use `DFLT`.
    #[serde(default = "default_script")]
    pub script: String,
    /// OpenType language tag; old mappings use `dflt`.
    #[serde(default = "default_language")]
    pub language: String,
    /// Two or more input glyphs in shaping order.
    pub components: Vec<GlyphName>,
    /// Existing replacement glyph.
    pub replacement: GlyphName,
}

fn default_script() -> String {
    "DFLT".into()
}
fn default_language() -> String {
    "dflt".into()
}

/// Provenance of one materialized source item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeneratedFeatureRecord {
    /// Generator owning the source item until frozen.
    pub generator: FeatureGenerator,
    /// Persistent identity of the materialized feature block.
    pub source_id: FeatureSourceId,
    /// Glyphs responsible for these rules, in deterministic order.
    pub glyphs: Vec<GlyphName>,
}

/// Persisted generation settings; all additional recipes default to disabled.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureGenerationSettings {
    /// Generate `.sc` small capitals.
    pub small_caps: bool,
    /// Generate the standard twenty stylistic sets.
    pub stylistic_sets: bool,
    /// Generate `.case` substitutions.
    pub case_forms: bool,
    /// Generate `.vert` substitutions.
    pub vertical_forms: bool,
    /// Generate unambiguous underscore ligatures.
    pub ligatures: bool,
    /// Explicit localized substitutions; an empty list disables this generator.
    pub localized: Vec<FeatureLocalizedMapping>,
    /// Explicit discretionary ligatures; an empty list disables this generator.
    pub discretionary_ligatures: Vec<FeatureLigatureMapping>,
    /// Ownership of live kerning and anchor writers.
    pub writer_policy: FeatureWriterPolicy,
    /// Materialized source identities, independently of legacy `automatic` flags.
    pub generated: Vec<GeneratedFeatureRecord>,
    /// Exported glyph-name fingerprint at the last explicit materialization.
    pub glyph_fingerprint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{FeatureGenerationSettings, FeatureLigatureMapping};

    #[test]
    fn old_documents_enable_no_name_recipes_but_keep_model_writers() {
        let settings: FeatureGenerationSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, FeatureGenerationSettings::default());
        assert!(settings.writer_policy.kerning && settings.writer_policy.anchors);
        assert!(!settings.small_caps && !settings.stylistic_sets && !settings.ligatures);
        assert!(settings.generated.is_empty());
    }

    #[test]
    fn ligature_mapping_defaults_and_explicit_scope_survive_serialization() {
        let mapping: FeatureLigatureMapping =
            serde_json::from_str(r#"{"components":["f","i"],"replacement":"f_i"}"#).unwrap();
        assert_eq!((&*mapping.script, &*mapping.language), ("DFLT", "dflt"));
        let scoped = FeatureLigatureMapping {
            script: "latn".into(),
            language: "TRK".into(),
            ..mapping
        };
        assert_eq!(
            serde_json::from_str::<FeatureLigatureMapping>(
                &serde_json::to_string(&scoped).unwrap()
            )
            .unwrap(),
            scoped
        );
    }
}
