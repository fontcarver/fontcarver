use std::sync::Arc;

use fw_api::{Api, ShapedRun};
use fw_doc::Session;
use fw_model::{Contour, FeatureBlock, Font, Glyph, Layer};
use kurbo::BezPath;
use serde_json::json;

fn ligature_font() -> Font {
    let mut font = Font::new_empty("Shaping Test", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicode, width) in [
        (".notdef", None, 600.0),
        ("f", Some(0x66), 300.0),
        ("i", Some(0x69), 280.0),
        ("f_i", None, 540.0),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        let mut path = BezPath::new();
        path.move_to((40.0, 0.0));
        path.line_to((40.0, 700.0));
        path.line_to((width - 40.0, 700.0));
        path.line_to((width - 40.0, 0.0));
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert shaping glyph");
    }
    font.features.features.push(FeatureBlock {
        tag: "liga".to_owned(),
        code: "sub f i by f_i;".to_owned(),
        ..FeatureBlock::default()
    });
    font
}

#[test]
fn shapes_a_ligature_through_dispatch() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(ligature_font(), None);
    let api = Api::new(session);

    let run: ShapedRun = serde_json::from_value(
        api.call("text.shape", json!({ "doc": doc, "text": "fi" }))
            .expect("shape text"),
    )
    .expect("decode shaped run");

    assert_eq!(run.names, vec![Some("f_i".to_owned())]);
    assert_eq!(run.glyphs.len(), 1);
    assert_eq!(run.glyphs[0].x_advance, 540.0);
    assert_eq!(run.glyphs[0].cluster, 0);
    assert!(!run.stale);
}

#[test]
fn feature_toggles_and_size_reach_the_shaper() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(ligature_font(), None);
    let api = Api::new(session);

    let run: ShapedRun = serde_json::from_value(
        api.call(
            "text.shape",
            json!({
                "doc": doc,
                "text": "fi",
                "features": [["liga", false]],
                "script": "Latn",
                "direction": "ltr",
                "size": 1000.0
            }),
        )
        .expect("shape text"),
    )
    .expect("decode shaped run");

    assert_eq!(
        run.names,
        vec![Some("f".to_owned()), Some("i".to_owned())],
        "{run:?}"
    );
    assert_eq!(run.glyphs[0].x_advance, 300.0);
    assert_eq!(run.size, Some(1000.0));
}

#[test]
fn shaping_an_unknown_master_is_a_not_found_error() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(ligature_font(), None);
    let api = Api::new(session);

    let error = api
        .call(
            "text.shape",
            json!({ "doc": doc, "text": "f", "master": "nope" }),
        )
        .expect_err("unknown master");

    assert_eq!(error.code, "not_found");
}

#[test]
fn width_edits_are_visible_before_the_shaping_font_is_rebuilt() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(ligature_font(), None);
    let api = Api::new(Arc::clone(&session));
    api.call("text.shape", json!({ "doc": doc, "text": "i" }))
        .expect("compile the first shaping font");
    let layer = session
        .with_doc(doc, |document| {
            let master = document.font.default_master().id.clone();
            document
                .font
                .glyph("i")
                .and_then(|glyph| glyph.master_layer(&master))
                .map(|layer| layer.id.to_string())
                .expect("master layer")
        })
        .expect("open document");

    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "SetWidth": { "glyph": "i", "layer": layer, "width": 500.0 } },
            "label": "Set width"
        }),
    )
    .expect("widen i");
    let run: ShapedRun = serde_json::from_value(
        api.call("text.shape", json!({ "doc": doc, "text": "i" }))
            .expect("shape after the edit"),
    )
    .expect("decode shaped run");

    assert!(run.stale, "{run:?}");
    assert_eq!(run.glyphs[0].x_advance, 500.0);
}
