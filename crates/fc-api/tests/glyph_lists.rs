//! Named glyph inventories persist exact names and remain reversible.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Font, Glyph};
use serde_json::{json, Value};

fn fixture(user_data: Value) -> (Api, Arc<Session>, DocId) {
    let mut font = Font::new_empty("Glyph lists", 1000);
    font.info.user_data = user_data;
    font.insert_glyph(Glyph::new("A")).unwrap();
    font.insert_glyph(Glyph::new("a.sc")).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session.clone()), session, doc)
}

fn read(api: &Api, doc: DocId) -> Value {
    api.call("inventory.get_lists", json!({"doc": doc}))
        .unwrap()
}

fn write(api: &Api, doc: DocId, lists: Value) -> Value {
    api.call("inventory.set_lists", json!({"doc": doc, "lists": lists}))
        .unwrap()
}

#[test]
fn stores_exact_ordered_names_and_classifies_presence() {
    let (api, session, doc) = fixture(Value::Null);
    assert_eq!(read(&api, doc), json!({"lists": []}));
    let result = write(
        &api,
        doc,
        json!([
            {"id":"latin", "name":" Latin set ", "glyphs":["A","a","a.sc","A","f_i"]},
            {"id":"empty", "name":"Empty", "glyphs":[]}
        ]),
    );
    assert_eq!(
        result["lists"][0],
        json!({
            "id":"latin", "name":"Latin set", "glyphs":["A","a","a.sc","f_i"],
            "present":["A","a.sc"], "missing":["a","f_i"]
        })
    );
    assert_eq!(result["lists"][1]["present"], json!([]));
    assert_eq!(result, read(&api, doc));
    session
        .with_doc_mut(doc, |document| {
            document.font.insert_glyph(Glyph::new("a")).unwrap();
        })
        .unwrap();
    assert_eq!(
        read(&api, doc)["lists"][0]["present"],
        json!(["A", "a", "a.sc"])
    );
}

#[test]
fn preserves_siblings_undo_redo_noop_and_native_roundtrip() {
    let original = json!({
        "other.vendor":{"preserved":[1,2]},
        "com.fontcarver.inventory":{"smart_filters":[{"id":"custom","expression":"name == A"}],"future":17}
    });
    let (api, session, doc) = fixture(original.clone());
    let lists = json!([{ "id":"latin", "name":"Latin", "glyphs":["A","B"] }]);
    let saved = write(&api, doc, lists.clone());
    let snapshot = session
        .with_doc(doc, |document| {
            assert_eq!(document.undo_len(), 1);
            document.font.clone()
        })
        .unwrap();
    assert_eq!(
        snapshot.info.user_data["other.vendor"],
        original["other.vendor"]
    );
    assert_eq!(
        snapshot.info.user_data["com.fontcarver.inventory"]["smart_filters"],
        original["com.fontcarver.inventory"]["smart_filters"]
    );
    assert_eq!(
        snapshot.info.user_data["com.fontcarver.inventory"]["future"],
        17
    );
    assert_eq!(write(&api, doc, lists), saved);
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        1
    );
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(read(&api, doc), json!({"lists":[]}));
    assert_eq!(
        session
            .with_doc(doc, |document| document.font.info.user_data.clone())
            .unwrap(),
        original
    );
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    assert_eq!(read(&api, doc), saved);

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("glyph-lists.fcf");
    fc_doc::fcf::write(&path, &snapshot).unwrap();
    let reopened = session.open_font(fc_doc::fcf::read(&path).unwrap(), Some(path));
    assert_eq!(read(&api, reopened), saved);
    write(&api, doc, json!([]));
    assert_eq!(read(&api, doc), json!({"lists":[]}));
    assert_eq!(read(&api, reopened), saved);
}

#[test]
fn rejects_invalid_inputs_without_editing_document() {
    let (api, session, doc) = fixture(json!({"keep":true}));
    let before = session
        .with_doc(doc, |document| document.font.clone())
        .unwrap();
    let mut invalid_lists = vec![
        json!([{"id":"", "name":"Latin", "glyphs":[]}]),
        json!([{"id":"bad id", "name":"Latin", "glyphs":[]}]),
        json!([{"id":"x".repeat(81), "name":"Latin", "glyphs":[]}]),
        json!([{"id":"valid", "name":"  ", "glyphs":[]}]),
        json!([{"id":"valid", "name":"x".repeat(129), "glyphs":[]}]),
        json!([{"id":"valid", "name":"Line\nbreak", "glyphs":[]}]),
        json!([{"id":"same", "name":"First", "glyphs":[]},{"id":"same", "name":"Second", "glyphs":[]}]),
        json!([{"id":"large", "name":"Large", "glyphs":vec!["A"; 8193]}]),
        json!([{"id":"large", "name":"Large", "glyphs":["A".repeat(128)]}]),
        json!((0..129)
            .map(|index| json!({"id":format!("list_{index}"), "name":"List", "glyphs":[]}))
            .collect::<Vec<_>>()),
    ];
    for token in [
        "",
        "A B",
        "A-Z",
        "U+0041",
        "@charset:GB2312",
        "A,B",
        "一",
        "12bad",
    ] {
        invalid_lists.push(json!([{"id":"invalid", "name":"Invalid", "glyphs":[token]}]));
    }
    for lists in invalid_lists {
        let error = api
            .call("inventory.set_lists", json!({"doc":doc,"lists":lists}))
            .unwrap_err();
        assert_eq!(error.code, "invalid_params", "{error}");
        assert_eq!(
            session
                .with_doc(doc, |document| document.undo_len())
                .unwrap(),
            0
        );
        assert_eq!(
            session
                .with_doc(doc, |document| document.font.clone())
                .unwrap(),
            before
        );
    }
}

#[test]
fn malformed_metadata_is_reported_and_not_overwritten() {
    for metadata in [
        json!("opaque"),
        json!({"com.fontcarver.inventory":17}),
        json!({"com.fontcarver.inventory":{"glyph_lists":{}}}),
    ] {
        let (api, session, doc) = fixture(metadata.clone());
        assert!(api.call("inventory.get_lists", json!({"doc":doc})).is_err());
        assert!(api
            .call("inventory.set_lists", json!({"doc":doc,"lists":[]}))
            .is_err());
        assert_eq!(
            session
                .with_doc(doc, |document| document.font.info.user_data.clone())
                .unwrap(),
            metadata
        );
        assert_eq!(
            session
                .with_doc(doc, |document| document.undo_len())
                .unwrap(),
            0
        );
    }
}
