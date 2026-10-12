mod common;

use fc_doc::{fcf, Command, Session};
use fc_model::{Component, Font, GlyphName, Layer, LayerId, LayerKind, MasterId};

use common::{temp_dir, test_font};

fn selection_font() -> Font {
    let mut font = test_font();
    let contour = font.glyph("base").unwrap().layers[0].contours[0].clone();
    let layer = &mut font.glyph_mut("composite").unwrap().layers[0];
    layer.name = "Foreground".to_owned();
    layer.contours = vec![contour.clone(), contour];
    layer.contours[1].points[0].x = 25.0;
    layer.components.push(Component {
        base: "base".into(),
        transform: kurbo::Affine::translate((125.0, 0.0)),
        ..Component::default()
    });
    layer.vertical_width = Some(950.0);
    layer.vertical_origin = Some(775.0);
    layer.guides.push(fc_model::Guide::default());
    font
}

#[test]
fn rename_layer_trims_and_round_trips_through_undo_redo() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let before = doc.font.clone();
            let change = doc
                .apply(
                    Command::RenameLayer {
                        glyph: glyph.clone(),
                        layer: layer.clone(),
                        name: " \t Review outline \u{3000}".to_owned(),
                    },
                    "Rename layer",
                )
                .unwrap();
            assert!(change.layers);
            assert_eq!(doc.undo_len(), 1);
            assert_eq!(
                doc.font.glyph(&glyph).unwrap().layers[0].name,
                "Review outline"
            );
            assert_eq!(doc.font.glyph(&glyph).unwrap().layers[0].id, layer);
            let after = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
            for name in ["", " \n\t\u{3000}"] {
                assert!(doc
                    .apply(
                        Command::RenameLayer {
                            glyph: glyph.clone(),
                            layer: layer.clone(),
                            name: name.to_owned(),
                        },
                        "Blank name",
                    )
                    .is_err());
                assert_eq!(doc.font, after);
                assert_eq!(doc.undo_len(), 1);
            }
        })
        .unwrap();
}

#[test]
fn selection_creates_background_in_source_order_once_and_is_one_undo_unit() {
    let session = Session::new();
    let id = session.open_font(selection_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("composite");
            let source = doc.font.glyph(&glyph).unwrap().layers[0].clone();
            let master = doc.font.default_master().id.clone();
            let before = doc.font.clone();
            let change = doc
                .apply(
                    Command::CopySelectionToBackground {
                        glyph: glyph.clone(),
                        from: source.id.clone(),
                        master: master.clone(),
                        contours: vec![1, 0, 1],
                        components: vec![1, 1],
                    },
                    "Copy selected paths",
                )
                .unwrap();
            assert!(change.layers);
            assert_eq!(doc.undo_len(), 1);
            let layers = &doc.font.glyph(&glyph).unwrap().layers;
            assert_eq!(layers.len(), 2);
            assert_eq!(layers[0], source);
            let background = &layers[1];
            assert_ne!(background.id, source.id);
            assert_eq!(background.kind, LayerKind::Background(master));
            assert_eq!(background.contours, source.contours);
            assert_eq!(background.components, vec![source.components[1].clone()]);
            assert_eq!(background.width, source.width);
            assert_eq!(background.vertical_width, source.vertical_width);
            assert_eq!(background.vertical_origin, source.vertical_origin);
            assert!(background.guides.is_empty());
            let after = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
        })
        .unwrap();
}

#[test]
fn selection_appends_preserving_background_identity_metadata_and_fcf() {
    let mut font = selection_font();
    let master = font.default_master().id.clone();
    let mut background = Layer {
        kind: LayerKind::Background(master.clone()),
        name: "Reference".to_owned(),
        width: 123.0,
        user_data: serde_json::json!({"reference": true}),
        ..Layer::default()
    };
    background.contours = font.glyph("base").unwrap().layers[0].contours.clone();
    font.glyph_mut("composite")
        .unwrap()
        .layers
        .push(background.clone());
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let source = doc.font.glyph("composite").unwrap().layers[0].clone();
            let before = doc.font.clone();
            doc.apply(
                Command::CopySelectionToBackground {
                    glyph: "composite".into(),
                    from: source.id.clone(),
                    master,
                    contours: vec![1],
                    components: vec![0],
                },
                "Append selection",
            )
            .unwrap();
            let mut expected = background.clone();
            expected.contours.push(source.contours[1].clone());
            expected.components.push(source.components[0].clone());
            assert_eq!(doc.font.glyph("composite").unwrap().layers[1], expected);
            let after_copy = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after_copy);
            doc.apply(
                Command::RenameLayer {
                    glyph: "composite".into(),
                    layer: background.id,
                    name: "Renamed reference".to_owned(),
                },
                "Rename background",
            )
            .unwrap();
            let dir = temp_dir("layer-workflows");
            for name in ["layers.fcf", "layers.fcf.gz"] {
                let path = dir.join(name);
                fcf::write(&path, &doc.font).unwrap();
                assert_eq!(fcf::read(&path).unwrap(), doc.font);
            }
            std::fs::remove_dir_all(dir).unwrap();
        })
        .unwrap();
}

