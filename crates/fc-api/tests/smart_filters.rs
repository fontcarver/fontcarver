//! Saved smart-filter transactions retain unrelated metadata and native history.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::Font;
use serde_json::{json, Value};

fn setup(user_data: Value) -> (Api, DocId) {
    let mut font = Font::new_empty("Smart filters", 1000);
    font.info.user_data = user_data;
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session), doc)
}

fn state(api: &Api, doc: DocId) -> Value {
    api.call("glyph.smart_filters.get", json!({"doc":doc}))
        .unwrap()
}

#[test]
fn saved_filters_preserve_metadata_and_support_native_history() {
    let original = json!({
        "vendor": {"private": 7},
        "com.fontcarver.inventory": {
            "glyph_lists": [{"id":"letters", "name":"Letters", "glyphs":["A"]}],
            "future": true
        }
    });
    let (api, doc) = setup(original.clone());
    assert_eq!(state(&api, doc), json!({"filters":[]}));
    let saved = api
        .call(
            "glyph.smart_filters.set",
            json!({
                "doc":doc,
                "filters":[{"name":" Capitals ","expression":" name == \"A\" "}]
            }),
        )
        .unwrap();
    assert_eq!(saved["filters"][0]["name"], "Capitals");
    assert_eq!(state(&api, doc), saved);
    let metadata =
        api.call("doc.font_info", json!({"doc":doc})).unwrap()["info"]["user_data"].clone();
    assert_eq!(metadata["vendor"], original["vendor"]);
    assert_eq!(
        metadata["com.fontcarver.inventory"]["glyph_lists"],
        original["com.fontcarver.inventory"]["glyph_lists"]
    );
    assert_eq!(metadata["com.fontcarver.inventory"]["future"], true);

    let directory = tempfile::tempdir().unwrap();
    for extension in ["fcf", "fcf.gz"] {
        let path = directory.path().join(format!("filters.{extension}"));
        api.call("session.save", json!({"doc":doc,"path":path}))
            .unwrap();
        let opened = api.call("session.open", json!({"path":path})).unwrap();
        let reopened: DocId = serde_json::from_value(opened["doc"].clone()).unwrap();
        assert_eq!(state(&api, reopened), saved);
    }
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(state(&api, doc), json!({"filters":[]}));
    assert_eq!(
        api.call("doc.font_info", json!({"doc":doc})).unwrap()["info"]["user_data"],
        original
    );
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    assert_eq!(state(&api, doc), saved);
}

#[test]
fn invalid_saved_filters_fail_without_mutation() {
    let (api, doc) = setup(Value::Null);
    let before = api.call("doc.summary", json!({"doc":doc})).unwrap();
    for filters in [
        json!([{"name":"Broken", "expression":"name =="}]),
        json!([{"name":" ", "expression":"name == \"A\""}]),
        json!([{"name":"A", "expression":"name == \"A\""}, {"name":" A ", "expression":"name == \"B\""}]),
        json!([{"name":"x".repeat(257), "expression":"name == \"A\""}]),
        json!(vec![
            json!({"name":"Too many", "expression":"name == \"A\""});
            257
        ]),
    ] {
        let error = api
            .call(
                "glyph.smart_filters.set",
                json!({"doc":doc,"filters":filters}),
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_params");
        assert_eq!(state(&api, doc), json!({"filters":[]}));
        assert_eq!(api.call("doc.summary", json!({"doc":doc})).unwrap(), before);
    }
}

#[test]
fn malformed_inventory_is_reported_without_overwriting_it() {
    for original in [
        json!(["foreign metadata"]),
        json!({"com.fontcarver.inventory": "foreign metadata"}),
        json!({"com.fontcarver.inventory": {"smart_filters": [7]}}),
    ] {
        let (api, doc) = setup(original.clone());
        assert!(api
            .call("glyph.smart_filters.get", json!({"doc":doc}))
            .is_err());
        let before = api.call("doc.summary", json!({"doc":doc})).unwrap();
        assert!(api
            .call("glyph.smart_filters.set", json!({"doc":doc,"filters":[]}))
            .is_err());
        assert_eq!(api.call("doc.summary", json!({"doc":doc})).unwrap(), before);
        assert_eq!(
            api.call("doc.font_info", json!({"doc":doc})).unwrap()["info"]["user_data"],
            original
        );
    }
}

#[test]
fn parse_normalizes_expressions_and_rejects_invalid_input() {
    let (api, _) = setup(Value::Null);
    let result = api
        .call(
            "glyph.smart_filters.parse",
            json!({"expression":" name == \"A\" "}),
        )
        .unwrap();
    assert!(result["root"].is_object());
    let again = api
        .call(
            "glyph.smart_filters.parse",
            json!({"expression":result["expression"]}),
        )
        .unwrap();
    assert_eq!(result, again);
    assert!(api
        .call(
            "glyph.smart_filters.parse",
            json!({"expression":"unknown == 1"})
        )
        .is_err());
}

#[test]
fn saving_identical_filters_does_not_create_an_undo_step() {
    let (api, doc) = setup(Value::Null);
    let params = json!({"doc":doc,"filters":[{"name":"A", "expression":"name == \"A\""}]});
    api.call("glyph.smart_filters.set", params.clone()).unwrap();
    let before = api.call("doc.summary", json!({"doc":doc})).unwrap();
    api.call("glyph.smart_filters.set", params).unwrap();
    assert_eq!(api.call("doc.summary", json!({"doc":doc})).unwrap(), before);
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(state(&api, doc), json!({"filters":[]}));
}
