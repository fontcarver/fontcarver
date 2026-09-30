//! Database-driven generation, the new grid orders and the layer colour cell.

use std::sync::Arc;

use fc_api::{Api, GenerateGlyphsResult, GlyphListResult};
use fc_doc::{DocId, Session};
use fc_model::{Anchor, Contour, Font, Glyph, Layer, MasterId};
use kurbo::Shape;
use serde_json::json;

fn base_font() -> Font {
    let mut font = Font::new_empty("Generate", 1_000);
    let master = font.default_master().id.clone();
    font.insert_glyph(letter(&master)).expect("A");
    font.insert_glyph(mark(&master)).expect("acutecomb");
    font
}

fn letter(master: &MasterId) -> Glyph {
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![0x0041];
    let mut layer = Layer::new_master(master.clone());
    layer.width = 600.0;
    layer.contours = Contour::from_bezpath(&kurbo::Rect::new(20.0, 0.0, 580.0, 700.0).to_path(0.1));
    layer.anchors.push(Anchor {
        name: "top".into(),
        x: 300.0,
        y: 700.0,
    });
    glyph.layers.push(layer);
    glyph
}

fn mark(master: &MasterId) -> Glyph {
    let mut glyph = Glyph::new("acutecomb");
    glyph.unicodes = vec![0x0301];
    let mut layer = Layer::new_master(master.clone());
    layer.contours =
        Contour::from_bezpath(&kurbo::Rect::new(-40.0, 620.0, 40.0, 760.0).to_path(0.1));
    layer.anchors.push(Anchor {
        name: "_top".into(),
        x: 0.0,
        y: 620.0,
    });
    glyph.layers.push(layer);
    glyph
}

fn open(font: Font) -> (Api, Arc<Session>, DocId) {
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session.clone()), session, doc)
}

fn generate(api: &Api, doc: DocId, recipe: &str, dry_run: bool) -> GenerateGlyphsResult {
    serde_json::from_value(
        api.call(
            "glyphdata.generate_glyphs",
            json!({"doc": doc, "recipe": recipe, "master": null, "dry_run": dry_run}),
        )
        .expect("generate glyphs"),
    )
    .expect("generate shape")
}

fn list(api: &Api, params: serde_json::Value) -> GlyphListResult {
    serde_json::from_value(api.call("glyphs.list", params).expect("list")).expect("list shape")
}

#[test]
fn generation_previews_and_applies_composites_in_one_undo_step() {
    let (api, session, doc) = open(base_font());
    let preview = generate(&api, doc, "Aacute Agrave a.sc", true);
    assert_eq!(
        preview
            .added
            .iter()
            .map(|cell| cell.name.as_str())
            .collect::<Vec<_>>(),
        ["Aacute", "Agrave", "a.sc"]
    );
    // `Agrave` needs `gravecomb`, which this font does not have.
    let outcomes: Vec<_> = preview
        .composed
        .iter()
        .map(|entry| {
            (
                entry.name.as_str(),
                entry.components.len(),
                entry.unavailable.len(),
            )
        })
        .collect();
    assert_eq!(
        outcomes,
        [("Aacute", 2, 0), ("Agrave", 0, 1), ("a.sc", 0, 0)]
    );
    assert!(preview.added[0].has_components);
    assert!(!preview.added[0].is_empty);
    assert_eq!(preview.added[0].width, 600.0);
    assert!(preview.added[1].is_empty);
    // A dry run changes nothing.
    session
        .with_doc(doc, |document| {
            assert!(document.font.glyph("Aacute").is_none());
            assert_eq!(document.undo_label(), None);
        })
        .unwrap();

    let applied = generate(&api, doc, "Aacute Agrave a.sc", false);
    assert_eq!(applied.added.len(), 3);
    assert!(applied.added.iter().all(|cell| cell.gid.0 != u32::MAX));
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.glyphs.len(), 5);
            assert_eq!(document.undo_label(), Some("Generate glyphs"));
            let layer = document
                .font
                .glyph("Aacute")
                .unwrap()
                .master_layer(&document.font.default_master().id)
                .unwrap();
            assert_eq!(layer.components.len(), 2);
            assert_eq!(
                layer.components[1].transform.translation(),
                kurbo::Vec2::new(300.0, 80.0)
            );
            assert!(layer.anchors.iter().any(|anchor| anchor.name == "top"));
        })
        .unwrap();

    api.call("doc.undo", json!({"doc": doc})).expect("undo");
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.glyphs.len(), 2);
        })
        .unwrap();
    api.call("doc.redo", json!({"doc": doc})).expect("redo");
    session
        .with_doc(doc, |document| {
            assert!(document.font.glyph("Aacute").is_some());
        })
        .unwrap();
}

