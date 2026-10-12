use std::{path::PathBuf, time::UNIX_EPOCH};

use fc_model::{BitmapStrike, Font};

use super::*;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fontcarver-autosave-pass-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn invalid_snapshot_keeps_last_good_recovery_and_does_not_block_other_documents() {
    let dir = TestDirectory::new();
    let session = Session::new();
    let initial = Font::new_empty("Previously recoverable", 1_000);
    let bad = session.open_font(initial.clone(), None);
    session.with_doc_mut(bad, |doc| doc.dirty = true).unwrap();
    autosave_pass(&session, &dir.0).unwrap();
    let saved = autosave_path(&dir.0, bad, None);
    let original_bytes = std::fs::read(&saved).unwrap();

    // Invalid source data must never replace a snapshot that can be reopened.
    session
        .with_doc_mut(bad, |doc| {
            doc.font.bitmap_strikes.push(BitmapStrike {
                ppem: 16,
                depth: 3,
                ..Default::default()
            });
        })
        .unwrap();
    let good_font = Font::new_empty("Other document", 1_000);
    let good = session.open_font(good_font.clone(), None);
    session.with_doc_mut(good, |doc| doc.dirty = true).unwrap();
    autosave_pass(&session, &dir.0).unwrap();
    assert_eq!(std::fs::read(&saved).unwrap(), original_bytes);
    assert_eq!(fcf::read(&saved).unwrap(), initial);
    assert_eq!(
        fcf::read(&autosave_path(&dir.0, good, None)).unwrap(),
        good_font
    );

    // Repairing the source lets the next periodic pass resume normally.
    session
        .with_doc_mut(bad, |doc| {
            doc.font.bitmap_strikes.clear();
            doc.font.info.family_name = "Repaired".into();
        })
        .unwrap();
    autosave_pass(&session, &dir.0).unwrap();
    assert_eq!(fcf::read(&saved).unwrap().info.family_name, "Repaired");
}

#[test]
fn clean_documents_are_skipped_and_saved_provenance_survives_recovery() {
    let dir = TestDirectory::new();
    let session = Session::new();
    let clean = session.open_font(Font::new_empty("Clean", 1_000), None);
    let original = dir.0.join("project.fcf");
    let expected = Font::new_empty("Dirty", 1_000);
    let dirty = session.open_font(expected.clone(), Some(original.clone()));
    session.with_doc_mut(dirty, |doc| doc.dirty = true).unwrap();
    autosave_pass(&session, &dir.0).unwrap();
    assert!(!autosave_path(&dir.0, clean, None).exists());
    let entries = list_recoverable(&dir.0);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].doc_path, Some(original));
    assert_eq!(fcf::read(&entries[0].autosave_path).unwrap(), expected);
    session.with_doc(dirty, |doc| assert!(doc.dirty)).unwrap();
}

#[test]
fn stopping_a_worker_does_not_wait_for_its_autosave_interval() {
    let dir = TestDirectory::new();
    let session = Arc::new(Session::new());
    let handle = spawn(
        session,
        AutosaveConfig {
            interval: Duration::from_secs(60 * 60),
            dir: dir.0.clone(),
        },
    );
    let started = std::time::Instant::now();
    handle.stop();
    assert!(started.elapsed() < Duration::from_secs(1));
}

/// Row 7's recovery half, end to end for a directory project: an interrupted
/// session leaves a complete snapshot in the recovery location, it points back
/// at the project it belongs to, reopening lists it, and recovering it produces
/// the unsaved work as a dirty document that still knows its original path.
///
/// The snapshot deliberately lives outside the project so that damage to any
/// project member — including the manifest a crash may have left unwritten —
/// cannot take the recovery with it. The `document_path` pointer is what ties
/// the two together.
#[test]
fn an_interrupted_project_session_is_offered_for_recovery_and_restores_its_work() {
    let dir = TestDirectory::new();
    let recovery = dir.0.join("recovery");
    let project = dir.0.join("Work.fcfp");
    let mut saved = Font::new_empty("Work", 1_000);
    crate::fcf::write(&project, &saved).unwrap();

    // The session edits the project without saving, then "crashes": the
    // periodic pass has already written its snapshot.
    let session = Session::new();
    let doc = session.open_path(&project, None).unwrap();
    session
        .with_doc_mut(doc, |document| {
            document.font.info.family_name = "Work in progress".into();
            document.dirty = true;
        })
        .unwrap();
    autosave_pass(&session, &recovery).unwrap();
    drop(session);

    // A fresh launch finds exactly one offer, pointing at the project.
    let entries = list_recoverable(&recovery);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].doc_path.as_deref(), Some(project.as_path()));
    assert!(entries[0].autosave_path.starts_with(&recovery));

    // The committed project on disk still holds the last *saved* state.
    assert_eq!(crate::fcf::read(&project).unwrap(), saved);

    // Recovering reopens the unsaved work, keeping the original path so saving
    // overwrites the project the snapshot belongs to.
    let next = Session::new();
    let recovered = next.open_path(&entries[0].autosave_path, None).unwrap();
    next.with_doc_mut(recovered, |document| {
        assert_eq!(document.font.info.family_name, "Work in progress");
        document.path = Some(project.clone());
        document.dirty = true;
        document.save(&project)
    })
    .unwrap()
    .unwrap();
    saved.info.family_name = "Work in progress".into();
    assert_eq!(crate::fcf::read(&project).unwrap(), saved);

    // Once the project is newer than the snapshot, it is no longer offered.
    assert!(list_recoverable(&recovery).is_empty());
}
