use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Font, Glyph, KernSide};
use serde_json::{json, Value};
use std::sync::Arc;

fn setup() -> (Api, Arc<Session>, DocId, String) {
    let mut font = Font::new_empty("API kerning", 1000);
    for name in ["A", "V", "W"] {
        font.insert_glyph(Glyph::new(name)).unwrap();
    }
    let master = font.masters[0].id.to_string();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session.clone()), session, doc, master)
}

fn apply(api: &Api, doc: DocId, command: Value) -> Value {
    api.call(
        "doc.apply",
        json!({"doc": doc, "command": command, "label": "Kerning"}),
    )
    .unwrap()
}

#[test]
fn real_commands_resolve_exception_priority_and_zero() {
    let (api, _, doc, master) = setup();
    for (side, name, member) in [
        ("side1", "public.kern1.A", "A"),
        ("side2", "public.kern2.V", "V"),
    ] {
        apply(
            &api,
            doc,
            json!({"SetKerningGroup": {"side": side, "name": name, "members": [member]}}),
        );
    }
    let candidates = [
        (json!({"glyph": "A"}), json!({"glyph": "V"}), 0),
        (
            json!({"glyph": "A"}),
            json!({"group": "public.kern2.V"}),
            -60,
        ),
        (
            json!({"group": "public.kern1.A"}),
            json!({"glyph": "V"}),
            -40,
        ),
        (
            json!({"group": "public.kern1.A"}),
            json!({"group": "public.kern2.V"}),
            -100,
        ),
    ];
    for (left, right, value) in &candidates {
        let change = apply(
            &api,
            doc,
            json!({"SetHorizontalKerning": {"master": master, "left": left, "right": right, "value": value}}),
        );
        assert_eq!(change["layout"], true);
        assert_eq!(change["glyphs"], json!([]));
    }
    for (left, right, value) in candidates {
        let result = api
            .call(
                "kerning.resolve",
                json!({"doc": doc, "master": master, "left": "A", "right": "V"}),
            )
            .unwrap();
        assert_eq!(result["value"].as_f64(), Some(value as f64));
        assert_eq!(result["rule"]["left"], left);
        assert_eq!(result["left_group"], "public.kern1.A");
        apply(
            &api,
            doc,
            json!({"SetHorizontalKerning": {"master": master, "left": left, "right": right, "value": null}}),
        );
    }
    let result = api
        .call(
            "kerning.resolve",
            json!({"doc": doc, "master": master, "left": "A", "right": "V"}),
        )
        .unwrap();
    assert_eq!(result["rule"], Value::Null);
    assert_eq!(result["value"], 0.0);
}

#[test]
fn group_pages_membership_and_removal_impact_are_bounded() {
    let (api, _, doc, master) = setup();
    apply(
        &api,
        doc,
        json!({"SetKerningGroup": {"side": "side1", "name": "public.kern1.A", "members": ["A", "W"]}}),
    );
    apply(
        &api,
        doc,
        json!({"SetHorizontalKerning": {"master": master, "left": {"group": "public.kern1.A"}, "right": {"glyph": "V"}, "value": -80}}),
    );
    let result = api
        .call(
            "kerning.groups",
            json!({"doc": doc, "side": "side1", "offset": 0, "limit": 1}),
        )
        .unwrap();
    assert_eq!(result["total"], 1);
    assert_eq!(result["items"][0]["member_count"], 2);
    assert_eq!(result["items"][0]["pair_count"], 1);
    let members = api
        .call(
            "kerning.members",
            json!({"doc": doc, "side": "side1", "name": "public.kern1.A", "offset": 0, "limit": 1}),
        )
        .unwrap();
    assert_eq!(members["total"], 2);
    assert_eq!(members["items"], json!(["A"]));
    let glyph = api
        .call("kerning.glyph", json!({"doc": doc, "glyph": "W"}))
        .unwrap();
    assert_eq!(glyph["side1"], "public.kern1.A");
    assert_eq!(glyph["side2"], Value::Null);
}

#[test]
fn pagination_rejects_stale_or_missing_generations_and_excessive_limits() {
    let (api, _, doc, master) = setup();
    let page = json!({"doc": doc, "master": master, "offset": 0, "limit": 1});
    let first = api.call("kerning.pairs", page.clone()).unwrap();
    apply(
        &api,
        doc,
        json!({"SetHorizontalKerning": {"master": master, "left": {"glyph": "A"}, "right": {"glyph": "V"}, "value": -80}}),
    );
    let mut stale = page;
    stale["generation"] = first["status"]["generation"].clone();
    assert_eq!(
        api.call("kerning.pairs", stale).unwrap_err().code,
        "stale_generation"
    );
    for params in [
        json!({"doc": doc, "master": master, "offset": 1, "limit": 1}),
        json!({"doc": doc, "master": master, "offset": 0, "limit": 201}),
    ] {
        assert_eq!(
            api.call("kerning.pairs", params).unwrap_err().code,
            "invalid_params"
        );
    }
}

