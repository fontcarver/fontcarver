mod common;
use fc_compile::{export, ExportFormat, ExportOptions};
use fc_model::{
    ColorPalette, ColorTable, ColrLayer, Font, NamePlatform, NameRecord, PaletteColor, Tag,
};
use read_fonts::FontRef;

fn font() -> Font {
    let mut font = common::scratch_font();
    font.color = Some(ColorTable {
        graph: None,
        palette_version: 1,
        palettes: vec![
            ColorPalette {
                colors: vec![PaletteColor {
                    red: 240,
                    green: 20,
                    blue: 10,
                    alpha: 128,
                }],
                flags: 2,
                label: Some(256),
            },
            ColorPalette {
                colors: vec![PaletteColor {
                    red: 10,
                    green: 60,
                    blue: 220,
                    alpha: 255,
                }],
                flags: 1,
                label: None,
            },
        ],
        entry_labels: vec![Some(257)],
        glyphs: [(
            "A".into(),
            vec![
                ColrLayer {
                    glyph: "A".into(),
                    palette_index: Some(0),
                },
                ColrLayer {
                    glyph: "V".into(),
                    palette_index: None,
                },
            ],
        )]
        .into(),
    });
    for (name_id, text) in [(256, "Dark"), (257, "Accent")] {
        font.info.names.push(NameRecord {
            name_id,
            text: text.into(),
            platform: NamePlatform::Windows,
            language: "en-US".into(),
        });
    }
    font.color_tables_owned = true;
    font
}

#[test]
fn colors_export_in_all_containers_with_actual_names_and_labels() {
    let source = font();
    for format in [
        ExportFormat::Ttf,
        ExportFormat::Otf,
        ExportFormat::Woff,
        ExportFormat::Woff2,
    ] {
        let report = export(
            &source,
            &|_| 1,
            &ExportOptions {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        let output = fc_sfnt::read_bytes(&report.bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        assert_eq!(output.color, source.color, "{format:?}");
        assert!(output.color_tables_owned);
        assert!(output
            .info
            .names
            .iter()
            .any(|n| n.name_id == 256 && n.text == "Dark"));
        assert!(report
            .warnings
            .iter()
            .all(|w| !w.message.contains("COLR") && !w.message.contains("CPAL")));
    }
}

#[test]
fn renamed_and_excluded_references_use_export_order() {
    let mut source = font();
    source.rename_glyph("A", "custom.color.base").unwrap();
    source.rename_glyph("V", "custom.color.layer").unwrap();
    for production_names in [false, true] {
        let report = export(
            &source,
            &|_| 1,
            &ExportOptions {
                production_names,
                ..Default::default()
            },
        )
        .unwrap();
        let face = FontRef::new(&report.bytes).unwrap();
        let colr = face
            .table_data(read_fonts::types::Tag::new(b"COLR"))
            .unwrap();
        let cpal = face
            .table_data(read_fonts::types::Tag::new(b"CPAL"))
            .unwrap();
        let order = source.glyph_order().cloned().collect::<Vec<_>>();
        assert_eq!(
            fc_sfnt::color::read_tables(Some(colr.as_bytes()), cpal.as_bytes(), &order).unwrap(),
            source.color.clone().unwrap()
        );
    }
    source.glyph_mut("custom.color.layer").unwrap().export = false;
    let report = export(&source, &|_| 1, &Default::default()).unwrap();
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "color-references-dropped"));
    let output = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert_eq!(output.color.unwrap().glyphs["custom.color.base"].len(), 1);
    assert_eq!(source.color.unwrap().glyphs["custom.color.base"].len(), 2);
}

#[test]
fn deletion_and_table_selection_never_restore_stale_color_data() {
    let mut source = font();
    source.color = None;
    for tag in [Tag(*b"COLR"), Tag(*b"CPAL")] {
        source.extra.raw_tables.insert(tag, vec![0, 0]);
    }
    let report = export(&source, &|_| 0, &Default::default()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    for tag in [b"COLR", b"CPAL"] {
        assert!(face.table_data(read_fonts::types::Tag::new(tag)).is_none());
    }
    let source = font();
    let report = export(
        &source,
        &|_| 0,
        &ExportOptions {
            table_subset: Some(vec![Tag(*b"COLR")]),
            ..Default::default()
        },
    )
    .unwrap();
    let output = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert_eq!(output.color, source.color);
}

#[test]
fn invalid_source_is_rejected() {
    let mut source = font();
    source.color.as_mut().unwrap().palettes[0].colors.clear();
    assert!(export(&source, &|_| 0, &Default::default()).is_err());
}
