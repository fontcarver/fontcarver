use fc_compile::{compile_static, validate_feature_candidate};
use fc_model::{FeatureBlock, Font, Glyph, Layer};

fn font(count: usize, rule: &str) -> Font {
    let mut font = Font::new_empty("Feature work budget", 1000);
    let master = font.default_master().id.clone();
    for i in 0..count {
        let mut glyph = Glyph::new(format!("g{i:04}"));
        let mut layer = Layer::new_master(master.clone());
        layer.width = 500.0;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.features.prefixes.push(FeatureBlock {
        code: format!(
            "@Many = [{}];",
            (0..count)
                .map(|i| format!("g{i:04}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        ..Default::default()
    });
    font.features.features.push(FeatureBlock {
        tag: if rule.contains("pos ") {
            "kern"
        } else {
            "liga"
        }
        .into(),
        code: rule.into(),
        ..Default::default()
    });
    font
}

fn rejected(font: &Font) {
    let result = validate_feature_candidate(font, &font.features, &font.feature_settings);
    assert!(!result.is_valid());
    assert!(result.compiled_masters.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "feature-source-limit"),
        "{:?}",
        result.diagnostics
    );
    let error = compile_static(font, &font.default_master().id, &Default::default()).unwrap_err();
    assert!(error.to_string().contains("expansion"), "{error}");
}

#[test]
fn cartesian_ligatures_are_rejected_before_compilation() {
    rejected(&font(256, "sub @Many @Many @Many by g0000;"));
}

#[test]
fn enumerated_pairs_are_rejected_before_compilation() {
    rejected(&font(3000, "enum pos @Many @Many -10;"));
}

#[test]
fn aliases_ranges_and_repeated_class_entries_are_bounded() {
    let mut font = font(256, "sub @Alias @Alias @Alias by g0000;");
    font.features.prefixes[0].code =
        "@Range = [g0000-g0255]; @Many = [@Range @Range]; @Alias = @Many;".into();
    rejected(&font);
    font.features.prefixes[0].code = "@Alias = [g0000-g0200];".into();
    rejected(&font);
    font.features.prefixes[0].code =
        "@Range = [g0000-g0200]; markClass @Range <anchor 0 0> @Alias;".into();
    rejected(&font);
}

#[test]
fn accumulated_cartesian_rules_cannot_bypass_the_per_rule_limit() {
    rejected(&font(64, &"enum pos @Many @Many -10;\n".repeat(128)));
}

#[test]
fn ordinary_ligatures_pairs_and_compact_class_kerning_still_compile() {
    for (count, rule) in [
        (4, "sub @Many @Many by g0000;"),
        (4, "enum pos @Many @Many -10;"),
        (400, "pos @Many @Many -10;"),
    ] {
        let font = font(count, rule);
        let result = validate_feature_candidate(&font, &font.features, &font.feature_settings);
        assert!(result.is_valid(), "{rule}: {:?}", result.diagnostics);
    }
}
