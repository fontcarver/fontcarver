mod common;

use common::test_font;
use fc_doc::{Command, Session};
use fc_model::{Component, Layer, LayerKind};
use kurbo::Affine;

#[test]
fn component_lock_defaults_and_native_round_trip() {
    let legacy: Component = serde_json::from_str(r#"{"base":"base"}"#).unwrap();
    assert!(!legacy.locked);
    let mut font = test_font();
    font.glyph_mut("composite").unwrap().layers[0].components[0].locked = true;
    let directory = common::temp_dir("component-lock");
    let path = directory.join("locked.fcf");
    fc_doc::fcf::write(&path, &font).unwrap();
    assert_eq!(fc_doc::fcf::read(&path).unwrap(), font);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn locked_component_rejects_direct_and_bulk_mutations_atomically() {
    let mut font = test_font();
    let target = &mut font.glyph_mut("composite").unwrap().layers[0];
    target.components[0].locked = true;
    target.components.push(Component {
        base: "base".into(),
        transform: Affine::translate((200.0, 0.0)),
        ..Default::default()
    });
    let layer = target.id.clone();
    let original = target.components[0].clone();
    let mut moved = original.clone();
    moved.transform = Affine::translate((10.0, 20.0));
    let mut unlock_and_move = moved.clone();
    unlock_and_move.locked = false;
    let mut reordered = target.components.clone();
    reordered.swap(0, 1);
    let commands = vec![
        Command::SetComponent {
            glyph: "composite".into(),
            layer: layer.clone(),
            index: 0,
            component: moved,
        },
        Command::SetComponent {
            glyph: "composite".into(),
            layer: layer.clone(),
            index: 0,
            component: unlock_and_move,
        },
        Command::RemoveComponents {
            glyph: "composite".into(),
            layer: layer.clone(),
            indices: vec![1, 0],
        },
        Command::DecomposeComponents {
            glyph: "composite".into(),
            layer: layer.clone(),
            indices: vec![1, 0],
        },
        Command::TransformSelection {
            glyph: "composite".into(),
            layer: layer.clone(),
            points: vec![],
            components: vec![1, 0],
            anchors: vec![],
            affine: [1.0, 0.0, 0.0, 1.0, 10.0, 0.0],
        },
        Command::TransformSelection {
            glyph: "composite".into(),
            layer: layer.clone(),
            points: vec![],
            components: vec![],
            anchors: vec![],
            affine: [1.0, 0.0, 0.0, 1.0, 10.0, 0.0],
        },
        Command::ReplaceLayerOutline {
            glyph: "composite".into(),
            layer: layer.clone(),
            contours: vec![],
            components: reordered,
            anchors: vec![],
        },
        Command::SetMetrics {
            glyph: "composite".into(),
            layer: layer.clone(),
            width: None,
            lsb: Some(30.0),
            rsb: None,
        },
        Command::Batch(vec![
            Command::SetWidth {
                glyph: "composite".into(),
                layer: layer.clone(),
                width: 900.0,
            },
            Command::RemoveComponents {
                glyph: "composite".into(),
                layer,
                indices: vec![0],
            },
        ]),
    ];
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            for command in commands {
                let error = doc.apply(command, "Locked edit").unwrap_err();
                assert!(error.to_string().contains("locked"), "{error}");
                assert_eq!(doc.font, before);
                assert_eq!(doc.undo_len(), 0);
                assert!(!doc.dirty);
            }
        })
        .unwrap();
}

#[test]
fn unlock_is_undoable_and_unrelated_components_remain_editable() {
    let mut font = test_font();
    let target = &mut font.glyph_mut("composite").unwrap().layers[0];
    target.components[0].locked = true;
    target.components.push(Component {
        base: "base".into(),
        ..Default::default()
    });
    let layer = target.id.clone();
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            doc.apply(
                Command::TransformSelection {
                    glyph: "composite".into(),
                    layer: layer.clone(),
                    points: vec![],
                    components: vec![1],
                    anchors: vec![],
                    affine: [1.0, 0.0, 0.0, 1.0, 10.0, 0.0],
                },
                "Move unlocked component",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph("composite").unwrap().layers[0].components[0],
                before.glyph("composite").unwrap().layers[0].components[0]
            );
            doc.undo().unwrap();
            let mut unlocked = doc.font.glyph("composite").unwrap().layers[0].components[0].clone();
            unlocked.locked = false;
            doc.apply(
                Command::SetComponent {
                    glyph: "composite".into(),
                    layer,
                    index: 0,
                    component: unlocked,
                },
                "Unlock component",
            )
            .unwrap();
            assert!(!doc.font.glyph("composite").unwrap().layers[0].components[0].locked);
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert!(!doc.font.glyph("composite").unwrap().layers[0].components[0].locked);
        })
        .unwrap();
}

