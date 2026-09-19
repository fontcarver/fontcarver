use fc_formats::sfd::{read_sfd, write_sfd};
use fc_model::{Contour, FeatureBlock, Font, Glyph, Layer, Point, PointKind};

fn source(records: &str) -> String {
    format!("SplineFontDB: 3.0\nFontName: Native\nFamilyName: Native\nAscent: 800\nDescent: 200\nLayer: 0 1 \"Back\" 1\nLayer: 1 1 \"Fore\" 0\nBeginChars: 1 1\nStartChar: A\nEncoding: 0 65 0\nWidth: 500\nFore\n{records}\nEndChar\nEndChars\nEndSplineFont\n")
}

#[test]
fn malformed_and_unsupported_native_data_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.sfd");
    for record in [
        "SplineSet\n0 0 m 1\n1 2 3 4 5 6 c 1\nEndSplineSet",
        "SplineSet\nNaN 0 m 1\nEndSplineSet",
        "Refer: 2000 -1 N 1 0 0 1 0 0 0",
        "Refer: 0 65 N 1 0 0 1 0 0 0",
        "Kerns2: 20 -30 \"missing\"",
        "AltUni2: 110000.ffffffff.0",
        "Layer: 500",
        "TtfInstrs: 20",
        "Image: 200 300",
        "Comment: \"+2AA-\"",
        "AnchorPoint: \"top\" NaN 5 basechar 0",
        "AnchorPoint: \"_top\" 200 500 basechar 0",
        "AnchorPoint: \"entry\" 200 500 basechar 0",
        "Substitution2: \"missing\" B",
    ] {
        std::fs::write(&path, source(record)).unwrap();
        assert!(read_sfd(&path).is_err(), "accepted {record}");
    }
    for text in [
        source("").replace("BeginChars: 1 1", "BeginChars: 1 999999999"),
        source("").replace("EndChar\n", ""),
        source("").replace("EndSplineFont", "BitmapFont: 12 10"),
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(read_sfd(&path).is_err());
    }
}

#[test]
fn unicode_notes_unknown_fields_and_quadratic_roundtrip() {
    let mut font = Font::new_empty("字体 + Sample", 1000);
    font.info.note = Some("备注 + français 😀\n下一行".into());
    font.info.postscript_font_name = Some("Unicode-Regular".into());
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("字形");
    glyph.note = "路径 + 😀\n附注".into();
    glyph.unicodes = vec![0x4e00, 0x4e8c];
    glyph.variation_selectors = vec![(0x4e00, 0xfe01)];
    let mut layer = Layer::new_master(master.clone());
    layer.width = 800.0;
    layer.contours.push(Contour {
        closed: true,
        points: vec![
            Point {
                x: 0.0,
                y: 0.0,
                kind: PointKind::Line,
                ..Default::default()
            },
            Point {
                x: 100.0,
                y: 500.0,
                kind: PointKind::OffCurve,
                ..Default::default()
            },
            Point {
                x: 500.0,
                y: 0.0,
                kind: PointKind::QCurve,
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unicode.sfd");
    write_sfd(&font, &master, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("Layer: 1 1"));
    assert!(text.contains(" 100 500 100 500 500 0 c"));
    std::fs::write(
        &path,
        text.replace("BeginChars:", "CustomVendorInfo: 19\nBeginChars:")
            .replace("Width: 800", "Width: 810"),
    )
    .unwrap();
    let loaded = read_sfd(&path).unwrap();
    assert_eq!(loaded.info.family_name, font.info.family_name);
    assert_eq!(loaded.info.note, font.info.note);
    let glyph = loaded.glyph("字形").unwrap();
    assert_eq!(glyph.note, "路径 + 😀\n附注");
    assert_eq!(glyph.unicodes, vec![0x4e00, 0x4e8c]);
    assert_eq!(glyph.variation_selectors, vec![(0x4e00, 0xfe01)]);
    let layer = glyph.master_layer(&loaded.default_master().id).unwrap();
    assert_eq!(layer.width, 810.0);
    assert!(layer.contours[0].is_quadratic());
    write_sfd(&loaded, &loaded.default_master().id, &path).unwrap();
    assert!(std::fs::read_to_string(path)
        .unwrap()
        .contains("CustomVendorInfo: 19"));
}

#[test]
fn unapplied_drafts_and_imported_tables_fail_before_writes() {
    let mut font = Font::new_empty("Guard", 1000);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("guard.sfd");
    std::fs::write(&path, b"original").unwrap();
    font.features.source_of_truth = fc_model::FeaturesSource::ImportedTables;
    assert!(matches!(
        write_sfd(&font, &font.default_master().id, &path),
        Err(fc_formats::Error::ImportedLayoutRequiresConversion)
    ));
    font.features.source_of_truth = fc_model::FeaturesSource::Fea;
    let mut source = font.features.clone();
    source.features.push(FeatureBlock {
        tag: "liga".into(),
        code: "invalid".into(),
        ..Default::default()
    });
    font.feature_draft = Some(fc_model::FeatureDraft {
        source,
        settings: font.feature_settings.clone(),
        ..Default::default()
    });
    assert!(matches!(
        write_sfd(&font, &font.default_master().id, &path),
        Err(fc_formats::Error::UnappliedFeatureDraft)
    ));
    assert_eq!(std::fs::read(path).unwrap(), b"original");
}

#[test]
fn shadow_cannot_replace_native_fields() {
    let mut font = Font::new_empty("Shadow", 1000);
    let dir = tempfile::tempdir().unwrap();
    for field in [
        "FamilyName: stale",
        "HheadAscent: 999",
        "OS2CapHeight: 999",
        "TTFWeight: 900",
        "UnderlineWidth: 900",
    ] {
        font.extra.lib.insert(
            "org.fontcarver.sfd.shadow".into(),
            serde_json::json!([field]),
        );
        assert!(
            write_sfd(
                &font,
                &font.default_master().id,
                &dir.path().join("font.sfd")
            )
            .is_err(),
            "{field}"
        );
    }
}

#[test]
fn additive_kern_lookups_cannot_collapse_into_one_pair() {
    let source = source("Kerns2: 0 -30 \"k1\"\nKerns2: 0 -40 \"k2\"").replace("BeginChars:","Lookup: 258 0 0 \"k1\" {\"k1\"} ['kern' ('DFLT' <'dflt'>)]\nLookup: 258 0 0 \"k2\" {\"k2\"} ['kern' ('DFLT' <'dflt'>)]\nBeginChars:");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("additive.sfd");
    std::fs::write(&path, source).unwrap();
    let error = read_sfd(&path).unwrap_err().to_string();
    assert!(error.contains("multiple kerning lookups"), "{error}");
}

#[test]
fn mark_to_mark_roles_are_rejected_instead_of_becoming_base_anchors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mark.sfd");
    std::fs::write(&path, source("AnchorPoint: \"top\" 250 700 basemark 0")).unwrap();
    assert!(read_sfd(&path)
        .unwrap_err()
        .to_string()
        .contains("mark-to-mark"));
}
