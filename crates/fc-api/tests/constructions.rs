use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Contour, Font, Glyph, Layer};
use kurbo::{BezPath, Rect, Shape};
use serde_json::{json, Value};
use std::sync::Arc;

fn fixture() -> (Api, Arc<Session>, DocId, Value) {
    let mut font = Font::new_empty("Reusable caps", 1000);
    let master = font.default_master().id.clone();
    let mut source = Glyph::new("cap");
    let mut layer = Layer::new_master(master.clone());
    layer.contours = Contour::from_bezpath(&Rect::new(-20., -10., 20., 10.).to_path(0.01));
    source.layers.push(layer);
    font.insert_glyph(source).unwrap();
    let mut carrier = Glyph::new("A");
    let mut layer = Layer::new_master(master);
    layer.contours = Contour::from_bezpath(&BezPath::from_svg("M0 0L100 0").unwrap());
    let id = layer.id.clone();
    carrier.layers.push(layer);
    font.insert_glyph(carrier).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let params = json!({
        "doc": doc, "glyph": "A", "layer": id, "constructions": [],
        "add": { "source": "cap", "kind": {
            "kind": "cap", "at": { "contour": 0, "point": 1 }, "scale": 1
        }, "keep_carrier": true },
        "dry_run": true, "bake": false, "expected_generation": null
    });
    (Api::new(session.clone()), session, doc, params)
}

fn font(session: &Session, doc: DocId) -> Font {
    session.with_doc(doc, |doc| doc.font.clone()).unwrap()
}

fn commit(api: &Api, params: &Value) -> Value {
    let preview = api.call("constructions.apply", params.clone()).unwrap();
    let mut params = params.clone();
    params["dry_run"] = json!(false);
    params["expected_generation"] = preview["generation"].clone();
    api.call("constructions.apply", params).unwrap()
}

#[test]
fn attachment_preview_commit_bake_and_undo_are_atomic() {
    let (api, session, doc, params) = fixture();
    let original = font(&session, doc);
    let preview = api.call("constructions.apply", params.clone()).unwrap();
    assert_ne!(preview["example"]["before"], preview["example"]["after"]);
    assert_eq!(font(&session, doc), original);
    assert_eq!(commit(&api, &params)["example"], preview["example"]);
    let attached = font(&session, doc);
    let layer = &attached.glyph("A").unwrap().layers[0];
    assert_eq!(layer.constructions.len(), 1);
    assert!(layer.contours[0].points[1].id.is_some());
    for (before, after) in original.glyph("A").unwrap().layers[0].contours[0]
        .points
        .iter()
        .zip(&layer.contours[0].points)
    {
        assert_eq!(
            (before.x, before.y, before.kind),
            (after.x, after.y, after.kind)
        );
    }
    session
        .with_doc_mut(doc, |doc| doc.undo())
        .unwrap()
        .unwrap();
    assert_eq!(font(&session, doc), original);
    session
        .with_doc_mut(doc, |doc| doc.redo())
        .unwrap()
        .unwrap();
    assert_eq!(font(&session, doc), attached);

    let mut bake = params.clone();
    bake["add"] = Value::Null;
    bake["constructions"] = serde_json::to_value(&layer.constructions).unwrap();
    bake["bake"] = json!(true);
    let evaluated = fc_geom::evaluate_layer(&attached, "A", &layer.id)
        .unwrap()
        .path;
    commit(&api, &bake);
    let baked = font(&session, doc);
    let baked_layer = &baked.glyph("A").unwrap().layers[0];
    assert!(baked_layer.constructions.is_empty());
    assert!(baked_layer.effects.is_empty());
    assert!(baked_layer.node_links.is_empty());
    assert!(baked_layer.linked_guides.is_empty());
    assert_eq!(
        fc_model::flatten_layer_by_id(&baked, "A", &layer.id).unwrap(),
        evaluated
    );
    session
        .with_doc_mut(doc, |doc| doc.undo())
        .unwrap()
        .unwrap();
    assert_eq!(font(&session, doc), attached);
}

