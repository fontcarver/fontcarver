use std::sync::Arc;

use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Component, Contour, Font, Glyph, GlyphId, Layer};
use kurbo::BezPath;
use serde_json::{json, Value};

fn glyph(font: &mut Font, name: &str, base: Option<&str>) {
    let mut glyph = Glyph::new(name);
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 620.;
    layer.contours = Contour::from_bezpath(&BezPath::from_svg("M0 0L200 0L100 400Z").unwrap());
    if let Some(base) = base {
        layer.components.push(Component {
            base: base.into(),
            ..Default::default()
        });
    }
    glyph.note = format!("source {name}");
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
}

fn fixture() -> (Api, Arc<Session>, DocId, DocId) {
    let mut source = Font::new_empty("Source", 1000);
    glyph(&mut source, "base", None);
    glyph(&mut source, "composite", Some("base"));
    let mut target = Font::new_empty("Target", 1000);
    glyph(&mut target, "destination", None);
    let session = Arc::new(Session::new());
    let source = session.open_font(source, None);
    let target = session.open_font(target, None);
    (Api::new(session.clone()), session, source, target)
}

fn gid(session: &Session, doc: DocId, name: &str) -> GlyphId {
    session
        .with_doc(doc, |d| d.glyph_id(name).unwrap())
        .unwrap()
}

fn font(session: &Session, doc: DocId) -> Font {
    session.with_doc(doc, |d| d.font.clone()).unwrap()
}

fn params(session: &Session, source: DocId, target: DocId) -> Value {
    json!({"doc":target,"source_doc":source,"mappings":[{"source":gid(session,source,"composite"),"target":gid(session,target,"destination")}],"destination":"selected","content":"all_data","missing_bases":"copy","dry_run":false})
}

#[test]
fn preview_copy_undo_redo_and_native_persistence_are_atomic() {
    let (api, session, source, target) = fixture();
    let source_before = font(&session, source);
    let target_before = font(&session, target);
    let mut request = params(&session, source, target);
    request["dry_run"] = json!(true);
    let preview = api.call("doc.paste_special", request.clone()).unwrap();
    assert!(preview["change"].is_null());
    assert_eq!(preview["added_bases"], json!(["base"]));
    assert_eq!(font(&session, target), target_before);
    request["dry_run"] = json!(false);
    request["expected_generation"] = preview["generation"].clone();
    request["expected_source_generation"] = preview["source_generation"].clone();
    api.call("doc.paste_special", request).unwrap();
    let after = font(&session, target);
    let copied = after.glyph("destination").unwrap();
    assert_eq!(copied.note, "source composite");
    assert_eq!(copied.layers[0].components[0].base.as_ref(), "base");
    assert_eq!(
        copied.layers[0].kind,
        target_before.glyph("destination").unwrap().layers[0].kind
    );
    assert_eq!(font(&session, source), source_before);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paste.fcf");
    fc_doc::fcf::write(&path, &after).unwrap();
    assert_eq!(fc_doc::fcf::read(&path).unwrap(), after);
    api.call("doc.undo", json!({"doc":target})).unwrap();
    assert_eq!(font(&session, target), target_before);
    api.call("doc.redo", json!({"doc":target})).unwrap();
    assert_eq!(font(&session, target), after);
}

#[test]
fn missing_base_policies_reject_mark_placeholder_or_decompose() {
    for policy in ["reject", "placeholder", "decompose"] {
        let (api, session, source, target) = fixture();
        let before = font(&session, target);
        let mut request = params(&session, source, target);
        request["missing_bases"] = json!(policy);
        let result = api.call("doc.paste_special", request);
        if policy == "reject" {
            assert!(result.is_err());
            assert_eq!(font(&session, target), before);
        } else {
            result.unwrap();
            let after = font(&session, target);
            let layer = &after.glyph("destination").unwrap().layers[0];
            if policy == "placeholder" {
                let base = after.glyph("base").unwrap();
                assert!(!base.export);
                assert!(base.tags.contains("missing-component-base"));
                assert_eq!(layer.components.len(), 1);
            } else {
                assert!(after.glyph("base").is_none());
                assert!(layer.components.is_empty());
                assert_eq!(layer.contours.len(), 2);
            }
        }
    }
}

