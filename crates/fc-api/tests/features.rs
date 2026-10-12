use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{FeaturesSource, Font, Glyph, Layer};
use serde_json::{json, Value};

fn setup(imported: bool) -> (Arc<Session>, Api, fc_doc::DocId) {
    let mut font = Font::new_empty("Feature API", 1000);
    let master = font.default_master().id.clone();
    for name in [".notdef", "A", "A.sc"] {
        let mut glyph = Glyph::new(name);
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    if imported {
        font.features.source_of_truth = FeaturesSource::ImportedTables;
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(Arc::clone(&session));
    (session, api, doc)
}

fn state(api: &Api, doc: fc_doc::DocId) -> Value {
    api.call("features.get", json!({"doc":doc})).unwrap()
}

fn candidate(api: &Api, doc: fc_doc::DocId, code: &str) -> Value {
    let before = state(api, doc);
    api.call(
        "features.set_draft",
        json!({
            "doc":doc,
            "expected_generation":before["generation"],
            "expected_draft_revision":before["draft_revision"],
            "source":{"features":[{"id":"test-feature","tag":"smcp","code":code}]},
            "settings":before["active_settings"]
        }),
    )
    .unwrap()
}

fn revisions(doc: fc_doc::DocId, state: &Value) -> Value {
    json!({"doc":doc,"expected_generation":state["generation"],
        "expected_compile_revision":state["compile_revision"],
        "expected_draft_revision":state["draft_revision"]})
}

#[test]
fn viewing_and_drafting_never_activate_source() {
    let (session, api, doc) = setup(false);
    let before = state(&api, doc);
    assert_eq!(before["draft"], Value::Null);
    assert_eq!(before["has_unapplied_changes"], false);
    assert!(!session.with_doc(doc, |document| document.dirty).unwrap());
    let draft = candidate(&api, doc, "this is invalid;");
    assert_eq!(draft["compile_revision"], before["compile_revision"]);
    assert_eq!(draft["active_source"], before["active_source"]);
    assert_eq!(draft["has_unapplied_changes"], true);
    assert_eq!(
        draft["draft"]["source"]["features"][0]["code"],
        "this is invalid;"
    );
    assert!(session.with_doc(doc, |document| document.dirty).unwrap());
}

#[test]
fn stale_draft_mutations_leave_the_newer_text_intact() {
    let (session, api, doc) = setup(false);
    let before = state(&api, doc);
    let after = candidate(&api, doc, "sub A by A.sc;");
    let undo_len = session
        .with_doc(doc, |document| document.undo_len())
        .unwrap();
    let error = api
        .call(
            "features.discard_draft",
            json!({"doc":doc,
        "expected_generation":before["generation"],
        "expected_draft_revision":before["draft_revision"]}),
        )
        .unwrap_err();
    assert_eq!(error.code, "stale_features");
    assert_eq!(state(&api, doc), after);
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        undo_len
    );
}

#[test]
fn invalid_apply_retains_draft_and_valid_apply_is_undoable() {
    let (session, api, doc) = setup(false);
    let invalid = candidate(&api, doc, "sub Missing by A.sc;");
    let undo_len = session
        .with_doc(doc, |document| document.undo_len())
        .unwrap();
    let rejected = api
        .call("features.apply", revisions(doc, &invalid))
        .unwrap();
    assert_eq!(rejected["applied"], false);
    assert_eq!(rejected["validation"]["valid"], false);
    assert!(!rejected["validation"]["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(state(&api, doc), invalid);
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        undo_len
    );

    let valid = candidate(&api, doc, "sub A by A.sc;");
    let validation = api
        .call("features.validate", revisions(doc, &valid))
        .unwrap();
    assert_eq!(validation["valid"], true, "{validation}");
    assert_eq!(state(&api, doc), valid);
    let accepted = api.call("features.apply", revisions(doc, &valid)).unwrap();
    assert_eq!(accepted["applied"], true, "{accepted}");
    assert_eq!(accepted["state"]["draft"], Value::Null);
    assert_eq!(
        accepted["state"]["active_source"]["features"][0]["code"],
        "sub A by A.sc;"
    );
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    let undone = state(&api, doc);
    assert_eq!(undone["active_source"], valid["active_source"]);
    assert_eq!(undone["draft"]["source"], valid["draft"]["source"]);
}

#[test]
fn imported_layout_and_forged_commands_cannot_bypass_conversion() {
    let (session, api, doc) = setup(true);
    let before = state(&api, doc);
    let error = api
        .call("features.apply", revisions(doc, &before))
        .unwrap_err();
    assert_eq!(error.code, "layout_conversion_required");
    for command in [
        json!({"SetFeatures":{"source":{"source_of_truth":"Fea"}}}),
        json!({"CommitPreparedFeatures":{"source":{"source_of_truth":"Fea"}}}),
    ] {
        assert!(api
            .call(
                "doc.apply",
                json!({"doc":doc,"label":"forged","command":command})
            )
            .is_err());
    }
    assert_eq!(state(&api, doc), before);
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        0
    );
}

#[test]
fn callers_cannot_forge_generated_source_provenance() {
    let (_session, api, doc) = setup(false);
    let before = state(&api, doc);
    let mut settings = before["active_settings"].clone();
    settings["generated"] = json!([{
        "generator":"SmallCaps", "source_id":"user-feature", "glyphs":["A"]
    }]);
    settings["glyph_fingerprint"] = json!("forged");
    let draft = api
        .call(
            "features.set_draft",
            json!({
                "doc":doc, "expected_generation":before["generation"],
                "expected_draft_revision":before["draft_revision"],
                "source":{"features":[{"id":"user-feature","tag":"smcp","code":"sub A by A.sc;"}]},
                "settings":settings
            }),
        )
        .unwrap();
    assert_eq!(draft["draft"]["settings"]["generated"], json!([]));
    assert_eq!(draft["draft"]["settings"]["glyph_fingerprint"], Value::Null);
    let result = api.call("features.apply", revisions(doc, &draft)).unwrap();
    assert_eq!(result["applied"], true, "{result}");
    assert_eq!(
        result["state"]["active_source"]["features"][0]["id"],
        "user-feature"
    );
}

#[test]
fn draft_edit_token_coalesces_and_discard_is_undoable() {
    let (session, api, doc) = setup(false);
    let token = api
        .call(
            "doc.edit_begin",
            json!({"doc":doc,"label":"Type feature source"}),
        )
        .unwrap();
    for code in ["s", "sub", "sub A by A.sc;"] {
        let before = state(&api, doc);
        api.call(
            "features.set_draft",
            json!({
                "doc":doc,"expected_generation":before["generation"],
                "expected_draft_revision":before["draft_revision"],"token":token,
                "source":{"features":[{"id":"typed","tag":"smcp","code":code}]},
                "settings":before["active_settings"]
            }),
        )
        .unwrap();
    }
    api.call("doc.edit_end", json!({"doc":doc,"token":token}))
        .unwrap();
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        1
    );
    let before = state(&api, doc);
    let discarded = api
        .call(
            "features.discard_draft",
            json!({
                "doc":doc,"expected_generation":before["generation"],
                "expected_draft_revision":before["draft_revision"]
            }),
        )
        .unwrap();
    assert_eq!(discarded["draft"], Value::Null);
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(
        state(&api, doc)["draft"]["source"],
        before["draft"]["source"]
    );
}

