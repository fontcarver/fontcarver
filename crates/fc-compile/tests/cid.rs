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
    let reopened = fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default())
        .expect("reopen CID-keyed OTF")
        .fonts
        .remove(0);
    let cid = reopened.cid.as_ref().expect("retain CID metadata");
    assert_eq!(cid.registry, "Adobe");
    assert_eq!(cid.ordering, "Identity");
    assert_eq!(cid.supplement, 0);
    assert_eq!(
        cid.cid_map.keys().copied().collect::<Vec<_>>(),
        [0, 12, 300, 500]
    );
    assert_eq!(cid.cid_map[&0].as_ref(), ".notdef");
    // CID-keyed CFF does not store glyph names. Match the re-imported names
    // through cmap so the assertion verifies CID identity rather than spelling.
    for (code, unicode, width) in [(12, 0x20, 250.0), (300, 0x41, 600.0), (500, 0x56, 600.0)] {
        let glyph = reopened.glyph(&cid.cid_map[&code]).expect("mapped glyph");
        assert_eq!(glyph.unicodes, [unicode]);
        assert_eq!(glyph.layers[0].width, width);
    }
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
        cid_map: [
            (0, ".notdef".into()),
            (12, "space".into()),
            (300, "A".into()),
        ]
        .into(),
        ..Default::default()
    });
    let error = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            format: fc_compile::ExportFormat::Otf,
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("exported glyph V has no CID mapping"),
        "{error}"
    );
    font.cid.as_mut().unwrap().cid_map = [
        (0, ".notdef".into()),
        (12, "space".into()),
        (300, "A".into()),
        (301, "A".into()),
        (500, "V".into()),
    ]
    .into();
    let error = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            format: fc_compile::ExportFormat::Otf,
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("glyph mappings must be unique"), "{error}");
}
