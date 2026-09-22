use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Font, Glyph};
use serde_json::{json, Value};
use std::sync::Arc;

fn fixture() -> (Api, DocId) {
    let mut font = Font::new_empty("CJK filters", 1000);
    for (name, unicodes) in [
        ("water", vec![0x6C34]),
        ("one", vec![0x4E00]),
        ("two", vec![0x4E8C]),
        ("green", vec![0x7DA0]),
        ("multi", vec![0x4E00, 0x6C34]),
        ("unencoded", vec![]),
        ("extension", vec![0x30000]),
        ("latin", vec![0x41]),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = unicodes;
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session), doc)
}

fn names(api: &Api, doc: DocId, filter: Value, sort: &str) -> Vec<String> {
    let result = api.call("glyphs.list", json!({"doc":doc,"filter":filter,"sort":sort,"offset":0,"limit":100,"include_paths":false})).unwrap();
    result["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn filters_one_encoding_with_all_conditions_and_preserves_source_readings() {
    let (api, doc) = fixture();
    let water = api
        .call("glyphs.unihan", json!({"unicode":0x6C34}))
        .unwrap();
    assert_eq!(water["mandarin"], "shuǐ");
    assert_eq!(water["radical_strokes"], "85.0");
    assert_eq!(water["version"], "17.0.0");
    assert_eq!(
        api.call("glyphs.unihan", json!({"unicode":0x41})).unwrap(),
        Value::Null
    );
    assert!(api
        .call("glyphs.unihan", json!({"unicode":0xD800}))
        .is_err());
    for filter in [
        json!({"cjk":{"radical":85,"strokes":[4,4],"reading":"shui3","language":"mandarin"}}),
        json!({"query":"radical:85 strokes:4 pinyin:shui"}),
    ] {
        assert_eq!(names(&api, doc, filter, "Name"), ["multi", "water"]);
    }
    // One glyph has both 一 and 水. Conditions must not borrow a different
    // encoding for the radical and the stroke count.
    for filter in [
        json!({"cjk":{"radical":1,"strokes":[4,4]}}),
        json!({"query":"radical:1 strokes:4"}),
    ] {
        assert!(names(&api, doc, filter, "Name").is_empty());
    }
    assert_eq!(
        names(
            &api,
            doc,
            json!({"cjk":{"reading":"mizu","language":"japanese"}}),
            "Name"
        ),
        ["multi", "water"]
    );
    assert!(
        !names(&api, doc, json!({"cjk":{"charsets":["jisx0208"]}}), "Name")
            .contains(&"extension".into())
    );
    assert_eq!(
        names(
            &api,
            doc,
            json!({"cjk":{"radical":1,"strokes":[2,2]}}),
            "Name"
        ),
        ["extension"]
    );
    let sorted = names(&api, doc, json!({"scripts":["Hani"]}), "Reading");
    assert_eq!(
        sorted,
        ["two", "green", "water", "one", "multi", "extension"]
    );
}

#[test]
fn rejects_invalid_filters_and_limits() {
    let (api, doc) = fixture();
    for cjk in [
        json!({"radical":215}),
        json!({"strokes":[20,1]}),
        json!({"language":"invented"}),
        json!({"charsets":["unknown"]}),
        json!({"reading":"x".repeat(129)}),
    ] {
        assert!(api
            .call(
                "glyphs.list",
                json!({"doc":doc,"filter":{"cjk":cjk},"offset":0,"limit":10,"include_paths":false})
            )
            .is_err());
    }
}

#[test]
fn cid_authoring_orders_numerically_and_rejects_stale_writes() {
    let (api, doc) = fixture();
    let state = api.call("cid.state", json!({"doc":doc})).unwrap();
    let args = json!({"doc":doc,"expected_generation":state["generation"],"cid":{"registry":"Adobe","ordering":"Identity","supplement":0,"cid_map":{"12":"water","300":"one","20":"green"}},"reorder":true,"label":"CID"});
    api.call("cid.apply", args.clone()).unwrap();
    assert!(api.call("cid.apply", args).is_err());
    let ordered = names(&api, doc, json!({}), "Order");
    assert_eq!(&ordered[..3], ["water", "green", "one"]);
    assert_eq!(
        &names(&api, doc, json!({}), "Cid")[..3],
        ["water", "green", "one"]
    );
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(
        api.call("cid.state", json!({"doc":doc})).unwrap()["cid"],
        Value::Null
    );
}

#[test]
fn batch_spacing_previews_without_changes_then_applies_one_undo_step() {
    let mut font = Font::new_empty("CJK batch", 1000);
    let master = font.masters[0].id.clone();
    for cp in [0x4E00, 0x4E8C] {
        let name = fc_glyphdata::name_for_unicode(cp);
        let mut glyph = Glyph::new(name);
        glyph.unicodes = vec![cp];
        let mut layer = fc_model::Layer::new_master(master.clone());
        layer.width = 600.;
        let mut path = kurbo::BezPath::new();
        path.move_to((20., 0.));
        path.line_to((220., 0.));
        path.line_to((220., 500.));
        path.close_path();
        layer.contours = fc_model::Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    let glyphs = session
        .with_doc(doc, |doc| {
            doc.font
                .glyph_order()
                .map(|name| doc.glyph_id(name).unwrap())
                .collect::<Vec<_>>()
        })
        .unwrap();
    let before = session.with_doc(doc, |doc| doc.font.clone()).unwrap();
    let mut params = json!({"doc":doc,"glyphs":glyphs,"master":master,"width":1000.,"center":true,"vertical":true,"advance":1100.,"origin":880.,"dry_run":true,"expected_generation":null,"label":"Batch spacing"});
    let preview = api.call("cjk.batch", params.clone()).unwrap();
    assert_eq!(preview["glyph_count"], 2);
    assert_eq!(preview["layer_count"], 2);
    assert_eq!(
        session.with_doc(doc, |doc| doc.font.clone()).unwrap(),
        before
    );
    params["dry_run"] = json!(false);
    params["expected_generation"] = preview["generation"].clone();
    api.call("cjk.batch", params.clone()).unwrap();
    session
        .with_doc(doc, |doc| {
            use kurbo::Shape;
            for glyph in doc.font.glyphs.values() {
                let layer = &glyph.layers[0];
                assert_eq!(layer.width, 1000.);
                assert_eq!(layer.vertical_width, Some(1100.));
                assert_eq!(layer.vertical_origin, Some(880.));
                assert_eq!(layer.contours[0].to_bezpath().bounding_box().x0, 400.);
            }
            assert_eq!(doc.undo_len(), 1);
        })
        .unwrap();
    assert!(api.call("cjk.batch", params.clone()).is_err());
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(
        session.with_doc(doc, |doc| doc.font.clone()).unwrap(),
        before
    );
    params["dry_run"] = json!(true);
    params["origin"] = json!(50000.);
    assert!(api.call("cjk.batch", params).is_err());
    assert_eq!(
        session.with_doc(doc, |doc| doc.font.clone()).unwrap(),
        before
    );
}

#[test]
fn large_cjk_search_and_batch_preview_have_bounded_work() {
    let mut font = Font::new_empty("CJK performance", 1000);
    for cp in fc_glyphdata::unihan_codepoints().take(65_535) {
        let mut glyph = Glyph::new(fc_glyphdata::name_for_unicode(cp));
        glyph.unicodes = vec![cp];
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    let start = std::time::Instant::now();
    let found = names(
        &api,
        doc,
        json!({"query":"radical:85 strokes:4-20 pinyin:sh"}),
        "Radical",
    );
    let elapsed = start.elapsed();
    assert!(!found.is_empty());
    let budget = if cfg!(debug_assertions) {
        std::time::Duration::from_secs(2)
    } else {
        std::time::Duration::from_millis(150)
    };
    assert!(elapsed < budget, "65k CJK search took {elapsed:?}");
    let glyphs = session
        .with_doc(doc, |doc| {
            doc.font
                .glyph_order()
                .map(|name| doc.glyph_id(name).unwrap())
                .collect::<Vec<_>>()
        })
        .unwrap();
    let start = std::time::Instant::now();
    let result=api.call("cjk.batch",json!({"doc":doc,"glyphs":glyphs,"master":null,"width":1000.,"center":false,"vertical":true,"advance":1000.,"origin":880.,"dry_run":true,"expected_generation":null,"label":"Batch"})).unwrap();
    assert_eq!(result["layer_count"], 65_535);
    assert_eq!(result["examples"].as_array().unwrap().len(), 12);
    assert!(
        start.elapsed() < std::time::Duration::from_secs(2),
        "65k batch preview took {:?}",
        start.elapsed()
    );
    eprintln!(
        "65k CJK search {elapsed:?}; batch preview {:?}",
        start.elapsed()
    );
}

#[test]
fn centering_selected_composites_and_bases_is_independent_of_selection_order() {
    use fc_model::{Component, Contour, Layer};
    use kurbo::{Affine, BezPath, Shape};
    for reverse in [false, true] {
        let mut font = Font::new_empty("Component spacing", 1000);
        let master = font.default_master().id.clone();
        let mut base = Glyph::new("base");
        let mut layer = Layer::new_master(master.clone());
        layer.width = 600.;
        layer.contours = Contour::from_bezpath(&BezPath::from_vec(vec![
            kurbo::PathEl::MoveTo((0., 0.).into()),
            kurbo::PathEl::LineTo((100., 0.).into()),
            kurbo::PathEl::LineTo((100., 400.).into()),
            kurbo::PathEl::ClosePath,
        ]));
        base.layers.push(layer.clone());
        font.insert_glyph(base).unwrap();
        // Direct contours plus a translated component ensure moving the base
        // changes the composite's width, not just its outline's position.
        let mut composite = Glyph::new("composite");
        layer.id = fc_model::LayerId::new();
        layer.components.push(Component {
            base: "base".into(),
            transform: Affine::translate((200., 0.)),
            ..Default::default()
        });
        composite.layers.push(layer);
        font.insert_glyph(composite).unwrap();
        let original = font.clone();
        let session = Arc::new(Session::new());
        let doc = session.open_font(font, None);
        let api = Api::new(session.clone());
        let mut glyphs = session
            .with_doc(doc, |d| {
                vec![
                    d.glyph_id("base").unwrap(),
                    d.glyph_id("composite").unwrap(),
                ]
            })
            .unwrap();
        if reverse {
            glyphs.reverse();
        }
        let mut args = json!({"doc":doc,"glyphs":glyphs,"master":master,"width":1000.,"center":true,"vertical":false,"dry_run":true,"label":"Center"});
        let preview = api.call("cjk.batch", args.clone()).unwrap();
        args["dry_run"] = json!(false);
        args["expected_generation"] = preview["generation"].clone();
        api.call("cjk.batch", args).unwrap();
        session
            .with_doc(doc, |d| {
                for name in ["base", "composite"] {
                    let bounds = fc_model::flatten_layer(&d.font, name, &master)
                        .unwrap()
                        .bounding_box();
                    assert!((bounds.x0 + bounds.x1 - 1000.).abs() < 1e-8);
                    assert_eq!(d.font.glyph(name).unwrap().layers[0].width, 1000.);
                }
                assert_eq!(d.undo_len(), 1);
            })
            .unwrap();
        api.call("doc.undo", json!({"doc":doc})).unwrap();
        assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), original);
    }
}
