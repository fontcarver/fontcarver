use std::sync::Arc;

use fw_api::{Api, GlyphCell, LayerOutline};
use fw_doc::Session;
use fw_model::{Component, Contour, Font, Glyph, Layer, MasterId};
use kurbo::{Affine, BezPath};
use serde_json::json;

fn dejavu() -> (Api, fw_doc::DocId) {
    let session = Arc::new(Session::new());
    let doc = session
        .open_path(&fw_fixtures::path("DejaVuSans.ttf"), None)
        .expect("open DejaVu Sans");
    (Api::new(session), doc)
}

fn gid_of(api: &Api, doc: fw_doc::DocId, name: &str) -> u32 {
    let cells: Vec<GlyphCell> = serde_json::from_value(
        api.call(
            "glyphs.search",
            json!({ "doc": doc, "query": name, "limit": 8 }),
        )
        .expect("search glyphs"),
    )
    .expect("decode search result");
    cells
        .into_iter()
        .find(|cell| cell.name == name)
        .unwrap_or_else(|| panic!("glyph `{name}` is missing"))
        .gid
        .0
}

fn composite_font() -> Font {
    let mut font = Font::new_empty("Editing Test", 1_000);
    let master = font.default_master().id.clone();
    let mut square = BezPath::new();
    square.move_to((50.0, 0.0));
    square.line_to((50.0, 500.0));
    square.line_to((450.0, 500.0));
    square.line_to((450.0, 0.0));
    square.close_path();

    let mut base = Glyph::new("a");
    base.unicodes.push(0x61);
    let mut layer = Layer::new_master(master.clone());
    layer.width = 500.0;
    layer.contours = Contour::from_bezpath(&square);
    base.layers.push(layer);
    font.insert_glyph(base).expect("insert base glyph");

    let mut composite = Glyph::new("aacute");
    composite.unicodes.push(0xE1);
    let mut layer = Layer::new_master(master);
    layer.width = 500.0;
    layer.components.push(Component {
        base: "a".into(),
        transform: Affine::translate((20.0, 10.0)),
        ..Component::default()
    });
    composite.layers.push(layer);
    font.insert_glyph(composite)
        .expect("insert composite glyph");
    font
}

#[test]
fn outline_of_a_reports_two_contours_and_resolved_metrics() {
    let (api, doc) = dejavu();
    let gid = gid_of(&api, doc, "A");

    let outline: LayerOutline = serde_json::from_value(
        api.call("glyphs.outline", json!({ "doc": doc, "gid": gid }))
            .expect("read outline"),
    )
    .expect("decode outline");

    assert_eq!(outline.contours.len(), 2);
    assert!(outline.components.is_empty());
    assert_eq!(outline.width, 1401.0);
    assert_eq!(outline.resolved_metrics.width, 1401.0);
    let bbox = outline.bbox.expect("A has bounds");
    assert!((outline.lsb - bbox[0]).abs() < 1e-9);
    assert!((outline.rsb - (outline.width - bbox[2])).abs() < 1e-9);
}

#[test]
fn outline_resolves_components_into_paths_and_bounds() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(composite_font(), None);
    let api = Api::new(session);
    let gid = gid_of(&api, doc, "aacute");

    let outline: LayerOutline = serde_json::from_value(
        api.call("glyphs.outline", json!({ "doc": doc, "gid": gid }))
            .expect("read outline"),
    )
    .expect("decode outline");

    assert_eq!(outline.components.len(), 1);
    let component = &outline.components[0];
    assert_eq!(component.component.base.as_ref(), "a");
    assert!(component.path.starts_with('M'), "{}", component.path);
    assert_eq!(component.bbox, Some([70.0, 10.0, 470.0, 510.0]));
    assert_eq!(outline.bbox, Some([70.0, 10.0, 470.0, 510.0]));

    let flattened = api
        .call(
            "glyphs.flattened_components",
            json!({ "doc": doc, "gid": gid }),
        )
        .expect("flatten components");
    assert_eq!(flattened["paths"].as_array().map(Vec::len), Some(1));
    assert_eq!(flattened["points"][0], json!([70.0, 10.0]));
}

