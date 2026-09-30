mod common;

use fc_compile::{
    compile_static, compile_variable, shaping_fingerprint, validate_feature_candidate,
    CompileOptions,
};
use fc_model::{Axis, FeatureBlock, LayerId, LayerKind, MasterId, Tag};

fn font(code: &str) -> fc_model::Font {
    let mut font = common::scratch_font();
    font.masters[0].kerning_h.0.clear();
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: code.into(),
        ..Default::default()
    });
    font
}

fn variable_font(code: &str) -> fc_model::Font {
    let mut font = font(code);
    let tag = Tag(*b"wght");
    font.axes.push(Axis {
        tag,
        name: "Weight".into(),
        min: 100.0,
        default: 100.0,
        max: 900.0,
        ..Default::default()
    });
    font.masters[0].location.insert(tag, 100.0);
    let mut heavy = font.masters[0].clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(tag, 900.0);
    for glyph in font.glyphs.values_mut() {
        let mut layer = glyph.layers[0].clone();
        layer.id = LayerId::new();
        layer.kind = LayerKind::Master(heavy.id.clone());
        layer.width += 100.0;
        glyph.layers.push(layer);
    }
    font.masters.push(heavy);
    font
}

#[test]
fn predicates_and_numeric_tokens_change_actual_shaping() {
    let font = font("pos [$[name == 'A']] ${-A:width / 10};");
    let original = font.clone();
    let bytes = compile_static(&font, &font.masters[0].id, &CompileOptions::default()).unwrap();
    assert_eq!(
        shaping_fingerprint(&bytes, "AV").unwrap(),
        vec![(2, 540, 0), (3, 600, 0)]
    );
    let validation = validate_feature_candidate(&font, &font.features, &font.feature_settings);
    assert!(validation.is_valid(), "{validation:?}");
    assert_eq!(font, original);
}

#[test]
fn target_conditionals_and_master_metrics_compile_into_static_and_variable_fonts() {
    let font = variable_font(
        "#ifdef VARIABLE\npos A ${-A:width / 5};\n#else\npos A ${-A:width / 10};\n#endif",
    );
    for (index, advance) in [(0, 540), (1, 630)] {
        let bytes =
            compile_static(&font, &font.masters[index].id, &CompileOptions::default()).unwrap();
        assert_eq!(
            shaping_fingerprint(&bytes, "A").unwrap(),
            vec![(2, advance, 0)]
        );
    }
    let variable = compile_variable(&font, &CompileOptions::default()).unwrap();
    assert_eq!(
        shaping_fingerprint(&variable.bytes, "A").unwrap(),
        vec![(2, 480, 0)]
    );
    let validation = validate_feature_candidate(&font, &font.features, &font.feature_settings);
    assert!(validation.is_valid(), "{validation:?}");
    assert_eq!(validation.compiled_masters.len(), 2);
}

#[test]
fn invalid_variable_branch_cannot_activate_but_does_not_block_static_export() {
    let font = variable_font("#ifdef VARIABLE\npos A $missing;\n#else\npos A -30;\n#endif");
    assert!(compile_static(&font, &font.masters[0].id, &CompileOptions::default()).is_ok());
    assert!(compile_variable(&font, &CompileOptions::default()).is_err());
    let validation = validate_feature_candidate(&font, &font.features, &font.feature_settings);
    assert!(!validation.is_valid());
}

#[test]
fn token_and_parser_errors_refer_to_original_unicode_source_offsets() {
    for (code, needle) in [
        ("# 中文 😀\npos A $unknown;", "$unknown"),
        ("# 中文 😀\npos A ${10 * 2};\npos Missing 5;", "Missing"),
    ] {
        let font = font(code);
        let validation = validate_feature_candidate(&font, &font.features, &font.feature_settings);
        assert!(!validation.is_valid());
        let expected = code[..code.find(needle).unwrap()].encode_utf16().count() as u32;
        assert!(
            validation.diagnostics.iter().any(|d| d.source_id.as_ref()
                == Some(&font.features.features[0].id)
                && d.span.as_ref().is_some_and(|s| s.start == expected)),
            "{validation:?}"
        );
    }
}
