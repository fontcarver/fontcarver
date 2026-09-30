use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Font, Glyph, Layer};
use serde_json::{json, Value};
use std::sync::Arc;

fn fixture() -> (Api, Arc<Session>, DocId, Value) {
    let mut font = Font::new_empty("Image workflows", 1000);
    let mut glyph = Glyph::new("A");
    let layer = Layer::new_master(font.default_master().id.clone());
    let target = json!({"glyph":"A","layer":layer.id});
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let mut target = target;
    target["doc"] = json!(doc);
    (Api::new(session.clone()), session, doc, target)
}

// Authored 2x2 RGBA source: black top-left, opaque white elsewhere.
fn png() -> Vec<u8> {
    PNG.to_vec()
}

fn import(api: &Api, target: &Value) -> Value {
    let mut params = target.clone();
    params["expected_generation"] =
        api.call("image.state", target.clone()).unwrap()["generation"].clone();
    params["path"] = Value::Null;
    params["data"] = json!(png());
    params["file_name"] = json!("source.png");
    params["embed"] = json!(true);
    api.call("image.import", params).unwrap()
}

#[test]
fn embedded_image_transform_trace_preview_apply_undo_and_native_source() {
    let (api, session, doc, target) = fixture();
    assert_eq!(
        api.call("image.state", target.clone()).unwrap()["image"],
        Value::Null
    );
    import(&api, &target);
    let mut state = api.call("image.state", target.clone()).unwrap();
    assert_eq!(state["width"], 2);
    assert_eq!(state["height"], 2);
    assert_eq!(state["png"], json!(png()));
    state["image"]["transform"] = json!([2, 0, 0, 3, 10, 20]);
    let mut set = target.clone();
    set["expected_generation"] = state["generation"].clone();
    set["image"] = state["image"].clone();
    api.call("image.set", set).unwrap();
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let mut params = target.clone();
    params["expected_generation"] = Value::Null;
    params["threshold"] = json!(128);
    params["tolerance"] = json!(0);
    params["dry_run"] = json!(true);
    params["append"] = json!(false);
    let preview = api.call("image.trace", params.clone()).unwrap();
    assert_eq!(preview["bounds"], json!([10.0, 23.0, 12.0, 26.0]));
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    params["dry_run"] = json!(false);
    params["expected_generation"] = preview["generation"].clone();
    params["expected_source_hash"] = preview["source_hash"].clone();
    let applied = api.call("image.trace", params.clone()).unwrap();
    assert_eq!(preview["contours"], applied["contours"]);
    let after = session.with_doc(doc, |d| d.font.clone()).unwrap();
    assert_eq!(
        after.glyph("A").unwrap().layers[0].background_image,
        before.glyph("A").unwrap().layers[0].background_image
    );
    assert!(!after.glyph("A").unwrap().layers[0].contours.is_empty());
    assert!(api
        .call("image.trace", params)
        .unwrap_err()
        .message
        .contains("stale"));
    session.with_doc_mut(doc, |d| d.undo()).unwrap().unwrap();
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    session.with_doc_mut(doc, |d| d.redo()).unwrap().unwrap();
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), after);
    let encoded = serde_json::to_vec(&after).unwrap();
    let reopened: Font = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(reopened, after);
}

#[test]
fn lock_rejects_moving_replacing_and_removing_but_allows_explicit_unlock() {
    let (api, session, doc, target) = fixture();
    import(&api, &target);
    let state = api.call("image.state", target.clone()).unwrap();
    let mut set = target.clone();
    set["expected_generation"] = state["generation"].clone();
    set["image"] = state["image"].clone();
    set["image"]["locked"] = json!(true);
    api.call("image.set", set.clone()).unwrap();
    let locked = api.call("image.state", target.clone()).unwrap();
    set["expected_generation"] = locked["generation"].clone();
    set["image"]["transform"] = json!([1, 0, 0, 1, 100, 0]);
    assert!(api
        .call("image.set", set.clone())
        .unwrap_err()
        .message
        .contains("locked"));
    set["image"] = Value::Null;
    assert!(api.call("image.set", set.clone()).is_err());
    set["image"] = locked["image"].clone();
    set["image"]["locked"] = json!(false);
    api.call("image.set", set).unwrap();
    assert!(!session
        .with_doc(doc, |d| d.font.glyph("A").unwrap().layers[0]
            .background_image
            .as_ref()
            .unwrap()
            .locked)
        .unwrap());
}

