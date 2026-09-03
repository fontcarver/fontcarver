mod common;

use fw_doc::{Command, HandleMode, PointRef, Session};

use common::test_font;

#[test]
fn fifty_preview_moves_coalesce_to_one_exact_undo() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.glyph("base").unwrap().clone();
            let layer = before.layers[0].id.clone();
            let token = doc.begin_edit("Drag point");
            for _ in 0..50 {
                doc.preview_edit(
                    &token,
                    Command::MovePoints {
                        glyph: "base".into(),
                        layer: layer.clone(),
                        points: vec![PointRef {
                            contour: 0,
                            point: 0,
                        }],
                        delta: (1.0, 0.0),
                        handles: HandleMode::PointsOnly,
                    },
                )
                .unwrap();
            }
            doc.end_edit(token);
            assert_eq!(doc.undo_len(), 1);
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours[0].points[0].x,
                50.0
            );
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap(), &before);
        })
        .unwrap();
}
