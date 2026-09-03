mod common;

use std::{sync::Arc, thread, time::Duration};

use common::{temp_dir, test_font};
use fw_doc::{
    autosave::{self, AutosaveConfig},
    Command, Error, FontOpener, Session,
};
use fw_model::Font;

struct FakeOpener;

impl FontOpener for FakeOpener {
    fn open_path(
        &self,
        _path: &std::path::Path,
        face_index: Option<u32>,
    ) -> Result<Vec<Font>, Error> {
        Ok(vec![Font::new_empty(
            &format!("Face {}", face_index.unwrap_or_default()),
            1_000,
        )])
    }
}

#[test]
fn session_opens_fwf_and_emits_apply_and_undo_events() {
    let dir = temp_dir("session");
    let path = dir.join("session.fwf");
    fw_doc::fwf::write(&path, &test_font()).unwrap();
    let session = Session::new();
    let mut events = session.subscribe();
    let id = session.open_path(&path, None).unwrap();
    assert_eq!(session.ids(), vec![id]);

    let glyph_id = session
        .with_doc_mut(id, |doc| {
            let glyph_id = doc.glyph_id("base").unwrap();
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let change = doc
                .apply(
                    Command::SetWidth {
                        glyph: "base".into(),
                        layer,
                        width: 777.0,
                    },
                    "Set width",
                )
                .unwrap();
            assert_eq!(change.glyphs, vec![glyph_id]);
            glyph_id
        })
        .unwrap();
    let applied = events.try_recv().unwrap();
    assert_eq!(applied.doc, id);
    assert_eq!(applied.change.glyphs, vec![glyph_id]);
    assert_eq!(applied.undo_label.as_deref(), Some("Set width"));

    session
        .with_doc_mut(id, |doc| {
            let change = doc.undo().unwrap().unwrap();
            assert_eq!(change.glyphs, vec![glyph_id]);
        })
        .unwrap();
    let undone = events.try_recv().unwrap();
    assert_eq!(undone.change.glyphs, vec![glyph_id]);
    assert_eq!(undone.redo_label.as_deref(), Some("Set width"));

    session.close(id).unwrap();
    assert!(session.ids().is_empty());
}

#[test]
fn autosave_writes_dirty_unsaved_documents_and_lists_them() {
    let dir = temp_dir("autosave");
    let session = Arc::new(Session::new());
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.apply(
                Command::SetWidth {
                    glyph: "base".into(),
                    layer,
                    width: 888.0,
                },
                "Set width",
            )
            .unwrap();
        })
        .unwrap();

    let handle = autosave::spawn(
        Arc::clone(&session),
        AutosaveConfig {
            interval: Duration::from_millis(50),
            dir: dir.clone(),
        },
    );
    thread::sleep(Duration::from_millis(180));
    handle.stop();

    let entries = autosave::list_recoverable(&dir);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].doc_path, None);
    let recovered = fw_doc::fwf::read(&entries[0].autosave_path).unwrap();
    assert_eq!(recovered.glyph("base").unwrap().layers[0].width, 888.0);
}

#[test]
fn session_is_send_sync_and_dispatches_binary_fonts_through_the_opener() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Session>();

    let session = Session::with_opener(Arc::new(FakeOpener));
    let id = session
        .open_path(std::path::Path::new("collection.ttf"), Some(3))
        .unwrap();
    session
        .with_doc(id, |doc| {
            assert_eq!(doc.font.info.family_name, "Face 3");
            assert_eq!(
                doc.path.as_deref(),
                Some(std::path::Path::new("collection.ttf"))
            );
        })
        .unwrap();
    assert!(matches!(
        session.open_path(std::path::Path::new("notes.txt"), None),
        Err(Error::UnsupportedFormat(_))
    ));
}

// enabled when fw-sfnt lands
#[test]
#[ignore]
fn session_opens_ttf() {
    let session = Session::new();
    let id = session
        .open_path(&fw_fixtures::path("DejaVuSans.ttf"), None)
        .unwrap();
    let dir = temp_dir("dejavu-roundtrip");
    let path = dir.join("DejaVuSans.fwf");
    session
        .with_doc_mut(id, |doc| {
            assert!(!doc.font.glyphs.is_empty());
            let mut expected = doc.font.clone();
            if let Some(source) = &mut expected.extra.source {
                source.original_bytes = None;
            }
            doc.save(&path).unwrap();
            assert_eq!(fw_doc::fwf::read(&path).unwrap(), expected);
        })
        .unwrap();
}