#[test]
fn add_creates_three_encoded_glyphs_from_a_range() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(Font::new_empty("Add Test", 1_000), None);
    let api = Api::new(session);

    let result = api
        .call("glyphs.add", json!({ "doc": doc, "recipe": "A-C" }))
        .expect("add glyphs");

    assert_eq!(result["skipped"], json!([]));
    let added: Vec<GlyphCell> =
        serde_json::from_value(result["added"].clone()).expect("decode added cells");
    assert_eq!(
        added
            .iter()
            .map(|cell| (cell.name.clone(), cell.unicodes.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("A".to_owned(), vec![0x41]),
            ("B".to_owned(), vec![0x42]),
            ("C".to_owned(), vec![0x43]),
        ]
    );
    assert_eq!(
        api.call("doc.summary", json!({ "doc": doc }))
            .expect("summary")["glyph_count"],
        3
    );
    assert_eq!(
        api.call("doc.summary", json!({ "doc": doc }))
            .expect("summary")["undo_label"],
        "Add glyphs"
    );
}

#[test]
fn add_skips_existing_names_and_reports_unknown_items() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(composite_font(), None);
    let api = Api::new(session);

    let result = api
        .call(
            "glyphs.add",
            json!({ "doc": doc, "recipe": "a a.sc f_i 12three" }),
        )
        .expect("add glyphs");

    let added: Vec<GlyphCell> =
        serde_json::from_value(result["added"].clone()).expect("decode added cells");
    assert_eq!(
        added
            .iter()
            .map(|cell| (cell.name.clone(), cell.unicodes.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("a.sc".to_owned(), Vec::new()),
            ("f_i".to_owned(), Vec::new()),
        ]
    );
    assert_eq!(result["skipped"], json!(["a", "12three"]));
}

#[test]
fn ids_returns_every_match_in_the_requested_order() {
    let (api, doc) = dejavu();

    let ids = api
        .call(
            "glyphs.ids",
            json!({ "doc": doc, "filter": { "query": "A" }, "sort": "Name" }),
        )
        .expect("list glyph ids");

    let ids = ids.as_array().expect("array of ids");
    assert!(ids.len() > 10, "unexpected match count {}", ids.len());
    let all = api
        .call("glyphs.ids", json!({ "doc": doc }))
        .expect("select all");
    assert_eq!(all.as_array().map(Vec::len), Some(6_253));
}

#[test]
fn metrics_resolve_follows_a_metrics_key() {
    let mut font = composite_font();
    font.glyph_mut("aacute")
        .expect("composite glyph")
        .metrics_keys
        .width = Some("=a+40".to_owned());
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    let gid = gid_of(&api, doc, "aacute");
    let master: MasterId = serde_json::from_value(
        api.call("doc.summary", json!({ "doc": doc }))
            .expect("summary")["masters"][0]["id"]
            .clone(),
    )
    .expect("master id");

    let metrics = api
        .call(
            "doc.metrics_resolve",
            json!({ "doc": doc, "gid": gid, "master": master }),
        )
        .expect("resolve metrics");

    assert_eq!(metrics["width"], 540.0);
    assert_eq!(metrics["source"], "Key");
}

#[test]
fn apply_accepts_outline_commands_through_json_dispatch() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(composite_font(), None);
    let api = Api::new(Arc::clone(&session));
    let gid = gid_of(&api, doc, "a");
    let layer = serde_json::from_value::<LayerOutline>(
        api.call("glyphs.outline", json!({ "doc": doc, "gid": gid }))
            .expect("read outline"),
    )
    .expect("decode outline")
    .layer;

    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "MovePoints": {
                "glyph": "a",
                "layer": layer,
                "points": [{ "contour": 0, "point": 0 }],
                "delta": [10.0, 0.0],
                "handles": "WithHandles"
            }},
            "label": "Move nodes"
        }),
    )
    .expect("move points through raw JSON");
    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "PathOp": {
                "glyph": "a",
                "layer": layer,
                "op": "CorrectDirection"
            }},
            "label": "Correct direction"
        }),
    )
    .expect("run a path operation through raw JSON");

    let outline: LayerOutline = serde_json::from_value(
        api.call("glyphs.outline", json!({ "doc": doc, "gid": gid }))
            .expect("read outline"),
    )
    .expect("decode outline");
    assert_eq!(outline.contours.len(), 1);
    assert!(
        outline.contours[0]
            .points
            .iter()
            .any(|point| point.x == 60.0),
        "the moved point is missing: {:?}",
        outline.contours[0].points
    );
}
