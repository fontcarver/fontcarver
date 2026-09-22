mod common;

use fc_compile::{
    compile_static_with_metadata, export, CompileOptions, ExportFormat, ExportOptions,
};
use fc_model::{Font, TtInstructionBinding, TtMasterData};

fn authored_font() -> Font {
    let mut font = common::scratch_font();
    let master = font.default_master().id.clone();
    let basis = compile_static_with_metadata(&font, &master, &CompileOptions::default()).unwrap();
    for name in ["A", "space"] {
        let id = basis
            .glyph_order
            .iter()
            .position(|n| n.as_ref() == name)
            .unwrap() as u16;
        let layer = font
            .glyph_mut(name)
            .unwrap()
            .master_layer_mut(&master)
            .unwrap();
        layer.tt_instructions = Some(vec![0x01, 0xb0, 0, 0x2f]);
        layer.tt_binding = Some(TtInstructionBinding {
            fingerprint: fc_sfnt::tt::point_basis(&basis.bytes, id)
                .unwrap()
                .fingerprint,
            authored: true,
        });
    }
    font.masters[0].tt = TtMasterData {
        authored_units_per_em: Some(font.info.units_per_em),
        cvt: vec![70],
        max_zones: Some(2),
        max_storage: Some(0),
        max_twilight_points: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(16),
        ..TtMasterData::default()
    };
    font
}

#[test]
fn authored_programs_export_even_when_imported_retention_is_off() {
    let font = authored_font();
    let before = font.clone();
    for format in [ExportFormat::Ttf, ExportFormat::Woff, ExportFormat::Woff2] {
        let result = export(
            &font,
            &|_| 1,
            &ExportOptions {
                format,
                keep_tt_instructions: false,
                ..Default::default()
            },
        )
        .unwrap();
        let imported = fc_sfnt::read_bytes(&result.bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        assert_eq!(imported.default_master().tt.cvt, vec![70]);
        for name in ["A", "space"] {
            assert_eq!(
                imported.glyph(name).unwrap().layers[0].tt_instructions,
                font.glyph(name).unwrap().layers[0].tt_instructions,
                "{format:?} {name}"
            );
        }
    }
    assert_eq!(font, before);
}

#[test]
fn source_and_metric_changes_reject_stale_bindings_but_keep_programs() {
    let original = authored_font();
    for change in 0..3 {
        let mut font = original.clone();
        match change {
            0 => font.glyph_mut("A").unwrap().layers[0].contours[0].points[0].x += 10.0,
            1 => font.glyph_mut("A").unwrap().layers[0].width += 10.0,
            _ => font.info.units_per_em += 1,
        }
        let before = font.clone();
        let error = export(&font, &|_| 1, &ExportOptions::default())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("stale") || error.contains("scale"),
            "{error}"
        );
        assert_eq!(font, before);
    }
}

#[test]
fn preprocessing_that_would_drop_authored_programs_is_rejected() {
    let font = authored_font();
    for options in [
        ExportOptions {
            remove_overlap: true,
            ..Default::default()
        },
        ExportOptions {
            autohint: true,
            ..Default::default()
        },
        ExportOptions {
            variable: true,
            ..Default::default()
        },
        ExportOptions {
            instance: Some(0),
            ..Default::default()
        },
    ] {
        let message = export(&font, &|_| 1, &options).unwrap_err().to_string();
        assert!(message.contains("authored TrueType"), "{message}");
    }
}

