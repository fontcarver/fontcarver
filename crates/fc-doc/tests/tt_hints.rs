mod common;

use fc_doc::{command::HintCommand, Command, Session};
use fc_model::TtMasterData;

fn globals() -> TtMasterData {
    TtMasterData {
        cvt: vec![40],
        fpgm: vec![0],
        prep: vec![1],
        max_zones: Some(2),
        max_stack_elements: Some(32),
        max_storage: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_twilight_points: Some(0),
        ..TtMasterData::default()
    }
}

#[test]
fn authored_programs_and_globals_are_one_undo_step_and_survive_geometry_edits() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            let master = doc.font.default_master().id.clone();
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let fingerprint = format!("tt-points-v1:{}", "a".repeat(64));
            doc.apply(
                Command::Batch(vec![
                    Command::Hints(HintCommand::Tt {
                        glyph: "base".into(),
                        layer: layer.clone(),
                        program: vec![0x00, 0xb0, 0, 0x2f],
                        fingerprint,
                    }),
                    Command::Hints(HintCommand::TtGlobals {
                        master,
                        data: globals(),
                    }),
                ]),
                "Edit TrueType programs",
            )
            .unwrap();
            let authored = doc.font.clone();
            assert!(
                authored.glyph("base").unwrap().layers[0]
                    .tt_binding
                    .as_ref()
                    .unwrap()
                    .authored
            );
            assert_eq!(
                authored.default_master().tt.authored_units_per_em,
                Some(authored.info.units_per_em)
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, authored);
            let mut contours = authored.glyph("base").unwrap().layers[0].contours.clone();
            contours[0].points[1].x += 30.0;
            doc.apply(
                Command::ReplaceLayerOutline {
                    glyph: "base".into(),
                    layer,
                    contours,
                    components: vec![],
                    anchors: vec![],
                },
                "Change outline",
            )
            .unwrap();
            let edited = &doc.font.glyph("base").unwrap().layers[0];
            let original = &authored.glyph("base").unwrap().layers[0];
            assert_eq!(edited.tt_instructions, original.tt_instructions);
            assert_eq!(edited.tt_binding, original.tt_binding);
            doc.undo().unwrap();
            assert_eq!(doc.font, authored);
            let directory = common::temp_dir("tt-bindings");
            for name in ["authored.fcf", "authored.fcf.gz"] {
                let path = directory.join(name);
                fc_doc::fcf::write(&path, &doc.font).unwrap();
                assert_eq!(fc_doc::fcf::read(&path).unwrap(), authored);
            }
            std::fs::remove_dir_all(directory).unwrap();
        })
        .unwrap();
}

#[test]
#[cfg(feature = "sfnt")]
fn imported_bindings_survive_native_reopen_without_original_font_bytes() {
    let result = fc_sfnt::read_path(
        &fc_fixtures::path("DejaVuSans.ttf"),
        &fc_sfnt::ReadOptions {
            keep_original_bytes: false,
            ..fc_sfnt::ReadOptions::default()
        },
    )
    .unwrap();
    let font = &result.fonts[0];
    let layer = &font.glyph("A").unwrap().layers[0];
    let binding = layer.tt_binding.as_ref().unwrap();
    assert!(!binding.authored);
    assert!(font.extra.source.as_ref().unwrap().original_bytes.is_none());
    let directory = common::temp_dir("tt-import-bindings");
    let path = directory.join("imported.fcf");
    fc_doc::fcf::write(&path, font).unwrap();
    let restored = fc_doc::fcf::read(&path).unwrap();
    let restored_layer = &restored.glyph("A").unwrap().layers[0];
    assert_eq!(restored_layer.tt_binding, layer.tt_binding);
    assert_eq!(restored_layer.tt_instructions, layer.tt_instructions);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_global_or_glyph_edits_roll_back_the_complete_batch() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            let generation = doc.generation;
            let master = doc.font.default_master().id.clone();
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let fingerprint = format!("tt-points-v1:{}", "a".repeat(64));
            let bad = TtMasterData {
                max_zones: Some(3),
                ..globals()
            };
            assert!(doc
                .apply(
                    Command::Batch(vec![
                        Command::Hints(HintCommand::Tt {
                            glyph: "base".into(),
                            layer: layer.clone(),
                            program: vec![0],
                            fingerprint: fingerprint.clone()
                        }),
                        Command::Hints(HintCommand::TtGlobals { master, data: bad }),
                    ]),
                    "Invalid batch"
                )
                .is_err());
            for (program, fingerprint) in [(vec![0xb0], fingerprint), (vec![0], "invalid".into())] {
                assert!(doc
                    .apply(
                        Command::Hints(HintCommand::Tt {
                            glyph: "base".into(),
                            layer: layer.clone(),
                            program,
                            fingerprint
                        }),
                        "Invalid program"
                    )
                    .is_err());
            }
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, generation);
        })
        .unwrap();
}
