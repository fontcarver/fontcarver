mod common;

use fw_doc::{Command, Session};
use fw_model::{Anchor, Component, Glyph, GlyphName, Guide};
use kurbo::Affine;

use common::test_font;

#[test]
fn components_anchors_and_guides_apply_undo_with_precise_flags() {
    let session = Session::new();
    let mut font = test_font();
    font.insert_glyph(Glyph::new("other")).unwrap();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let master = doc.font.default_master().id.clone();
            let commands = vec![
                Command::AddComponent {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    component: Component {
                        base: "other".into(),
                        transform: Affine::translate((10.0, 0.0)),
                        ..Component::default()
                    },
                },
                Command::AddAnchor {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    anchor: Anchor {
                        name: "top".to_owned(),
                        x: 50.0,
                        y: 100.0,
                    },
                },
                Command::AddGuide {
                    glyph: Some(glyph.clone()),
                    layer: Some(layer.clone()),
                    master: None,
                    guide: Guide::default(),
                },
                Command::AddGuide {
                    glyph: None,
                    layer: None,
                    master: Some(master),
                    guide: Guide::default(),
                },
                Command::AddGuide {
                    glyph: None,
                    layer: None,
                    master: None,
                    guide: Guide::default(),
                },
            ];
            for command in commands {
                let before = doc.font.clone();
                let change = doc.apply(command.clone(), "Structure").unwrap();
                assert_eq!(change.guides, matches!(command, Command::AddGuide { .. }));
                doc.undo().unwrap();
                assert_eq!(doc.font, before);
            }
        })
        .unwrap();
}

#[test]
fn decompose_components_flattens_with_transform_and_removes_reference() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("composite");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            doc.font.glyph_mut(&glyph).unwrap().layers[0].components[0].transform =
                Affine::translate((25.0, 30.0));
            let before = doc.font.glyph(&glyph).unwrap().clone();
            doc.apply(
                Command::DecomposeComponents {
                    glyph: glyph.clone(),
                    layer,
                    indices: vec![0],
                },
                "Decompose",
            )
            .unwrap();
            let decomposed = &doc.font.glyph(&glyph).unwrap().layers[0];
            assert!(decomposed.components.is_empty());
            assert_eq!(decomposed.contours[0].points[0].x, 25.0);
            assert_eq!(decomposed.contours[0].points[0].y, 30.0);
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
        })
        .unwrap();
}

#[test]
fn set_and_remove_structure_commands_round_trip() {
    let session = Session::new();
    let mut font = test_font();
    font.insert_glyph(Glyph::new("other")).unwrap();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let before = doc.font.clone();
            doc.apply(
                Command::AddComponent {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    component: Component {
                        base: "other".into(),
                        ..Component::default()
                    },
                },
                "Add component",
            )
            .unwrap();
            doc.apply(
                Command::SetComponent {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    index: 0,
                    component: Component {
                        base: "other".into(),
                        transform: Affine::translate((5.0, 6.0)),
                        ..Component::default()
                    },
                },
                "Set component",
            )
            .unwrap();
            doc.apply(
                Command::RemoveComponents {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    indices: vec![0],
                },
                "Remove component",
            )
            .unwrap();
            doc.apply(
                Command::AddAnchor {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    anchor: Anchor::default(),
                },
                "Add anchor",
            )
            .unwrap();
            doc.apply(
                Command::SetAnchor {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    index: 0,
                    anchor: Anchor {
                        name: "top".to_owned(),
                        x: 1.0,
                        y: 2.0,
                    },
                },
                "Set anchor",
            )
            .unwrap();
            doc.apply(
                Command::RemoveAnchors {
                    glyph,
                    layer,
                    indices: vec![0],
                },
                "Remove anchor",
            )
            .unwrap();
            doc.apply(
                Command::AddGuide {
                    glyph: None,
                    layer: None,
                    master: None,
                    guide: Guide::default(),
                },
                "Add guide",
            )
            .unwrap();
            doc.apply(
                Command::SetGuide {
                    glyph: None,
                    layer: None,
                    master: None,
                    index: 0,
                    guide: Guide {
                        x: 25.0,
                        ..Guide::default()
                    },
                },
                "Set guide",
            )
            .unwrap();
            doc.apply(
                Command::RemoveGuide {
                    glyph: None,
                    layer: None,
                    master: None,
                    index: 0,
                },
                "Remove guide",
            )
            .unwrap();
            for _ in 0..9 {
                doc.undo().unwrap();
            }
            assert_eq!(doc.font, before);
        })
        .unwrap();
}
