use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{CidInfo, Font, Glyph, PsPrivate};
use serde_json::json;

#[test]
fn cid_dictionaries_and_layer_assignment_are_atomic_revision_checked_and_persistent() {
    let mut font = Font::new_empty("CID editor", 1000);
    font.cid = Some(CidInfo::default());
    let master = font.default_master().id.clone();
    font.masters[0].private_dict.std_hw = Some(40.0);
    font.insert_glyph(Glyph::new("I")).unwrap();
    let layer = font.glyph("I").unwrap().layers[0].id.clone();
    let before = font.clone();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    let request = json!({"doc":doc,"master":master,"glyph":"I","layer":layer});
    let state = api.call("hints.state", request.clone()).unwrap();
    assert_eq!(state["is_cid"], true);
    assert_eq!(state["cff_private_dict_index"], 0);
    let private = PsPrivate {
        std_hw: Some(90.0),
        ..Default::default()
    };
    let commands = json!([
        {"CffPrivate":{"master":master,"index":1,"private":private}},
        {"CffPrivateAssignment":{"master":master,"glyph":"I","layer":layer,"index":1}}
    ]);
    let params = json!({"doc":doc,"expected_generation":state["generation"],"label":"CID hints","commands":commands});
    let mut invalid = params.clone();
    invalid["commands"][1]["CffPrivateAssignment"]["index"] = json!(2);
    assert!(api.call("hints.apply", invalid).is_err());
    session
        .with_doc(doc, |doc| assert_eq!(doc.font, before))
        .unwrap();
    api.call("hints.apply", params.clone()).unwrap();
    assert!(api.call("hints.apply", params).is_err());
    let edited = api.call("hints.state", request).unwrap();
    assert_eq!(edited["master"]["private_dict"]["std_hw"], 40.0);
    assert_eq!(edited["master"]["cff_private_dicts"][0]["std_hw"], 90.0);
    assert_eq!(edited["cff_private_dict_index"], 1);
    // Automatic analysis covers every dictionary the selection uses, so a glyph
    // assigned to an additional CID dictionary is analysable too.
    assert_eq!(edited["can_autohint_ps"], true);
    let analysis = api
        .call(
            "hints.analyze",
            json!({"doc":doc,"master":master,"glyphs":["I"]}),
        )
        .unwrap();
    assert_eq!(analysis["cid_private"][0][0], 1);
    assert_eq!(analysis["cid_private"][0][1]["std_hw"], 90.0);
    assert_eq!(analysis["cid_private"][1], serde_json::Value::Null);
    session
        .with_doc_mut(doc, |doc| {
            let after = doc.font.clone();
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("cid.fcf");
            fc_doc::fcf::write(&path, &after).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), after);
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
        })
        .unwrap();
}

#[test]
fn assignment_rejects_foreign_master_and_nonexistent_dictionary_without_mutation() {
    let mut font = Font::new_empty("CID validation", 1000);
    font.cid = Some(CidInfo::default());
    font.masters[0].cff_private_dicts.push(PsPrivate::default());
    let master = font.default_master().id.clone();
    let mut other = font.default_master().clone();
    other.id = fc_model::MasterId::new();
    let other_id = other.id.clone();
    font.masters.push(other);
    font.insert_glyph(Glyph::new("I")).unwrap();
    let layer = font
        .glyph("I")
        .unwrap()
        .master_layer(&master)
        .unwrap()
        .id
        .clone();
    let before = font.clone();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    for command in [
        json!({"CffPrivateAssignment":{"master":other_id,"glyph":"I","layer":layer,"index":1}}),
        json!({"CffPrivateAssignment":{"master":master,"glyph":"I","layer":layer,"index":2}}),
        json!({"CffPrivate":{"master":master,"index":3,"private":PsPrivate::default()}}),
        json!({"CffPrivate":{"master":master,"index":1,"private":{"std_hw":-10.0}}}),
    ] {
        assert!(api
            .call(
                "hints.apply",
                json!({"doc":doc,"expected_generation":0,"label":"Invalid","commands":[command]})
            )
            .is_err());
        session
            .with_doc(doc, |doc| assert_eq!(doc.font, before))
            .unwrap();
    }
}
