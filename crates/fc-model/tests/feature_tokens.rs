use fc_model::{FeatureBlock, Features, Font, Glyph, Layer, LayerKind};

fn fixture(code: &str) -> Font {
    let mut font = Font::new_empty("Feature tokens", 1000);
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: code.into(),
        ..Default::default()
    });
    for name in ["A", "A.sc", "B.sc"] {
        let mut glyph = Glyph::new(name);
        glyph.layers.push(Layer {
            kind: LayerKind::Master(font.masters[0].id.clone()),
            width: 600.0,
            ..Default::default()
        });
        font.glyphs.insert(glyph.name.clone(), glyph);
    }
    font
}

#[test]
fn expands_numbers_predicates_and_target_branches_without_mutation() {
    let font = fixture("#ifdef VARIABLE\npos A ${A:width / 10};\n#else\npos [$[name ENDSWITH '.sc']] ${-unitsPerEm / 10};\n#endif");
    let original = font.features.clone();
    let static_source = font
        .features
        .assemble_fea_for(&font, &font.masters[0].id, false)
        .unwrap();
    assert!(static_source.text.contains("pos [\\A.sc \\B.sc] -100;"));
    assert!(!static_source.text.contains("pos A 60"));
    let variable_source = font
        .features
        .assemble_fea_for(&font, &font.masters[0].id, true)
        .unwrap();
    assert!(variable_source.text.contains("pos A 60;"));
    assert_eq!(font.features, original);
}

#[test]
fn inactive_code_is_not_expanded_and_nested_directives_select_actual_target() {
    let font = fixture("#ifdef VARIABLE\n${missing}\n#else\n#ifndef VARIABLE\npos A 12;\n#else\n${alsoMissing}\n#endif\n#endif");
    assert!(font
        .features
        .assemble_fea_for(&font, &font.masters[0].id, false)
        .unwrap()
        .text
        .contains("pos A 12"));
    assert!(font
        .features
        .assemble_fea_for(&font, &font.masters[0].id, true)
        .is_err());
}

#[test]
fn comments_and_fea_strings_are_literal_and_copied_offsets_survive_expansion() {
    let code = "# 中文 😀 $missing\nnameid 1 \"$missing\";\npos A ${12 * 3}; bad;";
    let font = fixture(code);
    let assembled = font
        .features
        .assemble_fea_for(&font, &font.masters[0].id, false)
        .unwrap();
    assert!(assembled.text.contains("# 中文 😀 $missing"));
    assert!(assembled.text.contains("\"$missing\""));
    let offset = assembled.text.find("bad").unwrap();
    let location = assembled.locate(offset).unwrap();
    assert_eq!(location.byte_offset, code.find("bad"));
    assert_eq!(
        location.utf16_offset,
        Some(code[..code.find("bad").unwrap()].encode_utf16().count())
    );
}

#[test]
fn malformed_directives_and_unknown_tokens_are_located() {
    for code in [
        "#else",
        "#endif",
        "#ifdef OTHER\n#endif",
        "#ifdef VARIABLE",
        "#ifdef VARIABLE\n#else\n#else\n#endif",
        "#ifdef VARIABLE\n#elif VARIABLE\n#endif",
        "pos A $missing;",
    ] {
        let font = fixture(code);
        let error = font
            .features
            .assemble_fea_for(&font, &font.masters[0].id, false)
            .unwrap_err();
        assert_eq!(error.source_id, font.features.features[0].id);
        assert!(!error.message.is_empty());
    }
}

#[test]
fn conditional_whole_feature_blocks_are_wrapped_after_selection() {
    let font = fixture("#ifdef VARIABLE\nfeature kern { pos A 12; } kern;\n#else\nfeature kern { pos A 24; } kern;\n#endif");
    let assembled = font
        .features
        .assemble_fea_for(&font, &font.masters[0].id, false)
        .unwrap();
    assert_eq!(assembled.text.matches("feature kern").count(), 1);
}

#[test]
fn imported_and_disabled_sources_do_not_expand() {
    let mut font = fixture("$missing");
    font.features.features[0].disabled = true;
    assert!(font
        .features
        .assemble_fea_for(&font, &font.masters[0].id, false)
        .unwrap()
        .text
        .is_empty());
    font.features = Features {
        source_of_truth: fc_model::FeaturesSource::ImportedTables,
        ..Default::default()
    };
    assert!(font
        .features
        .assemble_fea_for(&font, &font.masters[0].id, false)
        .unwrap()
        .text
        .is_empty());
}
