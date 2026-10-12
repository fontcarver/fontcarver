use fc_doc::{Command, Session};
use fc_model::{ConstructionKind, Contour, Font, Glyph, GlyphConstruction, Layer};
use kurbo::{BezPath, Rect, Shape};

fn fixture() -> (Font, GlyphConstruction) {
    let mut font = Font::new_empty("Constructions", 1000);
    let master = font.default_master().id.clone();
    let mut source = Glyph::new("cap");
    let mut layer = Layer::new_master(master.clone());
    layer.contours = Contour::from_bezpath(&Rect::new(-20., -10., 20., 10.).to_path(0.01));
    source.layers.push(layer);
    font.insert_glyph(source).unwrap();
    let mut carrier = Glyph::new("carrier");
    let mut layer = Layer::new_master(master);
    layer.contours = Contour::from_bezpath(&BezPath::from_svg("M0 0L100 0").unwrap());
    let at = layer.ensure_node_id(0, 1).unwrap();
    let construction = GlyphConstruction {
        id: "cap-at-end".into(),
        source: "cap".into(),
        kind: ConstructionKind::Cap { at, scale: 1. },
        keep_carrier: true,
    };
    carrier.layers.push(layer);
    font.insert_glyph(carrier).unwrap();
    (font, construction)
}

#[test]
fn construction_history_preserves_carriers_and_tracks_source_edits() {
    let (font, construction) = fixture();
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let layer = doc.font.glyph("carrier").unwrap().layers[0].id.clone();
            let master = doc.font.default_master().id.clone();
            let gid = doc.glyph_id("carrier").unwrap();
            let before = doc.flattened_path(gid, &master).unwrap();
            doc.apply(
                Command::SetConstructions {
                    glyph: "carrier".into(),
                    layer: layer.clone(),
                    constructions: vec![construction.clone()],
                },
                "Attach cap",
            )
            .unwrap();
            let attached = doc.font.clone();
            let evaluated = doc.flattened_path(gid, &master).unwrap();
            assert_ne!(evaluated, before);
            assert_eq!(
                attached.glyph("carrier").unwrap().layers[0].contours,
                original.glyph("carrier").unwrap().layers[0].contours
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            assert_eq!(doc.flattened_path(gid, &master).unwrap(), before);
            doc.redo().unwrap();
            assert_eq!(doc.font, attached);
            assert_eq!(doc.flattened_path(gid, &master).unwrap(), evaluated);

            let source_layer = doc.font.glyph("cap").unwrap().layers[0].id.clone();
            let changed = doc
                .apply(
                    Command::MovePoints {
                        glyph: "cap".into(),
                        layer: source_layer,
                        points: vec![fc_doc::PointRef {
                            contour: 0,
                            point: 1,
                        }],
                        delta: (25., 10.),
                        handles: fc_doc::HandleMode::PointsOnly,
                    },
                    "Edit cap source",
                )
                .unwrap();
            assert!(changed.glyphs.contains(&gid));
            assert_ne!(doc.flattened_path(gid, &master).unwrap(), evaluated);
            doc.undo().unwrap();
            assert_eq!(doc.font, attached);
            assert_eq!(doc.flattened_path(gid, &master).unwrap(), evaluated);
        })
        .unwrap();
}

#[test]
fn invalid_construction_is_atomic() {
    let (font, mut construction) = fixture();
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let generation = doc.generation;
            construction.source = "missing".into();
            assert!(doc
                .apply(
                    Command::SetConstructions {
                        glyph: "carrier".into(),
                        layer: doc.font.glyph("carrier").unwrap().layers[0].id.clone(),
                        constructions: vec![construction],
                    },
                    "Invalid attachment",
                )
                .is_err());
            assert_eq!(doc.font, original);
            assert_eq!(doc.generation, generation);
        })
        .unwrap();
}

#[test]
fn background_swap_moves_constructions_and_guide_bindings_with_carriers() {
    let (mut font, construction) = fixture();
    let master = font.default_master().id.clone();
    let glyph = font.glyph_mut("carrier").unwrap();
    let layer = &mut glyph.layers[0];
    layer.constructions.push(construction);
    let node = layer.ensure_node_id(0, 1).unwrap();
    layer.guides.push(fc_model::Guide {
        id: Some("carrier-guide".into()),
        ..Default::default()
    });
    layer.linked_guides.push(fc_model::NodeLinkedGuide {
        guide_id: "carrier-guide".into(),
        nodes: vec![fc_model::GuidedNode { node, offset: 0. }],
    });
    glyph.layers.push(Layer {
        kind: fc_model::LayerKind::Background(master.clone()),
        ..Default::default()
    });
    let session = Session::new();
    let id = session.open_font(font.clone(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(
                Command::SwapWithBackground {
                    glyph: "carrier".into(),
                    master,
                },
                "Swap",
            )
            .unwrap();
            let layers = &doc.font.glyph("carrier").unwrap().layers;
            assert!(layers[0].constructions.is_empty());
            assert!(layers[0].linked_guides.is_empty());
            assert!(layers[0].guides.is_empty());
            assert_eq!(
                layers[1].constructions,
                font.glyph("carrier").unwrap().layers[0].constructions
            );
            assert_eq!(
                layers[1].linked_guides,
                font.glyph("carrier").unwrap().layers[0].linked_guides
            );
            layers[1].validate_constructions().unwrap();
            layers[1].validate_node_links().unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font, font);
        })
        .unwrap();
}
