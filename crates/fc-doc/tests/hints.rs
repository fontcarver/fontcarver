mod common;
use fc_doc::{command::HintCommand, Command, Session};
use fc_model::PsHints;

#[test]
fn hint_invalidation_stays_within_the_edited_master_and_ignores_background_sources() {
    let mut font = common::test_font();
    let first = font.default_master().id.clone();
    let mut second = font.default_master().clone();
    second.id = fc_model::MasterId::new();
    second.name = "Bold".into();
    let second_id = second.id.clone();
    font.masters.push(second);
    for glyph in font.glyphs.values_mut() {
        let mut layer = glyph.layers[0].clone();
        layer.id = fc_model::LayerId::new();
        layer.kind = fc_model::LayerKind::Master(second_id.clone());
        glyph.layers.push(layer);
    }
    let mut background = font.glyph("base").unwrap().layers[0].clone();
    background.id = fc_model::LayerId::new();
    background.kind = fc_model::LayerKind::Background(first.clone());
    let background_id = background.id.clone();
    font.glyph_mut("base").unwrap().layers.push(background);
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            for master in [&first, &second_id] {
                doc.apply(
                    Command::Hints(HintCommand::AutohintPs {
                        master: master.clone(),
                        glyphs: vec!["base".into(), "composite".into()],
                    }),
                    "Hint master",
                )
                .unwrap();
            }
            let hinted = doc.font.clone();
            for layer_id in [
                background_id,
                hinted.glyph("base").unwrap().layers[0].id.clone(),
            ] {
                let mut contours = hinted
                    .glyph("base")
                    .unwrap()
                    .layer(&layer_id)
                    .unwrap()
                    .contours
                    .clone();
                contours[0].points[1].x = 140.0;
                doc.apply(
                    Command::ReplaceLayerOutline {
                        glyph: "base".into(),
                        layer: layer_id.clone(),
                        contours,
                        components: vec![],
                        anchors: vec![],
                    },
                    "Edit source",
                )
                .unwrap();
                for name in ["base", "composite"] {
                    assert_eq!(
                        doc.font
                            .glyph(name)
                            .unwrap()
                            .master_layer(&second_id)
                            .unwrap()
                            .hints,
                        hinted
                            .glyph(name)
                            .unwrap()
                            .master_layer(&second_id)
                            .unwrap()
                            .hints
                    );
                    let hints = &doc
                        .font
                        .glyph(name)
                        .unwrap()
                        .master_layer(&first)
                        .unwrap()
                        .hints;
                    if layer_id == hinted.glyph("base").unwrap().layers[0].id {
                        assert!(hints.is_empty());
                    } else {
                        assert_eq!(
                            hints,
                            &hinted
                                .glyph(name)
                                .unwrap()
                                .master_layer(&first)
                                .unwrap()
                                .hints
                        );
                    }
                }
                doc.undo().unwrap();
                assert_eq!(doc.font, hinted);
            }
        })
        .unwrap();
}

#[test]
fn component_geometry_edits_clear_stale_hints_and_undo_restores_both() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let master = doc.font.default_master().id.clone();
            doc.apply(
                Command::Hints(HintCommand::AutohintPs {
                    master,
                    glyphs: vec!["base".into(), "composite".into()],
                }),
                "Hint selection",
            )
            .unwrap();
            let hinted = doc.font.clone();
            assert!(!hinted.glyph("composite").unwrap().layers[0]
                .hints
                .is_empty());
            let layer = hinted.glyph("base").unwrap().layers[0].id.clone();
            let mut contours = hinted.glyph("base").unwrap().layers[0].contours.clone();
            contours[0].points[1].x = 120.0;
            doc.apply(
                Command::ReplaceLayerOutline {
                    glyph: "base".into(),
                    layer,
                    contours,
                    components: vec![],
                    anchors: vec![],
                },
                "Edit base",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].hints,
                PsHints::default()
            );
            assert_eq!(
                doc.font.glyph("composite").unwrap().layers[0].hints,
                PsHints::default()
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, hinted);
        })
        .unwrap();
}
