use fc_model::{FeatureBlock, FeatureClass, FeatureSourceKind, Features, FeaturesSource};

fn block(tag: &str, code: &str) -> FeatureBlock {
    FeatureBlock {
        tag: tag.into(),
        code: code.into(),
        ..FeatureBlock::default()
    }
}

#[test]
fn opaque_prefix_preserves_exact_text_and_line_endings() {
    for code in [
        "# 中文 😀\r\nlanguagesystem DFLT dflt;\r\n",
        "languagesystem DFLT dflt; # no final newline",
    ] {
        let source = Features {
            prefixes: vec![block("UFO", code)],
            ..Features::default()
        };
        assert_eq!(source.to_fea(), code);
    }
}

#[test]
fn complete_feature_blocks_keep_comments_whitespace_and_spelling() {
    for code in [
        "  # 中文 😀\r\nfeature\r\nliga { sub f i by f_i; } liga;\r\n",
        "feature\tliga\tuseExtension { sub f i by f_i; } liga;",
        "# header\rfeature # tag\r\nliga { sub f i by f_i; } liga; # end",
    ] {
        let source = Features {
            features: vec![block("liga", code)],
            ..Features::default()
        };
        assert_eq!(source.to_fea(), code);
    }
}

#[test]
fn feature_references_in_aalt_are_still_wrapped_as_a_body() {
    let source = Features {
        features: vec![block("aalt", "feature salt;")],
        ..Features::default()
    };
    assert_eq!(source.to_fea(), "feature aalt {\nfeature salt;\n} aalt;");
}

#[test]
fn structured_source_adds_only_required_separators_and_class_wrappers() {
    let source = Features {
        prefixes: vec![block("Language", "languagesystem DFLT dflt; # trailing")],
        classes: vec![FeatureClass {
            name: "@Letters".into(),
            code: " A V # member\r\n".into(),
            ..FeatureClass::default()
        }],
        features: vec![block("kern", "pos A V -80; # end")],
        ..Features::default()
    };
    assert_eq!(source.to_fea(), "languagesystem DFLT dflt; # trailing\n@Letters = [ A V # member\r\n];\nfeature kern {\npos A V -80; # end\n} kern;");
}

#[test]
fn source_maps_distinguish_utf8_bytes_utf16_and_generated_wrappers() {
    let feature = block("kern", "# 中文 😀\r\npos A V -80;");
    let id = feature.id.clone();
    let source = Features {
        features: vec![feature],
        ..Features::default()
    };
    let assembled = source.assemble_fea();
    assert_eq!(assembled.text, source.to_fea());
    let location = assembled
        .locate(assembled.text.find("pos").unwrap())
        .unwrap();
    assert_eq!(location.item, id);
    assert_eq!(location.kind, FeatureSourceKind::Feature);
    assert_eq!(location.byte_offset, Some(15));
    assert_eq!(location.utf16_offset, Some(9));
    let wrapper = assembled.locate(0).unwrap();
    assert_eq!(wrapper.item, id);
    assert_eq!(wrapper.byte_offset, None);
    assert_eq!(wrapper.utf16_offset, None);
    assert!(assembled
        .locate(assembled.text.find('😀').unwrap() + 1)
        .is_none());
    assert!(assembled.locate(assembled.text.len() + 1).is_none());
    assert_eq!(assembled.locate(assembled.text.len()).unwrap().item, id);
}

#[test]
fn class_and_prefix_maps_retain_code_offsets_and_item_identity() {
    for code in ["A V", "  [A V] # suffix\r\n"] {
        let class = FeatureClass {
            name: "@Letters".into(),
            code: code.into(),
            ..FeatureClass::default()
        };
        let prefix = block("comment", "# 中文\r\n");
        let class_id = class.id.clone();
        let prefix_id = prefix.id.clone();
        let source = Features {
            prefixes: vec![prefix],
            classes: vec![class],
            ..Features::default()
        };
        let assembled = source.assemble_fea();
        let location = assembled.locate(assembled.text.find('V').unwrap()).unwrap();
        assert_eq!(location.kind, FeatureSourceKind::Class);
        assert_eq!(location.item, class_id);
        assert_eq!(location.byte_offset, code.find('V'));
        let location = assembled.locate(2).unwrap();
        assert_eq!(location.kind, FeatureSourceKind::Prefix);
        assert_eq!(location.item, prefix_id);
        assert_eq!(location.byte_offset, Some(2));
    }
}

#[test]
fn source_ids_survive_duplicate_tags_renames_and_reordering() {
    let mut source = Features {
        features: vec![
            block("liga", "sub f i by f_i;"),
            block("liga", "sub f l by f_l;"),
        ],
        ..Features::default()
    };
    let first = source.features[0].id.clone();
    let second = source.features[1].id.clone();
    assert_ne!(first, second);
    source.features.swap(0, 1);
    source.features[1].tag = "dlig".into();
    let source: Features = serde_json::from_value(serde_json::to_value(source).unwrap()).unwrap();
    let assembled = source.assemble_fea();
    assert_eq!(
        assembled
            .locate(assembled.text.find("sub f i").unwrap())
            .unwrap()
            .item,
        first
    );
    assert_eq!(
        assembled
            .locate(assembled.text.find("sub f l").unwrap())
            .unwrap()
            .item,
        second
    );
}

#[test]
fn commented_classes_and_bodies_cannot_swallow_generated_delimiters() {
    for (code, expected) in [
        ("A V # last comment", "@Letters = [A V # last comment\n];"),
        (
            " # leading\n[A V] # trailing",
            "@Letters =  # leading\n[A V] # trailing\n;",
        ),
    ] {
        let source = Features {
            classes: vec![FeatureClass {
                name: "Letters".into(),
                code: code.into(),
                ..FeatureClass::default()
            }],
            ..Features::default()
        };
        assert_eq!(source.to_fea(), expected);
    }
}

#[test]
fn disabled_and_imported_source_emit_no_code_or_mappings() {
    let mut source = Features {
        prefixes: vec![FeatureBlock {
            disabled: true,
            ..block("prefix", "bad")
        }],
        features: vec![FeatureBlock {
            disabled: true,
            ..block("liga", "bad")
        }],
        ..Features::default()
    };
    assert!(source.assemble_fea().mappings.is_empty());
    assert_eq!(source.to_fea(), "");
    source.prefixes[0].disabled = false;
    source.source_of_truth = FeaturesSource::ImportedTables;
    let before = source.clone();
    assert!(source.assemble_fea().mappings.is_empty());
    assert_eq!(source.to_fea(), "");
    assert_eq!(source, before);
}