#[test]
fn malformed_native_draft_cannot_switch_layout_authority() {
    let (session, api, doc) = setup(false);
    session
        .with_doc_mut(doc, |document| {
            document.font.feature_draft = Some(fc_model::FeatureDraft {
                source: fc_model::Features {
                    source_of_truth: FeaturesSource::ImportedTables,
                    ..Default::default()
                },
                settings: Default::default(),
                revision: 0,
            });
        })
        .unwrap();
    let before = state(&api, doc);
    let error = api
        .call("features.apply", revisions(doc, &before))
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
    assert_eq!(state(&api, doc), before);
}

#[test]
fn generic_command_batches_cannot_forge_feature_provenance() {
    let (session, api, doc) = setup(false);
    let before = state(&api, doc);
    let forged = json!({"SetFeatureDraft":{
        "expected_revision":before["draft_revision"],
        "source":{"features":[{"id":"victim","tag":"smcp","code":"sub A by A.sc;"}]},
        "settings":{"generated":[{"generator":"SmallCaps","source_id":"victim","glyphs":["A"]}],"glyph_fingerprint":"forged"}
    }});
    for command in [
        forged.clone(),
        json!({"Batch":[{"Batch":[forged.clone()]}]}),
    ] {
        let error = api
            .call(
                "doc.apply",
                json!({"doc":doc,"command":command,"label":"forged"}),
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_params");
    }
    let token = api
        .call("doc.edit_begin", json!({"doc":doc,"label":"forged edit"}))
        .unwrap();
    let error = api
        .call(
            "doc.edit_apply",
            json!({"doc":doc,"token":token,"command":{"Batch":[forged]}}),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
    assert_eq!(state(&api, doc), before);
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        0
    );
}

#[test]
fn freezing_automatic_source_changes_only_one_draft_transaction() {
    let (session, api, doc) = setup(false);
    let before = state(&api, doc);
    let mut settings = before["active_settings"].clone();
    settings["small_caps"] = json!(true);
    let draft=api.call("features.set_draft",json!({"doc":doc,
        "expected_generation":before["generation"],"expected_draft_revision":before["draft_revision"],
        "source":before["active_source"],"settings":settings})).unwrap();
    let applied = api.call("features.apply", revisions(doc, &draft)).unwrap();
    assert_eq!(applied["applied"], true, "{applied}");
    let active = &applied["state"];
    assert!(!active["active_settings"]["generated"]
        .as_array()
        .unwrap()
        .is_empty());
    let history = session
        .with_doc(doc, |document| document.undo_len())
        .unwrap();
    let mut params = revisions(doc, active);
    params["generator"] = json!("SmallCaps");
    let frozen = api.call("features.freeze", params).unwrap();
    assert_eq!(frozen["active_source"], active["active_source"]);
    assert_eq!(frozen["active_settings"], active["active_settings"]);
    assert_eq!(frozen["compile_revision"], active["compile_revision"]);
    assert_eq!(frozen["draft"]["settings"]["small_caps"], false);
    assert!(frozen["draft"]["settings"]["generated"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        history + 1
    );
    let result = api.call("features.apply", revisions(doc, &frozen)).unwrap();
    assert_eq!(result["applied"], true, "{result}");
    assert_eq!(
        result["state"]["active_source"]["features"][0]["automatic"],
        false
    );
}

#[test]
fn caller_supplied_validation_and_source_cannot_replace_the_saved_candidate() {
    let (_session, api, doc) = setup(false);
    let draft = candidate(&api, doc, "sub Missing by A.sc;");
    let mut request = revisions(doc, &draft);
    request["valid"] = json!(true);
    request["proof"] = json!({"valid":true,"compiled_masters":[]});
    request["source"] = json!({"features":[{"tag":"smcp","code":"sub A by A.sc;"}]});
    let result = api.call("features.apply", request).unwrap();
    assert_eq!(result["applied"], false);
    assert_eq!(state(&api, doc), draft);
}

#[test]
fn draft_typing_keeps_active_shaping_current() {
    let (session, api, doc) = setup(false);
    session
        .with_doc_mut(doc, |document| {
            document.font.glyph_mut("A").unwrap().unicodes = vec![65];
        })
        .unwrap();
    let before = api
        .call("text.shape", json!({"doc":doc,"text":"A"}))
        .unwrap();
    assert_eq!(before["stale"], false);
    candidate(&api, doc, "invalid unfinished source");
    let after = api
        .call("text.shape", json!({"doc":doc,"text":"A"}))
        .unwrap();
    assert_eq!(
        after["stale"], false,
        "draft typing must not schedule active layout recompilation"
    );
    assert_eq!(after["names"], before["names"]);
    assert_eq!(after["glyphs"], before["glyphs"]);
}

#[test]
fn generation_diagnostic_limit_is_explicit_without_activating_source() {
    let (session, api, doc) = setup(false);
    session
        .with_doc_mut(doc, |document| {
            let master = document.font.default_master().id.clone();
            for index in 0..120 {
                let mut glyph = Glyph::new(format!("missing{index}.sc"));
                glyph.layers.push(Layer::new_master(master.clone()));
                document.font.insert_glyph(glyph).unwrap();
            }
        })
        .unwrap();
    let before = state(&api, doc);
    let mut settings = before["active_settings"].clone();
    settings["small_caps"] = json!(true);
    let draft=api.call("features.set_draft",json!({"doc":doc,
        "expected_generation":before["generation"],"expected_draft_revision":before["draft_revision"],
        "source":before["active_source"],"settings":settings})).unwrap();
    let result = api
        .call("features.validate", revisions(doc, &draft))
        .unwrap();
    assert_eq!(result["truncated"], true, "{result}");
    assert!(result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic["code"] == "feature-generation-diagnostics-limit"));
    assert!(result["diagnostics"].as_array().unwrap().len() <= 200);
    assert_eq!(state(&api, doc), draft);
}
