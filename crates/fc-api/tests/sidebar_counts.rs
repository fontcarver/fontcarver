use std::sync::Arc;

use fc_api::{Api, SidebarCounts};
use fc_doc::Session;
use fc_model::{Font, Glyph};
use serde_json::json;

#[test]
fn sidebar_subcategories_match_filters_and_inventory_includes_secondary_scalars() {
    let mut font = Font::new_empty("Sidebar", 1_000);
    for (name, unicodes) in [
        ("A", vec![0x0041, 0x0391, 0x0061, 0xD800, 0x110000]),
        ("A.alt", vec![0x0041]),
        ("a", vec![0x0061]),
        ("one", vec![0x0031]),
        ("unencoded", vec![]),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = unicodes;
        // These are editable metadata; sidebar filters use Unicode properties.
        glyph.category = Some("Symbol".to_owned());
        glyph.sub_category = Some("Math".to_owned());
        glyph
            .tags
            .extend(["review".to_owned(), "accent".to_owned()]);
        font.insert_glyph(glyph).expect("insert test glyph");
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);

    let sidebar = api
        .call("doc.sidebar", json!({ "doc": doc }))
        .expect("sidebar counts");
    assert_eq!(
        sidebar["sub_categories"],
        json!([
            ["Letter", "Lowercase", 1],
            ["Letter", "Uppercase", 2],
            ["Number", "Decimal", 1]
        ])
    );
    assert_eq!(sidebar["unicodes"], json!([0x0031, 0x0041, 0x0061, 0x0391]));
    assert_eq!(sidebar["tags"], json!(["accent", "review"]));
    assert_eq!(sidebar["total"], 5);
    assert_eq!(sidebar["unencoded"], 1);
    for (category, expected) in [("Letter/Uppercase", 2), ("Letter/Lowercase", 1)] {
        let glyphs = api
            .call(
                "glyphs.list",
                json!({
                    "doc": doc,
                    "filter": { "categories": [category] },
                    "offset": 0,
                    "limit": 10,
                    "include_paths": false
                }),
            )
            .expect("list subcategory filter");
        assert_eq!(glyphs["total"], expected);
    }
}

#[test]
fn qualified_subcategory_filters_distinguish_other_buckets() {
    let mut font = Font::new_empty("Sidebar other buckets", 1_000);
    for (name, unicode) in [("alef", 0x05D0), ("romanOne", 0x2160), ("control", 0x0000)] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = vec![unicode];
        font.insert_glyph(glyph).expect("insert test glyph");
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    let sidebar = api
        .call("doc.sidebar", json!({ "doc": doc }))
        .expect("sidebar counts");
    assert_eq!(
        sidebar["sub_categories"],
        json!([
            ["Letter", "Other", 1],
            ["Number", "Other", 1],
            ["Other", "Other", 1]
        ])
    );
    let glyphs = api
        .call(
            "glyphs.list",
            json!({
                "doc": doc,
                "filter": { "categories": ["Letter/Other"] },
                "offset": 0,
                "limit": 10,
                "include_paths": false
            }),
        )
        .expect("list qualified Other subcategory");
    assert_eq!(glyphs["total"], 1);
    assert_eq!(glyphs["items"][0]["name"], "alef");
}

#[test]
fn sidebar_inventory_and_subcategories_refresh_after_unicode_edits_and_undo() {
    let mut font = Font::new_empty("Sidebar edits", 1_000);
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![0x0041];
    font.insert_glyph(glyph).expect("insert test glyph");
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    let initial = api
        .call("doc.sidebar", json!({ "doc": doc }))
        .expect("cache initial sidebar");

    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "Batch": [
                { "SetUnicodes": { "glyph": "A", "unicodes": [0x0061, 0x0391] } },
                { "SetGlyphMeta": { "glyph": "A", "tags": ["review"] } }
            ] },
            "label": "Change glyph metadata"
        }),
    )
    .expect("edit Unicode assignments");
    let changed = api
        .call("doc.sidebar", json!({ "doc": doc }))
        .expect("refresh sidebar");
    assert_eq!(
        changed["sub_categories"],
        json!([["Letter", "Lowercase", 1]])
    );
    assert_eq!(changed["unicodes"], json!([0x0061, 0x0391]));
    assert_eq!(changed["tags"], json!(["review"]));

    api.call("doc.undo", json!({ "doc": doc }))
        .expect("undo Unicode edit");
    assert_eq!(
        api.call("doc.sidebar", json!({ "doc": doc }))
            .expect("refresh sidebar after undo"),
        initial
    );
}

#[test]
fn older_sidebar_payloads_default_added_inventory_fields() {
    let counts: SidebarCounts = serde_json::from_value(json!({
        "categories": [],
        "scripts": [],
        "blocks": [],
        "unencoded": 0,
        "total": 0,
        "color_labels": []
    }))
    .expect("deserialize old sidebar response");
    let serialized = serde_json::to_value(counts).expect("serialize sidebar response");
    assert_eq!(serialized["sub_categories"], json!([]));
    assert_eq!(serialized["unicodes"], json!([]));
    assert_eq!(serialized["tags"], json!([]));
}
