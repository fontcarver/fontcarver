mod common;

use fc_doc::{Command, Session};
use fc_model::{Component, GlyphName, Layer, LayerKind};

use common::test_font;

#[test]
fn layer_copy_replace_swap_remove_and_undo_are_exact() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let master = doc.font.default_master().id.clone();
            let source = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let before = doc.font.glyph(&glyph).unwrap().clone();
            let change = doc
                .apply(
                    Command::CopyLayer {
                        glyph: glyph.clone(),
                        from: source.clone(),
                        to: LayerKind::Background(master.clone()),
                    },
                    "Copy background",
                )
                .unwrap();
            assert!(change.layers);
            let background = doc
                .font
                .glyph(&glyph)
                .unwrap()
                .layers
                .iter()
                .find(|layer| layer.kind == LayerKind::Background(master.clone()))
                .unwrap();
            assert_eq!(background.contours, before.layers[0].contours);
            let background_id = background.id.clone();
            doc.apply(
                Command::CopyLayer {
                    glyph: glyph.clone(),
                    from: source,
                    to: LayerKind::Background(master.clone()),
                },
                "Replace background",
            )
            .unwrap();
            assert_eq!(
                doc.font
                    .glyph(&glyph)
                    .unwrap()
                    .layers
                    .iter()
                    .find(|layer| layer.kind == LayerKind::Background(master.clone()))
                    .unwrap()
                    .id,
                background_id
            );
            doc.apply(
                Command::SwapWithBackground {
                    glyph: glyph.clone(),
                    master,
                },
                "Swap",
            )
            .unwrap();
            doc.apply(
                Command::RemoveLayer {
                    glyph: glyph.clone(),
                    layer: background_id,
                },
                "Remove",
            )
            .unwrap();
            doc.undo().unwrap();
            doc.undo().unwrap();
            doc.undo().unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
        })
        .unwrap();
}

#[test]
fn add_layer_duplicate_glyph_order_master_and_font_guides_undo() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = Layer {
                kind: LayerKind::Custom("sketch".to_owned()),
                ..Layer::default()
            };
            let before = doc.font.clone();
            doc.apply(
                Command::AddLayer {
                    glyph: glyph.clone(),
                    layer,
                },
                "Add layer",
            )
            .unwrap();
            doc.apply(
                Command::DuplicateGlyph {
                    source: glyph,
                    name: "base.copy".into(),
                },
                "Duplicate",
            )
            .unwrap();
            assert!(doc.font.glyph("base.copy").unwrap().unicodes.is_empty());
            let source_ids: Vec<_> = doc
                .font
                .glyph("base")
                .unwrap()
                .layers
                .iter()
                .map(|layer| layer.id.clone())
                .collect();
            let copy_ids: Vec<_> = doc
                .font
                .glyph("base.copy")
                .unwrap()
                .layers
                .iter()
                .map(|layer| layer.id.clone())
                .collect();
            assert_eq!(source_ids.len(), copy_ids.len());
            assert!(
                copy_ids.iter().all(|id| !source_ids.contains(id)),
                "the duplicate allocates fresh layer identifiers"
            );
            doc.apply(
                Command::SetGlyphOrder {
                    order: vec!["composite".into(), "base".into(), "base.copy".into()],
                },
                "Order",
            )
            .unwrap();
            let mut master = doc.font.default_master().clone();
            master.name = "Changed".to_owned();
            doc.apply(Command::SetMaster { master }, "Master").unwrap();
            doc.apply(
                Command::SetFontGuides {
                    guides: vec![fc_model::Guide::default()],
                },
                "Guides",
            )
            .unwrap();
            for _ in 0..5 {
                doc.undo().unwrap();
            }
            assert_eq!(doc.font, before);
        })
        .unwrap();
}

#[test]
fn master_layers_cannot_be_added_or_removed() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let master = doc.font.default_master().id.clone();
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            assert!(doc
                .apply(
                    Command::RemoveLayer {
                        glyph: glyph.clone(),
                        layer
                    },
                    "Remove"
                )
                .is_err());
            assert!(doc
                .apply(
                    Command::AddLayer {
                        glyph: glyph.clone(),
                        layer: Layer::new_master(master)
                    },
                    "Add"
                )
                .is_err());

            let before = doc.font.clone();
            let cyclic = Layer {
                kind: LayerKind::Custom("cycle".to_owned()),
                components: vec![Component {
                    base: glyph.clone(),
                    ..Component::default()
                }],
                ..Layer::default()
            };
            assert!(doc
                .apply(
                    Command::AddLayer {
                        glyph,
                        layer: cyclic,
                    },
                    "Add cycle",
                )
                .is_err());
            assert_eq!(doc.font, before);
        })
        .unwrap();
}
