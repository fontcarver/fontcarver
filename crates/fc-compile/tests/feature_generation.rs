mod common;

use fc_compile::{
    freeze_feature_source, generate_feature_source, generation_is_stale, validate_feature_candidate,
};
use fc_model::{
    FeatureBlock, FeatureGenerationSettings, FeatureGenerator, FeatureLigatureMapping,
    FeatureLocalizedMapping, FeatureSubstitution, Glyph, Layer,
};

fn font_with(names: &[&str]) -> fc_model::Font {
    let mut font = common::scratch_font();
    let master = font.masters[0].id.clone();
    for name in names {
        let mut glyph = Glyph::new(*name);
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    font
}

#[test]
fn name_generation_is_deterministic_materialized_and_opt_in() {
    let font = font_with(&["A.sc", "A.ss01", "A.case", "A.vert", "A_V"]);
    let settings = FeatureGenerationSettings {
        small_caps: true,
        stylistic_sets: true,
        case_forms: true,
        vertical_forms: true,
        ligatures: true,
        ..Default::default()
    };
    let generated = generate_feature_source(&font, &font.features, &settings);
    assert!(
        generated.diagnostics.is_empty(),
        "{:?}",
        generated.diagnostics
    );
    assert_eq!(generated.source.features.len(), 6);
    let again = generate_feature_source(&font, &generated.source, &generated.settings);
    assert_eq!(generated, again);
    assert!(!generation_is_stale(&font, &generated.settings));
    assert!(validate_feature_candidate(&font, &generated.source, &generated.settings).is_valid());
    assert!(
        generate_feature_source(&font, &font.features, &FeatureGenerationSettings::default())
            .source
            .features
            .is_empty()
    );
}

#[test]
fn generated_vertical_alternates_shape_by_default_and_toggle_independently() {
    let mut font = font_with(&["A.vert"]);
    font.glyph_mut("A").unwrap().unicodes = vec![0x3001];
    font.masters[0].metrics.default_vertical_advance = Some(1000.0);
    let settings = FeatureGenerationSettings {
        vertical_forms: true,
        ..Default::default()
    };
    let generated = generate_feature_source(&font, &font.features, &settings);
    assert!(
        generated.diagnostics.is_empty(),
        "{:?}",
        generated.diagnostics
    );
    font.features = generated.source;
    font.feature_settings = generated.settings;
    for format in [fc_compile::ExportFormat::Ttf, fc_compile::ExportFormat::Otf] {
        let report = fc_compile::export(
            &font,
            &|_| 0,
            &fc_compile::ExportOptions {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        let face = read_fonts::FontRef::new(&report.bytes).unwrap();
        let names = skrifa::GlyphNames::new(&face);
        let alternate = names
            .iter()
            .find(|(_, name)| name.as_str() == "A.vert")
            .expect("unencoded vertical alternate")
            .0
            .to_u32();
        let base = skrifa::charmap::Charmap::new(&face)
            .map('\u{3001}')
            .unwrap()
            .to_u32();
        let data = harfrust::ShaperData::new(&face);
        let shaper = data.shaper(&face).build();
        let shape = |direction, features: &[&str]| {
            let features: Vec<harfrust::Feature> = features
                .iter()
                .map(|feature| feature.parse().unwrap())
                .collect();
            let mut buffer = harfrust::UnicodeBuffer::new();
            buffer.push_str("\u{3001}");
            buffer.set_direction(direction);
            buffer.set_script(harfrust::script::HAN);
            buffer.guess_segment_properties();
            shaper
                .shape(buffer, harfrust::ShapeOptions::new().features(&features))
                .glyph_infos()
                .iter()
                .map(|glyph| glyph.glyph_id)
                .collect::<Vec<_>>()
        };
        assert_eq!(shape(harfrust::Direction::LeftToRight, &[]), [base]);
        assert_eq!(shape(harfrust::Direction::TopToBottom, &[]), [alternate]);
        assert_eq!(
            shape(harfrust::Direction::TopToBottom, &["vert=0", "vrt2=0"]),
            [base],
            "{format:?} disabled vertical alternates"
        );
        for features in [["vert=1", "vrt2=0"], ["vert=0", "vrt2=1"]] {
            assert_eq!(
                shape(harfrust::Direction::TopToBottom, &features),
                [alternate],
                "{format:?} {features:?}"
            );
        }
    }
}

#[test]
fn vertical_generation_preserves_manual_features_and_frozen_source() {
    let mut font = font_with(&["A.vert"]);
    let settings = FeatureGenerationSettings {
        vertical_forms: true,
        ..Default::default()
    };
    for tag in ["vert", "vrt2"] {
        let mut source = font.features.clone();
        source.features.push(FeatureBlock {
            tag: tag.into(),
            code: "sub A by V;".into(),
            ..Default::default()
        });
        let generated = generate_feature_source(&font, &source, &settings);
        assert!(generated
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "feature-generation-conflict"));
        // A collision skips only its own tag: the user's block is untouched and
        // the family's other tag still materializes.
        assert!(generated
            .source
            .features
            .iter()
            .any(|block| block.tag == tag && block.code == "sub A by V;" && !block.automatic));
        assert!(!generated
            .source
            .features
            .iter()
            .any(|block| block.tag == tag && block.automatic));
        let sibling = if tag == "vert" { "vrt2" } else { "vert" };
        assert!(generated
            .source
            .features
            .iter()
            .any(|block| block.tag == sibling && block.automatic));
    }
    let generated = generate_feature_source(&font, &font.features, &settings);
    let mut frozen = freeze_feature_source(
        &generated.source,
        &generated.settings,
        FeatureGenerator::VerticalForms,
        None,
    );
    assert!(!frozen.settings.vertical_forms);
    assert!(frozen.source.features.iter().all(|block| !block.automatic));
    frozen
        .source
        .features
        .iter_mut()
        .find(|block| block.tag == "vert")
        .unwrap()
        .code = "sub A by V;".into();
    let regenerated = generate_feature_source(&font, &frozen.source, &frozen.settings);
    assert!(regenerated.diagnostics.is_empty());
    assert_eq!(regenerated.source, frozen.source);
    font.features = regenerated.source;
    font.feature_settings = regenerated.settings;
    assert_eq!(
        glyph_ids(&font, "A", harfrust::script::LATIN, "en", "vert=1"),
        glyph_ids(&font, "V", harfrust::script::LATIN, "en", "vert=0")
    );
}

#[test]
fn missing_ambiguous_and_manual_collisions_fail_without_overwriting_source() {
    let font = font_with(&["missing.sc", "A_V", "A_V_A"]);
    let settings = FeatureGenerationSettings {
        small_caps: true,
        ligatures: true,
        ..Default::default()
    };
    let result = generate_feature_source(&font, &font.features, &settings);
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "feature-generation-missing-base"));
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "feature-generation-ambiguous"));
    let mut source = font.features.clone();
    source.prefixes.push(FeatureBlock {
        code: "feature smcp { sub A by V; } smcp;".into(),
        ..Default::default()
    });
    let generated = generate_feature_source(&font, &source, &settings);
    assert!(generated
        .diagnostics
        .iter()
        .any(|d| d.code == "feature-generation-conflict"));
    assert_eq!(generated.source.prefixes, source.prefixes);
}

