use fc_formats::metrics::pfm::{
    read_pfm, read_pfm_path, write_pfm, write_pfm_font, PfmExtendedMetrics, PfmHeader, PfmKernPair,
    PfmMetrics,
};
use fc_model::{FeatureBlock, Font, Glyph, KernSide};

fn source() -> PfmMetrics {
    PfmMetrics {
        copyright: {
            let mut bytes = vec![0; 60];
            bytes[0] = b'C';
            bytes[1] = 0x81;
            bytes[59] = 0xaa;
            bytes
        },
        device_name: b"PostScript".to_vec(),
        face_name: vec![b'F', 0xe9],
        postscript_name: b"OriginalPS".to_vec(),
        header: PfmHeader {
            points: 12,
            vertical_resolution: 300,
            horizontal_resolution: 300,
            ascent: 730,
            internal_leading: 15,
            external_leading: 27,
            italic: true,
            underline: true,
            strikeout: true,
            weight: 600,
            charset: 222,
            pixel_width: 0,
            pixel_height: 1000,
            pitch_and_family: 0x21,
            average_width: 575,
            maximum_width: 640,
            default_char: 1,
            break_char: 0,
        },
        extended: PfmExtendedMetrics {
            master_units: 1000,
            point_size: 120,
            master_height: 1000,
            cap_height: 700,
            x_height: 500,
            lowercase_ascent: 690,
            lowercase_descent: 210,
            underline_offset: 85,
            underline_width: 42,
            slant: 100,
            ..Default::default()
        },
        first_char: 0x80,
        widths: vec![610, 640],
        kerning: vec![PfmKernPair {
            left: 0x80,
            right: 0x81,
            value: -45,
        }],
    }
}

fn imported(dir: &std::path::Path) -> (Font, PfmMetrics) {
    let path = dir.join("source.pfm");
    let metrics = source();
    std::fs::write(&path, write_pfm(&metrics).unwrap()).unwrap();
    (read_pfm_path(&path).unwrap(), metrics)
}

fn ascii_font() -> Font {
    let mut font = Font::new_empty("Metrics", 1000);
    font.info.postscript_font_name = Some("Metrics-Regular".into());
    for (name, unicode, width) in [("A", 65, 600.0), ("V", 86, 650.0)] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = vec![unicode];
        font.insert_glyph(glyph).unwrap();
        font.glyph_mut(name).unwrap().layers[0].width = width;
    }
    font
}

#[test]
fn imported_byte_encoding_and_all_header_semantics_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let (font, original) = imported(dir.path());
    assert_eq!(font.info.family_name, "Fé");
    assert!(font.glyphs.values().all(|g| g.unicodes.is_empty()));
    assert_eq!(
        font.glyph("pfm80").unwrap().user_data["fontcarver.pfm.code"],
        128
    );
    assert!(font.user_data.get("fontcarver.pfm").is_some());
    let output = dir.path().join("output.pfm");
    write_pfm_font(&font, &font.default_master().id, &output).unwrap();
    assert_eq!(read_pfm(&std::fs::read(output).unwrap()).unwrap(), original);
}

