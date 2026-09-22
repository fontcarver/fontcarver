mod common;
use fc_model::CidInfo;
use read_fonts::{FontRef, TableProvider};
use skrifa::MetadataProvider;

#[test]
fn cid_keyed_export_preserves_non_identity_cids_and_unicode_metrics() {
    let mut font = common::scratch_font();
    font.cid = Some(CidInfo {
        registry: "Adobe".into(),
        ordering: "Identity".into(),
        supplement: 0,
        cid_map: [
            (0, ".notdef".into()),
            (12, "space".into()),
            (300, "A".into()),
            (500, "V".into()),
        ]
        .into(),
    });
    let report = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            format: fc_compile::ExportFormat::Otf,
            ..Default::default()
        },
    )
    .unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    assert!(face.cff().is_ok());
    assert_eq!(
        face.glyph_metrics(
            skrifa::instance::Size::unscaled(),
            skrifa::instance::LocationRef::default()
        )
        .advance_width(face.charmap().map('A').unwrap()),
        Some(600.0)
    );
    assert_eq!(
        fc_compile::shaping_fingerprint(&report.bytes, "AV").unwrap(),
        vec![(2, 550, 0), (3, 600, 0)]
    );
    if let Ok(path) = std::env::var("FC_CID_TEST_OUTPUT") {
        std::fs::write(path, &report.bytes).unwrap();
    }
}

#[test]
fn incomplete_or_duplicate_cid_maps_fail_without_falling_back_to_names() {
    let mut font = common::scratch_font();
    font.cid = Some(CidInfo {
        registry: "Adobe".into(),
        ordering: "Identity".into(),
        ..Default::default()
    });
    assert!(fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            format: fc_compile::ExportFormat::Otf,
            ..Default::default()
        }
    )
    .is_err());
    font.cid.as_mut().unwrap().cid_map = [(0, "A".into()), (1, "A".into())].into();
    assert!(fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            format: fc_compile::ExportFormat::Otf,
            ..Default::default()
        }
    )
    .is_err());
}
