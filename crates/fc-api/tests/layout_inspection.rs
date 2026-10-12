use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{FeatureBlock, FeaturesSource, Font, Glyph, Layer};
use serde_json::{json, Value};

fn setup() -> (Arc<Session>, Api, fc_doc::DocId) {
    let mut font = Font::new_empty("Layout inspection", 1000);
    let master = font.default_master().id.clone();
    for name in [".notdef", "A", "A.sc", "B", "B.sc"] {
        let mut glyph = Glyph::new(name);
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    font.features.prefixes.push(FeatureBlock {
        code: "lookup First { sub A by A.sc; } First; lookup Second { sub B by B.sc; } Second; feature smcp { lookup First; lookup Second; } smcp;".into(),
        ..Default::default()
    });
    let output =
        fc_compile::compile_static(&font, &master, &fc_compile::CompileOptions::default()).unwrap();
    let imported = fc_sfnt::read_bytes(&output, &fc_sfnt::ReadOptions::default())
        .unwrap()
        .fonts
        .remove(0);
    assert_eq!(
        imported.features.source_of_truth,
        FeaturesSource::ImportedTables
    );
    let session = Arc::new(Session::new());
    let doc = session.open_font(imported, None);
    let api = Api::new(Arc::clone(&session));
    (session, api, doc)
}

fn inspect(api: &Api, doc: fc_doc::DocId) -> Value {
    api.call(
        "layout.inspect",
        json!({"doc":doc,"generation":null,"offset":0,"limit":1}),
    )
    .unwrap()
}

#[test]
fn pages_are_bounded_read_only_and_return_stable_glyph_handles() {
    let (session, api, doc) = setup();
    let first = inspect(&api, doc);
    assert_eq!(first["items"].as_array().unwrap().len(), 1);
    assert!(first["total"].as_u64().unwrap() >= 2);
    assert_eq!(first["next_offset"], 1);
    assert!(!session.with_doc(doc, |document| document.dirty).unwrap());
    let lookup = &first["items"][0];
    let rules = api
        .call(
            "layout.rules",
            json!({"doc":doc,"table":lookup["table"],"lookup":lookup["index"],
        "generation":first["generation"],"offset":0,"limit":1}),
        )
        .unwrap();
    let row = &rules["items"][0];
    assert!(!row["text"].as_str().unwrap().is_empty());
    assert!(!row["glyphs"].as_array().unwrap().is_empty());
    assert!(row["glyphs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|glyph| !glyph["gid"].is_null()));
    let names = api
        .call(
            "layout.rule_glyphs",
            json!({"doc":doc,"table":lookup["table"],"lookup":lookup["index"],
        "rule":0,"generation":first["generation"],"offset":0,"limit":1}),
        )
        .unwrap();
    assert_eq!(names["items"].as_array().unwrap().len(), 1);
    assert_eq!(names["fingerprint"], first["fingerprint"]);
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        0
    );
}

#[test]
fn inspector_rejects_unbounded_and_stale_continuation_pages() {
    let (_session, api, doc) = setup();
    let first = inspect(&api, doc);
    for (offset, limit) in [(0, 0), (0, 201), (1, 1)] {
        assert!(api
            .call(
                "layout.inspect",
                json!({"doc":doc,"generation":null,"offset":offset,"limit":limit})
            )
            .is_err());
    }
    api.call("doc.apply",json!({"doc":doc,"label":"Change glyph metadata", "command":{"SetUnicodes":{"glyph":"A","unicodes":[65]}}})).unwrap();
    let error = api
        .call(
            "layout.inspect",
            json!({"doc":doc,"generation":first["generation"],"offset":1,"limit":1}),
        )
        .unwrap_err();
    assert_eq!(error.code, "stale_layout");
    let fresh = inspect(&api, doc);
    assert_eq!(fresh["fingerprint"], first["fingerprint"]);
    assert_ne!(fresh["compile_revision"], first["compile_revision"]);
}

#[test]
fn large_alternate_classes_page_all_references_beyond_preview_sample() {
    let mut font = Font::new_empty("Large layout class", 1000);
    let master = font.default_master().id.clone();
    let alternates = (0..70)
        .map(|index| format!("A.alt{index}"))
        .collect::<Vec<_>>();
    for name in [".notdef".to_owned(), "A".to_owned()]
        .into_iter()
        .chain(alternates.clone())
    {
        let mut glyph = Glyph::new(name);
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    font.features.features.push(FeatureBlock {
        tag: "salt".into(),
        code: format!("sub A from [{}];", alternates.join(" ")),
        ..Default::default()
    });
    let bytes =
        fc_compile::compile_static(&font, &master, &fc_compile::CompileOptions::default()).unwrap();
    let imported = fc_sfnt::read_bytes(&bytes, &fc_sfnt::ReadOptions::default())
        .unwrap()
        .fonts
        .remove(0);
    let session = Arc::new(Session::new());
    let doc = session.open_font(imported, None);
    let api = Api::new(session);
    let inspection = inspect(&api, doc);
    let lookup = &inspection["items"][0];
    let rules=api.call("layout.rules",json!({"doc":doc,"table":lookup["table"],"lookup":lookup["index"],"generation":inspection["generation"],"offset":0,"limit":1})).unwrap();
    assert_eq!(rules["items"][0]["glyphs"].as_array().unwrap().len(), 64);
    assert_eq!(rules["items"][0]["truncated"], true);
    let remaining=api.call("layout.rule_glyphs",json!({"doc":doc,"table":lookup["table"],"lookup":lookup["index"],"rule":0,"generation":inspection["generation"],"offset":64,"limit":7})).unwrap();
    assert_eq!(remaining["total"], 71);
    assert_eq!(remaining["items"].as_array().unwrap().len(), 7);
    assert_eq!(remaining["items"][6]["name"], "A.alt69");
    assert_eq!(remaining["next_offset"], Value::Null);
}