#[test]
fn edits_use_current_widths_kerning_names_and_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let (mut font, original) = imported(dir.path());
    font.rename_glyph("pfm80", "renamed").unwrap();
    font.glyph_mut("renamed").unwrap().layers[0].width = 700.0;
    font.info.family_name = "Edited Family".into();
    font.info.postscript_font_name = Some("EditedPS".into());
    font.masters[0].metrics.ascender = 740.0;
    font.masters[0].metrics.descender = -220.0;
    font.masters[0].metrics.cap_height = 710.0;
    font.masters[0].kerning_h.0.insert(
        (
            KernSide::Glyph("renamed".into()),
            KernSide::Glyph("pfm81".into()),
        ),
        -90.0,
    );
    let path = dir.path().join("edited.pfm");
    write_pfm_font(&font, &font.default_master().id, &path).unwrap();
    let edited = read_pfm(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(edited.widths, vec![700, 640]);
    assert_eq!(edited.kerning[0].value, -90);
    assert_eq!(edited.face_name, b"Edited Family");
    assert_eq!(edited.postscript_name, b"EditedPS");
    assert_eq!(edited.header.ascent, 740);
    assert_eq!(edited.extended.lowercase_descent, 220);
    assert_eq!(edited.extended.cap_height, 710);
    assert_eq!(edited.header.charset, original.header.charset);
    assert_eq!(
        edited.header.internal_leading,
        original.header.internal_leading
    );
    assert_eq!(
        edited.header.pitch_and_family,
        original.header.pitch_and_family
    );
    assert_eq!(
        edited.extended.lowercase_ascent,
        original.extended.lowercase_ascent
    );
}

#[test]
fn new_ascii_font_exports_sparse_byte_widths_and_pair_kerning() {
    let dir = tempfile::tempdir().unwrap();
    let mut font = ascii_font();
    font.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
        -80.0,
    );
    let path = dir.path().join("new.pfm");
    write_pfm_font(&font, &font.default_master().id, &path).unwrap();
    let metrics = read_pfm(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(metrics.header.charset, 0);
    assert_eq!(metrics.first_char, 65);
    assert_eq!(metrics.widths[0], 600);
    assert_eq!(metrics.widths[21], 650);
    assert_eq!(metrics.widths[1], 0);
    assert_eq!(
        metrics.kerning,
        vec![PfmKernPair {
            left: 65,
            right: 86,
            value: -80
        }]
    );
}

#[test]
fn invalid_numeric_and_byte_assignments_preserve_existing_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keep.pfm");
    let base = ascii_font();
    let mut cases = vec![];
    for width in [f64::NAN, f64::INFINITY, -1.0, 65536.0] {
        let mut font = base.clone();
        font.glyph_mut("A").unwrap().layers[0].width = width;
        cases.push(font);
    }
    let mut non_ascii = base.clone();
    non_ascii.glyph_mut("A").unwrap().unicodes = vec![0x100];
    cases.push(non_ascii);
    let mut duplicate = base;
    duplicate.glyph_mut("V").unwrap().unicodes = vec![65];
    cases.push(duplicate);
    for font in cases {
        std::fs::write(&path, b"untouched").unwrap();
        assert!(write_pfm_font(&font, &font.default_master().id, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"untouched");
    }
}

#[test]
fn layout_class_kerning_and_new_non_ansi_charset_are_rejected_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keep.pfm");
    let base = ascii_font();
    let mut layout = base.clone();
    layout.features.features.push(FeatureBlock {
        tag: "liga".into(),
        code: "sub A V by A;".into(),
        ..Default::default()
    });
    let mut class = base.clone();
    class.masters[0].kerning_h.0.insert(
        (KernSide::Group("left".into()), KernSide::Glyph("V".into())),
        -20.0,
    );
    let mut charset = base;
    charset.user_data = serde_json::json!({"fontcarver.pfm.charset": 2});
    for font in [layout, class, charset] {
        std::fs::write(&path, b"untouched").unwrap();
        assert!(write_pfm_font(&font, &font.default_master().id, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"untouched");
    }
}

#[test]
fn raw_non_ascii_names_are_preserved_but_unsupported_edits_fail_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let (font, _) = imported(dir.path());
    let path = dir.path().join("keep.pfm");
    let mut invalid_name = font.clone();
    invalid_name.info.family_name = "Editedé".into();
    let mut bad_code = font.clone();
    bad_code.glyph_mut("pfm80").unwrap().user_data =
        serde_json::json!({"fontcarver.pfm.code": 256});
    let mut missing_code = font.clone();
    missing_code.glyph_mut("pfm80").unwrap().user_data = serde_json::Value::Null;
    let mut invalid_source = font;
    invalid_source.user_data = serde_json::json!({"fontcarver.pfm": "corrupt"});
    for font in [invalid_name, bad_code, missing_code, invalid_source] {
        std::fs::write(&path, b"untouched").unwrap();
        assert!(write_pfm_font(&font, &font.default_master().id, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"untouched");
    }
}

