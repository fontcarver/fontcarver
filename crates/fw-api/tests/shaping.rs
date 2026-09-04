use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

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

fn layer_of(session: &Session, doc: fw_doc::DocId, glyph: &str) -> String {
    session
        .with_doc(doc, |document| {
            let master = document.font.default_master().id.clone();
            document
                .font
                .glyph(glyph)
                .and_then(|glyph| glyph.master_layer(&master))
                .map(|layer| layer.id.to_string())
                .expect("master layer")
        })
        .expect("open document")
}

#[test]
fn a_shaping_compile_does_not_block_document_edits() {
    let font = fw_sfnt::read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &fw_sfnt::ReadOptions::default(),
    )
    .expect("read DejaVu Sans")
    .fonts
    .remove(0);
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Arc::new(Api::new(Arc::clone(&session)));
    let layer = layer_of(&session, doc, "A");

    // The shaping thread reports when its compile starts and when it ends; the
    // assertion is structural — the edit has to complete inside that window —
    // so it does not depend on how fast this machine is.
    let (started, compile_started) = std::sync::mpsc::channel();
    let shaping = {
        let api = Arc::clone(&api);
        thread::spawn(move || {
            let began = Instant::now();
            started.send(began).expect("report the compile start");
            api.call("text.shape", json!({ "doc": doc, "text": "AVWA" }))
                .expect("shape the imported font");
            (began, Instant::now())
        })
    };
    let compile_began = compile_started.recv().expect("the compile start instant");
    // Give the synchronous first compile time to get under way.
    thread::sleep(Duration::from_millis(250));

    let edit_began = Instant::now();
    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "SetWidth": { "glyph": "A", "layer": layer, "width": 1400.0 } },
            "label": "Set width"
        }),
    )
    .expect("apply an edit while the shaping font compiles");
    let edit_ended = Instant::now();
    let (_, compile_ended) = shaping.join().expect("join the shaping thread");

    let compile = compile_ended.duration_since(compile_began);
    assert!(
        compile > Duration::from_millis(500),
        "the compile finished too quickly to prove anything: {compile:?}"
    );
    assert!(
        edit_began > compile_began,
        "the edit started before the compile did"
    );
    assert!(
        edit_ended < compile_ended,
        "the edit did not complete while the compile was still running: \
         edit ended {:?} after the compile started, compile took {compile:?}",
        edit_ended.duration_since(compile_began)
    );
}

#[test]
fn a_failed_background_rebuild_is_reported_and_retried() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(ligature_font(), None);
    let api = Api::new(Arc::clone(&session));
    let layer = layer_of(&session, doc, "i");
    api.call("text.shape", json!({ "doc": doc, "text": "fi" }))
        .expect("compile the first shaping font");

    // Break the feature source, then edit so the rebuild picks the break up.
    session
        .with_doc_mut(doc, |document| {
            document.font.features.features[0].code = "this is not valid feature code".to_owned();
        })
        .expect("break the feature source");
    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "SetWidth": { "glyph": "i", "layer": layer, "width": 500.0 } },
            "label": "Set width"
        }),
    )
    .expect("widen i");

    let failed = wait_for_run(&api, doc, |run| {
        run.warnings
            .iter()
            .any(|warning| warning.starts_with("compile-failed"))
    });
    assert!(failed.stale, "{failed:?}");

    // Repairing the source and editing again retries the rebuild.
    session
        .with_doc_mut(doc, |document| {
            document.font.features.features[0].code = "sub f i by f_i;".to_owned();
        })
        .expect("repair the feature source");
    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "SetWidth": { "glyph": "i", "layer": layer, "width": 520.0 } },
            "label": "Set width"
        }),
    )
    .expect("widen i again");

    let recovered = wait_for_run(&api, doc, |run| !run.stale);
    assert!(recovered.warnings.is_empty(), "{:?}", recovered.warnings);
    assert_eq!(recovered.names, vec![Some("f_i".to_owned())]);
}

/// Shapes `fi` until `accept` is happy with the run or the deadline passes.
fn wait_for_run(api: &Api, doc: fw_doc::DocId, accept: impl Fn(&ShapedRun) -> bool) -> ShapedRun {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let run: ShapedRun = serde_json::from_value(
            api.call("text.shape", json!({ "doc": doc, "text": "fi" }))
                .expect("shape text"),
        )
        .expect("decode shaped run");
        if accept(&run) {
            return run;
        }
        assert!(
            Instant::now() < deadline,
            "the shaper never reached the expected state: {run:?}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
