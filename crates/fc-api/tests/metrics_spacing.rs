//! The Text tab's spacing matrix, beam and exception reporting through the API.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Contour, Font, Glyph, Layer, MasterId, Point, PointKind};
use serde_json::{json, Value};

/// A glyph holding one filled rectangle from `x0` to `x1` up to `top`.
fn box_glyph(master: &MasterId, name: &str, x0: f64, x1: f64, top: f64, width: f64) -> Glyph {
    let mut glyph = Glyph::new(name);
    let mut layer = Layer::new_master(master.clone());
    layer.width = width;
    layer.contours.push(Contour {
        closed: true,
        points: [(x0, 0.0), (x1, 0.0), (x1, top), (x0, top)]
            .into_iter()
            .map(|(x, y)| Point {
                x,
                y,
                kind: PointKind::Line,
                ..Point::default()
            })
            .collect(),
        ..Contour::default()
    });
    glyph.layers.push(layer);
    glyph
}

/// A glyph with an advance and no ink, like a space.
fn blank_glyph(master: &MasterId, name: &str, width: f64) -> Glyph {
    let mut glyph = Glyph::new(name);
    let mut layer = Layer::new_master(master.clone());
    layer.width = width;
    glyph.layers.push(layer);
    glyph
}

fn setup() -> (Api, DocId, String) {
    let mut font = Font::new_empty("Spacing", 1000);
    let master = font.default_master().id.clone();
    font.insert_glyph(box_glyph(&master, "H", 80.0, 520.0, 700.0, 600.0))
        .unwrap();
    font.insert_glyph(box_glyph(&master, "n", 60.0, 540.0, 500.0, 620.0))
        .unwrap();
    font.insert_glyph(blank_glyph(&master, "space", 250.0))
        .unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session), doc, master.to_string())
}

fn apply(api: &Api, doc: DocId, command: Value) {
    api.call(
        "doc.apply",
        json!({"doc": doc, "command": command, "label": "Spacing"}),
    )
    .unwrap();
}

