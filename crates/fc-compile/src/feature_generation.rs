//! Deterministic opt-in source generation; compilation never regenerates text.

use std::collections::{BTreeMap, BTreeSet};

use fc_model::{
    FeatureBlock, FeatureGenerationSettings, FeatureGenerator, FeatureSourceId, Features,
    FeaturesSource, Font, GeneratedFeatureRecord, GlyphName,
};
use fea_rs::typed::AstNode;
use sha2::{Digest, Sha256};

use crate::{
    feature_validation::parse_source, FeatureDiagnostic, FeatureDiagnosticSeverity,
    FeatureValidation,
};

/// Materialized source and matching generator provenance, ready for validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFeatureSource {
    /// Candidate source, preserving every user-owned item.
    pub source: Features,
    /// Candidate settings recording materialized IDs and glyph fingerprint.
    pub settings: FeatureGenerationSettings,
    /// Generation errors; callers must not activate a candidate containing any.
    pub diagnostics: Vec<FeatureDiagnostic>,
}

#[derive(Default)]
struct Rules {
    code: Vec<String>,
    glyphs: BTreeSet<GlyphName>,
}

/// Whether exported glyph names changed since explicit materialization.
pub fn generation_is_stale(font: &Font, settings: &FeatureGenerationSettings) -> bool {
    has_generators(settings)
        && settings.glyph_fingerprint.as_ref() != Some(&glyph_fingerprint(font))
}

/// Generate a materialized candidate without changing active source or the font.
///
/// Stored legacy `automatic` flags never enable generation. Only source IDs in
/// server-maintained provenance are replaced. Frozen and user-authored items
/// are retained, and a feature-tag collision is an explicit error.
pub fn generate_feature_source(
    font: &Font,
    source: &Features,
    settings: &FeatureGenerationSettings,
) -> GeneratedFeatureSource {
    let mut result = GeneratedFeatureSource {
        source: source.clone(),
        settings: settings.clone(),
        diagnostics: Vec::new(),
    };
    if source.source_of_truth != FeaturesSource::Fea {
        result.diagnostics.push(FeatureDiagnostic::error(
            "feature-generation-imported",
            "Imported layout must be converted before generating editable source.",
        ));
        return result;
    }
    let owned: BTreeSet<&str> = settings
        .generated
        .iter()
        .map(|r| r.source_id.0.as_str())
        .collect();
    result
        .source
        .features
        .retain(|block| !owned.contains(block.id.0.as_str()));
    result.settings.generated.clear();
    result.settings.glyph_fingerprint = has_generators(settings).then(|| glyph_fingerprint(font));
    let assembled = result.source.assemble_fea();
    let mut parsing = FeatureValidation::default();
    let parsed = parse_source(font, &assembled, &mut parsing);
    let manual_tags: BTreeSet<String> = parsed
        .as_ref()
        .filter(|_| {
            !parsing
                .diagnostics
                .iter()
                .any(|d| d.severity == FeatureDiagnosticSeverity::Error)
        })
        .map(|(tree, _)| {
            tree.typed_root()
                .statements()
                .filter_map(fea_rs::typed::Feature::cast)
                .map(|f| f.tag().to_raw().to_string())
                .collect()
        })
        .unwrap_or_default();
    result.diagnostics.extend(parsing.diagnostics);
    if result
        .diagnostics
        .iter()
        .any(|d| d.severity == FeatureDiagnosticSeverity::Error)
    {
        return result;
    }
    let names: BTreeSet<GlyphName> = font
        .glyphs
        .values()
        .filter(|g| g.export)
        .map(|g| g.name.clone())
        .collect();
    for (generator, enabled) in [
        (FeatureGenerator::SmallCaps, settings.small_caps),
        (FeatureGenerator::StylisticSets, settings.stylistic_sets),
        (FeatureGenerator::CaseForms, settings.case_forms),
        (FeatureGenerator::VerticalForms, settings.vertical_forms),
        (FeatureGenerator::Ligatures, settings.ligatures),
        (FeatureGenerator::Localized, !settings.localized.is_empty()),
        (
            FeatureGenerator::DiscretionaryLigatures,
            !settings.discretionary_ligatures.is_empty(),
        ),
    ] {
        if !enabled {
            continue;
        }
        let expected_tags = generator_tags(generator);
        if let Some(tag) = expected_tags.iter().find(|tag| manual_tags.contains(*tag)) {
            result.error("feature-generation-conflict", format!("The user-owned {tag} feature conflicts with its enabled generator. Freeze or disable generation before editing this feature."));
            continue;
        }
        let mut by_tag = BTreeMap::<String, Rules>::new();
        match generator {
            FeatureGenerator::SmallCaps => {
                suffix_rules(&names, ".sc", &["smcp"], &mut by_tag, &mut result)
            }
            FeatureGenerator::CaseForms => {
                suffix_rules(&names, ".case", &["case"], &mut by_tag, &mut result)
            }
            FeatureGenerator::VerticalForms => {
                suffix_rules(&names, ".vert", &["vert", "vrt2"], &mut by_tag, &mut result)
            }
            FeatureGenerator::StylisticSets => {
                for index in 1..=20 {
                    let tag = format!("ss{index:02}");
                    suffix_rules(
                        &names,
                        &format!(".{tag}"),
                        &[&tag],
                        &mut by_tag,
                        &mut result,
                    );
                }
            }
            FeatureGenerator::Ligatures => underscore_rules(&names, &mut by_tag, &mut result),
            FeatureGenerator::Localized => {
                localized_rules(&names, settings, &mut by_tag, &mut result)
            }
            FeatureGenerator::DiscretionaryLigatures => {
                discretionary_rules(&names, settings, &mut by_tag, &mut result)
            }
        }
        for (tag, rules) in by_tag {
            if rules.code.is_empty() {
                continue;
            }
            let old = settings.generated.iter().find(|record| {
                record.generator == generator
                    && source
                        .features
                        .iter()
                        .any(|block| block.id == record.source_id && block.tag == tag)
            });
            let id = old
                .map(|record| record.source_id.clone())
                .unwrap_or_else(|| FeatureSourceId(format!("fontcarver-generated-{tag}")));
            if result
                .source
                .prefixes
                .iter()
                .chain(result.source.features.iter())
                .any(|b| b.id == id)
                || result.source.classes.iter().any(|c| c.id == id)
            {
                result.error("feature-generation-identity-conflict",format!("Generated source identity for {tag} is already used by a user source item."));
                continue;
            }
            result.source.features.push(FeatureBlock {
                id: id.clone(),
                tag,
                code: rules.code.join("\n"),
                automatic: true,
                ..FeatureBlock::default()
            });
            result.settings.generated.push(GeneratedFeatureRecord {
                generator,
                source_id: id,
                glyphs: rules.glyphs.into_iter().collect(),
            });
        }
    }
    result
}