#[test]
fn generation_skips_existing_unknown_and_oversized_recipe_items() {
    let (api, _session, doc) = open(base_font());
    let result = generate(&api, doc, "A 12three U+0000-U+10FFFF Aacute Aacute", true);
    assert_eq!(
        result
            .added
            .iter()
            .map(|cell| cell.name.as_str())
            .collect::<Vec<_>>(),
        ["Aacute"]
    );
    assert_eq!(
        result.skipped,
        ["A", "12three", "too-large: U+0000-U+10FFFF", "Aacute"]
    );
}

#[test]
fn a_count_only_request_answers_without_building_cells() {
    let (api, session, doc) = open(base_font());
    let counted: GenerateGlyphsResult = serde_json::from_value(
        api.call(
            "glyphdata.generate_glyphs",
            json!({
                "doc": doc,
                "recipe": "A Aacute Agrave uni4E00-uni4E0F",
                "master": null,
                "dry_run": true,
                "count_only": true
            }),
        )
        .expect("count only"),
    )
    .expect("shape");
    // `A` exists; the other 18 names would be created.
    assert_eq!(counted.added_count, 18);
    assert!(counted.added.is_empty(), "no cells are built for a count");
    assert!(counted.composed.is_empty());
    assert_eq!(counted.skipped, ["A"]);
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.glyphs.len(), 2, "counting changes nothing");
        })
        .unwrap();

    // The same recipe as an ordinary dry run reports the same number and the cells.
    let previewed = generate(&api, doc, "A Aacute Agrave uni4E00-uni4E0F", true);
    assert_eq!(previewed.added_count, 18);
    assert_eq!(previewed.added.len(), 18);
    assert_eq!(previewed.composed.len(), 18);
}

#[test]
fn generation_is_bounded_and_composes_from_glyphs_made_in_the_same_run() {
    let mut font = Font::new_empty("Staged", 1_000);
    let master = font.default_master().id.clone();
    font.insert_glyph(mark(&master)).expect("acutecomb");
    let (api, session, doc) = open(font);
    // `A` is created first in recipe order, so `Aacute` can use it.
    let result = generate(&api, doc, "A Aacute", false);
    assert_eq!(result.added.len(), 2);
    assert_eq!(result.composed[1].components, ["A", "acutecomb"]);
    session
        .with_doc(doc, |document| {
            let layer = document
                .font
                .glyph("Aacute")
                .unwrap()
                .master_layer(&document.font.default_master().id)
                .unwrap();
            assert_eq!(layer.components.len(), 2);
            // The staged `A` has no anchors, so the mark stays at the origin
            // rather than being placed somewhere invented.
            assert_eq!(
                layer.components[1].transform.translation(),
                kurbo::Vec2::new(0.0, 0.0)
            );
        })
        .unwrap();

    let capped = generate(&api, doc, "U+1000-U+4000", true);
    assert_eq!(capped.added.len(), 8_192);
    assert!(capped
        .skipped
        .iter()
        .any(|entry| entry.starts_with("too-many: ")));
}