#[test]
fn replacing_or_removing_a_layer_cannot_bypass_component_locks() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    let target = font.glyph_mut("composite").unwrap();
    let source = target.layers[0].id.clone();
    let mut background = Layer::new_master(master.clone());
    background.kind = LayerKind::Background(master.clone());
    background.components.push(Component {
        base: "base".into(),
        locked: true,
        ..Default::default()
    });
    let layer = background.id.clone();
    target.layers.push(background);
    let mut replacement = target.clone();
    replacement.layers[1].components.clear();
    let commands = vec![
        Command::RemoveLayer {
            glyph: "composite".into(),
            layer,
        },
        Command::CopyLayer {
            glyph: "composite".into(),
            from: source,
            to: LayerKind::Background(master.clone()),
        },
        Command::SwapWithBackground {
            glyph: "composite".into(),
            master,
        },
        Command::ReplaceGlyph { glyph: replacement },
        Command::RemoveGlyphs {
            names: vec!["composite".into()],
        },
    ];
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            for command in commands {
                assert!(doc
                    .apply(command, "Replace locked content")
                    .unwrap_err()
                    .to_string()
                    .contains("locked"));
                assert_eq!(doc.font, before);
                assert_eq!(doc.undo_len(), 0);
            }
        })
        .unwrap();
}

/// A carrier with a base and a mark component, and the anchors that pair them.
fn accented_font() -> fc_model::Font {
    use fc_model::{Anchor, Glyph};
    let mut font = test_font();
    let master = font.default_master().id.clone();

    let base = font.glyph_mut("base").unwrap();
    base.layers[0].anchors.push(Anchor {
        name: "top".into(),
        x: 50.0,
        y: 700.0,
    });

    let mut acute = Glyph::new("acute");
    let mut acute_layer = Layer::new_master(master.clone());
    acute_layer.width = 300.0;
    acute_layer.anchors.push(Anchor {
        name: "_top".into(),
        x: 150.0,
        y: 0.0,
    });
    acute.layers.push(acute_layer);
    font.insert_glyph(acute).expect("acute is valid");

    let mut carrier = Glyph::new("aacute");
    let mut layer = Layer::new_master(master);
    layer.width = 500.0;
    layer.components.push(Component {
        base: "base".into(),
        ..Default::default()
    });
    layer.components.push(Component {
        base: "acute".into(),
        auto_align: true,
        transform: Affine::translate((900.0, 900.0)),
        ..Default::default()
    });
    carrier.layers.push(layer);
    font.insert_glyph(carrier).expect("aacute is valid");
    font
}

#[test]
fn an_auto_aligned_component_is_placed_on_the_matching_anchors() {
    let session = Session::new();
    let id = session.open_font(accented_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("aacute").unwrap().layers[0].id.clone();
            let before = doc.font.clone();
            doc.apply(
                Command::AlignComponents {
                    glyph: "aacute".into(),
                    layer: layer.clone(),
                },
                "Align components to anchors",
            )
            .unwrap();
            let components = &doc.font.glyph("aacute").unwrap().layers[0].components;
            // The mark's `_top` at (150, 0) lands on the base's `top` at (50, 700).
            assert_eq!(
                components[1].transform.as_coeffs(),
                [1.0, 0.0, 0.0, 1.0, -100.0, 700.0]
            );
            // The base component is not auto-aligned and is left alone.
            assert_eq!(components[0].transform, Affine::IDENTITY);

            // One undo unit, and the native file keeps the placement.
            let placed = doc.font.clone();
            let directory = common::temp_dir("component-align");
            let path = directory.join("aligned.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), placed);
            std::fs::remove_dir_all(directory).unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, placed);
        })
        .unwrap();
}

#[test]
fn alignment_keeps_scale_and_leaves_an_unresolvable_pair_alone() {
    let mut font = accented_font();
    {
        let layer = &mut font.glyph_mut("aacute").unwrap().layers[0];
        // A half-size mark, and a second auto-aligned component whose base has
        // no mark anchor at all.
        layer.components[1].transform = Affine::scale(0.5).then_translate((7.0, 9.0).into());
        layer.components.push(Component {
            base: "base".into(),
            auto_align: true,
            transform: Affine::translate((12.0, 34.0)),
            ..Default::default()
        });
    }
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("aacute").unwrap().layers[0].id.clone();
            doc.apply(
                Command::AlignComponents {
                    glyph: "aacute".into(),
                    layer: layer.clone(),
                },
                "Align components to anchors",
            )
            .unwrap();
            let components = &doc.font.glyph("aacute").unwrap().layers[0].components;
            // Half scale keeps its linear part; only the translation is solved.
            let coeffs = components[1].transform.as_coeffs();
            assert_eq!(
                [coeffs[0], coeffs[1], coeffs[2], coeffs[3]],
                [0.5, 0.0, 0.0, 0.5]
            );
            assert_eq!([coeffs[4], coeffs[5]], [50.0 - 75.0, 700.0]);
            // No mark anchor to match, so the component stays where it was.
            assert_eq!(components[2].transform, Affine::translate((12.0, 34.0)));
        })
        .unwrap();
}