/// Freeze exactly the visible materialized snapshot and disable its recipe.
///
/// This does not generate or compile. Callers persist the returned candidate as
/// one undoable draft mutation, then use guarded activation as for any edit.
pub fn freeze_feature_source(
    source: &Features,
    settings: &FeatureGenerationSettings,
    generator: FeatureGenerator,
) -> GeneratedFeatureSource {
    let mut result = GeneratedFeatureSource {
        source: source.clone(),
        settings: settings.clone(),
        diagnostics: Vec::new(),
    };
    if source.source_of_truth != FeaturesSource::Fea {
        result.error(
            "feature-generation-imported",
            "Imported layout must be converted before freezing editable source.",
        );
        return result;
    }
    let owned: BTreeSet<&str> = settings
        .generated
        .iter()
        .filter(|r| r.generator == generator)
        .map(|r| r.source_id.0.as_str())
        .collect();
    for block in &mut result.source.features {
        if owned.contains(block.id.0.as_str()) {
            block.automatic = false;
        }
    }
    result
        .settings
        .generated
        .retain(|r| r.generator != generator);
    match generator {
        FeatureGenerator::SmallCaps => result.settings.small_caps = false,
        FeatureGenerator::StylisticSets => result.settings.stylistic_sets = false,
        FeatureGenerator::CaseForms => result.settings.case_forms = false,
        FeatureGenerator::VerticalForms => result.settings.vertical_forms = false,
        FeatureGenerator::Ligatures => result.settings.ligatures = false,
        FeatureGenerator::Localized => result.settings.localized.clear(),
        FeatureGenerator::DiscretionaryLigatures => result.settings.discretionary_ligatures.clear(),
    }
    if !has_generators(&result.settings) {
        result.settings.glyph_fingerprint = None;
    }
    result
}

impl GeneratedFeatureSource {
    fn error(&mut self, code: &str, message: impl Into<String>) {
        if self.diagnostics.len() < 99 {
            self.diagnostics
                .push(FeatureDiagnostic::error(code, message));
        } else if self.diagnostics.len() == 99 {
            self.diagnostics.push(FeatureDiagnostic::error(
                "feature-generation-diagnostics-limit",
                "Additional generation diagnostics were omitted.",
            ));
        }
    }
}

fn has_generators(settings: &FeatureGenerationSettings) -> bool {
    settings.small_caps
        || settings.stylistic_sets
        || settings.case_forms
        || settings.vertical_forms
        || settings.ligatures
        || !settings.localized.is_empty()
        || !settings.discretionary_ligatures.is_empty()
}

