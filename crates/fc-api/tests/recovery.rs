//! Crash-recovery behaviour that spans a restart: the dirty baseline of a
//! recovered document, snapshot identity and snapshot cleanup.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use fc_api::Api;
use fc_doc::{
    autosave::{spawn, AutosaveConfig},
    Session,
};
use fc_model::{Font, Glyph};
use serde_json::{json, Value};

/// Waits for the autosave worker to publish a recoverable snapshot.
fn first_recoverable(api: &Api) -> Value {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let entries = api.call("session.recover_list", json!({})).unwrap();
        if let Some(entry) = entries.as_array().unwrap().first() {
            return entry.clone();
        }
        assert!(Instant::now() < deadline, "autosave did not complete");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn dirty_document(session: &Session, family: &str) -> fc_api::DocId {
    let mut font = Font::new_empty(family, 1000);
    font.insert_glyph(Glyph::new("A")).unwrap();
    let doc = session.open_font(font, None);
    session
        .with_doc_mut(doc, |document| {
            document.font.info.family_name = format!("{family} edited");
            document.dirty = true;
        })
        .unwrap();
    doc
}

#[test]
fn a_recovered_document_stays_dirty_after_an_edit_is_undone() {
    let directory = tempfile::tempdir().unwrap();
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .unwrap();
    dirty_document(&session, "Unsaved");
    let handle = spawn(
        Arc::clone(&session),
        AutosaveConfig {
            interval: Duration::from_millis(10),
            dir: directory.path().join("config/autosave"),
        },
    );
    let entry = first_recoverable(&api);
    handle.stop();

    let recovered = api
        .call(
            "session.recover",
            json!({"autosave_path": entry["autosave_path"]}),
        )
        .unwrap();
    assert_eq!(recovered["dirty"], true);
    let doc = recovered["doc"].clone();
    api.call("glyphs.add", json!({"doc": doc, "recipe": "B"}))
        .unwrap();
    api.call("doc.undo", json!({"doc": doc})).unwrap();

    let summary = api.call("doc.summary", json!({"doc": doc})).unwrap();
    assert_eq!(
        summary["dirty"], true,
        "the recovered state is on no disk, so undoing an edit cannot make it clean"
    );
    let saved = api
        .call(
            "session.save",
            json!({"doc": doc, "path": directory.path().join("Recovered.fcf")}),
        )
        .unwrap();
    assert_eq!(saved["dirty"], false);
}

#[test]
fn closing_a_document_with_discarded_changes_reclaims_its_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .unwrap();
    let doc = dirty_document(&session, "Abandoned");
    let handle = spawn(
        Arc::clone(&session),
        AutosaveConfig {
            interval: Duration::from_millis(10),
            dir: directory.path().join("config/autosave"),
        },
    );
    first_recoverable(&api);
    handle.stop();

    api.call("session.close", json!({"doc": doc})).unwrap();

    let entries = api.call("session.recover_list", json!({})).unwrap();
    assert_eq!(
        entries.as_array().unwrap().len(),
        0,
        "changes the user chose to abandon must not be offered after a normal exit"
    );
}

#[test]
fn saving_reclaims_the_snapshot_of_the_document_and_of_its_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .unwrap();
    dirty_document(&session, "Interrupted");
    let handle = spawn(
        Arc::clone(&session),
        AutosaveConfig {
            interval: Duration::from_millis(10),
            dir: directory.path().join("config/autosave"),
        },
    );
    let entry = first_recoverable(&api);
    handle.stop();

    let recovered = api
        .call(
            "session.recover",
            json!({"autosave_path": entry["autosave_path"]}),
        )
        .unwrap();
    api.call(
        "session.save",
        json!({"doc": recovered["doc"], "path": directory.path().join("Saved.fcf")}),
    )
    .unwrap();

    let entries = api.call("session.recover_list", json!({})).unwrap();
    assert_eq!(
        entries.as_array().unwrap().len(),
        0,
        "a saved document is the file on disk, so no snapshot of it is still waiting"
    );
}
