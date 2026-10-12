mod common;

use fc_compile::{validate_feature_candidate, FeatureDiagnosticSeverity};
use fc_model::{FeatureBlock, FeatureGenerationSettings};

#[test]
fn semantic_diagnostic_maps_unicode_source_to_utf16() {
    let font = common::scratch_font();
    let mut source = font.features.clone();
    let block = FeatureBlock {
        tag: "liga".into(),
        code: "# 中文 😀\nsub A by missing;".into(),
        ..Default::default()
    };
    let id = block.id.clone();
    let start = block.code[..block.code.find("missing").unwrap()]
        .encode_utf16()
        .count() as u32;
    source.features.push(block);
    let result = validate_feature_candidate(&font, &source, &FeatureGenerationSettings::default());
    assert!(!result.is_valid());
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == "feature-semantic")
        .unwrap_or_else(|| panic!("{:?}", result.diagnostics));
    assert_eq!(diagnostic.source_id.as_ref(), Some(&id));
    assert_eq!(diagnostic.span.as_ref().unwrap().start, start);
    assert!(!diagnostic.generated);
    assert!(result.compiled_masters.is_empty());
}

#[test]
fn invalid_generated_wrapper_is_owned_by_its_source_item() {
    let font = common::scratch_font();
    let mut source = font.features.clone();
    let block = FeatureBlock {
        tag: "toolong".into(),
        code: "sub A by V;".into(),
        ..Default::default()
    };
    let id = block.id.clone();
    source.features.push(block);
    let result = validate_feature_candidate(&font, &source, &FeatureGenerationSettings::default());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.source_id.as_ref() == Some(&id) && d.generated && d.span.is_none()));
}

#[test]
fn includes_are_located_and_never_loaded_from_disk() {
    let font = common::scratch_font();
    let mut source = font.features.clone();
    let block = FeatureBlock {
        code: "# 😀\ninclude(/etc/passwd);".into(),
        ..Default::default()
    };
    let id = block.id.clone();
    source.prefixes.push(block);
    let result = validate_feature_candidate(&font, &source, &FeatureGenerationSettings::default());
    assert!(!result.is_valid());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "feature-include-disabled"
            && d.source_id.as_ref() == Some(&id)
            && d.span.is_some()));
}

#[test]
fn validation_compiles_every_master_and_reports_missing_layer() {
    let mut font = common::scratch_font();
    let mut second = font.masters[0].clone();
    second.id = fc_model::MasterId("second".into());
    font.masters.push(second);
    let result =
        validate_feature_candidate(&font, &font.features, &FeatureGenerationSettings::default());
    assert!(!result.is_valid());
    assert_eq!(result.compiled_masters, vec![font.masters[0].id.clone()]);
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "feature-compile-failed"
            && d.master.as_ref() == Some(&font.masters[1].id)));
}

#[test]
fn valid_candidate_runs_real_fontc_without_mutating_font() {
    let font = common::scratch_font();
    let mut source = font.features.clone();
    source.features.push(FeatureBlock {
        tag: "liga".into(),
        code: "sub A A by V;".into(),
        ..Default::default()
    });
    let result = validate_feature_candidate(&font, &source, &FeatureGenerationSettings::default());
    assert!(result.is_valid(), "{:?}", result.diagnostics);
    assert_eq!(result.compiled_masters, vec![font.masters[0].id.clone()]);
    assert!(font.features.features.is_empty());
}

#[test]
fn diagnostics_and_source_size_are_bounded() {
    let font = common::scratch_font();
    let mut source = font.features.clone();
    source.features.push(FeatureBlock {
        tag: "liga".into(),
        code: (0..300)
            .map(|i| format!("sub A by missing{i};\n"))
            .collect(),
        ..Default::default()
    });
    let result = validate_feature_candidate(&font, &source, &FeatureGenerationSettings::default());
    assert!(result.truncated);
    assert!(result.diagnostics.len() <= 100);
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.severity == FeatureDiagnosticSeverity::Error));
    source.features[0].code = " ".repeat(2_000_000);
    let result = validate_feature_candidate(&font, &source, &FeatureGenerationSettings::default());
    assert_eq!(result.diagnostics[0].code, "feature-source-limit");
}

#[test]
fn upstream_contextual_compiler_panic_is_a_diagnostic_not_an_unwind() {
    let mut font = fc_model::Font::new_empty("Contextual compiler boundary", 1000);
    let master = font.masters[0].id.clone();
    for name in [".notdef", "a", "b", "c", "d"] {
        let mut glyph = fc_model::Glyph::new(name);
        if name.len() == 1 {
            glyph.unicodes.push(u32::from(name.as_bytes()[0]));
        }
        let mut layer = fc_model::Layer::new_master(master.clone());
        layer.width = 500.0;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.features.prefixes.push(FeatureBlock {
        code: "lookup L { sub b by c; } L; feature calt { sub [a]' [b]' lookup L; } calt;".into(),
        ..Default::default()
    });
    // fontc 1.0 catches this upstream fea-rs worker panic as Error::Panic.
    // Keep the real pipeline regression so a dependency update cannot silently
    // change our candidate-validation boundary into an application unwind.
    let result = validate_feature_candidate(&font, &font.features, &font.feature_settings);
    assert!(!result.is_valid());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "feature-compile-failed"));
    assert!(fc_compile::compile_static(&font, &master, &Default::default()).is_err());
}