#[test]
fn freeze_preserves_visible_source_and_disables_only_its_recipe() {
    let mut font = font_with(&["A.sc", "V.ss02"]);
    let settings = FeatureGenerationSettings {
        small_caps: true,
        stylistic_sets: true,
        ..Default::default()
    };
    let generated = generate_feature_source(&font, &font.features, &settings);
    let frozen = freeze_feature_source(
        &generated.source,
        &generated.settings,
        FeatureGenerator::SmallCaps,
        None,
    );
    assert!(!frozen.settings.small_caps);
    assert!(frozen.settings.stylistic_sets);
    let small_caps = frozen
        .source
        .features
        .iter()
        .find(|f| f.tag == "smcp")
        .unwrap();
    assert!(!small_caps.automatic);
    let frozen_code = small_caps.code.clone();
    font.glyphs.swap_remove("A.sc");
    assert!(generation_is_stale(&font, &generated.settings));
    let again = generate_feature_source(&font, &frozen.source, &frozen.settings);
    assert_eq!(
        again
            .source
            .features
            .iter()
            .find(|f| f.tag == "smcp")
            .unwrap()
            .code,
        frozen_code
    );
}

#[test]
fn explicit_localization_and_discretionary_mappings_compile() {
    let mut font = font_with(&["A.trk", "A_V.dlig"]);
    let settings = FeatureGenerationSettings {
        localized: vec![FeatureLocalizedMapping {
            script: "latn".into(),
            language: "TRK".into(),
            substitutions: vec![FeatureSubstitution {
                from: "A".into(),
                to: "A.trk".into(),
            }],
        }],
        discretionary_ligatures: vec![FeatureLigatureMapping {
            script: "DFLT".into(),
            language: "dflt".into(),
            components: vec!["A".into(), "V".into()],
            replacement: "A_V.dlig".into(),
        }],
        ..Default::default()
    };
    let generated = generate_feature_source(&font, &font.features, &settings);
    assert!(
        generated.diagnostics.is_empty(),
        "{:?}",
        generated.diagnostics
    );
    let validation = validate_feature_candidate(&font, &generated.source, &generated.settings);
    assert!(validation.is_valid(), "{:?}", validation.diagnostics);
    assert!(generated.source.to_fea().contains("language TRK"));
    font.features = generated.source;
    font.feature_settings = generated.settings;
    assert_ne!(
        glyph_ids(&font, "A", harfrust::script::LATIN, "tr", "locl=1"),
        glyph_ids(&font, "A", harfrust::script::LATIN, "en", "locl=1")
    );
    assert_eq!(
        glyph_ids(&font, "A", harfrust::script::LATIN, "tr", "locl=0"),
        glyph_ids(&font, "A", harfrust::script::LATIN, "en", "locl=1")
    );
}