#[test]
fn aligning_an_unknown_layer_is_rejected_without_changing_the_font() {
    let session = Session::new();
    let id = session.open_font(accented_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            assert!(doc
                .apply(
                    Command::AlignComponents {
                        glyph: "aacute".into(),
                        layer: fc_model::LayerId::new(),
                    },
                    "Align components to anchors",
                )
                .is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.undo_label(), None);
        })
        .unwrap();
}

#[test]
fn a_locked_component_is_left_where_it_is_and_does_not_block_the_others() {
    let mut font = accented_font();
    {
        let layer = &mut font.glyph_mut("aacute").unwrap().layers[0];
        layer.components[1].locked = true;
        // A second, unlocked mark that wants the same anchor.
        layer.components.push(Component {
            base: "acute".into(),
            auto_align: true,
            transform: Affine::translate((900.0, 900.0)),
            ..Default::default()
        });
    }
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("aacute").unwrap().layers[0].id.clone();
            doc.apply(
                Command::AlignComponents {
                    glyph: "aacute".into(),
                    layer,
                },
                "Align components to anchors",
            )
            .unwrap();
            let components = &doc.font.glyph("aacute").unwrap().layers[0].components;
            // The lock promises the component is preserved, so it is.
            assert_eq!(components[1].transform, Affine::translate((900.0, 900.0)));
            // The unlocked one is still placed.
            assert_eq!(
                components[2].transform.as_coeffs(),
                [1.0, 0.0, 0.0, 1.0, -100.0, 700.0]
            );
        })
        .unwrap();
}

#[test]
fn a_chained_composite_follows_the_component_it_attaches_to() {
    use fc_model::{Anchor, Glyph};
    let mut font = accented_font();
    let master = font.default_master().id.clone();
    // `acute` gains a `top` anchor of its own so a second mark can stack on it.
    font.glyph_mut("acute").unwrap().layers[0]
        .anchors
        .push(Anchor {
            name: "top".into(),
            x: 150.0,
            y: 200.0,
        });
    let mut grave = Glyph::new("grave");
    let mut grave_layer = Layer::new_master(master);
    grave_layer.width = 300.0;
    grave_layer.anchors.push(Anchor {
        name: "_top".into(),
        x: 10.0,
        y: 0.0,
    });
    grave.layers.push(grave_layer);
    font.insert_glyph(grave).expect("grave is valid");
    font.glyph_mut("aacute").unwrap().layers[0]
        .components
        .push(Component {
            base: "grave".into(),
            auto_align: true,
            transform: Affine::translate((-999.0, -999.0)),
            ..Default::default()
        });

    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("aacute").unwrap().layers[0].id.clone();
            doc.apply(
                Command::AlignComponents {
                    glyph: "aacute".into(),
                    layer,
                },
                "Align components to anchors",
            )
            .unwrap();
            let components = &doc.font.glyph("aacute").unwrap().layers[0].components;
            // `acute` lands at (−100, 700), so its own `top` anchor is now at
            // (50, 900) and `grave`'s `_top` at (10, 0) has to follow it there.
            assert_eq!(
                components[1].transform.as_coeffs(),
                [1.0, 0.0, 0.0, 1.0, -100.0, 700.0]
            );
            assert_eq!(
                components[2].transform.as_coeffs(),
                [1.0, 0.0, 0.0, 1.0, 40.0, 900.0]
            );
        })
        .unwrap();
}

#[test]
fn moving_a_base_anchor_moves_its_dependents_in_the_same_undo_entry() {
    let session = Session::new();
    let id = session.open_font(accented_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let carrier = doc.font.glyph("aacute").unwrap().layers[0].id.clone();
            let base = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.apply(
                Command::AlignComponents {
                    glyph: "aacute".into(),
                    layer: carrier,
                },
                "Align components to anchors",
            )
            .unwrap();
            let placed = doc.font.clone();

            // Dragging the base's `top` anchor 40 units up and 10 right.
            doc.apply(
                Command::SetAnchor {
                    glyph: "base".into(),
                    layer: base,
                    index: 0,
                    anchor: fc_model::Anchor {
                        name: "top".into(),
                        x: 60.0,
                        y: 740.0,
                    },
                },
                "Move anchor",
            )
            .unwrap();
            let components = &doc.font.glyph("aacute").unwrap().layers[0].components;
            assert_eq!(
                components[1].transform.as_coeffs(),
                [1.0, 0.0, 0.0, 1.0, -90.0, 740.0]
            );

            // One gesture, one undo entry: undoing the anchor move also puts
            // the composite back.
            doc.undo().unwrap();
            assert_eq!(doc.font, placed);
            doc.redo().unwrap();
            assert_eq!(
                doc.font.glyph("aacute").unwrap().layers[0].components[1]
                    .transform
                    .as_coeffs(),
                [1.0, 0.0, 0.0, 1.0, -90.0, 740.0]
            );
        })
        .unwrap();
}
