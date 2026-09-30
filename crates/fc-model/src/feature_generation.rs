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
    /// Uppercase-to-small-capital substitutions in `c2sc`.
    CapsToSmallCaps,
    /// Figure-style substitutions in `lnum`, `onum`, `tnum` and `pnum`.
    FigureStyles,
    /// Positional figures in `sups`, `sinf`, `numr` and `dnom`.
    PositionalFigures,
    /// Contextual numerator/denominator rules in `frac`.
    Fractions,
    /// Ordinal substitutions after figures in `ordn`.
    Ordinals,
    /// Base-and-mark composition and dotless forms in `ccmp`.
    Composition,
    /// Alternate aggregation in `aalt`.
    AccessAllAlternates,
    /// Glyph classes derived from glyph metadata.
    Classes,
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
    /// Generate `c2sc` uppercase-to-small-capital substitutions.
    pub caps_to_small_caps: bool,
    /// Generate `lnum`, `onum`, `tnum` and `pnum` figure styles.
    pub figure_styles: bool,
    /// Generate `sups`, `sinf`, `numr` and `dnom` positional figures.
    pub positional_figures: bool,
    /// Generate contextual `frac` numerator and denominator rules.
    pub fractions: bool,
    /// Generate `ordn` ordinal substitutions after figures.
    pub ordinals: bool,
    /// Generate `ccmp` composition and dotless rules.
    pub composition: bool,
    /// Generate `aalt` references to the alternate features present.
    pub access_all_alternates: bool,
    /// Glyph class names generated from glyph metadata; empty disables them.
    pub automatic_classes: Vec<String>,
    /// Feature tags and class names held manual inside an enabled family.
    pub manual_entries: Vec<String>,
    /// Ownership of live kerning and anchor writers.
    pub writer_policy: FeatureWriterPolicy,
    /// Materialized source identities, independently of legacy `automatic` flags.
    pub generated: Vec<GeneratedFeatureRecord>,
    /// Exported glyph-name fingerprint at the last explicit materialization.
    pub glyph_fingerprint: Option<String>,
}

/// Every generator family, in the order candidates are materialized.
///
/// `AccessAllAlternates` is last so it can reference the tags the other
/// families just produced.
pub const FEATURE_GENERATORS: [FeatureGenerator; 15] = [
    FeatureGenerator::SmallCaps,
    FeatureGenerator::CapsToSmallCaps,
    FeatureGenerator::StylisticSets,
    FeatureGenerator::CaseForms,
    FeatureGenerator::VerticalForms,
    FeatureGenerator::Ligatures,
    FeatureGenerator::Localized,
    FeatureGenerator::DiscretionaryLigatures,
    FeatureGenerator::FigureStyles,
    FeatureGenerator::PositionalFigures,
    FeatureGenerator::Fractions,
    FeatureGenerator::Ordinals,
    FeatureGenerator::Composition,
    FeatureGenerator::Classes,
    FeatureGenerator::AccessAllAlternates,
];

impl FeatureGenerationSettings {
    /// Whether this family's recipe is switched on.
    pub fn family_enabled(&self, generator: FeatureGenerator) -> bool {
        match generator {
            FeatureGenerator::SmallCaps => self.small_caps,
            FeatureGenerator::StylisticSets => self.stylistic_sets,
            FeatureGenerator::CaseForms => self.case_forms,
            FeatureGenerator::VerticalForms => self.vertical_forms,
            FeatureGenerator::Ligatures => self.ligatures,
            FeatureGenerator::Localized => !self.localized.is_empty(),
            FeatureGenerator::DiscretionaryLigatures => !self.discretionary_ligatures.is_empty(),
            FeatureGenerator::CapsToSmallCaps => self.caps_to_small_caps,
            FeatureGenerator::FigureStyles => self.figure_styles,
            FeatureGenerator::PositionalFigures => self.positional_figures,
            FeatureGenerator::Fractions => self.fractions,
            FeatureGenerator::Ordinals => self.ordinals,
            FeatureGenerator::Composition => self.composition,
            FeatureGenerator::AccessAllAlternates => self.access_all_alternates,
            FeatureGenerator::Classes => !self.automatic_classes.is_empty(),
        }
    }