fn glyph_ids(
    font: &fc_model::Font,
    text: &str,
    script: harfrust::Script,
    language: &str,
    feature: &str,
) -> Vec<u32> {
    let bytes = fc_compile::compile_static(font, &font.masters[0].id, &Default::default()).unwrap();
    let face = read_fonts::FontRef::new(&bytes).unwrap();
    let data = harfrust::ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let mut buffer = harfrust::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_script(script);
    buffer.set_language(language.parse().unwrap());
    buffer.guess_segment_properties();
    let features = [feature.parse::<harfrust::Feature>().unwrap()];
    shaper
        .shape(buffer, harfrust::ShapeOptions::new().features(&features))
        .glyph_infos()
        .iter()
        .map(|g| g.glyph_id)
        .collect()
}

#[test]
fn discretionary_ligatures_respect_explicit_script_and_language_scopes() {
    let mut font = font_with(&["A_V.trk", "A_V.srb"]);
    let settings = FeatureGenerationSettings {
        discretionary_ligatures: vec![
            FeatureLigatureMapping {
                script: "latn".into(),
                language: "TRK".into(),
                components: vec!["A".into(), "V".into()],
                replacement: "A_V.trk".into(),
            },
            FeatureLigatureMapping {
                script: "cyrl".into(),
                language: "SRB".into(),
                components: vec!["A".into(), "V".into()],
                replacement: "A_V.srb".into(),
            },
        ],
        ..Default::default()
    };
    let generated = generate_feature_source(&font, &font.features, &settings);
    assert!(
        generated.diagnostics.is_empty(),
        "{:?}",
        generated.diagnostics
    );
    font.features = generated.source;
    font.feature_settings = generated.settings;
    let turkish = glyph_ids(&font, "AV", harfrust::script::LATIN, "tr", "dlig=1");
    let serbian = glyph_ids(&font, "AV", harfrust::script::CYRILLIC, "sr", "dlig=1");
    assert_eq!(turkish.len(), 1);
    assert_eq!(serbian.len(), 1);
    assert_ne!(turkish, serbian);
    assert_eq!(
        glyph_ids(&font, "AV", harfrust::script::LATIN, "en", "dlig=1").len(),
        2
    );
    assert_eq!(
        glyph_ids(&font, "AV", harfrust::script::CYRILLIC, "tr", "dlig=1").len(),
        2
    );
    assert_eq!(
        glyph_ids(&font, "AV", harfrust::script::LATIN, "tr", "dlig=0").len(),
        2
    );
}

