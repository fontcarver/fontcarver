mod common;

use fc_compile::{export, ExportFormat, ExportOptions};
use fc_model::{FeatureBlock, FeatureDraft};

#[test]
fn unapplied_feature_draft_blocks_every_binary_export() {
    let mut font = common::scratch_font();
    let mut source = font.features.clone();
    source.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "pos A V -30;".into(),
        ..FeatureBlock::default()
    });
    font.feature_draft = Some(FeatureDraft {
        source,
        settings: font.feature_settings.clone(),
        revision: 1,
    });
    for format in [ExportFormat::Ttf, ExportFormat::Woff, ExportFormat::Woff2] {
        let error = export(
            &font,
            &|_| 0,
            &ExportOptions {
                format,
                ..ExportOptions::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("draft"), "{error}");
    }
}

#[test]
fn identical_saved_draft_keeps_binary_export_available() {
    let mut font = common::scratch_font();
    font.feature_draft = Some(FeatureDraft {
        source: font.features.clone(),
        settings: font.feature_settings.clone(),
        revision: 1,
    });
    assert!(export(&font, &|_| 0, &ExportOptions::default()).is_ok());
}
