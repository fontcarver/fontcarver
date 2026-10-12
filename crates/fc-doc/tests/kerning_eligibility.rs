use fc_doc::kerning::{check_horizontal_editing, HorizontalKerningEditBlock as Block};
use fc_model::{FeatureBlock, FeatureClass, FeaturesSource, Font, Glyph};

fn font() -> Font {
    let mut font = Font::new_empty("Kerning eligibility", 1000);
    for name in [".notdef", "A", "V", "A-V", "kern"] {
        font.insert_glyph(Glyph::new(name)).unwrap();
    }
    font
}

fn block(tag: &str, code: &str) -> FeatureBlock {
    FeatureBlock {
        tag: tag.into(),
        code: code.into(),
        ..FeatureBlock::default()
    }
}

#[test]
fn fresh_source_font_is_eligible() {
    assert_eq!(check_horizontal_editing(&font()), Ok(()));
}

#[test]
fn imported_layout_blocks_even_with_empty_or_invalid_fea() {
    let mut font = font();
    font.features.source_of_truth = FeaturesSource::ImportedTables;
    assert_eq!(check_horizontal_editing(&font), Err(Block::ImportedLayout));
    font.features.prefixes.push(block("prefix", "invalid {"));
    assert_eq!(check_horizontal_editing(&font), Err(Block::ImportedLayout));
}

#[test]
fn kern_bodies_full_blocks_and_empty_blocks_reserve_kerning() {
    for code in ["pos A V -80;", "feature kern { pos A V -80; } kern;", ""] {
        let mut font = font();
        font.features.features.push(block("kern", code));
        assert_eq!(
            check_horizontal_editing(&font),
            Err(Block::SourceOwnedKerning)
        );
    }
}

#[test]
fn parsed_feature_tag_takes_precedence_over_metadata() {
    for (tag, code, expected) in [
        (
            "liga",
            "feature kern { pos A V -80; } kern;",
            Err(Block::SourceOwnedKerning),
        ),
        ("kern", "feature liga { sub A V by A; } liga;", Ok(())),
    ] {
        let mut font = font();
        font.features.features.push(block(tag, code));
        assert_eq!(check_horizontal_editing(&font), expected);
    }
}

#[test]
fn commented_crlf_full_blocks_preserve_parsed_kerning_ownership() {
    for (tag, code, expected) in [
        (
            "liga",
            " \r\n# 中文注释 🪶\r\nfeature\r\n kern { pos A V -80; } kern;\r\n",
            Err(Block::SourceOwnedKerning),
        ),
        (
            "kern",
            "# feature kern is only a comment\r\nfeature\t liga { sub A V by A; } liga;\r\n",
            Ok(()),
        ),
        (
            "kern",
            "# body-only rule still belongs to kern\r\npos A V -80;",
            Err(Block::SourceOwnedKerning),
        ),
    ] {
        let mut font = font();
        font.features.features.push(block(tag, code));
        let before = font.clone();
        assert_eq!(check_horizontal_editing(&font), expected, "{code:?}");
        assert_eq!(font, before);
    }
}

#[test]
fn prefix_features_and_lookup_references_reserve_kerning() {
    for code in [
        "feature\n kern { pos A V -80; } kern;",
        "lookup Spacing { pos A V -80; } Spacing; feature kern { lookup Spacing; } kern;",
        "feature kern { # Automatic Code\n } kern;",
    ] {
        let mut font = font();
        font.features.prefixes.push(block("prefix", code));
        assert_eq!(
            check_horizontal_editing(&font),
            Err(Block::SourceOwnedKerning)
        );
    }
}

#[test]
fn conditional_kern_reserves_kerning() {
    let mut font = font();
    font.features.prefixes.push(block(
        "prefix",
        "conditionset Heavy { wght 600 900; } Heavy; variation kern Heavy { pos A V -80; } kern;",
    ));
    assert_eq!(
        check_horizontal_editing(&font),
        Err(Block::SourceOwnedKerning)
    );
}

#[test]
fn disabled_features_and_prefixes_are_excluded() {
    let mut font = font();
    let mut kern = block("kern", "pos A V -80;");
    kern.disabled = true;
    font.features.features.push(kern);
    let mut prefix = block("prefix", "include(missing.fea); invalid {");
    prefix.disabled = true;
    font.features.prefixes.push(prefix);
    assert_eq!(check_horizontal_editing(&font), Ok(()));
}

#[test]
fn automatic_flag_does_not_disable_active_source() {
    let mut font = font();
    let mut kern = block("kern", "pos A V -80;");
    kern.automatic = true;
    font.features.features.push(kern);
    assert_eq!(
        check_horizontal_editing(&font),
        Err(Block::SourceOwnedKerning)
    );
}

#[test]
fn comments_names_classes_and_other_features_do_not_reserve_kern() {
    for class_code in ["A V", "[A V]"] {
        let mut font = font();
        font.features.prefixes.push(block(
            "kern",
            "# feature kern { pos A V -80; } kern;\nlookup kern { sub A by V; } kern;",
        ));
        font.features.classes.push(FeatureClass {
            name: "@kern".into(),
            code: class_code.into(),
            ..FeatureClass::default()
        });
        font.features
            .features
            .push(block("liga", "lookup kern; sub @kern V by A;"));
        font.features.features.push(block("vkrn", "pos A V -80;"));
        assert_eq!(check_horizontal_editing(&font), Ok(()));
    }
}

#[test]
fn glyph_map_disambiguates_hyphenated_names() {
    let mut font = font();
    font.features
        .features
        .push(block("liga", "sub A-V V by A;"));
    assert_eq!(check_horizontal_editing(&font), Ok(()));
}

#[test]
fn malformed_source_blocks_without_mutation_or_partial_success() {
    for code in [
        "feature liga { sub A by V; }",
        "feature kern { pos A V -80; } kern; invalid {",
        "feature liga { sub A by V; } kern;",
    ] {
        let mut font = font();
        font.features.prefixes.push(block("prefix", code));
        let before = font.clone();
        assert_eq!(
            check_horizontal_editing(&font),
            Err(Block::InvalidFeatureSource)
        );
        assert_eq!(font, before);
    }
}

#[test]
fn includes_fail_closed_without_loading_files_or_panicking() {
    for code in [
        "include(missing.fea);",
        "include(/etc/passwd);",
        "feature kern { include(spacing.fea); } kern;",
        "include(fontcarver-kerning.fea);",
    ] {
        let mut font = font();
        font.features.prefixes.push(block("prefix", code));
        assert_eq!(
            check_horizontal_editing(&font),
            Err(Block::InvalidFeatureSource)
        );
    }
}

#[test]
fn ownership_checks_leave_stored_pairs_and_source_unchanged() {
    let mut font = font();
    font.masters[0].kerning_h.0.insert(
        (
            fc_model::KernSide::Glyph("A".into()),
            fc_model::KernSide::Glyph("V".into()),
        ),
        -80.0,
    );
    for source in [FeaturesSource::Fea, FeaturesSource::ImportedTables] {
        font.features.source_of_truth = source;
        let before = font.clone();
        let _ = check_horizontal_editing(&font);
        assert_eq!(font, before);
    }
}

#[test]
fn disabled_model_writer_blocks_even_empty_active_source() {
    let mut font = font();
    font.feature_settings.writer_policy.kerning = false;
    assert_eq!(
        check_horizontal_editing(&font),
        Err(Block::SourceOwnedKerning)
    );
}