#[test]
fn imported_programs_are_retained_only_when_requested_and_still_bound() {
    let mut font = authored_font();
    let master = font.default_master().id.clone();
    let compiled =
        compile_static_with_metadata(&font, &master, &CompileOptions::default()).unwrap();
    let gid = compiled
        .glyph_order
        .iter()
        .position(|name| name.as_ref() == "V")
        .unwrap() as u16;
    let layer = &mut font.glyph_mut("V").unwrap().layers[0];
    layer.tt_instructions = Some(vec![0]);
    layer.tt_binding = Some(TtInstructionBinding {
        authored: false,
        fingerprint: fc_sfnt::tt::point_basis(&compiled.bytes, gid)
            .unwrap()
            .fingerprint,
    });
    for keep in [false, true] {
        let result = export(
            &font,
            &|_| 1,
            &ExportOptions {
                keep_tt_instructions: keep,
                ..Default::default()
            },
        )
        .unwrap();
        let imported = fc_sfnt::read_bytes(&result.bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        assert_eq!(
            imported.glyph("V").unwrap().layers[0].tt_instructions,
            keep.then_some(vec![0])
        );
    }
    font.glyph_mut("V").unwrap().layers[0].width += 10.0;
    let result = export(&font, &|_| 1, &ExportOptions::default()).unwrap();
    assert!(result
        .warnings
        .iter()
        .any(|w| w.code == "tt-imported-programs-omitted"));
    let imported = fc_sfnt::read_bytes(&result.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert!(imported.glyph("V").unwrap().layers[0]
        .tt_instructions
        .is_none());
    assert!(imported.glyph("A").unwrap().layers[0]
        .tt_instructions
        .is_some());
}

#[test]
fn component_source_edits_invalidate_the_authored_composite_program() {
    let mut font = authored_font();
    // Keep A as an ordinary source so the failure must come from the dependent.
    font.glyph_mut("A").unwrap().layers[0].tt_instructions = None;
    font.glyph_mut("A").unwrap().layers[0].tt_binding = None;
    let master = font.default_master().id.clone();
    let mut glyph = fc_model::Glyph::new("A.comp");
    let mut layer = fc_model::Layer::new_master(master.clone());
    layer.width = 600.0;
    layer.components.push(fc_model::Component {
        base: "A".into(),
        ..Default::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let compiled =
        compile_static_with_metadata(&font, &master, &CompileOptions::default()).unwrap();
    let gid = compiled
        .glyph_order
        .iter()
        .position(|name| name.as_ref() == "A.comp")
        .unwrap() as u16;
    let layer = &mut font.glyph_mut("A.comp").unwrap().layers[0];
    layer.tt_binding = Some(TtInstructionBinding {
        authored: true,
        fingerprint: fc_sfnt::tt::point_basis(&compiled.bytes, gid)
            .unwrap()
            .fingerprint,
    });
    layer.tt_instructions = Some(vec![0]);
    export(&font, &|_| 1, &ExportOptions::default()).unwrap();
    font.glyph_mut("A").unwrap().layers[0].contours[0].points[0].x += 20.0;
    let error = export(&font, &|_| 1, &ExportOptions::default())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("stale") && error.contains("A.comp"),
        "{error}"
    );
}

#[test]
fn cff_output_discloses_authored_tt_loss_and_table_subsets_keep_required_tt_data() {
    let font = authored_font();
    let cff = export(
        &font,
        &|_| 1,
        &ExportOptions {
            format: ExportFormat::Otf,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(cff
        .warnings
        .iter()
        .any(|w| w.code == "tt-authored-programs-omitted"));
    let subset = export(
        &font,
        &|_| 1,
        &ExportOptions {
            table_subset: Some(vec![]),
            production_names: true,
            ..Default::default()
        },
    )
    .unwrap();
    let imported = fc_sfnt::read_bytes(&subset.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert_eq!(imported.default_master().tt.cvt, vec![70]);
    assert_eq!(
        imported.glyph("A").unwrap().layers[0].tt_instructions,
        Some(vec![1, 0xb0, 0, 0x2f])
    );
}

#[test]
fn cff_preprocessing_still_discloses_glyph_only_authored_programs() {
    let mut font = authored_font();
    font.masters[0].tt.authored_units_per_em = None;
    let before = font.clone();
    let result = export(
        &font,
        &|_| 1,
        &ExportOptions {
            format: ExportFormat::Otf,
            remove_overlap: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        result
            .warnings
            .iter()
            .filter(|w| w.code == "tt-authored-programs-omitted")
            .count(),
        1
    );
    assert_eq!(font, before);
}