    /// Switch one family off, or on where a boolean alone decides it.
    ///
    /// The mapping-driven families (`locl`, `dlig` and automatic classes) are
    /// only ever cleared here; their content is the switch.
    pub fn set_family_enabled(&mut self, generator: FeatureGenerator, enabled: bool) {
        match generator {
            FeatureGenerator::SmallCaps => self.small_caps = enabled,
            FeatureGenerator::StylisticSets => self.stylistic_sets = enabled,
            FeatureGenerator::CaseForms => self.case_forms = enabled,
            FeatureGenerator::VerticalForms => self.vertical_forms = enabled,
            FeatureGenerator::Ligatures => self.ligatures = enabled,
            FeatureGenerator::CapsToSmallCaps => self.caps_to_small_caps = enabled,
            FeatureGenerator::FigureStyles => self.figure_styles = enabled,
            FeatureGenerator::PositionalFigures => self.positional_figures = enabled,
            FeatureGenerator::Fractions => self.fractions = enabled,
            FeatureGenerator::Ordinals => self.ordinals = enabled,
            FeatureGenerator::Composition => self.composition = enabled,
            FeatureGenerator::AccessAllAlternates => self.access_all_alternates = enabled,
            FeatureGenerator::Localized if !enabled => self.localized.clear(),
            FeatureGenerator::DiscretionaryLigatures if !enabled => {
                self.discretionary_ligatures.clear()
            }
            FeatureGenerator::Classes if !enabled => self.automatic_classes.clear(),
            FeatureGenerator::Localized
            | FeatureGenerator::DiscretionaryLigatures
            | FeatureGenerator::Classes => {}
        }
    }

    /// Whether this feature tag or class name is held manual inside its family.
    pub fn entry_is_manual(&self, name: &str) -> bool {
        self.manual_entries.iter().any(|entry| entry == name)
    }

    /// The family that can own a feature tag or glyph class name, if any.
    pub fn generator_for_entry(&self, name: &str) -> Option<FeatureGenerator> {
        FEATURE_GENERATORS.into_iter().find(|generator| {
            crate::features::family_names(*generator, self)
                .iter()
                .any(|candidate| candidate == name)
                || (*generator == FeatureGenerator::Classes
                    && crate::features::automatic_class_registry().contains(&name))
        })
    }
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
    fn p5_recipe_switches_default_off_and_survive_a_serialization_round_trip() {
        use super::{FeatureGenerationSettings, FEATURE_GENERATORS};
        // A P2b-era document has none of the new fields.
        let old: FeatureGenerationSettings = serde_json::from_str(
            r#"{"small_caps":true,"writer_policy":{"kerning":true,"anchors":false}}"#,
        )
        .unwrap();
        assert!(old.small_caps && !old.writer_policy.anchors);
        assert!(old.automatic_classes.is_empty() && old.manual_entries.is_empty());
        for generator in FEATURE_GENERATORS {
            assert_eq!(
                old.family_enabled(generator),
                generator == crate::FeatureGenerator::SmallCaps,
                "{generator:?}"
            );
        }
        // Every new switch round trips, including the two list-driven ones.
        let mut settings = FeatureGenerationSettings::default();
        for generator in FEATURE_GENERATORS {
            settings.set_family_enabled(generator, true);
        }
        settings.automatic_classes = vec!["Uppercase".into(), "Figures".into()];
        settings.manual_entries = vec!["ss03".into(), "Figures".into()];
        let text = serde_json::to_string(&settings).unwrap();
        assert_eq!(
            serde_json::from_str::<FeatureGenerationSettings>(&text).unwrap(),
            settings
        );
        for generator in FEATURE_GENERATORS {
            let mapping_driven = matches!(
                generator,
                crate::FeatureGenerator::Localized
                    | crate::FeatureGenerator::DiscretionaryLigatures
            );
            assert_eq!(
                settings.family_enabled(generator),
                !mapping_driven,
                "{generator:?}"
            );
        }
        assert!(settings.entry_is_manual("ss03") && !settings.entry_is_manual("ss04"));
        assert_eq!(
            settings.generator_for_entry("ss03"),
            Some(crate::FeatureGenerator::StylisticSets)
        );
        assert_eq!(
            settings.generator_for_entry("Uppercase"),
            Some(crate::FeatureGenerator::Classes)
        );
        assert_eq!(settings.generator_for_entry("zzzz"), None);
        // Turning a family off again clears its list, not another family's.
        settings.set_family_enabled(crate::FeatureGenerator::Classes, false);
        assert!(settings.automatic_classes.is_empty());
        assert!(settings.small_caps);
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
