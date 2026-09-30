//! Database overlays and inferred metadata form reversible document edits.
use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Anchor, Font, Glyph, Layer};
use serde_json::{json, Value};
use std::sync::Arc;

fn fixture() -> (Api, Arc<Session>, DocId, Value) {
    let mut font = Font::new_empty("Glyph data", 1000);
    let mut glyph = Glyph::new("Aacute");
    glyph.unicodes = vec![65];
    glyph.category = Some("Symbol".into());
    glyph.note = "Preserve this custom note".into();
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 620.;
    layer.anchors.push(Anchor {
        name: "custom".into(),
        x: 250.,
        y: 720.,
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font.insert_glyph(Glyph::new("Aacute.alt")).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let gids = session
        .with_doc(doc, |doc| {
            json!([
                doc.glyph_id("Aacute").unwrap(),
                doc.glyph_id("Aacute.alt").unwrap()
            ])
        })
        .unwrap();
    (Api::new(session.clone()), session, doc, gids)
}

fn state(api: &Api, doc: DocId, gid: &Value) -> Value {
    api.call("glyphdata.state", json!({"doc":doc,"gid":gid}))
        .unwrap()
}

#[test]
fn overlay_update_undo_redo_and_native_roundtrip_preserve_semantics() {
    let (api, session, doc, gids) = fixture();
    let before = session.with_doc(doc, |doc| doc.font.clone()).unwrap();
    let initial = state(&api, doc, &gids[0]);
    assert_eq!(initial["info"]["unicodes"], json!([193]));
    assert_eq!(initial["info"]["composition"], json!(["A", "acutecomb"]));
    let overlay = json!({"version":1,"glyphs":{"Aacute":{"production_name":"Aacute.custom","anchors":["top","bottom"]}}});
    api.call("glyphdata.set_overlay", json!({"doc":doc,"expected_generation":initial["generation"],"overlay_json":overlay.to_string(),"label":"Set glyph data overlay"})).unwrap();
    let custom = state(&api, doc, &gids[0]);
    assert_eq!(custom["custom"], true);
    assert_eq!(custom["info"]["production_name"], "Aacute.custom");
    api.call("glyphdata.update", json!({"doc":doc,"glyphs":gids,"expected_generation":custom["generation"],"label":"Update Glyph Info"})).unwrap();
    let updated = session.with_doc(doc, |doc| doc.font.clone()).unwrap();
    let a = updated.glyph("Aacute").unwrap();
    assert_eq!(a.unicodes, [193]);
    assert_eq!(a.layers, before.glyph("Aacute").unwrap().layers);
    assert_eq!(a.note, before.glyph("Aacute").unwrap().note);
    assert_eq!(a.category.as_deref(), Some("Letter"));
    assert_eq!(a.production_name.as_deref(), Some("Aacute.custom"));
    assert!(updated.glyph("Aacute.alt").unwrap().unicodes.is_empty());
    assert_eq!(
        updated.glyph("Aacute.alt").unwrap().category.as_deref(),
        Some("Letter")
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("glyph-data.fcf");
    fc_doc::fcf::write(&path, &updated).unwrap();
    assert_eq!(fc_doc::fcf::read(&path).unwrap(), updated);
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(
        session
            .with_doc(doc, |doc| doc
                .font
                .glyph("Aacute")
                .unwrap()
                .unicodes
                .clone())
            .unwrap(),
        [65]
    );
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(
        session.with_doc(doc, |doc| doc.font.clone()).unwrap(),
        before
    );
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    assert_eq!(
        session.with_doc(doc, |doc| doc.font.clone()).unwrap(),
        updated
    );
}

#[test]
fn stale_malformed_and_locked_calls_are_atomic() {
    let (api, session, doc, gids) = fixture();
    let current = state(&api, doc, &gids[0]);
    let before = session.with_doc(doc, |doc| doc.font.clone()).unwrap();
    for overlay in [
        "not JSON",
        r#"{"version":900,"glyphs":{}}"#,
        r#"{"version":1,"glyphs":{"A":{"unicodes":[55296]}}}"#,
    ] {
        assert!(api.call("glyphdata.set_overlay", json!({"doc":doc,"expected_generation":current["generation"],"overlay_json":overlay,"label":"Invalid"})).is_err());
    }
    assert!(api
        .call(
            "glyphdata.update",
            json!({"doc":doc,"glyphs":gids,"expected_generation":999,"label":"Stale"})
        )
        .is_err());
    assert_eq!(
        session.with_doc(doc, |doc| doc.font.clone()).unwrap(),
        before
    );
    session
        .with_doc_mut(doc, |doc| {
            doc.font.glyph_mut("Aacute.alt").unwrap().locked = true
        })
        .unwrap();
    assert!(api.call("glyphdata.update", json!({"doc":doc,"glyphs":gids,"expected_generation":current["generation"],"label":"Locked"})).is_err());
    assert_eq!(
        session
            .with_doc(doc, |doc| doc
                .font
                .glyph("Aacute")
                .unwrap()
                .unicodes
                .clone())
            .unwrap(),
        [65]
    );
}
