use std::{fs, sync::Arc};

use fw_api::{Api, GlyphListResult};
use fw_doc::Session;
use fw_glyphdata::{category, Category};
use serde_json::json;

#[test]
fn lists_filters_sorts_pages_paths_and_searches_glyphs() {
    let config_dir = test_config_dir("glyph-list");
    std::env::set_var("FW_CONFIG_DIR", &config_dir);
    let api = Api::new(Arc::new(Session::new()));
    let opened = api
        .call(
            "session.open",
            json!({
                "path": fw_fixtures::path("DejaVuSans.ttf"),
                "face_index": null
            }),
        )
        .expect("open fixture");
    let doc = opened["doc"].clone();

    let letters = list(
        &api,
        json!({
            "doc": doc,
            "master": null,
            "filter": { "categories": ["Letter"] },
            "sort": "Unicode",
            "offset": 0,
            "limit": 2000,
            "include_paths": false
        }),
    );
    assert!(letters.total > 0);
    assert!(letters.items.iter().all(|glyph| {
        glyph
            .unicodes
            .first()
            .is_some_and(|unicode| category(*unicode) == Category::Letter)
    }));
    assert!(letters
        .items
        .windows(2)
        .all(|pair| pair[0].unicodes[0] <= pair[1].unicodes[0]));

    let glyph_a = list(
        &api,
        json!({
            "doc": doc,
            "master": null,
            "filter": { "query": "U+0041" },
            "sort": "Order",
            "offset": 0,
            "limit": 10,
            "include_paths": true
        }),
    );
    assert_eq!(glyph_a.total, 1);
    assert_eq!(glyph_a.items[0].name, "A");
    assert!(glyph_a.items[0]
        .path
        .as_deref()
        .is_some_and(|path| path.starts_with('M')));

    let first_page = list(&api, page_params(&doc, 0, 2));
    let second_page = list(&api, page_params(&doc, 2, 2));
    assert_eq!(first_page.total, second_page.total);
    assert_eq!(first_page.items.len(), 2);
    assert_eq!(second_page.items.len(), 2);
    assert_ne!(first_page.items[0].gid, second_page.items[0].gid);

    let search = api
        .call(
            "glyphs.search",
            json!({ "doc": doc, "query": "U+0041", "limit": 10 }),
        )
        .expect("search glyphs");
    assert_eq!(search[0]["name"], "A");

    fs::remove_dir_all(config_dir).ok();
}

fn list(api: &Api, params: serde_json::Value) -> GlyphListResult {
    serde_json::from_value(api.call("glyphs.list", params).expect("list glyphs"))
        .expect("deserialize glyph list")
}

fn page_params(doc: &serde_json::Value, offset: u32, limit: u32) -> serde_json::Value {
    json!({
        "doc": doc,
        "master": null,
        "filter": {},
        "sort": "Order",
        "offset": offset,
        "limit": limit,
        "include_paths": false
    })
}

fn test_config_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fontworkshop-fw-api-{label}-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&dir).ok();
    dir
}