#[test]
fn invalid_selection_and_rename_leave_font_and_history_unchanged() {
    let session = Session::new();
    let id = session.open_font(selection_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            let source = doc.font.glyph("composite").unwrap().layers[0].id.clone();
            let master = doc.font.default_master().id.clone();
            let selections = [
                (source.clone(), master.clone(), vec![], vec![]),
                (source.clone(), master.clone(), vec![0, 99], vec![0]),
                (source.clone(), master.clone(), vec![0], vec![99]),
                (LayerId::new(), master.clone(), vec![0], vec![]),
                (source, MasterId::new(), vec![0], vec![]),
            ];
            for (from, master, contours, components) in selections {
                assert!(doc
                    .apply(
                        Command::CopySelectionToBackground {
                            glyph: "composite".into(),
                            from,
                            master,
                            contours,
                            components,
                        },
                        "Invalid selection",
                    )
                    .is_err());
                assert_eq!(doc.font, before);
                assert_eq!(doc.undo_len(), 0);
            }
            assert!(doc
                .apply(
                    Command::RenameLayer {
                        glyph: "composite".into(),
                        layer: LayerId::new(),
                        name: "Missing".to_owned(),
                    },
                    "Missing layer",
                )
                .is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.undo_len(), 0);
        })
        .unwrap();
}

#[test]
fn component_only_copy_works_and_self_copy_is_rejected() {
    let session = Session::new();
    let id = session.open_font(selection_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let source = doc.font.glyph("composite").unwrap().layers[0].clone();
            let master = doc.font.default_master().id.clone();
            doc.apply(
                Command::CopySelectionToBackground {
                    glyph: "composite".into(),
                    from: source.id,
                    master: master.clone(),
                    contours: vec![],
                    components: vec![1],
                },
                "Copy component",
            )
            .unwrap();
            let background = &doc.font.glyph("composite").unwrap().layers[1];
            assert!(background.contours.is_empty());
            assert_eq!(background.components, vec![source.components[1].clone()]);
            let background_id = background.id.clone();
            let before = doc.font.clone();
            assert!(doc
                .apply(
                    Command::CopySelectionToBackground {
                        glyph: "composite".into(),
                        from: background_id,
                        master,
                        contours: vec![],
                        components: vec![0],
                    },
                    "Copy background to itself",
                )
                .is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.undo_len(), 1);
        })
        .unwrap();
}

#[test]
fn add_and_full_layer_copy_preserve_metadata_with_exact_undo_redo() {
    let session = Session::new();
    let id = session.open_font(selection_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            let source = doc.font.glyph("composite").unwrap().layers[0].clone();
            let mut added = source.clone();
            added.id = LayerId::new();
            added.kind = LayerKind::Custom("Study".to_owned());
            added.name = "Study".to_owned();
            doc.apply(
                Command::AddLayer {
                    glyph: "composite".into(),
                    layer: added.clone(),
                },
                "Add study",
            )
            .unwrap();
            assert_eq!(doc.font.glyph("composite").unwrap().layers[1], added);
            let destination = LayerKind::Backup {
                of: doc.font.default_master().id.clone(),
                time: 1_790_000_000,
            };
            doc.apply(
                Command::CopyLayer {
                    glyph: "composite".into(),
                    from: source.id.clone(),
                    to: destination.clone(),
                },
                "Backup source",
            )
            .unwrap();
            let backup = &doc.font.glyph("composite").unwrap().layers[2];
            assert_ne!(backup.id, source.id);
            let mut expected = source;
            expected.id = backup.id.clone();
            expected.kind = destination;
            assert_eq!(*backup, expected);
            let after = doc.font.clone();
            doc.undo().unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
        })
        .unwrap();
}

#[test]
fn repeated_selection_copies_do_not_reuse_locked_background_path_or_node_ids() {
    let mut font = selection_font();
    let master = font.default_master().id.clone();
    let source = &mut font.glyph_mut("composite").unwrap().layers[0];
    source.contours[0].id = Some("shared-contour".to_owned());
    source.contours[0].points[0].id = Some("shared-node".to_owned());
    let source = source.clone();
    let background = Layer {
        kind: LayerKind::Background(master.clone()),
        contours: vec![source.contours[0].clone()],
        user_data: serde_json::json!({
            "com.fontcarver.editor": { "locked_paths": ["shared-contour"] }
        }),
        ..Layer::default()
    };
    font.glyph_mut("composite")
        .unwrap()
        .layers
        .push(background.clone());
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            for _ in 0..2 {
                doc.apply(
                    Command::CopySelectionToBackground {
                        glyph: "composite".into(),
                        from: source.id.clone(),
                        master: master.clone(),
                        contours: vec![0],
                        components: vec![],
                    },
                    "Copy independent path",
                )
                .unwrap();
            }
            let layers = &doc.font.glyph("composite").unwrap().layers;
            assert_eq!(layers[0], source);
            assert_eq!(layers[1].contours.len(), 3);
            assert_eq!(layers[1].contours[0], background.contours[0]);
            assert_eq!(layers[1].user_data, background.user_data);
            for copied in &layers[1].contours[1..] {
                assert!(copied.id.is_none());
                assert!(copied.points.iter().all(|point| point.id.is_none()));
            }
            let after = doc.font.clone();
            doc.undo().unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
        })
        .unwrap();
}
