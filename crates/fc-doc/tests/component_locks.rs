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
