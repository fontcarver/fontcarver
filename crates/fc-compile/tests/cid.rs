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

/// Row 110: a CID-keyed font with two FontDICTs must build a real FDArray and a
/// FDSelect that routes each glyph to its own private dictionary.
#[test]
fn cid_keyed_export_writes_an_fdarray_and_fdselect_for_two_font_dicts() {
    let mut font = common::scratch_font();
    font.cid = Some(CidInfo {
        registry: "Adobe".into(),
        ordering: "Japan1".into(),
        supplement: 6,
        cid_map: [
            (0, ".notdef".into()),
            (1, "space".into()),
            (34, "A".into()),
            (59, "V".into()),
        ]
        .into(),
    });
    let master = font.masters[0].id.clone();
    // FD 0 keeps the master's own dictionary; FD 1 is the CJK dictionary that a
    // CID-keyed font gives its ideographs, marked by LanguageGroup 1.
    font.masters[0].private_dict.std_vw = Some(88.0);
    font.masters[0].private_dict.blue_values = vec![-12.0, 0.0, 700.0, 712.0];
    font.masters[0].cff_private_dicts = vec![fc_model::PsPrivate {
        language_group: Some(1),
        std_vw: Some(44.0),
        std_hw: Some(42.0),
        ..Default::default()
    }];
    for name in ["A", "V"] {
        let layer = font
            .glyph_mut(name)
            .unwrap()
            .master_layer_mut(&master)
            .unwrap();
        layer.cff_private_dict_index = Some(1);
    }
    let report = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            format: fc_compile::ExportFormat::Otf,
            ..Default::default()
        },
    )
    .expect("export two-FD CID-keyed OTF");
    let face = FontRef::new(&report.bytes).unwrap();
    let data = face
        .table_data(read_fonts::types::Tag::new(b"CFF "))
        .expect("CFF table");
    let cff = read_fonts::ps::cff::CffFontRef::new_cff(data.as_bytes(), 0, Some(1000))
        .expect("parse CID-keyed CFF");
    assert!(cff.is_cid(), "expected a CID-keyed CFF");
    // FDSelect routes each glyph to its own FontDICT, and both dictionaries are
    // reachable with the private values they were given.
    for (gid, expected) in [(0u32, 0usize), (1, 0), (2, 1), (3, 1)] {
        let index = cff
            .subfont_index(read_fonts::types::GlyphId::new(gid))
            .expect("FDSelect entry");
        assert_eq!(index as usize, expected, "glyph {gid} selected FD {index}");
        assert!(cff.subfont(index, &[]).is_ok(), "FD {index} is unreadable");
    }
    assert!(
        cff.subfont_index(read_fonts::types::GlyphId::new(2))
            != cff.subfont_index(read_fonts::types::GlyphId::new(1)),
        "FDSelect collapsed the two FontDICTs"
    );
    // Reopening keeps both dictionaries and the per-glyph selection.
    let reopened = fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default())
        .expect("reopen two-FD CID-keyed OTF")
        .fonts
        .remove(0);
    let reopened_master = &reopened.masters[0];
    assert_eq!(reopened_master.cff_private_dicts.len(), 1);
    assert_eq!(reopened_master.private_dict.std_vw, Some(88.0));
    assert_eq!(reopened_master.cff_private_dicts[0].std_vw, Some(44.0));
    assert_eq!(reopened_master.cff_private_dicts[0].language_group, Some(1));
    let cid = reopened.cid.as_ref().expect("retain CID metadata");
    assert_eq!(cid.ordering, "Japan1");
    assert_eq!(cid.supplement, 6);
    for (code, index) in [(34u32, Some(1u8)), (59, Some(1)), (1, None)] {
        let layer = &reopened.glyph(&cid.cid_map[&code]).unwrap().layers[0];
        assert_eq!(
            layer.cff_private_dict_index.filter(|value| *value != 0),
            index,
            "CID {code} lost its FontDICT selection"
        );
    }
    if let Ok(path) = std::env::var("FC_CID_TWO_FD_OUTPUT") {
        std::fs::write(path, &report.bytes).unwrap();
    }
}
