//! The autosave worker follows the stored preferences while it runs.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use fc_api::Api;
use fc_doc::Session;
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

/// Stores one autosave interval through the public preference handlers.
fn set_autosave_seconds(api: &Api, seconds: u32) {
    let mut prefs = api.call("prefs.get", json!({})).unwrap();
    prefs["autosave_seconds"] = json!(seconds);
    api.call("prefs.set", json!({"prefs": prefs})).unwrap();
}

fn discard_every_snapshot(api: &Api) {
    for entry in api
        .call("session.recover_list", json!({}))
        .unwrap()
        .as_array()
        .unwrap()
    {
        api.call(
            "session.discard_recovery",
            json!({"autosave_path": entry["autosave_path"]}),
        )
        .unwrap();
    }
}

#[test]
fn an_autosave_interval_change_reaches_the_running_worker() {
    let directory = tempfile::tempdir().unwrap();
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .unwrap();
    set_autosave_seconds(&api, 1);
    api.start_autosave().unwrap();
    let mut font = Font::new_empty("Interval", 1_000);
    font.insert_glyph(Glyph::new("A")).unwrap();
    let doc = session.open_font(font, None);
    session
        .with_doc_mut(doc, |document| {
            document.font.info.family_name = "Interval edit".to_owned();
            document.dirty = true;
        })
        .unwrap();
    first_recoverable(&api);

    // Turning autosave off stops the worker instead of waiting for a restart.
    set_autosave_seconds(&api, 0);
    discard_every_snapshot(&api);
    session
        .with_doc_mut(doc, |document| {
            document.font.info.family_name = "Interval again".into();
            document.dirty = true;
        })
        .unwrap();
    std::thread::sleep(Duration::from_millis(2_500));
    let entries = api.call("session.recover_list", json!({})).unwrap();
    assert_eq!(
        entries.as_array().unwrap().len(),
        0,
        "autosave was switched off, so no further snapshot may be written"
    );

    // Turning it back on starts the worker again.
    set_autosave_seconds(&api, 1);
    first_recoverable(&api);
}
