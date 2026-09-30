use std::sync::Arc;

use fc_api::{Api, GlyphCell};
use fc_doc::Session;
use fc_model::Font;
use serde_json::json;

#[test]
fn add_recipe_combines_names_characters_codepoints_and_ranges_with_one_undo() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(Font::new_empty("Recipes", 1000), None);
    let api = Api::new(session);
    let recipe = "custom.alt é U+03B1 U+0041-U+0043 x-z";
    let preview = api
        .call(
            "glyphs.add",
            json!({ "doc": doc, "recipe": recipe, "dry_run": true }),
        )
        .unwrap();
    assert_eq!(preview["skipped"], json!([]));
    let result = api
        .call("glyphs.add", json!({ "doc": doc, "recipe": recipe }))
        .unwrap();
    let added: Vec<GlyphCell> = serde_json::from_value(result["added"].clone()).unwrap();
    assert_eq!(added.len(), 9);
    assert_eq!(added[0].name, "custom.alt");
    assert!(added[0].unicodes.is_empty());
    for codepoint in [0xE9, 0x3B1, 0x41, 0x42, 0x43, 0x78, 0x79, 0x7A] {
        assert!(added.iter().any(|cell| cell.unicodes == vec![codepoint]));
    }
    let preview_names: Vec<_> = preview["added"]
        .as_array()
        .unwrap()
        .iter()
        .map(|cell| cell["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        preview_names,
        added
            .iter()
            .map(|cell| cell.name.as_str())
            .collect::<Vec<_>>()
    );
    api.call("doc.undo", json!({ "doc": doc })).unwrap();
    assert_eq!(
        api.call("doc.summary", json!({ "doc": doc })).unwrap()["glyph_count"],
        0
    );
    api.call("doc.redo", json!({ "doc": doc })).unwrap();
    assert_eq!(
        api.call("doc.summary", json!({ "doc": doc })).unwrap()["glyph_count"],
        9
    );
}
