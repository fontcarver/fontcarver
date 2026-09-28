#[allow(dead_code)]
mod common;
use fc_doc::{Command, Session};
use fc_model::{
    Component, Contour, EffectBoolean, Font, Glyph, Layer, LayerEffect, LayerEffectKind,
};
use kurbo::{Affine, Rect, Shape};

fn fixture() -> Font {
    let mut font = Font::new_empty("Effects", 1000);
    let mut glyph = Glyph::new("base");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.contours = Contour::from_bezpath(&Rect::new(0., 0., 100., 100.).to_path(0.01));
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let mut glyph = Glyph::new("component");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.components.push(Component {
        base: "base".into(),
        transform: Affine::translate((200., 0.)),
        ..Default::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

#[test]
fn effect_history_invalidates_dependents_and_preserves_native_source() {
    let session = Session::new();
    let id = session.open_font(fixture(), None);
    session
        .with_doc_mut(id, |doc| {
            let master = doc.font.default_master().id.clone();
            let id = doc.glyph_id("component").unwrap();
            assert_eq!(
                doc.flattened_path(id, &master).unwrap().bounding_box(),
                Rect::new(200., 0., 300., 100.)
            );
            let original = doc.font.glyph("base").unwrap().layers[0].clone();
            let effects = vec![LayerEffect {
                enabled: true,
                effect: LayerEffectKind::Boolean {
                    operation: EffectBoolean::Union,
                    path: "M100 0L150 0L150 100L100 100Z".into(),
                },
            }];
            doc.apply(
                Command::SetLayerEffects {
                    glyph: "base".into(),
                    layer: original.id.clone(),
                    effects: effects.clone(),
                },
                "Live union",
            )
            .unwrap();
            assert_eq!(
                doc.flattened_path(id, &master).unwrap().bounding_box(),
                Rect::new(200., 0., 350., 100.)
            );
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours,
                original.contours
            );
            let dir = common::temp_dir("effects");
            let path = dir.join("effects.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            let reopened = fc_doc::fcf::read(&path).unwrap();
            assert_eq!(reopened.glyph("base").unwrap().layers[0].effects, effects);
            assert_eq!(
                reopened.glyph("base").unwrap().layers[0].contours,
                original.contours
            );
            doc.undo().unwrap();
            assert_eq!(
                doc.flattened_path(id, &master).unwrap().bounding_box(),
                Rect::new(200., 0., 300., 100.)
            );
            doc.redo().unwrap();
            assert_eq!(
                doc.flattened_path(id, &master).unwrap().bounding_box(),
                Rect::new(200., 0., 350., 100.)
            );
        })
        .unwrap();
}

#[test]
fn invalid_source_edit_under_a_live_boolean_rolls_back_exactly() {
    let session = Session::new();
    let doc = session.open_font(fixture(), None);
    session
        .with_doc_mut(doc, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].clone();
            doc.apply(
                Command::SetLayerEffects {
                    glyph: "base".into(),
                    layer: layer.id.clone(),
                    effects: vec![LayerEffect {
                        enabled: true,
                        effect: LayerEffectKind::Boolean {
                            operation: EffectBoolean::Union,
                            path: String::new(),
                        },
                    }],
                },
                "Boolean",
            )
            .unwrap();
            let before = doc.font.clone();
            let generation = doc.generation;
            let mut contours = layer.contours;
            contours[0].closed = false;
            assert!(doc
                .apply(
                    Command::ReplaceLayerOutline {
                        glyph: "base".into(),
                        layer: layer.id,
                        contours,
                        components: vec![],
                        anchors: vec![]
                    },
                    "Open source"
                )
                .is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, generation);
        })
        .unwrap();
}