#[test]
fn unsupported_layout_is_structured_and_cannot_be_edited_via_api() {
    let (api, session, doc, master) = setup();
    session
        .with_doc_mut(doc, |document| {
            document.font.features.source_of_truth = fc_model::FeaturesSource::ImportedTables
        })
        .unwrap();
    let status = api.call("kerning.status", json!({"doc": doc})).unwrap();
    assert_eq!(status["editable"], false);
    assert_eq!(status["reason"], "imported_layout");
    let result = api
        .call(
            "kerning.resolve",
            json!({"doc": doc, "master": master, "left": "A", "right": "V"}),
        )
        .unwrap();
    assert_eq!(result["value"], Value::Null);
    let err = api.call("doc.apply", json!({"doc": doc, "label": "Bypass", "command": {"SetHorizontalKerning": {"master": master, "left": {"glyph": "A"}, "right": {"glyph": "V"}, "value": -80}}})).unwrap_err();
    assert_eq!(err.code, "kerning_unavailable");
}

#[test]
fn unresolved_and_nonfinite_imported_records_are_retained_and_reported() {
    let (api, session, doc, master) = setup();
    session
        .with_doc_mut(doc, |document| {
            document.font.masters[0].kerning_h.0.insert(
                (
                    KernSide::Glyph("missing".into()),
                    KernSide::Glyph("V".into()),
                ),
                -80.0,
            );
            document.font.masters[0].kerning_h.0.insert(
                (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
                f64::NAN,
            );
        })
        .unwrap();
    let result = api
        .call(
            "kerning.pairs",
            json!({"doc": doc, "master": master, "offset": 0, "limit": 10}),
        )
        .unwrap();
    assert_eq!(result["total"], 2);
    assert!(result["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["diagnostic"].is_string()));
    assert!(result["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["value"].is_null()));
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.masters[0].kerning_h.0.len(), 2);
            assert_eq!(document.generation, 0);
        })
        .unwrap();
}

#[test]
fn invalid_membership_diagnostics_match_resolution_and_authoring() {
    for name in ["public.kern1.   ", "public.kern1.A\n"] {
        let (api, session, doc, master) = setup();
        session
            .with_doc_mut(doc, |document| {
                document
                    .font
                    .kerning_groups
                    .side1
                    .insert(name.into(), ["A".into()].into());
            })
            .unwrap();
        let glyph = api
            .call("kerning.glyph", json!({"doc": doc, "glyph": "A"}))
            .unwrap();
        assert_eq!(glyph["diagnostics"].as_array().unwrap().len(), 1);
        assert!(api
            .call(
                "kerning.resolve",
                json!({"doc": doc, "master": master, "left": "A", "right": "V"})
            )
            .is_err());
        assert!(api.call("doc.apply", json!({"doc": doc, "label": "Invalid", "command": {"SetHorizontalKerning": {"master": master, "left": {"glyph": "A"}, "right": {"glyph": "V"}, "value": -80}}})).is_err());
        assert_eq!(
            api.call("kerning.status", json!({"doc": doc})).unwrap()["generation"],
            0
        );
    }
    let (api, session, doc, master) = setup();
    session
        .with_doc_mut(doc, |document| {
            document.font.kerning_groups.side1.insert(
                "public.kern1.A".into(),
                ["A".into(), "missing".into()].into(),
            );
            document.font.masters[0].kerning_h.0.insert(
                (
                    KernSide::Group("public.kern1.A".into()),
                    KernSide::Glyph("V".into()),
                ),
                -100.0,
            );
        })
        .unwrap();
    let glyph = api
        .call("kerning.glyph", json!({"doc": doc, "glyph": "A"}))
        .unwrap();
    assert_eq!(glyph["diagnostics"].as_array().unwrap().len(), 1);
    let params = json!({"doc": doc, "master": master, "left": "A", "right": "V"});
    assert!(api.call("kerning.resolve", params.clone()).is_err());
    apply(
        &api,
        doc,
        json!({"SetHorizontalKerning": {"master": master, "left": {"glyph": "A"}, "right": {"glyph": "V"}, "value": -80}}),
    );
    let result = api.call("kerning.resolve", params).unwrap();
    assert_eq!(result["value"], -80.0);
    assert_eq!(result["records"].as_array().unwrap().len(), 1);
}

#[test]
fn group_and_member_continuation_pages_are_generation_bound() {
    let (api, _, doc, _) = setup();
    for (name, members) in [
        ("public.kern1.A", vec!["A", "V"]),
        ("public.kern1.W", vec!["W"]),
    ] {
        apply(
            &api,
            doc,
            json!({"SetKerningGroup": {"side": "side1", "name": name, "members": members}}),
        );
    }
    let first = api
        .call(
            "kerning.groups",
            json!({"doc": doc, "side": "side1", "offset": 0, "limit": 1}),
        )
        .unwrap();
    let generation = first["status"]["generation"].clone();
    let group_params =
        json!({"doc": doc, "side": "side1", "offset": 1, "limit": 1, "generation": generation});
    let member_params = json!({"doc": doc, "side": "side1", "name": "public.kern1.A", "offset": 1, "limit": 1, "generation": generation});
    let groups = api.call("kerning.groups", group_params.clone()).unwrap();
    assert_eq!(groups["items"][0]["name"], "public.kern1.W");
    let members = api.call("kerning.members", member_params.clone()).unwrap();
    assert_eq!(members["items"], json!(["V"]));
    apply(
        &api,
        doc,
        json!({"AssignKerningGroup": {"side": "side1", "glyph": "V", "group": null}}),
    );
    for (method, params) in [
        ("kerning.groups", group_params),
        ("kerning.members", member_params),
    ] {
        assert_eq!(
            api.call(method, params).unwrap_err().code,
            "stale_generation"
        );
    }
}
