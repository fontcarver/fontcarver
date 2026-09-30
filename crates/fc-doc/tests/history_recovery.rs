mod common;

use std::{
    fs::File,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

use fc_doc::{autosave, fcf, Command, Document, Error, Session};
use fc_model::Font;
use serde_json::{json, Value};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        Self(common::temp_dir(label))
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn width_command(doc: &Document, width: f64) -> Command {
    Command::SetWidth {
        glyph: "base".into(),
        layer: doc.font.glyph("base").unwrap().layers[0].id.clone(),
        width,
    }
}

fn write_snapshot(path: &Path, font: &Font, original: Option<&Path>, modified: u64) {
    fcf::write(path, font).unwrap();
    let mut envelope: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    envelope["document_path"] = json!(original);
    std::fs::write(path, serde_json::to_vec(&envelope).unwrap()).unwrap();
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(modified))
        .unwrap();
}

#[test]
fn recovery_listing_rejects_unreadable_fonts_inside_valid_envelopes() {
    let dir = TestDirectory::new("recovery-corruption");
    let font = common::test_font();
    let good = dir.0.join("good.autosave.fcf");
    write_snapshot(&good, &font, None, 100);
    let template: Value = serde_json::from_slice(&std::fs::read(&good).unwrap()).unwrap();
    let invalid_bitmap = json!([{
        "ppem": 16,
        "depth": 1,
        "glyphs": {
            "base": {
                "bitmap": { "width": 8, "height": 2, "depth": 1, "rows": [255] }
            }
        }
    }]);
    let mut malformed_font = template.clone();
    malformed_font["font"] = Value::Null;
    let mut invalid_font = template.clone();
    invalid_font["font"]["bitmap_strikes"] = invalid_bitmap;
    let mut future = template.clone();
    future["version"] = json!(u32::MAX);
    let mut unknown_format = template;
    unknown_format["format"] = json!("not-a-fontcarver-document");
    for (name, envelope) in [
        ("malformed", malformed_font),
        ("invalid-bitmap", invalid_font),
        ("future", future),
        ("unknown", unknown_format),
    ] {
        let path = dir.0.join(format!("{name}.autosave.fcf"));
        std::fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert!(fcf::read(&path).is_err(), "{name} must be unreadable");
    }
    std::fs::write(dir.0.join("truncated.autosave.fcf"), b"{\"font\":").unwrap();
    std::fs::create_dir(dir.0.join("directory.autosave.fcf")).unwrap();
    fcf::write(&dir.0.join("unrelated.fcf"), &font).unwrap();

    let entries = autosave::list_recoverable(&dir.0);
    assert_eq!(entries.len(), 1, "only a readable font is recoverable");
    assert_eq!(entries[0].autosave_path, good);
}

#[test]
fn recovery_listing_preserves_provenance_freshness_order_and_legacy_snapshots() {
    let dir = TestDirectory::new("recovery-freshness");
    let font = common::test_font();
    let original = dir.0.join("saved.fcf");
    write_snapshot(&original, &font, None, 200);
    let missing_original = dir.0.join("missing.fcf");
    let newest = dir.0.join("newest.autosave.fcf");
    let legacy = dir.0.join("legacy.autosave.fwf");
    let missing = dir.0.join("missing.autosave.fcf");
    let untitled = dir.0.join("untitled.autosave.fcf");
    write_snapshot(&newest, &font, Some(&original), 500);
    write_snapshot(&legacy, &font, None, 400);
    write_snapshot(&missing, &font, Some(&missing_original), 300);
    write_snapshot(&untitled, &font, None, 250);
    write_snapshot(
        &dir.0.join("stale.autosave.fcf"),
        &font,
        Some(&original),
        100,
    );

    let entries = autosave::list_recoverable(&dir.0);
    assert_eq!(
        entries
            .iter()
            .map(|entry| &entry.autosave_path)
            .collect::<Vec<_>>(),
        vec![&newest, &legacy, &missing, &untitled]
    );
    assert_eq!(entries[0].doc_path.as_deref(), Some(original.as_path()));
    assert_eq!(entries[1].doc_path, None);
    assert_eq!(entries[2].doc_path, Some(missing_original));
    assert_eq!(entries[3].doc_path, None);
    assert!(autosave::list_recoverable(&dir.0.join("absent")).is_empty());
}

#[test]
fn directory_project_recovery_uses_the_committed_manifest_timestamp() {
    let dir = TestDirectory::new("project-recovery-freshness");
    let font = common::test_font();
    let project = dir.0.join("saved.fcfp");
    fcf::write(&project, &font).unwrap();
    let manifest = project.join("project.json");
    let set_manifest_time = |seconds| {
        File::options()
            .write(true)
            .open(&manifest)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
            .unwrap();
    };
    set_manifest_time(100);
    let snapshot = dir.0.join("project.autosave.fcf");
    write_snapshot(&snapshot, &font, Some(&project), 200);

    // A crash during a future save may leave newer uncommitted directory
    // contents. Only the manifest defines which version was actually saved.
    std::fs::write(project.join("interrupted-save.tmp"), b"incomplete").unwrap();
    assert!(
        std::fs::metadata(&project).unwrap().modified().unwrap()
            > UNIX_EPOCH + Duration::from_secs(200)
    );
    let entries = autosave::list_recoverable(&dir.0);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].autosave_path, snapshot);
    assert_eq!(entries[0].doc_path, Some(project));

    set_manifest_time(300);
    assert!(autosave::list_recoverable(&dir.0).is_empty());
}

