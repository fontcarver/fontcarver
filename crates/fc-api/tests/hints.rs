use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Contour, Font, Glyph};
use kurbo::{Rect, Shape};
use serde_json::json;

#[test]
fn analysis_is_read_only_and_hint_edits_are_atomic_revision_checked_and_undoable() {
    let mut font = Font::new_empty("Hints API", 1000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("I");
    glyph.unicodes.push(73);
    font.insert_glyph(glyph).unwrap();
    let layer = font
        .glyph_mut("I")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap();
    layer.width = 300.0;
    layer.contours = Contour::from_bezpath(&Rect::new(100.0, 0.0, 180.0, 700.0).to_path(0.1));
    let before = font.clone();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    let state = api
        .call(
            "hints.state",
            json!({"doc":doc,"master":master,"glyph":"I","layer":null}),
        )
        .unwrap();
    assert_eq!(state["can_autohint_ps"], true);
    let preview = api
        .call(
            "hints.analyze",
            json!({"doc":doc,"master":master,"glyphs":["I"]}),
        )
        .unwrap();
    assert_eq!(preview["generation"], state["generation"]);
    assert!(!preview["hints"][0][1]["v_stems"]
        .as_array()
        .unwrap()
        .is_empty());
    session
        .with_doc(doc, |doc| assert_eq!(doc.font, before))
        .unwrap();
    let command = json!({"Ps":{"glyph":"I","layer":state["layer"],"hints":preview["hints"][0][1]}});
    let mut invalid = preview["hints"][0][1].clone();
    invalid["v_stems"][0]["width"] = json!(-12);
    let failed = json!({"doc":doc,"expected_generation":state["generation"],"label":"Hints","commands":[command,{"Ps":{"glyph":"I","layer":state["layer"],"hints":invalid}}]});
    assert!(api.call("hints.apply", failed).is_err());
    session
        .with_doc(doc, |doc| assert_eq!(doc.font, before))
        .unwrap();
    let params = json!({"doc":doc,"expected_generation":state["generation"],"label":"Hints","commands":[command,{"Private":{"master":master,"private":preview["private"],"zones":[],"stems":[]}}]});
    api.call("hints.apply", params.clone()).unwrap();
    assert!(api.call("hints.apply", params).is_err());
    session
        .with_doc_mut(doc, |doc| {
            let after = doc.font.clone();
            assert_eq!(
                after.glyph("I").unwrap().layers[0].contours,
                before.glyph("I").unwrap().layers[0].contours
            );
            let bytes = serde_json::to_vec(&after).unwrap();
            assert_eq!(serde_json::from_slice::<Font>(&bytes).unwrap(), after);
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
        })
        .unwrap();
}

#[test]
fn background_state_cannot_analyze_the_unrelated_master_outline() {
    let mut font = Font::new_empty("Hint sources", 1000);
    let master = font.default_master().id.clone();
    font.insert_glyph(Glyph::new("I")).unwrap();
    let mut layer = fc_model::Layer::new_master(master.clone());
    layer.kind = fc_model::LayerKind::Background(master.clone());
    let layer_id = layer.id.clone();
    font.glyph_mut("I").unwrap().layers.push(layer);
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    let state = api
        .call(
            "hints.state",
            json!({"doc":doc,"master":master,"glyph":"I","layer":layer_id}),
        )
        .unwrap();
    assert_eq!(state["can_autohint_ps"], false);
    assert_eq!(state["layer"], json!(layer_id));
}