#[test]
fn missing_links_invalid_png_and_stale_writes_are_reported_without_mutation() {
    let (api, session, doc, target) = fixture();
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let mut params = target.clone();
    params["expected_generation"] = json!(0);
    params["path"] = Value::Null;
    params["data"] = json!([1, 2, 3]);
    params["file_name"] = json!("bad.png");
    params["embed"] = json!(true);
    assert!(api.call("image.import", params).is_err());
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    session
        .with_doc_mut(doc, |d| {
            d.font.glyph_mut("A").unwrap().layers[0].background_image = Some(fc_model::Image {
                file_name: "missing.png".into(),
                ..Default::default()
            })
        })
        .unwrap();
    let state = api.call("image.state", target).unwrap();
    assert!(state["error"].as_str().is_some());
    assert_eq!(state["png"], Value::Null);
}

const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0,
    0, 0, 114, 182, 13, 36, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 96, 96, 96, 248, 15, 2, 16,
    18, 8, 0, 95, 178, 12, 244, 216, 82, 206, 121, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[test]
fn relative_png_links_report_breakage_and_reject_changed_preview_sources() {
    let (api, session, doc, target) = fixture();
    let dir = tempfile::tempdir().unwrap();
    let document = dir.path().join("font.fcf");
    let source = dir.path().join("source.png");
    std::fs::write(&source, png()).unwrap();
    session
        .with_doc_mut(doc, |d| d.save(&document))
        .unwrap()
        .unwrap();
    let mut params = target.clone();
    params["expected_generation"] =
        api.call("image.state", target.clone()).unwrap()["generation"].clone();
    params["path"] = json!(source);
    params["data"] = Value::Null;
    params["file_name"] = json!("");
    params["embed"] = json!(false);
    api.call("image.import", params).unwrap();
    let state = api.call("image.state", target.clone()).unwrap();
    assert_eq!(state["image"]["file_name"], "source.png");
    assert_eq!(state["image"]["data"], Value::Null);
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let mut trace = target.clone();
    trace["threshold"] = json!(128);
    trace["tolerance"] = json!(0);
    trace["append"] = json!(false);
    trace["dry_run"] = json!(true);
    trace["expected_generation"] = Value::Null;
    let preview = api.call("image.trace", trace.clone()).unwrap();
    std::fs::write(&source, WHITE_PNG).unwrap();
    trace["dry_run"] = json!(false);
    trace["expected_generation"] = preview["generation"].clone();
    trace["expected_source_hash"] = preview["source_hash"].clone();
    assert!(api
        .call("image.trace", trace)
        .unwrap_err()
        .message
        .contains("source changed"));
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    std::fs::remove_file(&source).unwrap();
    let broken = api.call("image.state", target).unwrap();
    assert!(broken["error"].is_string());
    assert_eq!(broken["image"], state["image"]);
    assert_eq!(broken["png"], Value::Null);
}

#[test]
fn invalid_placement_and_glyph_locks_are_atomic() {
    let (api, session, doc, target) = fixture();
    import(&api, &target);
    let state = api.call("image.state", target.clone()).unwrap();
    let original = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let mut set = target;
    set["expected_generation"] = state["generation"].clone();
    set["image"] = state["image"].clone();
    set["image"]["transform"] = json!([0, 0, 0, 0, 0, 0]);
    assert!(api.call("image.set", set.clone()).is_err());
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), original);
    session
        .with_doc_mut(doc, |d| d.font.glyph_mut("A").unwrap().locked = true)
        .unwrap();
    set["image"] = Value::Null;
    assert!(api
        .call("image.set", set)
        .unwrap_err()
        .message
        .contains("locked"));
}

const WHITE_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0,
    0, 0, 114, 182, 13, 36, 0, 0, 0, 14, 73, 68, 65, 84, 120, 156, 99, 248, 15, 5, 12, 48, 6, 0,
    143, 130, 15, 241, 60, 165, 86, 81, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[test]
fn a_corrupt_embedded_image_can_be_unlocked_and_removed_for_repair() {
    let (api, session, doc, target) = fixture();
    session
        .with_doc_mut(doc, |d| {
            d.font.glyph_mut("A").unwrap().layers[0].background_image = Some(fc_model::Image {
                file_name: "broken.png".into(),
                locked: true,
                data: Some(vec![1, 2, 3]),
                ..Default::default()
            });
        })
        .unwrap();
    let state = api.call("image.state", target.clone()).unwrap();
    assert!(state["error"].is_string());
    let mut params = target.clone();
    params["expected_generation"] = state["generation"].clone();
    params["image"] = state["image"].clone();
    params["image"]["locked"] = json!(false);
    api.call("image.set", params.clone()).unwrap();
    params["expected_generation"] = api.call("image.state", target).unwrap()["generation"].clone();
    params["image"] = Value::Null;
    api.call("image.set", params).unwrap();
    assert!(session
        .with_doc(doc, |d| d.font.glyph("A").unwrap().layers[0]
            .background_image
            .is_none())
        .unwrap());
}
