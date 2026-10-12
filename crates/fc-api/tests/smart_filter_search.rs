//! Search and smart filters share listing, paging and index lookup semantics.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Font, Glyph};
use serde_json::{json, Value};

#[test]
fn smart_search_intersects_filters_and_preserves_index_lookup() {
    let mut font = Font::new_empty("Filter fixture", 1000);
    for (name, scalar, note, export) in [
        ("A", 0x41, "Review capital", true),
        ("a", 0x61, "review lower", false),
        ("B", 0x42, "Approved", true),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(scalar);
        glyph.note = note.into();
        glyph.export = export;
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    let list =
        |filter: Value| {
            api.call("glyphs.list", json!({
        "doc":doc,"filter":filter,"sort":"Name","offset":0,"limit":10,"include_paths":false
    })).unwrap()
        };
    assert_eq!(
        list(json!({"query":"review","query_scope":"Note"}))["total"],
        2
    );
    assert_eq!(
        list(json!({"query":"review","query_scope":"Note","case_sensitive":true}))["items"][0]
            ["name"],
        "a"
    );
    let filter = json!({"query":"^review", "query_scope":"Note", "regex":true,
        "smart_expression":"export and (name:A or name:B)","glyph_names":["A","a"]});
    let result = list(filter.clone());
    assert_eq!(result["total"], 1);
    assert_eq!(result["items"][0]["name"], "A");
    assert_eq!(
        api.call(
            "glyphs.index_of",
            json!({"doc":doc,"gid":result["items"][0]["gid"],"filter":filter,"sort":"Name"})
        )
        .unwrap(),
        0
    );
    assert_eq!(list(json!({"glyph_names":[]}))["total"], 0);
    assert_eq!(list(json!({"glyph_names":["a"]}))["items"][0]["name"], "a");
    assert_eq!(list(json!({"categories":["Letter/Uppercase"]}))["total"], 2);
    assert_eq!(list(json!({"categories":["Letter/Lowercase"]}))["total"], 1);
    for filter in [
        json!({"query":"[","regex":true}),
        json!({"smart_expression":"export and unknown"}),
    ] {
        let error = api
            .call(
                "glyphs.list",
                json!({"doc":doc,"filter":filter,"offset":0,"limit":10,"include_paths":false}),
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_params");
    }
}
