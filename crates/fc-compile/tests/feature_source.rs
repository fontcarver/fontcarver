mod common;

use fc_compile::{export, shaping_fingerprint, ExportOptions};
use fc_model::{FeatureBlock, FeatureClass};

#[test]
fn assembled_source_preserves_prefix_lookups_classes_and_disabled_blocks() {
    for class_code in ["A # member\n", "[A # member\n]"] {
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
