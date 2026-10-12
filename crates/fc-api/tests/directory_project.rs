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
use serde_json::json;

#[test]
fn directory_project_api_save_reopen_recovery_and_save_again() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Native.fcfp");
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .unwrap();
    let mut font = Font::new_empty("Native project", 1000);
    let mut glyph = Glyph::new("A");
    glyph.unicodes.push(65);
    font.insert_glyph(glyph).unwrap();
    let doc = session.open_font(font.clone(), None);
    let saved = api
        .call("session.save", json!({"doc": doc, "path": path}))
        .unwrap();
    assert_eq!(saved["format"], "fcfp");
    assert_eq!(saved["dirty"], false);
    let opened = api.call("session.open", json!({"path": path})).unwrap();
    assert_eq!(opened["glyph_count"], 1);
    assert_eq!(opened["format"], "fcfp");
    assert_eq!(fc_doc::fcf::read(&path).unwrap(), font);
    session
        .with_doc_mut(doc, |document| {
            document.font.info.family_name = "Recovered project".into();
            document.dirty = true;
        })
        .unwrap();
    let handle = spawn(
        Arc::clone(&session),
        AutosaveConfig {
            interval: Duration::from_millis(10),
            dir: directory.path().join("config/autosave"),
        },
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    let entry = loop {
        let entries = api.call("session.recover_list", json!({})).unwrap();
        if let Some(entry) = entries.as_array().unwrap().first() {
            break entry.clone();
        }
        assert!(Instant::now() < deadline, "autosave did not complete");
        std::thread::sleep(Duration::from_millis(10));
    };
    handle.stop();
    let recovered = api
        .call(
            "session.recover",
            json!({"autosave_path": entry["autosave_path"]}),
        )
        .unwrap();
    assert_eq!(recovered["family_name"], "Recovered project");
    assert_eq!(recovered["format"], "fcfp");
    assert_eq!(recovered["dirty"], true);
    let saved = api
        .call("session.save", json!({"doc": recovered["doc"]}))
        .unwrap();
    assert_eq!(saved["dirty"], false);
    assert!(saved.get("recovered_from").is_none());
    assert_eq!(
        fc_doc::fcf::read(&path).unwrap().info.family_name,
        "Recovered project"
    );
}