fn glyph_fingerprint(font: &Font) -> String {
    let mut hash = Sha256::new();
    let names: BTreeSet<_> = font
        .glyphs
        .values()
        .filter(|g| g.export)
        .map(|g| g.name.as_ref())
        .collect();
    for name in names {
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn generator_tags(generator: FeatureGenerator) -> Vec<String> {
    match generator {
        FeatureGenerator::SmallCaps => vec!["smcp".into()],
        FeatureGenerator::StylisticSets => (1..=20).map(|i| format!("ss{i:02}")).collect(),
        FeatureGenerator::CaseForms => vec!["case".into()],
        FeatureGenerator::VerticalForms => vec!["vert".into(), "vrt2".into()],
        FeatureGenerator::Ligatures => vec!["liga".into()],
        FeatureGenerator::Localized => vec!["locl".into()],
        FeatureGenerator::DiscretionaryLigatures => vec!["dlig".into()],
    }
}

fn glyph_token(name: &GlyphName, result: &mut GeneratedFeatureSource) -> Option<String> {
    // Escaping protects keywords such as `sub`; numeric-leading names would be
    // parsed as CIDs after a backslash and therefore cannot be safely emitted.
    let mut chars = name.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '_' | '.'))
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        result.error(
            "feature-generation-glyph-name",
            format!(
                "Glyph name {name:?} cannot be represented safely in generated feature syntax."
            ),
        );
        return None;
    }
    Some(format!("\\{name}"))
}

fn substitution_rule(
    from: &GlyphName,
    to: &GlyphName,
    names: &BTreeSet<GlyphName>,
    result: &mut GeneratedFeatureSource,
) -> Option<String> {
    if !names.contains(from) || !names.contains(to) {
        result.error(
            "feature-generation-missing-base",
            format!("Both glyphs must exist and export for substitution {from} → {to}."),
        );
        return None;
    }
    if from == to {
        result.error(
            "feature-generation-duplicate",
            format!("Substitution {from} → {to} has identical input and output."),
        );
        return None;
    }
    Some(format!(
        "sub {} by {};",
        glyph_token(from, result)?,
        glyph_token(to, result)?
    ))
}

fn suffix_rules(
    names: &BTreeSet<GlyphName>,
    suffix: &str,
    tags: &[&str],
    by_tag: &mut BTreeMap<String, Rules>,
    result: &mut GeneratedFeatureSource,
) {
    for target in names {
        let Some(base) = target.strip_suffix(suffix) else {
            continue;
        };
        let base = GlyphName::from(base);
        let Some(code) = substitution_rule(&base, target, names, result) else {
            continue;
        };
        for tag in tags {
            let rules = by_tag.entry((*tag).into()).or_default();
            rules.code.push(code.clone());
            rules.glyphs.extend([base.clone(), target.clone()]);
        }
    }
}

fn underscore_rules(
    names: &BTreeSet<GlyphName>,
    by_tag: &mut BTreeMap<String, Rules>,
    result: &mut GeneratedFeatureSource,
) {
    for target in names.iter().filter(|name| name.contains('_')) {
        let parts: Vec<_> = target.split('_').collect();
        if parts.len() > 32 {
            result.error(
                "feature-generation-ambiguous",
                format!("Ligature {target} exceeds the bounded component limit."),
            );
            continue;
        }
        let mut paths = vec![Vec::<Vec<GlyphName>>::new(); parts.len() + 1];
        paths[0].push(Vec::new());
        for end in 1..=parts.len() {
            for start in 0..end {
                if start == 0 && end == parts.len() {
                    continue;
                }
                let piece = GlyphName::from(parts[start..end].join("_"));
                if !names.contains(&piece) {
                    continue;
                }
                for prefix in paths[start].clone() {
                    if paths[end].len() >= 2 {
                        break;
                    }
                    let mut path = prefix;
                    path.push(piece.clone());
                    paths[end].push(path);
                }
            }
        }
        match paths[parts.len()].as_slice() {
            [components] => add_ligature(components,target,"liga",names,by_tag,result),
            []=>result.error("feature-generation-missing-base",format!("Ligature {target} has no complete sequence of exported component glyphs.")),
            _=>result.error("feature-generation-ambiguous",format!("Ligature {target} has more than one valid component sequence; configure it explicitly as a discretionary ligature.")),
        }
    }
}