#[test]
fn stale_source_and_late_mapping_failure_never_modify_destination() {
    let (api, session, source, target) = fixture();
    let before = font(&session, target);
    let mut request = params(&session, source, target);
    request["expected_source_generation"] = json!(999);
    assert!(api
        .call("doc.paste_special", request.clone())
        .unwrap_err()
        .message
        .contains("stale"));
    request["expected_source_generation"] = Value::Null;
    request["mappings"]
        .as_array_mut()
        .unwrap()
        .push(json!({"source":999999,"target":null}));
    assert!(api.call("doc.paste_special", request).is_err());
    assert_eq!(font(&session, target), before);
    assert!(api
        .call("doc.undo", json!({"doc":target}))
        .unwrap()
        .is_null());
}

#[test]
fn relink_preserves_transform_is_undoable_and_rejects_cycle() {
    let (api, session, source, _) = fixture();
    let before = font(&session, source);
    let request = json!({"doc":source,"gids":[gid(&session,source,"composite")],"from":"base","to":gid(&session,source,"composite")});
    assert!(api.call("doc.relink_components", request).is_err());
    assert_eq!(font(&session, source), before);
    let add = json!({"doc":source,"source_doc":source,"mappings":[{"source":gid(&session,source,"base"),"target":gid(&session,source,"composite")}],"destination":"selected","content":"layer","missing_bases":"reject"});
    api.call("doc.paste_special", add).unwrap();
    api.call("doc.undo", json!({"doc":source})).unwrap();
    assert_eq!(font(&session, source), before);
    session
        .with_doc_mut(source, |doc| {
            let replacement = doc.font.glyph("base").unwrap().clone();
            let mut replacement = replacement;
            replacement.name = "replacement".into();
            replacement.locked = true;
            doc.apply(fc_doc::Command::AddGlyph { glyph: replacement }, "Fixture")
                .unwrap();
        })
        .unwrap();
    let before = font(&session, source);
    let relink = json!({"doc":source,"gids":[gid(&session,source,"composite"),gid(&session,source,"replacement")],"from":"base","to":gid(&session,source,"replacement")});
    api.call("doc.relink_components", relink).unwrap();
    let after = font(&session, source);
    let component = &after.glyph("composite").unwrap().layers[0].components[0];
    assert_eq!(component.base.as_ref(), "replacement");
    assert_eq!(
        component.transform,
        before.glyph("composite").unwrap().layers[0].components[0].transform
    );
    api.call("doc.undo", json!({"doc":source})).unwrap();
    assert_eq!(font(&session, source), before);
}

#[test]
fn destination_and_content_matrix_preserves_unselected_fields() {
    for destination in ["same_name", "selected", "components"] {
        for content in ["all_data", "layer", "kerning_groups", "metric_keys"] {
            let (api, session, source, target) = fixture();
            session
                .with_doc_mut(source, |doc| {
                    let glyph = doc.font.glyph_mut("composite").unwrap();
                    glyph.metrics_keys.width = Some("=720".into());
                    glyph.layers[0].metrics_keys = Some(fc_model::MetricsKeys {
                        left: Some("==20".into()),
                        ..Default::default()
                    });
                    doc.apply(
                        fc_doc::Command::CreateKerningGroup {
                            side: fc_doc::kerning::KerningGroupSide::Side1,
                            name: "public.kern1.Source".into(),
                            members: vec!["composite".into()],
                        },
                        "Fixture",
                    )
                    .unwrap();
                })
                .unwrap();
            let before = font(&session, target);
            let mut request = params(&session, source, target);
            request["destination"] = json!(destination);
            request["content"] = json!(content);
            let target_name = if destination == "same_name" {
                request["mappings"][0]["target"] = Value::Null;
                "composite"
            } else {
                "destination"
            };
            api.call("doc.paste_special", request)
                .unwrap_or_else(|error| panic!("{destination}/{content}: {error}"));
            let after = font(&session, target);
            let copied = after.glyph(target_name).unwrap();
            if content == "kerning_groups" {
                assert_eq!(
                    copied.kern_group_side1.as_deref(),
                    Some("public.kern1.Source")
                );
                assert!(after.kerning_groups.side1["public.kern1.Source"].contains(target_name));
            }
            if content == "metric_keys" {
                assert_eq!(copied.metrics_keys.width.as_deref(), Some("=720"));
                assert_eq!(
                    copied.layers[0]
                        .metrics_keys
                        .as_ref()
                        .unwrap()
                        .left
                        .as_deref(),
                    Some("==20")
                );
            }
            if destination != "same_name" && content != "all_data" {
                assert_eq!(copied.note, before.glyph("destination").unwrap().note);
            }
            if destination == "components" && matches!(content, "all_data" | "layer") {
                assert_eq!(
                    copied.layers[0].contours,
                    before.glyph("destination").unwrap().layers[0].contours
                );
                assert_eq!(copied.layers[0].components[0].base.as_ref(), "composite");
            }
            api.call("doc.undo", json!({"doc":target})).unwrap();
            assert_eq!(font(&session, target), before);
        }
    }
}