#[test]
fn gesture_labels_and_snapshots_survive_failed_previews_undo_and_redo() {
    let session = Session::new();
    let initial = common::test_font();
    let id = session.open_font(initial.clone(), None);
    let mut events = session.subscribe();
    session
        .with_doc_mut(id, |doc| {
            let token = doc.begin_edit("Drag metrics");
            doc.preview_edit(&token, width_command(doc, 610.0)).unwrap();
            let before_failure = doc.font.clone();
            let generation = doc.generation;
            let failure = Command::Batch(vec![
                width_command(doc, 920.0),
                Command::SetUnicodes {
                    glyph: "missing".into(),
                    unicodes: vec![65],
                },
            ]);
            assert!(doc.preview_edit(&token, failure).is_err());
            assert_eq!(doc.font, before_failure);
            assert_eq!(doc.generation, generation);
            assert_eq!(doc.undo_len(), 1);
            assert_eq!(doc.undo_label(), Some("Drag metrics"));

            doc.preview_edit(&token, width_command(doc, 720.0)).unwrap();
            doc.preview_edit(
                &token,
                Command::SetUnicodes {
                    glyph: "composite".into(),
                    unicodes: vec![66],
                },
            )
            .unwrap();
            doc.end_edit(token);
            let final_font = doc.font.clone();
            assert_eq!(doc.undo_len(), 1);
            doc.undo().unwrap().unwrap();
            assert_eq!(doc.font, initial);
            assert!(!doc.dirty);
            assert_eq!(doc.undo_label(), None);
            assert_eq!(doc.redo_label(), Some("Drag metrics"));
            doc.redo().unwrap().unwrap();
            assert_eq!(doc.font, final_font);
            assert!(doc.dirty);
            assert_eq!(doc.undo_label(), Some("Drag metrics"));
            assert_eq!(doc.redo_label(), None);
        })
        .unwrap();

    for expected_generation in 1..=3 {
        let event = events.try_recv().unwrap();
        assert_eq!(event.doc, id);
        assert_eq!(event.generation, expected_generation);
        assert_eq!(event.undo_label.as_deref(), Some("Drag metrics"));
        assert_eq!(event.redo_label, None);
        assert!(event.dirty);
    }
    let undone = events.try_recv().unwrap();
    assert_eq!(undone.generation, 4);
    assert_eq!(undone.undo_label, None);
    assert_eq!(undone.redo_label.as_deref(), Some("Drag metrics"));
    assert!(!undone.dirty);
    let redone = events.try_recv().unwrap();
    assert_eq!(redone.generation, 5);
    assert_eq!(redone.undo_label.as_deref(), Some("Drag metrics"));
    assert_eq!(redone.redo_label, None);
    assert!(redone.dirty);
    assert!(events.try_recv().is_err(), "failed previews emit no event");
}

#[test]
fn empty_or_failed_gestures_leave_existing_redo_intact() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(width_command(doc, 700.0), "Prior width").unwrap();
            doc.undo().unwrap().unwrap();
            let before = doc.font.clone();
            let generation = doc.generation;
            let empty = doc.begin_edit("Empty drag");
            doc.end_edit(empty);
            let failed = doc.begin_edit("Failed drag");
            assert!(doc
                .preview_edit(
                    &failed,
                    Command::SetUnicodes {
                        glyph: "missing".into(),
                        unicodes: vec![65],
                    }
                )
                .is_err());
            doc.end_edit(failed);
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, generation);
            assert_eq!(doc.undo_len(), 1);
            assert_eq!(doc.undo_label(), None);
            assert_eq!(doc.redo_label(), Some("Prior width"));
            assert!(!doc.dirty);
            doc.redo().unwrap().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 700.0);
        })
        .unwrap();
}

#[test]
fn ending_and_undoing_a_cancelled_gesture_restores_saved_state_and_prior_label() {
    let dir = TestDirectory::new("cancelled-gesture");
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(width_command(doc, 700.0), "Saved width").unwrap();
            doc.save(&dir.0.join("saved.fcf")).unwrap();
            let saved = doc.font.clone();
            let token = doc.begin_edit("Cancelled drag");
            doc.preview_edit(&token, width_command(doc, 800.0)).unwrap();
            doc.preview_edit(&token, width_command(doc, 900.0)).unwrap();
            // The editor cancels successful gestures using this end + undo sequence.
            doc.end_edit(token);
            doc.undo().unwrap().unwrap();
            assert_eq!(doc.font, saved);
            assert!(!doc.dirty);
            assert_eq!(doc.undo_label(), Some("Saved width"));
            assert_eq!(doc.redo_label(), Some("Cancelled drag"));
            assert!(matches!(
                doc.preview_edit(&token, width_command(doc, 950.0)),
                Err(Error::InvalidEditToken)
            ));
            assert_eq!(doc.font, saved);
        })
        .unwrap();
}
