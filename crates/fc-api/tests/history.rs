//! `doc.history` and `doc.history_go`: what the History panel reads and clicks.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Font, Glyph};
use serde_json::json;

/// A font with one glyph, opened in its own session.
fn open() -> (Arc<Session>, Api, u32) {
    let mut font = Font::new_empty("History", 1000);
    font.insert_glyph(Glyph::new("A")).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(Arc::clone(&session));
    (session, api, doc.0)
}

/// Renames the family, which is one font-level entry.
fn rename(api: &Api, doc: u32, style: &str) {
    let info = api.call("doc.font_info", json!({ "doc": doc })).unwrap();
    let mut value = info["info"].clone();
    value["style_name"] = json!(style);
    api.call(
        "doc.apply",
        json!({ "doc": doc, "command": { "SetFontInfo": { "info": value } },
                "label": format!("Rename to {style}") }),
    )
    .unwrap();
}

#[test]
fn history_lists_each_gesture_with_its_scope_and_cursor() {
    let (_session, api, doc) = open();
    let empty = api.call("doc.history", json!({ "doc": doc })).unwrap();
    assert_eq!(empty["entries"].as_array().unwrap().len(), 0);
    assert_eq!(empty["cursor"], 0);
    assert_eq!(empty["len"], 0);
    assert_eq!(empty["truncated"], false);

    rename(&api, doc, "Bold");
    api.call(
        "doc.apply",
        json!({ "doc": doc,
                "command": { "SetGlyphMeta": { "glyph": "A", "export": false } },
                "label": "Stop exporting A" }),
    )
    .unwrap();

    let listed = api.call("doc.history", json!({ "doc": doc })).unwrap();
    let entries = listed["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2, "{listed}");
    assert_eq!(listed["cursor"], 2);
    assert_eq!(entries[0]["label"], "Rename to Bold");
    assert_eq!(entries[0]["font_level"], true);
    assert_eq!(entries[0]["undone"], false);
    assert_eq!(entries[1]["glyphs"], json!(["A"]));
    // An unnamed layer reports a role, never English prose and never a UUID.
    for layer in entries[1]["layers"].as_array().unwrap() {
        assert!(
            layer["name"].is_null() || layer["role"].is_null(),
            "{layer}"
        );
        if let Some(role) = layer["role"].as_str() {
            assert!(
                [
                    "master",
                    "background",
                    "intermediate",
                    "alternate",
                    "smart",
                    "backup",
                    "other"
                ]
                .contains(&role),
                "{role}"
            );
        }
    }
}

#[test]
fn history_go_moves_the_cursor_both_ways_and_leaves_the_document_consistent() {
    let (session, api, doc) = open();
    rename(&api, doc, "Bold");
    rename(&api, doc, "Black");
    let style = |session: &Session| {
        session
            .with_doc(fc_doc::DocId(doc), |document| {
                document.font.info.style_name.clone()
            })
            .unwrap()
    };
    assert_eq!(style(&session), "Black");

    // Back to the state before any edit, then forward to the first one.
    let start = api
        .call(
            "doc.history_go",
            json!({ "doc": doc, "cursor": 0, "expected_len": 2 }),
        )
        .unwrap();
    assert_eq!(start["cursor"], 0);
    assert_eq!(start["entries"].as_array().unwrap()[0]["undone"], true);
    assert_eq!(style(&session), "Regular");

    let first = api
        .call(
            "doc.history_go",
            json!({ "doc": doc, "cursor": 1, "expected_len": 2 }),
        )
        .unwrap();
    assert_eq!(first["cursor"], 1);
    assert_eq!(style(&session), "Bold");
}

#[test]
fn a_stale_or_impossible_request_is_refused_without_moving_the_cursor() {
    let (session, api, doc) = open();
    rename(&api, doc, "Bold");
    rename(&api, doc, "Black");
    let cursor = || {
        session
            .with_doc(fc_doc::DocId(doc), |document| document.history().cursor())
            .unwrap()
    };
    assert_eq!(cursor(), 2);

    for (params, needle) in [
        (
            json!({ "doc": doc, "cursor": 0, "expected_len": 5 }),
            "entries",
        ),
        (
            json!({ "doc": doc, "cursor": 9, "expected_len": 2 }),
            "beyond",
        ),
    ] {
        let error = api.call("doc.history_go", params.clone()).unwrap_err();
        assert_eq!(error.code, "invalid_params", "{params}");
        assert!(error.message.contains(needle), "{}", error.message);
        assert_eq!(cursor(), 2, "the cursor moved on a refused request");
    }
}

/// A replay a document refuses part way through must not leave the cursor
/// half-moved: the document goes back where it was and the caller is told.
#[test]
fn a_refused_replay_leaves_the_cursor_where_it_started() {
    let (session, api, doc) = open();
    rename(&api, doc, "Bold");
    rename(&api, doc, "Black");
    // A read-only reference refuses every mutation, including undo.
    session
        .with_doc_mut(fc_doc::DocId(doc), |document| {
            document.set_read_only(true);
        })
        .unwrap();

    let error = api
        .call(
            "doc.history_go",
            json!({ "doc": doc, "cursor": 0, "expected_len": 2 }),
        )
        .unwrap_err();
    assert!(error.message.contains("read-only"), "{}", error.message);
    session
        .with_doc(fc_doc::DocId(doc), |document| {
            assert_eq!(document.history().cursor(), 2);
            assert_eq!(document.font.info.style_name, "Black");
        })
        .unwrap();
}