#[test]
fn selected_batch_relinks_selected_names_in_any_mapping_order() {
    let (api, session, source, target) = fixture();
    session
        .with_doc_mut(target, |doc| {
            doc.apply(
                fc_doc::Command::AddGlyph {
                    glyph: Glyph::new("renamed_base"),
                },
                "Fixture",
            )
        })
        .unwrap()
        .unwrap();
    let mut request = params(&session, source, target);
    request["missing_bases"] = json!("reject");
    request["mappings"].as_array_mut().unwrap().push(
        json!({"source":gid(&session,source,"base"),"target":gid(&session,target,"renamed_base")}),
    );
    api.call("doc.paste_special", request).unwrap();
    let after = font(&session, target);
    assert!(after.glyph("base").is_none());
    assert_eq!(
        after.glyph("destination").unwrap().layers[0].components[0]
            .base
            .as_ref(),
        "renamed_base"
    );
}

#[test]
fn same_name_component_copy_uses_a_distinct_base_and_same_doc_cycle_rejects() {
    let (api, session, source, target) = fixture();
    session
        .with_doc_mut(target, |doc| {
            doc.apply(
                fc_doc::Command::AddGlyph {
                    glyph: Glyph::new("base"),
                },
                "Fixture",
            )
        })
        .unwrap()
        .unwrap();
    let request = json!({"doc":target,"source_doc":source,"mappings":[{"source":gid(&session,source,"base"),"target":gid(&session,target,"base")}],"destination":"components","content":"layer","missing_bases":"copy"});
    let result = api.call("doc.paste_special", request).unwrap();
    assert_eq!(result["added_bases"], json!(["base.001"]));
    assert_eq!(
        font(&session, target).glyph("base").unwrap().layers[0].components[0]
            .base
            .as_ref(),
        "base.001"
    );
    let before = font(&session, source);
    let request = json!({"doc":source,"source_doc":source,"mappings":[{"source":gid(&session,source,"base"),"target":gid(&session,source,"base")}],"destination":"components","content":"layer","missing_bases":"copy"});
    assert!(api.call("doc.paste_special", request).is_err());
    assert_eq!(font(&session, source), before);
}

#[test]
fn explicit_cff_dictionary_remaps_by_value_and_unmatched_non_cid_rejects() {
    let (api, session, source, target) = fixture();
    session
        .with_doc_mut(source, |doc| {
            let private = doc.font.masters[0].private_dict.clone();
            doc.font.masters[0].cff_private_dicts.push(private);
            doc.font.glyph_mut("composite").unwrap().layers[0].cff_private_dict_index = Some(1);
        })
        .unwrap();
    api.call("doc.paste_special", params(&session, source, target))
        .unwrap();
    assert_eq!(
        font(&session, target).glyph("destination").unwrap().layers[0].cff_private_dict_index,
        None
    );
    let before = font(&session, target);
    session
        .with_doc_mut(source, |doc| {
            doc.font.masters[0].cff_private_dicts[0].std_hw = Some(83.)
        })
        .unwrap();
    assert!(api
        .call("doc.paste_special", params(&session, source, target))
        .unwrap_err()
        .message
        .contains("CID destination"));
    assert_eq!(font(&session, target), before);
}
