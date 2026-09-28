#[allow(dead_code)]
mod common;
use fc_doc::{Command, DeleteMode, HandleMode, PointRef, Session};
use fc_model::{Component, Contour, Font, Glyph, Layer};
use kurbo::{Affine, Rect, Shape};

#[test]
fn radii_follow_source_nodes_through_edits_history_and_native_reopen() {
    let mut font = Font::new_empty("Corners", 1000);
    let master = font.default_master().id.clone();
    let mut layer = Layer::new_master(master.clone());
    layer.contours = Contour::from_bezpath(&Rect::new(0., 0., 100., 100.).to_path(0.01));
    let layer_id = layer.id.clone();
    let mut base = Glyph::new("base");
    base.layers.push(layer);
    font.insert_glyph(base).unwrap();
    let mut dependent = Glyph::new("dependent");
    let mut layer = Layer::new_master(master.clone());
    layer.components.push(Component {
        base: "base".into(),
        transform: Affine::translate((200., 0.)),
        ..Default::default()
    });
    dependent.layers.push(layer);
    font.insert_glyph(dependent).unwrap();
    let session = Session::new();
    let doc = session.open_font(font, None);
    session
        .with_doc_mut(doc, |doc| {
            let gid = doc.glyph_id("dependent").unwrap();
            assert_ne!(
                doc.flattened_path(gid, &master)
                    .unwrap()
                    .winding((299., 1.).into()),
                0
            );
            let command = |radius| Command::SetSmartCorner {
                glyph: "base".into(),
                layer: layer_id.clone(),
                points: vec![PointRef {
                    contour: 0,
                    point: 1,
                }],
                radius,
            };
            doc.apply(command(20.), "Round corner").unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours[0]
                    .points
                    .len(),
                4
            );
            assert_eq!(
                doc.flattened_path(gid, &master)
                    .unwrap()
                    .winding((299., 1.).into()),
                0
            );
            let saved = doc.font.clone();
            assert!(doc.apply(command(200.), "Oversized corner").is_err());
            assert_eq!(doc.font, saved);
            doc.undo().unwrap();
            assert_ne!(
                doc.flattened_path(gid, &master)
                    .unwrap()
                    .winding((299., 1.).into()),
                0
            );
            doc.redo().unwrap();
            assert_eq!(doc.font, saved);
            doc.apply(
                Command::MovePoints {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    points: vec![PointRef {
                        contour: 0,
                        point: 1,
                    }],
                    delta: (10., 0.),
                    handles: HandleMode::WithHandles,
                },
                "Move live corner",
            )
            .unwrap();
            let moved = &doc.font.glyph("base").unwrap().layers[0].contours[0].points[1];
            assert_eq!((moved.x, moved.corner_radius), (110., 20.));
            doc.undo().unwrap();
            assert_eq!(doc.font, saved);
            doc.apply(
                Command::DeletePoints {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    points: vec![PointRef {
                        contour: 0,
                        point: 1,
                    }],
                    mode: DeleteMode::Simple,
                },
                "Delete live corner",
            )
            .unwrap();
            assert!(!doc.font.glyph("base").unwrap().layers[0].has_smart_corners());
            doc.undo().unwrap();
            assert_eq!(doc.font, saved);
            doc.apply(
                Command::InsertPointOnSegment {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    contour: 0,
                    segment: 2,
                    t: 0.5,
                },
                "Insert node",
            )
            .unwrap();
            doc.apply(
                Command::ReverseContours {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    contours: vec![0],
                },
                "Reverse",
            )
            .unwrap();
            let contour = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            assert_eq!(
                contour
                    .points
                    .iter()
                    .find(|p| p.x == 100. && p.y == 0.)
                    .unwrap()
                    .corner_radius,
                20.
            );
            let directory = common::temp_dir("smart-corners");
            let path = directory.join("source.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), doc.font);
            std::fs::remove_dir_all(directory).unwrap();
        })
        .unwrap();
}