#[test]
fn stale_locked_and_invalid_requests_leave_document_unchanged() {
    let (api, session, doc, params) = fixture();
    let original = font(&session, doc);
    for (key, value) in [("source", json!("missing")), ("source", json!("A"))] {
        let mut invalid = params.clone();
        invalid["add"][key] = value;
        assert!(api.call("constructions.apply", invalid).is_err());
        assert_eq!(font(&session, doc), original);
    }
    let mut invalid = params.clone();
    invalid["add"]["kind"]["at"]["point"] = json!(200);
    assert!(api.call("constructions.apply", invalid).is_err());
    assert_eq!(font(&session, doc), original);

    let preview = api.call("constructions.apply", params.clone()).unwrap();
    commit(&api, &params);
    let attached = font(&session, doc);
    let mut stale = params.clone();
    stale["dry_run"] = json!(false);
    stale["expected_generation"] = preview["generation"].clone();
    assert!(api
        .call("constructions.apply", stale)
        .unwrap_err()
        .message
        .contains("stale"));
    assert_eq!(font(&session, doc), attached);
    session
        .with_doc_mut(doc, |doc| doc.font.glyph_mut("A").unwrap().locked = true)
        .unwrap();
    let locked = font(&session, doc);
    assert!(api
        .call("constructions.apply", params)
        .unwrap_err()
        .message
        .contains("locked"));
    assert_eq!(font(&session, doc), locked);
}

#[test]
fn stack_edits_and_removal_preserve_point_identities() {
    let (api, session, doc, params) = fixture();
    commit(&api, &params);
    let attached = font(&session, doc);
    let layer = &attached.glyph("A").unwrap().layers[0];
    let mut edit = params;
    edit["add"] = Value::Null;
    let mut constructions = layer.constructions.clone();
    let fc_model::ConstructionKind::Cap { scale, .. } = &mut constructions[0].kind else {
        panic!("fixture is a cap");
    };
    *scale = 2.;
    edit["constructions"] = serde_json::to_value(&constructions).unwrap();
    commit(&api, &edit);
    let edited = font(&session, doc);
    assert_eq!(
        edited.glyph("A").unwrap().layers[0].contours,
        layer.contours
    );
    assert_eq!(
        edited.glyph("A").unwrap().layers[0].constructions,
        constructions
    );
    edit["constructions"] = json!([]);
    commit(&api, &edit);
    let removed = font(&session, doc);
    assert_eq!(
        removed.glyph("A").unwrap().layers[0].contours,
        layer.contours
    );
    assert!(removed.glyph("A").unwrap().layers[0]
        .constructions
        .is_empty());
    session
        .with_doc_mut(doc, |doc| doc.undo())
        .unwrap()
        .unwrap();
    assert_eq!(font(&session, doc), edited);
}

#[test]
fn effects_bake_clears_constructions_without_double_evaluation() {
    let (api, session, doc, params) = fixture();
    commit(&api, &params);
    let attached = font(&session, doc);
    let layer = &attached.glyph("A").unwrap().layers[0];
    let effects = json!({
        "doc": doc, "glyph": "A", "layer": layer.id, "effects": [],
        "dry_run": true, "bake": true, "expected_generation": null
    });
    let preview = api.call("effects.apply", effects.clone()).unwrap();
    let mut effects = effects;
    effects["dry_run"] = json!(false);
    effects["expected_generation"] = preview["generation"].clone();
    api.call("effects.apply", effects).unwrap();
    let baked = font(&session, doc);
    assert!(baked.glyph("A").unwrap().layers[0].constructions.is_empty());
    assert_eq!(
        fc_geom::evaluate_layer(&baked, "A", &layer.id)
            .unwrap()
            .path
            .to_svg(),
        preview["example"]["after"].as_str().unwrap()
    );
    session
        .with_doc_mut(doc, |doc| doc.undo())
        .unwrap()
        .unwrap();
    assert_eq!(font(&session, doc), attached);
}
