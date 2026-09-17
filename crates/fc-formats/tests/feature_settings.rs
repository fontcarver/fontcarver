use fc_compile::{export, shaping_fingerprint, ExportOptions};
use fc_formats::ufo::{read_designspace, read_ufo, write_designspace, write_ufo, UfoWriteOptions};
use fc_model::{
    Anchor, Axis, FeatureBlock, FeatureGenerationSettings, FeatureGenerator, FeatureWriterPolicy,
    FeaturesSource, Font, GeneratedFeatureRecord, Glyph, KernSide, Layer, Tag,
};

const WRITERS: &str = "com.github.googlei18n.ufo2ft.featureWriters";

#[test]
fn ufo_reopen_preserves_writer_policy_and_marker_shaping() {
    for kerning in [false, true] {
        for anchors in [false, true] {
            let mut font = font_with_model_layout();
            font.feature_settings.writer_policy = FeatureWriterPolicy { kerning, anchors };
            font.features.features.push(FeatureBlock {
                tag: "kern".into(),
                code: if kerning {
                    "# Automatic Code".into()
                } else {
                    "# Automatic Code\npos A V -20;".into()
                },
                ..FeatureBlock::default()
            });
            let before = compile(&font);
            let expected_advance = if kerning { 510 } else { 580 };
            assert_eq!(
                shaping_fingerprint(&before, "AV").unwrap()[0].1,
                expected_advance
            );
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("Policy.ufo");
            write_ufo(
                &font,
                &font.default_master().id,
                &path,
                &UfoWriteOptions::default(),
            )
            .unwrap();
            let reopened = read_ufo(&path).unwrap();
            assert_eq!(
                reopened.feature_settings.writer_policy,
                font.feature_settings.writer_policy
            );
            let after = compile(&reopened);
            assert_eq!(
                shaping_fingerprint(&before, "AV").unwrap(),
                shaping_fingerprint(&after, "AV").unwrap()
            );
            assert_eq!(reopened.features.to_fea(), font.features.to_fea());
        }
    }
}

