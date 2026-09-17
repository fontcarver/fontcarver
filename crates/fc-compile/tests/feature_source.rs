mod common;

use fc_compile::{export, shaping_fingerprint, ExportOptions};
use fc_model::{FeatureBlock, FeatureClass};

#[test]
fn assembled_source_preserves_prefix_lookups_classes_and_disabled_blocks() {
    for class_code in [
        "A # member\n",
        "[A # member\n]",
        "A # member",
        "[A] # member",
    ] {
        let mut font = common::scratch_font();
        font.masters[0].kerning_h.0.clear();
        font.features.prefixes.push(FeatureBlock {
            tag: "Spacing".into(),
            code: "lookup Spacing { pos A V -80; } Spacing;".into(),
            ..FeatureBlock::default()
        });
        font.features.classes.push(FeatureClass {
            name: "@First".into(),
            code: class_code.into(),
            ..FeatureClass::default()
        });
        font.features.features.extend([
            FeatureBlock {
                tag: "kern".into(),
                code: "lookup Spacing;".into(),
                ..FeatureBlock::default()
            },
            FeatureBlock {
                tag: "calt".into(),
                code: "feature calt { sub @First @First' by V; } calt;".into(),
                ..FeatureBlock::default()
            },
            FeatureBlock {
                tag: "liga".into(),
                code: "invalid {".into(),
                disabled: true,
                ..FeatureBlock::default()
            },
        ]);
        let report = export(&font, &|_| 0, &ExportOptions::default()).unwrap();
        for text in ["AV", "AA"] {
            assert_eq!(
                shaping_fingerprint(&report.bytes, text).unwrap(),
                vec![(2, 520, 0), (3, 600, 0)],
                "{class_code:?}: {text}"
            );
        }
    }
}

#[test]
fn commented_complete_feature_blocks_compile_without_double_wrapping() {
    let mut font = common::scratch_font();
    font.masters[0].kerning_h.0.clear();
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "  # 中文 😀\r\nfeature\r\nkern { pos A V -80; } kern;\r\n".into(),
        ..FeatureBlock::default()
    });
    let report = export(&font, &|_| 0, &ExportOptions::default()).unwrap();
    assert_eq!(
        shaping_fingerprint(&report.bytes, "AV").unwrap(),
        vec![(2, 520, 0), (3, 600, 0)]
    );
}

#[test]
fn aalt_feature_references_remain_in_their_feature_body() {
    let mut font = common::scratch_font();
    font.features.features.extend([
        FeatureBlock {
            tag: "salt".into(),
            code: "sub A by V;".into(),
            ..FeatureBlock::default()
        },
        FeatureBlock {
            tag: "aalt".into(),
            code: "feature salt;".into(),
            ..FeatureBlock::default()
        },
    ]);
    let report = export(&font, &|_| 0, &ExportOptions::default()).unwrap();
    let face = read_fonts::FontRef::new(&report.bytes).unwrap();
    let data = harfrust::ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let mut buffer = harfrust::UnicodeBuffer::new();
    buffer.push_str("A");
    buffer.guess_segment_properties();
    let features = ["aalt=1".parse::<harfrust::Feature>().unwrap()];
    let shaped = shaper.shape(buffer, harfrust::ShapeOptions::new().features(&features));
    let expected = shaping_fingerprint(&report.bytes, "V").unwrap();
    assert_eq!(shaped.glyph_infos().len(), 1);
    assert_eq!(shaped.glyph_infos()[0].glyph_id, expected[0].0);
}