#[test]
fn deleting_a_pair_and_excluding_a_glyph_update_imported_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let (mut font, original) = imported(dir.path());
    font.masters[0].kerning_h.0.clear();
    font.glyph_mut("pfm80").unwrap().export = false;
    let path = dir.path().join("edited.pfm");
    write_pfm_font(&font, &font.default_master().id, &path).unwrap();
    let edited = read_pfm(&std::fs::read(path).unwrap()).unwrap();
    assert!(edited.kerning.is_empty());
    assert_eq!(edited.widths, vec![0, 640]);
    assert_eq!(edited.first_char, original.first_char);
    assert_eq!(edited.header.default_char, original.header.default_char);
    assert_eq!(edited.header.break_char, original.header.break_char);
}

#[test]
fn changing_fixed_pitch_to_proportional_allows_new_advances() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("fixed.pfm");
    let mut metrics = source();
    metrics.widths = vec![600, 600];
    metrics.header.pixel_width = 600;
    metrics.header.pitch_and_family &= !1;
    std::fs::write(&input, write_pfm(&metrics).unwrap()).unwrap();
    let mut font = read_pfm_path(&input).unwrap();
    font.info.postscript_is_fixed_pitch = Some(false);
    font.glyph_mut("pfm80").unwrap().layers[0].width = 700.0;
    let output = dir.path().join("proportional.pfm");
    write_pfm_font(&font, &font.default_master().id, &output).unwrap();
    let edited = read_pfm(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(edited.header.pixel_width, 0);
    assert_eq!(edited.header.pitch_and_family & 1, 1);
    assert_eq!(edited.widths, vec![700, 600]);
}

#[test]
fn changed_fixed_pitch_advances_require_consistency_even_without_pixel_width() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("fixed.pfm");
    let mut metrics = source();
    metrics.widths = vec![600, 600];
    metrics.header.pitch_and_family &= !1;
    std::fs::write(&input, write_pfm(&metrics).unwrap()).unwrap();
    let mut font = read_pfm_path(&input).unwrap();
    font.glyph_mut("pfm80").unwrap().layers[0].width = 700.0;
    let output = dir.path().join("keep.pfm");
    std::fs::write(&output, b"untouched").unwrap();
    assert!(write_pfm_font(&font, &font.default_master().id, &output).is_err());
    assert_eq!(std::fs::read(output).unwrap(), b"untouched");
}

#[test]
fn clearing_optional_imported_metrics_clears_their_pfm_values() {
    let dir = tempfile::tempdir().unwrap();
    let (mut font, _) = imported(dir.path());
    font.info.postscript_underline_position = None;
    font.info.postscript_underline_thickness = None;
    font.info.open_type_os2_strikeout_position = None;
    font.info.open_type_os2_strikeout_size = None;
    let output = dir.path().join("cleared.pfm");
    write_pfm_font(&font, &font.default_master().id, &output).unwrap();
    let edited = read_pfm(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(edited.extended.underline_offset, 0);
    assert_eq!(edited.extended.underline_width, 0);
    assert_eq!(edited.extended.strikeout_offset, 0);
    assert_eq!(edited.extended.strikeout_width, 0);
}

#[test]
fn vertical_master_metrics_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut font = ascii_font();
    font.masters[0].metrics.default_vertical_advance = Some(1000.0);
    let output = dir.path().join("keep.pfm");
    std::fs::write(&output, b"untouched").unwrap();
    assert!(write_pfm_font(&font, &font.default_master().id, &output).is_err());
    assert_eq!(std::fs::read(output).unwrap(), b"untouched");
}