#[test]
fn grid_orders_by_category_script_and_badge_weight() {
    let mut font = Font::new_empty("Orders", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicodes) in [
        ("one", vec![0x0031_u32]),
        ("A", vec![0x0041]),
        ("alpha", vec![0x03B1]),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = unicodes;
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).expect("insert");
    }
    // `one` carries two warning badges and a notice; `A` carries one notice.
    font.glyph_mut("one").unwrap().metrics_keys.width = Some("=missing".into());
    font.glyph_mut("one").unwrap().note = "check".into();
    font.glyph_mut("A").unwrap().locked = true;
    let (api, _session, doc) = open(font);

    let by_category = list(
        &api,
        json!({"doc": doc, "filter": {}, "sort": "Category", "offset": 0, "limit": 10, "include_paths": false}),
    );
    assert_eq!(
        by_category
            .items
            .iter()
            .map(|cell| cell.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "A", "one"],
        "alphabetically by the text the column shows: Letter/Lowercase, \
         Letter/Uppercase, then Number/Decimal"
    );

    let by_script = list(
        &api,
        json!({"doc": doc, "filter": {}, "sort": "Script", "offset": 0, "limit": 10, "include_paths": false}),
    );
    assert_eq!(
        by_script
            .items
            .iter()
            .map(|cell| cell.script.as_str())
            .collect::<Vec<_>>(),
        ["Grek", "Latn", "Zyyy"]
    );

    let by_status = list(
        &api,
        json!({"doc": doc, "filter": {}, "sort": "Status", "offset": 0, "limit": 10, "include_paths": false}),
    );
    assert_eq!(
        by_status
            .items
            .iter()
            .map(|cell| cell.name.as_str())
            .collect::<Vec<_>>(),
        ["one", "A", "alpha"],
        "most badges first"
    );
    let index = api
        .call(
            "glyphs.index_of",
            json!({"doc": doc, "gid": 1, "filter": {}, "sort": "Status"}),
        )
        .expect("index");
    assert_eq!(index, json!(1), "locked `A` is second in badge order");
}

/// The custom-info badge follows the document's own overlay, not the bundled
/// data alone, so refreshing from an overlay does not relight it.
#[test]
fn custom_glyph_info_follows_the_document_overlay() {
    let mut font = base_font();
    font.info.user_data = json!({
        "com.fontcarver.glyphData": {
            "version": 1,
            "glyphs": {"A": {"production_name": "A.house", "script": "Grek"}}
        }
    });
    let (api, session, doc) = open(font);

    let before = list(
        &api,
        json!({"doc": doc, "filter": {}, "sort": "Order", "offset": 0, "limit": 10, "include_paths": false}),
    );
    let cell = |page: &GlyphListResult| {
        page.items
            .iter()
            .find(|cell| cell.name == "A")
            .expect("A")
            .clone()
    };
    assert!(
        !cell(&before).has_custom_glyph_info,
        "an untouched glyph is not custom just because the overlay mentions it"
    );

    let gid = session
        .with_doc(doc, |document| document.glyph_id("A").unwrap())
        .unwrap();
    let state = api
        .call("glyphdata.state", json!({"doc": doc, "gid": gid}))
        .expect("state");
    assert_eq!(state["info"]["production_name"], json!("A.house"));
    // The panel's flag answers a different question — "does the database carry
    // a project record for this glyph?" — so it is true here while the badge,
    // which asks whether the glyph departs from that database, is not.
    assert_eq!(state["custom"], json!(true));
    api.call(
        "glyphdata.update",
        json!({
            "doc": doc,
            "glyphs": [gid],
            "expected_generation": state["generation"],
            "label": "Update Glyph Info"
        }),
    )
    .expect("update");

    let after = list(
        &api,
        json!({"doc": doc, "filter": {}, "sort": "Order", "offset": 0, "limit": 10, "include_paths": false}),
    );
    assert_eq!(
        cell(&after).script,
        "Grek",
        "the refresh applied the overlay"
    );
    assert!(
        !cell(&after).has_custom_glyph_info,
        "a refresh from the document's own overlay is not custom glyph info"
    );
    let matched = list(
        &api,
        json!({
            "doc": doc,
            "filter": {"smart_expression": "has_custom_glyph_info"},
            "sort": "Order",
            "offset": 0,
            "limit": 10,
            "include_paths": false
        }),
    );
    assert_eq!(matched.total, 0, "the predicate agrees with the badge");
}

