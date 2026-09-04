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

#[test]
fn preview_edit_and_apply_in_edit_produce_the_same_state() {
    let drag = |use_preview: bool| {
        let session = Session::new();
        let id = session.open_font(test_font(), None);
        session
            .with_doc_mut(id, |doc| {
                let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
                let token = doc.begin_edit("Drag");
                let mut changes = Vec::new();
                for step in 0..5 {
                    let command = Command::MovePoints {
                        glyph: "base".into(),
                        layer: layer.clone(),
                        points: vec![PointRef {
                            contour: 0,
                            point: step % 4,
                        }],
                        delta: (2.0, 3.0),
                        handles: HandleMode::PointsOnly,
                    };
                    let change = if use_preview {
                        doc.preview_edit(&token, command)
                    } else {
                        doc.apply_in_edit(&token, command)
                    }
                    .unwrap();
                    changes.push(change);
                }
                doc.end_edit(token);
                (
                    doc.font.glyph("base").unwrap().layers[0].contours.clone(),
                    doc.undo_len(),
                    changes,
                )
            })
            .unwrap()
    };
    let (previewed, preview_entries, preview_changes) = drag(true);
    let (applied, applied_entries, applied_changes) = drag(false);
    assert_eq!(previewed, applied);
    assert_eq!(preview_entries, applied_entries);
    assert_eq!(preview_entries, 1);
    assert_eq!(preview_changes.len(), applied_changes.len());
    for (preview, applied) in preview_changes.iter().zip(&applied_changes) {
        assert_eq!(preview.metrics, applied.metrics);
        assert_eq!(preview.layers, applied.layers);
        assert_eq!(preview.guides, applied.guides);
        assert_eq!(preview.glyphs.len(), applied.glyphs.len());
    }
}

#[test]
fn stale_tokens_and_live_edits_are_rejected_or_finalised() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let command = |delta: f64| Command::MovePoints {
                glyph: "base".into(),
                layer: layer.clone(),
                points: vec![PointRef {
                    contour: 0,
                    point: 0,
                }],
                delta: (delta, 0.0),
                handles: HandleMode::PointsOnly,
            };

            let first = doc.begin_edit("First");
            doc.apply_in_edit(&first, command(1.0)).unwrap();
            // Beginning another gesture finalises the previous one.
            let second = doc.begin_edit("Second");
            assert!(matches!(
                doc.apply_in_edit(&first, command(1.0)),
                Err(fw_doc::Error::InvalidEditToken)
            ));
            doc.apply_in_edit(&second, command(2.0)).unwrap();
            assert_eq!(doc.undo_len(), 2);

            // A regular apply finalises the live gesture too.
            doc.apply(command(4.0), "Nudge").unwrap();
            assert!(matches!(
                doc.apply_in_edit(&second, command(1.0)),
                Err(fw_doc::Error::InvalidEditToken)
            ));
            assert_eq!(doc.undo_len(), 3);
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours[0].points[0].x,
                7.0
            );

            doc.undo().unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours[0].points[0].x,
                3.0
            );
            doc.redo().unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours[0].points[0].x,
                7.0
            );
        })
        .unwrap();
}

#[test]
fn outline_edits_on_a_master_layer_invalidate_metrics() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = doc.font.glyph("base").unwrap().clone();
            let layer = glyph.layers[0].id.clone();
            let change = doc
                .apply(
                    Command::MovePoints {
                        glyph: "base".into(),
                        layer,
                        points: vec![PointRef {
                            contour: 0,
                            point: 0,
                        }],
                        delta: (3.0, 0.0),
                        handles: HandleMode::PointsOnly,
                    },
                    "Nudge",
                )
                .unwrap();
            assert!(change.metrics, "derived sidebearings changed");

            let change = doc
                .apply(
                    Command::RemoveGlyphs {
                        names: vec!["composite".into()],
                    },
                    "Remove",
                )
                .unwrap();
            assert!(change.layers);
            assert!(change.metrics);
        })
        .unwrap();
}