#[test]
fn reserved_names_escape_and_legacy_automatic_is_metadata_only() {
    let font = font_with(&["sub", "sub.sc"]);
    let settings = FeatureGenerationSettings {
        small_caps: true,
        ..Default::default()
    };
    let generated = generate_feature_source(&font, &font.features, &settings);
    assert!(generated.source.to_fea().contains("sub \\sub by \\sub.sc;"));
    assert!(validate_feature_candidate(&font, &generated.source, &generated.settings).is_valid());
    let mut legacy = font.features.clone();
    legacy.features.push(FeatureBlock {
        tag: "smcp".into(),
        automatic: true,
        code: "sub A by V;".into(),
        ..Default::default()
    });
    assert_eq!(
        generate_feature_source(&font, &legacy, &FeatureGenerationSettings::default()).source,
        legacy
    );
}

#[test]
fn warning_only_user_source_keeps_materialized_rules_and_checks_conflicts() {
    let font = font_with(&["A.sc"]);
    let settings = FeatureGenerationSettings {
        small_caps: true,
        ..Default::default()
    };
    let generated = generate_feature_source(&font, &font.features, &settings);
    let mut source = generated.source.clone();
    source.features.push(FeatureBlock {
        tag: "liga".into(),
        code: "sub A by V;;".into(),
        ..Default::default()
    });
    let next = generate_feature_source(&font, &source, &generated.settings);
    assert!(next.source.features.iter().any(|block| block.tag == "smcp"));
    assert!(next
        .diagnostics
        .iter()
        .all(|d| d.severity != fc_compile::FeatureDiagnosticSeverity::Error));
    source.features.last_mut().unwrap().tag = "smcp".into();
    let conflicted = generate_feature_source(&font, &source, &generated.settings);
    assert!(conflicted
        .diagnostics
        .iter()
        .any(|d| d.code == "feature-generation-conflict"));
}

#[test]
fn padded_language_tags_cannot_bypass_mapping_collision_checks() {
    let font = font_with(&["A.trk", "A_V.trk"]);
    let settings = FeatureGenerationSettings {
        localized: ["TRK", "TRK "]
            .into_iter()
            .map(|language| FeatureLocalizedMapping {
                script: "latn".into(),
                language: language.into(),
                substitutions: vec![FeatureSubstitution {
                    from: "A".into(),
                    to: "A.trk".into(),
                }],
            })
            .collect(),
        discretionary_ligatures: ["TRK", "TRK "]
            .into_iter()
            .map(|language| FeatureLigatureMapping {
                script: "latn".into(),
                language: language.into(),
                components: vec!["A".into(), "V".into()],
                replacement: "A_V.trk".into(),
            })
            .collect(),
        ..Default::default()
    };
    let generated = generate_feature_source(&font, &font.features, &settings);
    assert_eq!(
        generated
            .diagnostics
            .iter()
            .filter(|d| d.code == "feature-generation-duplicate")
            .count(),
        2
    );
}
