mod common;

use fc_doc::{Command, Session};

#[test]
fn cancel_restores_only_the_matching_gesture_and_discards_its_redo() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let width = |width| Command::SetWidth {
                glyph: "base".into(),
                layer: layer.clone(),
                width,
            };
            doc.apply(width(510.0), "Previous edit").unwrap();
            let before = doc.font.clone();
            let token = doc.begin_edit("Repair");
            doc.apply_in_edit(&token, width(520.0)).unwrap();
            doc.apply_in_edit(&token, width(530.0)).unwrap();
            assert!(doc.cancel_edit(token).unwrap().is_some());
            assert_eq!(doc.font, before);
            assert_eq!(doc.undo_len(), 1);
            assert_eq!(doc.undo_label(), Some("Previous edit"));
            assert_eq!(doc.redo_label(), None);
            assert!(doc.cancel_edit(token).is_err());
            assert_eq!(doc.font, before);
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 500.0);
        })
        .unwrap();
}

#[test]
fn stale_cancel_cannot_close_or_undo_a_newer_gesture() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let width = |width| Command::SetWidth {
                glyph: "base".into(),
                layer: layer.clone(),
                width,
            };
            let old = doc.begin_edit("Older");
            doc.apply_in_edit(&old, width(510.0)).unwrap();
            let current = doc.begin_edit("Newer");
            doc.apply_in_edit(&current, width(520.0)).unwrap();
            assert!(doc.cancel_edit(old).is_err());
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 520.0);
            doc.apply_in_edit(&current, width(530.0)).unwrap();
            doc.end_edit(current);
            assert!(doc.cancel_edit(current).is_err());
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 530.0);
            assert_eq!(doc.undo_len(), 2);
        })
        .unwrap();
}

#[test]
fn empty_or_failed_gesture_cancel_preserves_history_and_read_only_documents() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            let token = doc.begin_edit("No changes");
            assert!(doc
                .apply_in_edit(
                    &token,
                    Command::SetWidth {
                        glyph: "missing".into(),
                        layer: fc_model::LayerId::new(),
                        width: 1.0
                    }
                )
                .is_err());
            assert!(doc.cancel_edit(token).unwrap().is_none());
            assert_eq!(doc.font, before);
            assert_eq!(doc.undo_len(), 0);
            assert!(!doc.dirty);
            doc.set_read_only(true);
            let token = doc.begin_edit("Read only");
            assert!(doc.cancel_edit(token).is_err());
            assert_eq!(doc.font, before);
        })
        .unwrap();
}

#[test]
fn empty_cancel_preserves_an_existing_redo_tail() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.apply(
                Command::SetWidth {
                    glyph: "base".into(),
                    layer,
                    width: 700.0,
                },
                "Old edit",
            )
            .unwrap();
            doc.undo().unwrap();
            let token = doc.begin_edit("Empty");
            assert!(doc.cancel_edit(token).unwrap().is_none());
            assert_eq!(doc.redo_label(), Some("Old edit"));
            doc.redo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 700.0);
        })
        .unwrap();
}

#[test]
fn applied_cancel_restores_the_previous_redo_branch_and_dirty_state() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    let mut events = session.subscribe();
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let width = |width| Command::SetWidth {
                glyph: "base".into(),
                layer: layer.clone(),
                width,
            };
            doc.apply(width(700.0), "Previous redo").unwrap();
            doc.undo().unwrap();
            let before = doc.font.clone();
            let token = doc.begin_edit("Failed repair");
            doc.apply_in_edit(&token, width(800.0)).unwrap();
            doc.apply_in_edit(&token, width(900.0)).unwrap();
            while events.try_recv().is_ok() {}
            doc.cancel_edit(token).unwrap();
            let event = events.try_recv().unwrap();
            assert_eq!(event.undo_label, None);
            assert_eq!(event.redo_label.as_deref(), Some("Previous redo"));
            assert!(!event.dirty);
            assert!(events.try_recv().is_err());
            assert_eq!(doc.font, before);
            assert!(!doc.dirty);
            assert_eq!(doc.undo_label(), None);
            assert_eq!(doc.redo_label(), Some("Previous redo"));
            assert_eq!(doc.undo_len(), 1);
            doc.redo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 700.0);
        })
        .unwrap();
}
