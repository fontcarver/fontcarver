use fc_formats::ufo::{read_ufo, write_designspace, write_ufo, UfoWriteOptions};
use fc_model::{FeatureBlock, FeatureDraft, FeaturesSource, Font};

fn source_font() -> Font {
    read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap()
}

fn assert_source_writers_reject_without_touching(font: &Font, message: &str) {
    let directory = tempfile::tempdir().unwrap();
    for preexisting in [false, true] {
        for designspace in [false, true] {
            let path = directory
                .path()
                .join(format!("output-{preexisting}-{designspace}"));
            if preexisting {
                std::fs::create_dir(&path).unwrap();
                std::fs::write(path.join("sentinel"), b"keep me").unwrap();
            }
            let error = if designspace {
                write_designspace(font, &path).unwrap_err()
            } else {
                write_ufo(
                    font,
                    &font.default_master().id,
                    &path,
                    &UfoWriteOptions::default(),
                )
                .unwrap_err()
            };
            assert!(error.to_string().contains(message), "{error}");
            if preexisting {
                assert_eq!(std::fs::read(path.join("sentinel")).unwrap(), b"keep me");
                assert_eq!(std::fs::read_dir(&path).unwrap().count(), 1);
            } else {
                assert!(!path.exists());
            }
        }
    }
}

#[test]
fn imported_layout_requires_conversion_before_source_export() {
    let mut font = source_font();
    font.features.source_of_truth = FeaturesSource::ImportedTables;
    assert_source_writers_reject_without_touching(&font, "conversion");
}

#[test]
fn unapplied_drafts_block_source_export_before_any_output_changes() {
    let mut font = source_font();
    let mut source = font.features.clone();
    source.prefixes.push(FeatureBlock {
        code: "invalid {".into(),
        ..FeatureBlock::default()
    });
    font.feature_draft = Some(FeatureDraft {
        source,
        settings: font.feature_settings.clone(),
        revision: 1,
    });
    assert_source_writers_reject_without_touching(&font, "draft");
}

#[test]
fn identical_saved_draft_does_not_block_source_export() {
    let mut font = source_font();
    font.feature_draft = Some(FeatureDraft {
        source: font.features.clone(),
        settings: font.feature_settings.clone(),
        revision: 1,
    });
    let directory = tempfile::tempdir().unwrap();
    write_ufo(
        &font,
        &font.default_master().id,
        &directory.path().join("ok.ufo"),
        &UfoWriteOptions::default(),
    )
    .unwrap();
}