fn add_ligature(
    components: &[GlyphName],
    target: &GlyphName,
    tag: &str,
    names: &BTreeSet<GlyphName>,
    by_tag: &mut BTreeMap<String, Rules>,
    result: &mut GeneratedFeatureSource,
) {
    if components.len() < 2
        || components.len() > 32
        || !names.contains(target)
        || components.iter().any(|name| !names.contains(name))
    {
        result.error(
            "feature-generation-missing-base",
            format!(
                "Ligature {target} requires 2–32 exported components and an exported replacement."
            ),
        );
        return;
    }
    let Some(tokens) = components
        .iter()
        .map(|g| glyph_token(g, result))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let Some(replacement) = glyph_token(target, result) else {
        return;
    };
    let rules = by_tag.entry(tag.into()).or_default();
    rules
        .code
        .push(format!("sub {} by {replacement};", tokens.join(" ")));
    rules.glyphs.extend(components.iter().cloned());
    rules.glyphs.insert(target.clone());
}

fn valid_tag(tag: &str, language: bool) -> bool {
    let len = tag.len();
    (len == 4 || (language && len == 3))
        && tag.bytes().all(|b| b.is_ascii_alphanumeric() || b == b' ')
}

fn localized_rules(
    names: &BTreeSet<GlyphName>,
    settings: &FeatureGenerationSettings,
    by_tag: &mut BTreeMap<String, Rules>,
    result: &mut GeneratedFeatureSource,
) {
    let mut mappings = BTreeMap::new();
    let mut targets = BTreeSet::new();
    for mapping in &settings.localized {
        if !valid_tag(&mapping.script, false) || !valid_tag(&mapping.language, true) {
            result.error(
                "feature-generation-language",
                format!(
                    "Invalid OpenType script/language tags {:?}/{:?}.",
                    mapping.script, mapping.language
                ),
            );
            continue;
        }
        let language = mapping.language.trim_end_matches(' ').to_owned();
        for pair in &mapping.substitutions {
            let key = (
                mapping.script.clone(),
                language != "dflt",
                language.clone(),
                pair.from.clone(),
            );
            if mappings.insert(key, pair.to.clone()).is_some()
                || !targets.insert((mapping.script.clone(), language.clone(), pair.to.clone()))
            {
                result.error(
                    "feature-generation-duplicate",
                    format!(
                        "Duplicate localized input or target for {}/{}.",
                        mapping.script, mapping.language
                    ),
                );
            }
        }
    }
    let mut context = None;
    for ((script, _, language, from), to) in mappings {
        let Some(code) = substitution_rule(&from, &to, names, result) else {
            continue;
        };
        let rules = by_tag.entry("locl".into()).or_default();
        if context.as_ref() != Some(&(script.clone(), language.clone())) {
            rules.code.push(language_context(&script, &language));
            context = Some((script, language));
        }
        rules.code.push(code);
        rules.glyphs.extend([from, to]);
    }
}

fn discretionary_rules(
    names: &BTreeSet<GlyphName>,
    settings: &FeatureGenerationSettings,
    by_tag: &mut BTreeMap<String, Rules>,
    result: &mut GeneratedFeatureSource,
) {
    let mut mappings = BTreeMap::new();
    let mut targets = BTreeSet::new();
    for mapping in &settings.discretionary_ligatures {
        if !valid_tag(&mapping.script, false) || !valid_tag(&mapping.language, true) {
            result.error(
                "feature-generation-language",
                format!(
                    "Invalid OpenType script/language tags {:?}/{:?}.",
                    mapping.script, mapping.language
                ),
            );
            continue;
        }
        let language = mapping.language.trim_end_matches(' ').to_owned();
        if mappings
            .insert(
                (
                    mapping.script.clone(),
                    language != "dflt",
                    language.clone(),
                    mapping.components.clone(),
                ),
                mapping.replacement.clone(),
            )
            .is_some()
            || !targets.insert((
                mapping.script.clone(),
                language.clone(),
                mapping.replacement.clone(),
            ))
        {
            result.error(
                "feature-generation-duplicate",
                "Duplicate discretionary ligature components or target.",
            );
        }
    }
    let mut context = None;
    for ((script, _, language, components), target) in mappings {
        let mut generated = BTreeMap::new();
        add_ligature(&components, &target, "dlig", names, &mut generated, result);
        let Some(generated) = generated.remove("dlig") else {
            continue;
        };
        let rules = by_tag.entry("dlig".into()).or_default();
        if context.as_ref() != Some(&(script.clone(), language.clone())) {
            rules.code.push(language_context(&script, &language));
            context = Some((script, language));
        }
        rules.code.extend(generated.code);
        rules.glyphs.extend(generated.glyphs);
    }
}

fn language_context(script: &str, language: &str) -> String {
    let policy = if language == "dflt" {
        ""
    } else {
        " exclude_dflt"
    };
    format!("script {script};\nlanguage {language}{policy};")
}
