//! Real outline silhouettes, revision fences and atomic optical edits.
use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Component, Contour, Font, Glyph, KernSide, Layer, MasterId};
use kurbo::{Affine, Circle, Rect, Shape};
use serde_json::{json, Value};
use std::sync::Arc;

fn fixture() -> (Api, Arc<Session>, DocId, Value) {
    let mut font = Font::new_empty("Optical fixture", 1000);
    let mut second = font.masters[0].clone();
    second.id = MasterId::new();
    second.name = "Second".into();
    font.masters.push(second);
    for name in ["H", "O", "component", "space", "mark"] {
        let mut glyph = Glyph::new(name);
        for master in &font.masters {
            let mut layer = Layer::new_master(master.id.clone());
            layer.width = if name == "mark" { 0. } else { 800. };
            match name {
                "H" | "mark" => {
                    layer.contours =
                        Contour::from_bezpath(&Rect::new(100., 0., 600., 500.).to_path(0.01))
                }
                "O" => {
                    layer.contours =
                        Contour::from_bezpath(&Circle::new((350., 250.), 250.).to_path(0.01))
                }
                "component" => layer.components.push(Component {
                    base: "O".into(),
                    transform: Affine::translate((75., 0.)),
                    ..Default::default()
                }),
                _ => (),
            }
            glyph.layers.push(layer);
        }
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let glyphs = session
        .with_doc(doc, |d| {
            ["component", "H", "O", "space", "mark"].map(|n| d.glyph_id(n).unwrap())
        })
        .unwrap();
    let params = json!({"doc":doc, "glyphs":glyphs, "master":null, "operation":"spacing", "density":1., "replace_existing":false, "dry_run":true, "expected_generation":null, "label":"Optical spacing"});
    (Api::new(session.clone()), session, doc, params)
}
fn font(session: &Session, doc: DocId) -> Font {
    session.with_doc(doc, |d| d.font.clone()).unwrap()
}
// Native JSON parsing can change cubic control coordinates by a final binary
// digit. Keep every topology, metadata, metric and component assertion exact;
// permit only a few machine epsilons in outline coordinates before comparing.
fn assert_outline_roundtrip(mut reopened: Font, expected: &Font) {
    for (name, glyph) in &expected.glyphs {
        let actual = reopened.glyph_mut(name).unwrap();
        assert_eq!(actual.layers.len(), glyph.layers.len());
        for (actual_layer, layer) in actual.layers.iter_mut().zip(&glyph.layers) {
            assert_eq!(actual_layer.contours.len(), layer.contours.len());
            for (actual_contour, contour) in actual_layer.contours.iter_mut().zip(&layer.contours) {
                assert_eq!(actual_contour.points.len(), contour.points.len());
                for (actual_point, point) in actual_contour.points.iter_mut().zip(&contour.points) {
                    for (actual_value, expected_value) in
                        [(actual_point.x, point.x), (actual_point.y, point.y)]
                    {
                        let tolerance = 4. * f64::EPSILON * expected_value.abs().max(1.);
                        assert!(
                            (actual_value - expected_value).abs() <= tolerance,
                            "{name} outline changed on reopen: {actual_value} versus {expected_value}"
                        );
                    }
                    actual_point.x = point.x;
                    actual_point.y = point.y;
                }
            }
        }
    }
    assert_eq!(&reopened, expected);
}

fn apply(api: &Api, params: &Value) -> Value {
    let mut request = params.clone();
    let preview = api.call("optical.batch", request.clone()).unwrap();
    request["dry_run"] = json!(false);
    request["expected_generation"] = preview["generation"].clone();
    api.call("optical.batch", request).unwrap()
}

#[test]
fn optical_curves_preview_component_order_undo_and_native_persistence() {
    let (api, session, doc, mut params) = fixture();
    let before = font(&session, doc);
    let preview = api.call("optical.batch", params.clone()).unwrap();
    assert_eq!(preview["changed_count"], 6);
    assert_eq!(preview["skipped"].as_array().unwrap().len(), 4);
    assert_eq!(font(&session, doc), before);
    apply(&api, &params);
    let after = font(&session, doc);
    for master in &after.masters {
        let h = fc_geom::layer_bbox(&after, "H", &master.id)
            .unwrap()
            .unwrap();
        let o = fc_geom::layer_bbox(&after, "O", &master.id)
            .unwrap()
            .unwrap();
        let composite = fc_geom::layer_bbox(&after, "component", &master.id)
            .unwrap()
            .unwrap();
        assert_eq!(h.x0, 50.);
        assert!(o.x0 < h.x0);
        assert_eq!(composite, o);
        assert_eq!(
            after
                .glyph("component")
                .unwrap()
                .master_layer(&master.id)
                .unwrap()
                .components
                .len(),
            1
        );
    }
    for example in preview["examples"].as_array().unwrap() {
        let name = example["glyph"].as_str().unwrap();
        let master = after
            .masters
            .iter()
            .find(|m| m.name == example["master"])
            .unwrap();
        let layer = after.glyph(name).unwrap().master_layer(&master.id).unwrap();
        assert_eq!(layer.width, example["after"][2].as_f64().unwrap());
    }
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(font(&session, doc), before);
    params["glyphs"].as_array_mut().unwrap().reverse();
    apply(&api, &params);
    assert_eq!(font(&session, doc), after);
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    assert_eq!(font(&session, doc), after);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("optical.fcf");
    api.call("session.save", json!({"doc":doc,"path":path}))
        .unwrap();
    assert_outline_roundtrip(fc_doc::fcf::read(&path).unwrap(), &after);
}

#[test]
fn stale_invalid_and_metrics_linked_operations_preserve_source() {
    let (api, session, doc, mut params) = fixture();
    let before = font(&session, doc);
    params["dry_run"] = json!(false);
    params["expected_generation"] = json!(9999);
    assert!(api.call("optical.batch", params.clone()).is_err());
    params["dry_run"] = json!(true);
    params["density"] = json!(0.);
    assert!(api.call("optical.batch", params.clone()).is_err());
    assert_eq!(font(&session, doc), before);
    session
        .with_doc_mut(doc, |d| {
            d.font.glyph_mut("H").unwrap().metrics_keys.left = Some("=20".into())
        })
        .unwrap();
    params["density"] = json!(1.);
    let before = font(&session, doc);
    let preview = apply(&api, &params);
    assert!(preview["skipped"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["reason"] == "metrics_key"));
    assert_eq!(font(&session, doc).glyph("H"), before.glyph("H"));
}

#[test]
fn automatic_pairs_protect_class_rules_and_explicit_zero_and_respect_master_scope() {
    let (api, session, doc, mut params) = fixture();
    let master = session
        .with_doc_mut(doc, |d| {
            d.font
                .kerning_groups
                .side1
                .insert("public.kern1.H".into(), ["H".into()].into());
            d.font
                .kerning_groups
                .side2
                .insert("public.kern2.O".into(), ["O".into()].into());
            let master = &mut d.font.masters[0];
            master.kerning_h.0.insert(
                (
                    KernSide::Group("public.kern1.H".into()),
                    KernSide::Group("public.kern2.O".into()),
                ),
                -25.,
            );
            master.kerning_h.0.insert(
                (KernSide::Glyph("O".into()), KernSide::Glyph("H".into())),
                0.,
            );
            master.id.clone()
        })
        .unwrap();
    params["operation"] = json!("kerning");
    params["master"] = json!(master);
    let before = font(&session, doc);
    let preview = apply(&api, &params);
    assert!(
        preview["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["reason"] == "existing_kerning")
            .count()
            == 2
    );
    let after = font(&session, doc);
    assert_eq!(after.masters[1].kerning_h, before.masters[1].kerning_h);
    for (pair, value) in &before.masters[0].kerning_h.0 {
        assert_eq!(after.masters[0].kerning_h.0.get(pair), Some(value));
    }
    params["replace_existing"] = json!(true);
    apply(&api, &params);
    assert!(font(&session, doc).masters[0]
        .kerning_h
        .0
        .contains_key(&(KernSide::Glyph("H".into()), KernSide::Glyph("O".into()))));
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(font(&session, doc), after);
}

#[test]
fn realistic_outline_batches_are_bounded_and_do_not_edit_imported_kerning() {
    for fixture in ["NotoSans-Regular.ttf", "SourceSans3Min.otf"] {
        let input = fc_sfnt::read_path(&fc_fixtures::path(fixture), &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let session = Arc::new(Session::new());
        let doc = session.open_font(input, None);
        let glyphs = session
            .with_doc(doc, |d| {
                ["A", "V", "o"]
                    .iter()
                    .map(|n| d.glyph_id(n).unwrap())
                    .collect::<Vec<_>>()
            })
            .unwrap();
        let api = Api::new(session.clone());
        let mut params = json!({"doc":doc,"glyphs":glyphs,"master":null,"operation":"spacing","density":1.,"replace_existing":false,"dry_run":true,"expected_generation":null,"label":"Optical spacing"});
        let before = font(&session, doc);
        let started = std::time::Instant::now();
        let preview = api.call("optical.batch", params.clone()).unwrap();
        assert!(preview["changed_count"].as_u64().unwrap() >= 3);
        params["density"] = json!(2.);
        let tighter = api.call("optical.batch", params.clone()).unwrap();
        for (normal, dense) in preview["examples"]
            .as_array()
            .unwrap()
            .iter()
            .zip(tighter["examples"].as_array().unwrap())
        {
            assert!(dense["after"][2].as_f64().unwrap() < normal["after"][2].as_f64().unwrap());
        }
        params["operation"] = json!("kerning");
        assert!(api.call("optical.batch", params.clone()).is_err());
        assert_eq!(font(&session, doc), before);
        // Use imported outlines in an explicitly created editable source font;
        // do not imply that this bypass converts or preserves imported layout.
        session
            .with_doc_mut(doc, |d| d.font.features = Default::default())
            .unwrap();
        let before = font(&session, doc);
        let preview = apply(&api, &params);
        assert!(preview["changed_count"].as_u64().unwrap() > 0);
        api.call("doc.undo", json!({"doc":doc})).unwrap();
        assert_eq!(font(&session, doc), before);
        assert!(
            started.elapsed().as_secs_f64() < 15.,
            "{fixture} exceeded the 15 second bounded-batch budget"
        );
    }
}

#[test]
fn excessive_pairs_and_late_geometry_errors_leave_no_partial_edit() {
    let (api, session, doc, mut params) = fixture();
    let generation = session
        .with_doc_mut(doc, |d| {
            d.font.glyph_mut("component").unwrap().layers[1].components[0].base = "missing".into();
            d.generation
        })
        .unwrap();
    let before = font(&session, doc);
    params["dry_run"] = json!(false);
    params["expected_generation"] = json!(generation);
    assert!(api.call("optical.batch", params.clone()).is_err());
    assert_eq!(font(&session, doc), before);
    assert_eq!(session.with_doc(doc, |d| d.generation).unwrap(), generation);
    params["glyphs"] = json!(vec![0; 129]);
    assert!(api.call("optical.batch", params).is_err());
}

#[test]
fn composite_preview_contains_actual_geometry_after_different_base_shifts() {
    let (api, session, doc, params) = fixture();
    session
        .with_doc_mut(doc, |d| {
            for layer in &mut d.font.glyph_mut("component").unwrap().layers {
                layer.components.push(Component {
                    base: "H".into(),
                    transform: Affine::translate((400., 0.)),
                    ..Default::default()
                });
            }
        })
        .unwrap();
    let preview = api.call("optical.batch", params.clone()).unwrap();
    apply(&api, &params);
    let after = font(&session, doc);
    for example in preview["examples"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["glyph"] == "component")
    {
        let master = after
            .masters
            .iter()
            .find(|m| m.name == example["master"])
            .unwrap();
        let actual = fc_geom::flatten_layer(&after, "component", &master.id).unwrap();
        let shown = kurbo::BezPath::from_svg(example["after_outline"].as_str().unwrap()).unwrap();
        assert_eq!(actual, shown);
        let original = kurbo::BezPath::from_svg(example["outline"].as_str().unwrap()).unwrap();
        let shift = example["after"][0].as_f64().unwrap() - example["before"][0].as_f64().unwrap();
        assert_ne!(Affine::translate((shift, 0.)) * original, shown);
    }
}

#[test]
fn a_review_applies_only_the_accepted_proposals_and_leaves_the_rest() {
    let (api, session, doc, mut params) = fixture();
    let preview = api.call("optical.batch", params.clone()).unwrap();
    let proposals = preview["proposals"].as_array().unwrap().clone();
    assert!(
        proposals.len() >= 4,
        "the review needs several rows: {proposals:?}"
    );
    assert_eq!(
        preview["changed_count"].as_u64().unwrap() as usize,
        proposals.len(),
        "an unfiltered preview proposes exactly what it would change"
    );
    // Accept only the "H" rows; every other layer must keep its metrics.
    let accepted: Vec<Value> = proposals
        .iter()
        .filter(|entry| entry["glyph"] == json!("H"))
        .map(|entry| {
            json!({"glyph": entry["glyph"], "right": entry["right"], "master": entry["master"]})
        })
        .collect();
    assert_eq!(accepted.len(), 2, "H has a layer in both masters");
    let before = font(&session, doc);
    params["accepted"] = json!(accepted);
    let applied = apply(&api, &params);
    assert_eq!(applied["changed_count"], json!(2));
    assert!(applied["change"].is_object());
    let after = font(&session, doc);
    for (name, glyph) in &after.glyphs {
        let original = before.glyph(name).unwrap();
        for (layer, source) in glyph.layers.iter().zip(&original.layers) {
            if &**name == "H" {
                assert_ne!(
                    (layer.width, layer.contours[0].points[0].x),
                    (source.width, source.contours[0].points[0].x),
                    "the accepted glyph must have moved"
                );
            } else {
                assert_eq!(layer, source, "{name} was not accepted and must not move");
            }
        }
    }
}

#[test]
fn an_empty_acceptance_changes_nothing_and_a_stale_one_is_refused() {
    let (api, session, doc, mut params) = fixture();
    let before = font(&session, doc);
    let generation = api.call("optical.batch", params.clone()).unwrap()["generation"].clone();

    params["accepted"] = json!([]);
    let preview = api.call("optical.batch", params.clone()).unwrap();
    assert_eq!(preview["changed_count"], json!(0));
    assert!(!preview["proposals"].as_array().unwrap().is_empty());
    let mut request = params.clone();
    request["dry_run"] = json!(false);
    request["expected_generation"] = generation.clone();
    let applied = api.call("optical.batch", request).unwrap();
    assert!(applied["change"].is_null(), "nothing to apply");
    assert_eq!(font(&session, doc), before);

    // A row the recomputed proposal does not hold is refused outright.
    let master = api.call("optical.batch", params.clone()).unwrap()["proposals"][0]["master"]
        .as_str()
        .unwrap()
        .to_string();
    for stale in [
        json!({"glyph": "H", "right": "O", "master": master.clone()}),
        json!({"glyph": "notaglyph", "right": null, "master": master.clone()}),
        json!({"glyph": "space", "right": null, "master": master}),
    ] {
        let mut request = params.clone();
        request["accepted"] = json!([stale]);
        request["dry_run"] = json!(false);
        request["expected_generation"] = generation.clone();
        let error = api.call("optical.batch", request).unwrap_err().message;
        assert!(
            error.contains("no longer exists"),
            "unexpected error: {error}"
        );
        assert_eq!(font(&session, doc), before);
    }
}

#[test]
fn an_accepted_kerning_subset_writes_only_those_pairs() {
    let (api, session, doc, mut params) = fixture();
    params["operation"] = json!("kerning");
    let preview = api.call("optical.batch", params.clone()).unwrap();
    let proposals = preview["proposals"].as_array().unwrap().clone();
    assert!(proposals.len() >= 4, "{proposals:?}");
    let chosen = &proposals[0];
    params["accepted"] = json!([
        {"glyph": chosen["glyph"], "right": chosen["right"], "master": chosen["master"]}
    ]);
    let applied = apply(&api, &params);
    assert_eq!(applied["changed_count"], json!(1));
    let after = font(&session, doc);
    let master: MasterId = serde_json::from_value(chosen["master"].clone()).unwrap();
    let written = after
        .masters
        .iter()
        .find(|entry| entry.id == master)
        .expect("accepted master");
    assert_eq!(
        written.kerning_h.0.len(),
        1,
        "only the accepted pair is written"
    );
    let (pair, value) = written.kerning_h.0.iter().next().unwrap();
    assert_eq!(
        *pair,
        (
            KernSide::Glyph(chosen["glyph"].as_str().unwrap().into()),
            KernSide::Glyph(chosen["right"].as_str().unwrap().into())
        )
    );
    assert_eq!(*value, chosen["after"][0].as_f64().unwrap());
    assert!(after
        .masters
        .iter()
        .filter(|entry| entry.id != master)
        .all(|entry| entry.kerning_h.0.is_empty()));
}