#[test]
fn a_cross_document_placeholder_is_badged_and_filterable() {
    let mut font = base_font();
    // Exactly what a cross-document transfer writes for a missing base:
    // an empty, non-exporting glyph carrying the documented tag.
    let mut stand_in = Glyph::new("ccaron");
    stand_in.unicodes = vec![0x010D];
    stand_in.export = false;
    stand_in.tags.insert("missing-component-base".into());
    stand_in
        .layers
        .push(Layer::new_master(font.default_master().id.clone()));
    font.insert_glyph(stand_in).expect("placeholder");
    let (api, _session, doc) = open(font);

    let cells = list(
        &api,
        json!({"doc": doc, "filter": {}, "sort": "Order", "offset": 0, "limit": 10, "include_paths": false}),
    );
    let placeholder = cells
        .items
        .iter()
        .find(|cell| cell.name == "ccaron")
        .expect("placeholder cell");
    assert!(placeholder.placeholder);
    assert!(!placeholder.export);
    assert!(placeholder.is_empty);
    assert!(
        cells.items.iter().filter(|cell| cell.placeholder).count() == 1,
        "an ordinary glyph is not a placeholder"
    );

    for expression in [
        "is_placeholder",
        "placeholder:\"true\"",
        "tags~\"missing-component-base\"",
    ] {
        let matched = list(
            &api,
            json!({
                "doc": doc,
                "filter": {"smart_expression": expression},
                "sort": "Order",
                "offset": 0,
                "limit": 10,
                "include_paths": false
            }),
        );
        assert_eq!(matched.total, 1, "{expression}");
        assert_eq!(matched.items[0].name, "ccaron", "{expression}");
    }

    // Two warning-weight badges put it above the other glyphs in badge order.
    let ordered = list(
        &api,
        json!({"doc": doc, "filter": {}, "sort": "Status", "offset": 0, "limit": 10, "include_paths": false}),
    );
    assert_eq!(ordered.items[0].name, "ccaron");
}

#[test]
fn cells_report_the_layer_colour_label_and_locked_is_filterable() {
    let mut font = base_font();
    let master = font.default_master().id.clone();
    font.glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .user_data = json!({"com.fontcarver.colorLabel": 7});
    font.glyph_mut("acutecomb")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .user_data = json!({"com.fontcarver.colorLabel": 99});
    font.glyph_mut("acutecomb").unwrap().locked = true;
    let (api, _session, doc) = open(font);

    let cells = list(
        &api,
        json!({"doc": doc, "filter": {}, "sort": "Order", "offset": 0, "limit": 10, "include_paths": false}),
    );
    assert_eq!(cells.items[0].layer_color_label, Some(7));
    assert_eq!(
        cells.items[1].layer_color_label, None,
        "an out-of-range stored label is ignored rather than shown"
    );

    let locked = list(
        &api,
        json!({
            "doc": doc,
            "filter": {"smart_expression": "locked"},
            "sort": "Order",
            "offset": 0,
            "limit": 10,
            "include_paths": false
        }),
    );
    assert_eq!(locked.total, 1);
    assert_eq!(locked.items[0].name, "acutecomb");
    assert!(locked.items[0].locked);

    let unlocked = list(
        &api,
        json!({
            "doc": doc,
            "filter": {"smart_expression": "not locked"},
            "sort": "Order",
            "offset": 0,
            "limit": 10,
            "include_paths": false
        }),
    );
    assert_eq!(unlocked.total, 1);
    assert_eq!(unlocked.items[0].name, "A");
}