/// The layer id of one glyph's default master layer.
fn layer_of(api: &Api, doc: DocId, name: &str) -> String {
    let cells = api
        .call(
            "glyphs.search",
            json!({"doc": doc, "query": name, "limit": 8}),
        )
        .unwrap();
    let gid = cells
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["name"] == name)
        .unwrap()["gid"]
        .clone();
    api.call("glyphs.outline", json!({"doc": doc, "gid": gid}))
        .unwrap()["layer"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn spacing(api: &Api, doc: DocId, glyphs: Value, beam: Value) -> Value {
    api.call(
        "metrics.spacing",
        json!({"doc": doc, "master": null, "glyphs": glyphs, "beam": beam}),
    )
    .unwrap()
}

#[test]
fn the_matrix_reports_stored_metrics_formulas_and_synchronisation() {
    let (api, doc, _) = setup();
    apply(
        &api,
        doc,
        json!({"SetMetricsKeys": {"glyph": "n", "keys": {"left": "=H"}}}),
    );
    let answer = spacing(&api, doc, json!(["H", "n", "space", "absent"]), json!(null));
    let rows = answer["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 4);

    assert_eq!(rows[0]["glyph"], "H");
    assert_eq!(rows[0]["width"], 600.0);
    assert_eq!(rows[0]["lsb"], 80.0);
    assert_eq!(rows[0]["rsb"], 80.0);
    assert_eq!(rows[0]["out_of_sync"], false);
    assert_eq!(rows[0]["glyph_keys"]["left"], Value::Null);
    assert_eq!(rows[0]["layer_keys"], Value::Null);

    // `n` copies H's left sidebearing, which its stored outline does not match.
    assert_eq!(rows[1]["glyph_keys"]["left"], "=H");
    assert_eq!(rows[1]["resolved"]["lsb"], 80.0);
    assert_eq!(rows[1]["lsb"], 60.0);
    assert_eq!(rows[1]["out_of_sync"], true);
    assert_eq!(rows[1]["key_error"], Value::Null);

    // An empty glyph has no sidebearings but still has an advance and a layer.
    assert_eq!(rows[2]["glyph"], "space");
    assert_eq!(rows[2]["lsb"], Value::Null);
    assert!(rows[2]["layer"].is_string());
    assert_eq!(rows[2]["missing"], false);

    // A name the document does not have is one bad row, not a failed request.
    assert_eq!(rows[3]["missing"], true);
    assert_eq!(rows[3]["layer"], Value::Null);

    // Synchronising clears the flag without touching the formula.
    apply(
        &api,
        doc,
        json!({"UpdateMetrics": {"glyph": "n", "master": null}}),
    );
    let answer = spacing(&api, doc, json!(["n"]), json!(null));
    assert_eq!(answer["rows"][0]["out_of_sync"], false);
    assert_eq!(answer["rows"][0]["lsb"], 80.0);
    assert_eq!(answer["rows"][0]["glyph_keys"]["left"], "=H");
}

#[test]
fn a_cyclic_formula_is_reported_in_its_own_row() {
    let (api, doc, _) = setup();
    apply(
        &api,
        doc,
        json!({"SetMetricsKeys": {"glyph": "n", "keys": {"left": "=H"}}}),
    );
    apply(
        &api,
        doc,
        json!({"SetMetricsKeys": {"glyph": "H", "keys": {"left": "=n"}}}),
    );
    let rows = spacing(&api, doc, json!(["H", "n"]), json!(null));
    for row in rows["rows"].as_array().unwrap() {
        assert!(row["key_error"].is_string(), "{row}");
        assert_eq!(row["out_of_sync"], true);
        assert_eq!(row["resolved"], Value::Null);
        // The stored numbers stay readable so the panel is not blank.
        assert!(row["width"].is_number());
    }
}

#[test]
fn a_layer_formula_is_reported_beside_the_glyph_formula() {
    let (api, doc, _) = setup();
    let layer = layer_of(&api, doc, "n");
    apply(
        &api,
        doc,
        json!({"SetMetricsKeys": {"glyph": "n", "keys": {"left": "=H"}}}),
    );
    apply(
        &api,
        doc,
        json!({"SetLayerMetricsKeys": {
            "glyph": "n",
            "layer": layer,
            "keys": {"left": "==120"},
        }}),
    );
    let rows = spacing(&api, doc, json!(["n"]), json!(null));
    assert_eq!(rows["rows"][0]["glyph_keys"]["left"], "=H");
    assert_eq!(rows["rows"][0]["layer_keys"]["left"], "==120");
    assert_eq!(rows["rows"][0]["resolved"]["lsb"], 120.0);
}

#[test]
fn the_beam_measures_margins_at_a_height_and_skips_what_it_misses() {
    let (api, doc, _) = setup();
    // 400 crosses both boxes; `n` is only 500 tall, so 600 misses it.
    let rows = spacing(&api, doc, json!(["H", "n", "space"]), json!(400.0));
    assert_eq!(rows["rows"][0]["beam_left"], 80.0);
    assert_eq!(rows["rows"][0]["beam_right"], 80.0);
    assert_eq!(rows["rows"][1]["beam_left"], 60.0);
    assert_eq!(rows["rows"][1]["beam_right"], 80.0);
    assert_eq!(rows["rows"][2]["beam_left"], Value::Null);

    let rows = spacing(&api, doc, json!(["H", "n"]), json!(600.0));
    assert_eq!(rows["rows"][0]["beam_left"], 80.0);
    assert_eq!(rows["rows"][1]["beam_left"], Value::Null);
    assert_eq!(rows["rows"][1]["beam_right"], Value::Null);
}

#[test]
fn the_request_is_bounded_and_rejects_an_unusable_beam() {
    let (api, doc, _) = setup();
    let many: Vec<String> = (0..257).map(|index| format!("g{index}")).collect();
    assert_eq!(
        api.call(
            "metrics.spacing",
            json!({"doc": doc, "glyphs": many, "beam": null}),
        )
        .unwrap_err()
        .code,
        "invalid_params"
    );
    // A beam height that is not a number is refused by request decoding; a
    // non-finite one cannot cross JSON at all, and the handler rejects it for
    // in-process callers.
    assert!(api
        .call(
            "metrics.spacing",
            json!({"doc": doc, "glyphs": ["H"], "beam": "x"}),
        )
        .is_err());
}

#[test]
fn stored_records_and_resolved_pairs_report_whether_they_are_exceptions() {
    let (api, doc, master) = setup();
    for (side, name, member) in [
        ("side1", "public.kern1.H", "H"),
        ("side2", "public.kern2.n", "n"),
    ] {
        apply(
            &api,
            doc,
            json!({"SetKerningGroup": {"side": side, "name": name, "members": [member]}}),
        );
    }
    apply(
        &api,
        doc,
        json!({"SetHorizontalKerning": {
            "master": master,
            "left": {"group": "public.kern1.H"},
            "right": {"group": "public.kern2.n"},
            "value": -40.0,
        }}),
    );
    let resolved = api
        .call(
            "kerning.resolve",
            json!({"doc": doc, "master": master, "left": "H", "right": "n"}),
        )
        .unwrap();
    assert_eq!(resolved["value"], -40.0);
    assert_eq!(resolved["exception"], false);

    // A glyph-side record where that glyph has a group is an exception.
    apply(
        &api,
        doc,
        json!({"SetHorizontalKerning": {
            "master": master,
            "left": {"glyph": "H"},
            "right": {"group": "public.kern2.n"},
            "value": -10.0,
        }}),
    );
    let resolved = api
        .call(
            "kerning.resolve",
            json!({"doc": doc, "master": master, "left": "H", "right": "n"}),
        )
        .unwrap();
    assert_eq!(resolved["value"], -10.0);
    assert_eq!(resolved["exception"], true);

    let pairs = api
        .call(
            "kerning.pairs",
            json!({"doc": doc, "master": master, "offset": 0, "limit": 50}),
        )
        .unwrap();
    let flags: Vec<_> = pairs["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| (record["left"].clone(), record["exception"].clone()))
        .collect();
    assert!(flags.contains(&(json!({"glyph": "H"}), json!(true))));
    assert!(flags.contains(&(json!({"group": "public.kern1.H"}), json!(false))));
}

#[test]
fn boundaries_report_every_exception_of_a_run_in_one_request() {
    let (api, doc, master) = setup();
    for (side, name, member) in [
        ("side1", "public.kern1.H", "H"),
        ("side2", "public.kern2.n", "n"),
    ] {
        apply(
            &api,
            doc,
            json!({"SetKerningGroup": {"side": side, "name": name, "members": [member]}}),
        );
    }
    for (left, right, value) in [
        (
            json!({"group": "public.kern1.H"}),
            json!({"group": "public.kern2.n"}),
            -40.0,
        ),
        (json!({"glyph": "H"}), json!({"glyph": "H"}), -5.0),
    ] {
        apply(
            &api,
            doc,
            json!({"SetHorizontalKerning": {
                "master": master, "left": left, "right": right, "value": value,
            }}),
        );
    }
    let answer = api
        .call(
            "kerning.boundaries",
            json!({
                "doc": doc,
                "master": master,
                "vertical": false,
                "pairs": [["H", "n"], ["H", "H"], ["n", "n"], ["H", "absent"]],
            }),
        )
        .unwrap();
    let items = answer["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    // The class rule is not an exception; the glyph pair on a grouped side is.
    assert_eq!(items[0]["value"], -40.0);
    assert_eq!(items[0]["exception"], false);
    assert_eq!(items[1]["value"], -5.0);
    assert_eq!(items[1]["exception"], true);
    // No rule at all is zero, and not an exception.
    assert_eq!(items[2]["value"], 0.0);
    assert_eq!(items[2]["exception"], false);
    // A boundary that cannot be read is reported, not fatal.
    assert_eq!(items[3]["value"], Value::Null);
    assert_eq!(items[3]["exception"], false);
    assert_eq!(answer["status"]["editable"], true);
}

#[test]
fn a_boundary_request_is_bounded_and_checks_its_master() {
    let (api, doc, master) = setup();
    let many: Vec<[String; 2]> = (0..257).map(|_| ["H".to_owned(), "n".to_owned()]).collect();
    assert_eq!(
        api.call(
            "kerning.boundaries",
            json!({"doc": doc, "master": master, "pairs": many}),
        )
        .unwrap_err()
        .code,
        "invalid_params"
    );
    assert!(api
        .call(
            "kerning.boundaries",
            json!({
                "doc": doc,
                "master": fc_model::MasterId::new().to_string(),
                "pairs": [["H", "n"]],
            }),
        )
        .is_err());
}
