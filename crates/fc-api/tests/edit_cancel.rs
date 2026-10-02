use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Font, Glyph, Layer};
use serde_json::json;

#[test]
fn cancellation_is_token_checked_and_restores_the_live_entry() {
    let mut font = Font::new_empty("Cancellation", 1000);
    let mut glyph = Glyph::new("A");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 500.0;
    let layer_id = layer.id.clone();
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    let begin = |label| {
        api.call("doc.edit_begin", json!({"doc":doc,"label":label}))
            .unwrap()
    };
    let apply = |token, width| {
        api.call("doc.edit_apply", json!({"doc":doc,"token":token,"command":{"SetWidth":{"glyph":"A","layer":layer_id,"width":width}}})).unwrap()
    };
    let old = begin("Old");
    apply(old.clone(), 510.0);
    let new = begin("New");
    apply(new.clone(), 520.0);
    assert!(api
        .call("doc.edit_cancel", json!({"doc":doc,"token":old}))
        .is_err());
    let restored = api
        .call("doc.edit_cancel", json!({"doc":doc,"token":new}))
        .unwrap();
    assert!(restored["metrics"].as_bool().unwrap());
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.glyph("A").unwrap().layers[0].width, 510.0);
            assert_eq!(document.undo_label(), Some("Old"));
            assert_eq!(document.redo_label(), None);
        })
        .unwrap();
}