#[test]
fn disabled_model_anchors_do_not_reappear_after_ufo_reopen() {
    let mut font = font_with_model_layout();
    font.feature_settings.writer_policy = FeatureWriterPolicy {
        kerning: false,
        anchors: false,
    };
    let before = compile(&font);
    assert!(!compiled_tables(&before).contains_key(&Tag(*b"GPOS")));
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("NoGeneratedLayout.ufo");
    write_ufo(
        &font,
        &font.default_master().id,
        &path,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    let ufo = norad::Font::load(&path).unwrap();
    assert_eq!(ufo.lib.get(WRITERS), Some(&plist::Value::Array(vec![])));
    let reopened = read_ufo(&path).unwrap();
    assert!(!compiled_tables(&compile(&reopened)).contains_key(&Tag(*b"GPOS")));
    let mut with_anchors = font.clone();
    with_anchors.feature_settings.writer_policy.anchors = true;
    assert!(compiled_tables(&compile(&with_anchors)).contains_key(&Tag(*b"GPOS")));
}

#[test]
fn flattened_generated_source_reopens_as_frozen_source_without_stale_ids() {
    let mut font = font_with_model_layout();
    let block = FeatureBlock {
        tag: "liga".into(),
        code: "sub A A by V;".into(),
        automatic: true,
        ..FeatureBlock::default()
    };
    font.feature_settings = FeatureGenerationSettings {
        small_caps: true,
        ligatures: true,
        generated: vec![GeneratedFeatureRecord {
            generator: FeatureGenerator::Ligatures,
            source_id: block.id.clone(),
            glyphs: vec!["A".into(), "V".into()],
        }],
        glyph_fingerprint: Some("old-source-fingerprint".into()),
        writer_policy: FeatureWriterPolicy {
            kerning: false,
            anchors: false,
        },
        ..FeatureGenerationSettings::default()
    };
    font.features.features.push(block);
    let expected_source = font.features.to_fea();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("Frozen.ufo");
    write_ufo(
        &font,
        &font.default_master().id,
        &path,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    let reopened = read_ufo(&path).unwrap();
    assert_eq!(reopened.features.to_fea(), expected_source);
    assert_eq!(
        reopened.feature_settings,
        FeatureGenerationSettings {
            writer_policy: font.feature_settings.writer_policy.clone(),
            ..FeatureGenerationSettings::default()
        }
    );
    assert!(!reopened.features.prefixes[0].automatic);
    assert_eq!(
        shaping_fingerprint(&compile(&font), "AA").unwrap(),
        shaping_fingerprint(&compile(&reopened), "AA").unwrap()
    );
}

#[test]
fn designspace_policy_overrides_stale_lib_metadata_and_reopens_consistently() {
    let mut font = font_with_model_layout();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.0,
        default: 400.0,
        max: 900.0,
        ..Axis::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 400.0);
    font.feature_settings.writer_policy = FeatureWriterPolicy {
        kerning: false,
        anchors: false,
    };
    font.extra.lib.insert(
        "ufo.designspace.lib".into(),
        serde_json::json!({ WRITERS: [{"class": "KernFeatureWriter"}]}),
    );
    let temp = tempfile::tempdir().unwrap();
    let path = write_designspace(&font, temp.path()).unwrap();
    let written = norad::designspace::DesignSpaceDocument::load(&path).unwrap();
    assert_eq!(written.lib[WRITERS], plist::Value::Array(vec![]));
    let reopened = read_designspace(&path).unwrap();
    assert_eq!(
        reopened.feature_settings.writer_policy,
        font.feature_settings.writer_policy
    );
    assert_eq!(
        shaping_fingerprint(&compile(&font), "AV").unwrap(),
        shaping_fingerprint(&compile(&reopened), "AV").unwrap()
    );
}

#[test]
fn source_ufo_omits_original_layout_backups_but_keeps_unrelated_tables() {
    let mut font = font_with_model_layout();
    for tag in [
        *b"GSUB", *b"GPOS", *b"GDEF", *b"BASE", *b"kern", *b"kerx", *b"morx", *b"mort", *b"JSTF",
        *b"TEST",
    ] {
        font.extra.raw_tables.insert(Tag(tag), vec![1, 2, 3]);
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("Source.ufo");
    write_ufo(
        &font,
        &font.default_master().id,
        &path,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    let reopened = read_ufo(&path).unwrap();
    assert_eq!(reopened.features.source_of_truth, FeaturesSource::Fea);
    assert!(reopened.layout_conversion.is_none());
    assert_eq!(
        reopened.extra.raw_tables,
        [(Tag(*b"TEST"), vec![1, 2, 3])].into()
    );
    assert_eq!(
        shaping_fingerprint(&compile(&font), "AV").unwrap(),
        shaping_fingerprint(&compile(&reopened), "AV").unwrap()
    );
}

#[test]
fn unrepresentable_ufo_writer_settings_are_rejected() {
    for config in [
        serde_json::json!([{"class": "KernFeatureWriter", "options": {"mode": "append"}}]),
        serde_json::json!([{"class": "MarkFeatureWriter"}]),
        serde_json::json!([{"class": "KernFeatureWriter"}, {"class": "KernFeatureWriter"}]),
        serde_json::json!([{"class": "KernFeatureWriter", "options": {"features": ["kern"]}}]),
        serde_json::json!([{"class": "CustomFeatureWriter"}]),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("Unrepresentable.ufo");
        let mut ufo = norad::Font::new();
        ufo.lib
            .insert(WRITERS.into(), plist::to_value(&config).unwrap());
        ufo.save(&path).unwrap();
        let error = read_ufo(&path).unwrap_err();
        assert!(error.to_string().contains(WRITERS), "{error}");
    }
}

#[test]
fn designspace_writer_setting_takes_precedence_over_source_ufo() {
    let mut font = font_with_model_layout();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.0,
        default: 400.0,
        max: 900.0,
        ..Axis::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 400.0);
    let temp = tempfile::tempdir().unwrap();
    let path = write_designspace(&font, temp.path()).unwrap();
    let mut document = norad::designspace::DesignSpaceDocument::load(&path).unwrap();
    document
        .lib
        .insert(WRITERS.into(), plist::Value::Array(vec![]));
    document.save(&path).unwrap();
    let reopened = read_designspace(&path).unwrap();
    assert_eq!(
        reopened.feature_settings.writer_policy,
        FeatureWriterPolicy {
            kerning: false,
            anchors: false
        }
    );
    assert_eq!(
        shaping_fingerprint(&compile(&reopened), "AV").unwrap()[0].1,
        600
    );
}

fn font_with_model_layout() -> Font {
    let mut font = Font::new_empty("Feature Policy", 1000);
    let master = font.default_master().id.clone();
    for (name, unicode, width, anchor) in [
        (".notdef", None, 600.0, None),
        ("A", Some(0x41), 600.0, Some(("top", 300.0, 700.0))),
        ("V", Some(0x56), 600.0, None),
        ("acutecomb", Some(0x301), 0.0, Some(("_top", 100.0, 0.0))),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        if let Some((name, x, y)) = anchor {
            layer.anchors.push(Anchor {
                name: name.into(),
                x,
                y,
            });
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
        -90.0,
    );
    font
}

fn compile(font: &Font) -> Vec<u8> {
    export(font, &|_| 0, &ExportOptions::default())
        .unwrap()
        .bytes
}

fn compiled_tables(bytes: &[u8]) -> std::collections::BTreeMap<Tag, Vec<u8>> {
    fc_sfnt::read_bytes(bytes, &fc_sfnt::ReadOptions::default())
        .unwrap()
        .fonts
        .remove(0)
        .extra
        .raw_tables
}
