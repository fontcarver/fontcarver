//! Read-only comparisons must describe actual drawing changes across documents.
use std::sync::Arc;

use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Component, Contour, Font, Glyph, Layer, MasterId};
use kurbo::{Affine, Rect, Shape};
use serde_json::{json, Value};

fn fixture_font() -> Font {
    let mut font = Font::new_empty("Compare fixture", 1000);
    for name in ["A", "B", "C", "D", "E"] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = vec![u32::from(name.as_bytes()[0])];
        let mut layer = Layer::new_master(font.default_master().id.clone());
        layer.width = 600.;
        layer.contours = Contour::from_bezpath(&Rect::new(20., 0., 400., 700.).to_path(0.01));
        if name == "B" {
            layer.contours.clear();
            layer.components.push(Component {
                base: "A".into(),
                transform: Affine::translate((100., 0.)),
                ..Default::default()
            });
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font
}

fn pair(target: Font, source: Font) -> (Api, Arc<Session>, DocId, DocId) {
    let session = Arc::new(Session::new());
    let doc = session.open_font(target, None);
    let source_doc = session.open_font(source, None);
    (Api::new(session.clone()), session, doc, source_doc)
}

fn row<'a>(result: &'a Value, name: &str) -> &'a Value {
    result["glyphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap()
}

#[test]
fn compare_reports_geometry_metrics_unicode_missing_and_full_counts_without_mutation() {
    let target = fixture_font();
    let mut source = target.clone();
    source.glyph_mut("A").unwrap().layers[0].contours[0].points[1].x += 25.;
    source.glyph_mut("C").unwrap().layers[0].width = 700.;
    source.glyph_mut("D").unwrap().unicodes = vec![0x1040];
    source.glyphs.shift_remove("E");
    let mut extra = Glyph::new("Z");
    extra
        .layers
        .push(Layer::new_master(source.default_master().id.clone()));
    source.insert_glyph(extra).unwrap();
    let (api, session, doc, source_doc) = pair(target.clone(), source.clone());
    let result = api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .unwrap();
    assert_eq!(result["total"], 6);
    assert_eq!(result["counts"]["outlines"], 2);
    assert_eq!(result["counts"]["metrics"], 1);
    assert_eq!(result["counts"]["unicodes"], 1);
    assert_eq!(result["counts"]["missing_source"], 1);
    assert_eq!(result["counts"]["missing_target"], 1);
    assert_eq!(row(&result, "B")["outline_changed"], true);
    assert_eq!(row(&result, "C")["metrics_changed"], true);
    assert_eq!(row(&result, "D")["unicode_changed"], true);
    assert_eq!(row(&result, "E")["status"], "missing_source");
    assert_eq!(row(&result, "Z")["status"], "missing_target");
    assert_eq!(result["compatibility"]["glyph_set"], false);
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), target);
    assert_eq!(
        session.with_doc(source_doc, |d| d.font.clone()).unwrap(),
        source
    );
    assert_eq!(session.with_doc(doc, |d| d.generation).unwrap(), 0);
}

#[test]
fn ids_unicode_order_and_pagination_do_not_create_false_differences() {
    let mut target = fixture_font();
    let mut source = fixture_font();
    target.glyph_mut("A").unwrap().unicodes = vec![65, 1000];
    source.glyph_mut("A").unwrap().unicodes = vec![1000, 65];
    source.glyph_mut("E").unwrap().layers[0].width += 10.;
    let (api, _, doc, source_doc) = pair(target, source);
    let result = api
        .call(
            "font.compare",
            json!({"doc":doc,"source_doc":source_doc,"limit":1,"offset":1}),
        )
        .unwrap();
    assert_eq!(result["total"], 5);
    assert_eq!(result["glyphs"][0]["name"], "B");
    assert_eq!(result["counts"]["equal"], 4);
    assert_eq!(result["compatibility"]["masters"], true);
    let changed = api
        .call(
            "font.compare",
            json!({"doc":doc,"source_doc":source_doc,"differing_only":true}),
        )
        .unwrap();
    assert_eq!(changed["total"], 1);
    assert_eq!(changed["glyphs"][0]["name"], "E");
    assert_eq!(changed["counts"]["equal"], 4);
}

#[test]
fn overlay_resolves_components_and_family_compatibility_respects_requested_masters() {
    let target = fixture_font();
    let mut source = fixture_font();
    source.info.family_name = "Another family".into();
    source.info.units_per_em = 2000;
    let mut master = source.default_master().clone();
    master.id = MasterId::new();
    master.name = "Bold".into();
    master.metrics.ascender += 25.;
    for glyph in source.glyphs.values_mut() {
        let mut layer = glyph.layers[0].clone();
        layer.id = Default::default();
        layer.kind = fc_model::LayerKind::Master(master.id.clone());
        layer.width = 800.;
        glyph.layers.push(layer);
    }
    let source_master = master.id.clone();
    source.masters.push(master);
    let (api, _, doc, source_doc) = pair(target, source);
    let params = json!({"doc":doc,"source_doc":source_doc,"source_master":source_master});
    let result = api.call("font.compare", params.clone()).unwrap();
    assert_eq!(result["compatibility"]["family_name"], false);
    assert_eq!(result["compatibility"]["units_per_em"], false);
    assert_eq!(result["compatibility"]["masters"], false);
    assert_eq!(result["compatibility"]["master_metrics"], false);
    assert_eq!(result["counts"]["metrics"], 5);
    let mut overlay_params = params;
    overlay_params["name"] = json!("B");
    let overlay = api.call("font.compare_glyph", overlay_params).unwrap();
    let path = kurbo::BezPath::from_svg(overlay["target"]["path"].as_str().unwrap()).unwrap();
    assert_eq!(path.bounding_box(), Rect::new(120., 0., 500., 700.));
    assert_eq!(overlay["source"]["width"], 800.);
    assert_eq!(overlay["source"]["upem"], 2000);
    assert_eq!(overlay["source"]["bounds"], json!([120., 0., 500., 700.]));
}

#[test]
fn invalid_requests_and_damaged_components_fail_without_mutation() {
    let (api, session, doc, source_doc) = pair(fixture_font(), fixture_font());
    for patch in [
        json!({"limit":513}),
        json!({"limit":0}),
        json!({"master":"missing"}),
        json!({"source_master":"missing"}),
    ] {
        let mut params = json!({"doc":doc,"source_doc":source_doc});
        params
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert!(api.call("font.compare", params).is_err());
    }
    assert!(api
        .call(
            "font.compare_glyph",
            json!({"doc":doc,"source_doc":source_doc,"name":"missing"})
        )
        .is_err());
    session
        .with_doc_mut(source_doc, |d| {
            d.font.glyph_mut("B").unwrap().layers[0].components[0].base = "B".into();
        })
        .unwrap();
    let result = api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .unwrap();
    assert!(!row(&result, "B")["issues"].as_array().unwrap().is_empty());
    assert_eq!(row(&result, "B")["status"], "different");
    assert!(api
        .call(
            "font.compare_glyph",
            json!({"doc":doc,"source_doc":source_doc,"name":"B"})
        )
        .is_err());
    assert_eq!(session.with_doc(doc, |d| d.generation).unwrap(), 0);
}

#[test]
fn topology_missing_master_layers_and_variation_sequences_are_reported() {
    let target = fixture_font();
    let mut source = fixture_font();
    source.glyph_mut("A").unwrap().layers[0].contours[0].closed = false;
    source.glyph_mut("C").unwrap().layers.clear();
    source.glyph_mut("D").unwrap().variation_selectors = vec![(0x44, 0xfe00)];
    let (api, _, doc, source_doc) = pair(target, source);
    let result = api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .unwrap();
    assert_eq!(row(&result, "A")["compatible"], false);
    assert_eq!(row(&result, "C")["compatible"], false);
    assert_eq!(row(&result, "C")["outline_changed"], true);
    assert_eq!(row(&result, "D")["unicode_changed"], true);
    let overlay = api
        .call(
            "font.compare_glyph",
            json!({"doc":doc,"source_doc":source_doc,"name":"C"}),
        )
        .unwrap();
    assert!(overlay["source"].is_null());
    assert!(overlay["target"].is_object());
}

#[test]
fn axes_compare_design_coordinates_and_ignore_display_labels() {
    let mut target = fixture_font();
    let axis = fc_model::Axis {
        tag: "wght".parse().unwrap(),
        name: "Weight".into(),
        min: 100.,
        default: 400.,
        max: 900.,
        map: vec![(100., 0.), (400., 0.5), (900., 1.)],
        ..Default::default()
    };
    target.axes.push(axis);
    let mut source = target.clone();
    source.axes[0].name = "Display label".into();
    let (api, session, doc, source_doc) = pair(target, source);
    let params = json!({"doc":doc,"source_doc":source_doc});
    assert_eq!(
        api.call("font.compare", params.clone()).unwrap()["compatibility"]["axes"],
        true
    );
    session
        .with_doc_mut(source_doc, |d| d.font.axes[0].map[1].1 = 0.6)
        .unwrap();
    assert_eq!(
        api.call("font.compare", params).unwrap()["compatibility"]["axes"],
        false
    );
}

#[test]
fn compiled_binary_reference_opens_and_compares_without_editing_either_document() {
    let (api, session, doc, _) = pair(fixture_font(), fixture_font());
    let dir = tempfile::tempdir().unwrap();
    api.configure_config_dir(dir.path().join("config")).unwrap();
    let path = dir.path().join("reference.ttf");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Ttf"}}),
    )
    .unwrap();
    let opened = api.call("session.open", json!({"path":path})).unwrap();
    let source_doc: DocId = serde_json::from_value(opened["doc"].clone()).unwrap();
    let target_before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let source_before = session.with_doc(source_doc, |d| d.font.clone()).unwrap();
    let result = api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .unwrap();
    assert_eq!(row(&result, "A")["metrics_changed"], false);
    assert_eq!(row(&result, "A")["unicode_changed"], false);
    let overlay = api
        .call(
            "font.compare_glyph",
            json!({"doc":doc,"source_doc":source_doc,"name":"B"}),
        )
        .unwrap();
    let path = kurbo::BezPath::from_svg(overlay["source"]["path"].as_str().unwrap()).unwrap();
    assert_eq!(path.bounding_box(), Rect::new(120., 0., 500., 700.));
    assert_eq!(
        session.with_doc(doc, |d| d.font.clone()).unwrap(),
        target_before
    );
    assert_eq!(
        session.with_doc(source_doc, |d| d.font.clone()).unwrap(),
        source_before
    );
}

#[test]
fn oversized_source_geometry_is_rejected_before_comparison() {
    let target = fixture_font();
    let mut source = fixture_font();
    let contour = &mut source.glyph_mut("A").unwrap().layers[0].contours[0];
    contour.points = vec![contour.points[0].clone(); 100_001];
    let (api, _, doc, source_doc) = pair(target, source);
    for (method, params) in [
        ("font.compare", json!({"doc":doc,"source_doc":source_doc})),
        (
            "font.compare_glyph",
            json!({"doc":doc,"source_doc":source_doc,"name":"A"}),
        ),
    ] {
        assert!(api
            .call(method, params)
            .unwrap_err()
            .message
            .contains("100000"));
    }
}

#[test]
fn empty_component_expansion_is_bounded_before_evaluating_the_graph() {
    let mut source = fixture_font();
    let master = source.default_master().id.clone();
    let mut previous = "Empty".to_owned();
    let mut base = Glyph::new(previous.as_str());
    base.layers.push(Layer::new_master(master.clone()));
    source.insert_glyph(base).unwrap();
    for index in 0..35 {
        let name = format!("node{index}");
        let mut glyph = Glyph::new(name.as_str());
        let mut layer = Layer::new_master(master.clone());
        layer.components = vec![
            Component {
                base: previous.clone().into(),
                ..Default::default()
            };
            2
        ];
        glyph.layers.push(layer);
        source.insert_glyph(glyph).unwrap();
        previous = name;
    }
    // The first sorted glyph reaches the entire graph, so comparison preflight
    // must reject before any exponentially repeated empty-source evaluation.
    source.glyph_mut("A").unwrap().layers[0]
        .components
        .push(Component {
            base: previous.clone().into(),
            ..Default::default()
        });
    let (api, _, doc, source_doc) = pair(fixture_font(), source);
    let started = std::time::Instant::now();
    for (method, params) in [
        ("font.compare", json!({"doc":doc,"source_doc":source_doc})),
        (
            "font.compare_glyph",
            json!({"doc":doc,"source_doc":source_doc,"name":previous}),
        ),
    ] {
        assert!(api
            .call(method, params)
            .unwrap_err()
            .message
            .contains("work budget"));
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn reviewed_overlays_reject_either_changed_document_generation() {
    let (api, session, doc, source_doc) = pair(fixture_font(), fixture_font());
    let comparison = api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .unwrap();
    let mut params = json!({"doc":doc,"source_doc":source_doc,"name":"A",
        "expected_generation":comparison["generation"],
        "expected_source_generation":comparison["source_generation"]});
    assert!(api.call("font.compare_glyph", params.clone()).is_ok());
    for changed in [doc, source_doc] {
        params["expected_generation"] = json!(session.with_doc(doc, |d| d.generation).unwrap());
        params["expected_source_generation"] =
            json!(session.with_doc(source_doc, |d| d.generation).unwrap());
        session
            .with_doc_mut(changed, |document| {
                let layer = document.font.glyph("A").unwrap().layers[0].id.clone();
                document
                    .apply(
                        fc_doc::Command::SetWidth {
                            glyph: "A".into(),
                            layer,
                            width: 700.,
                        },
                        "Metrics",
                    )
                    .unwrap();
            })
            .unwrap();
        assert!(api
            .call("font.compare_glyph", params.clone())
            .unwrap_err()
            .message
            .contains("stale"));
        api.call("doc.undo", json!({"doc":changed})).unwrap();
    }
}

#[test]
fn metadata_kerning_and_layout_differences_are_reported_with_flags() {
    let target = fixture_font();
    let mut source = fixture_font();
    source.info.open_type_name_designer = Some("Reference designer".into());
    source.info.version_major = 7;
    source
        .kerning_groups
        .side1
        .insert("public.kern1.A".into(), ["A".into()].into_iter().collect());
    source.glyph_mut("A").unwrap().kern_group_side1 = Some("public.kern1.A".into());
    let master = source.default_master().id.clone();
    source
        .masters
        .iter_mut()
        .find(|candidate| candidate.id == master)
        .unwrap()
        .kerning_h
        .0
        .insert(
            (
                fc_model::KernSide::Group("public.kern1.A".into()),
                fc_model::KernSide::Glyph("B".into()),
            ),
            -42.,
        );
    source.features.features.push(fc_model::FeatureBlock {
        tag: "kern".into(),
        code: "feature kern { pos A B -42; } kern;".into(),
        ..Default::default()
    });
    let (api, _, doc, source_doc) = pair(target, source);

    let result = api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .unwrap();

    let compatibility = &result["compatibility"];
    assert_eq!(compatibility["metadata"], false);
    assert_eq!(compatibility["kerning"], false);
    assert_eq!(compatibility["layout"], false);
    assert_eq!(compatibility["family_name"], true);
    let fields: Vec<&str> = result["metadata"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["field"].as_str().unwrap())
        .collect();
    assert!(fields.contains(&"open_type_name_designer") && fields.contains(&"version_major"));
    assert!(!fields.contains(&"family_name") && !fields.contains(&"units_per_em"));
    assert_eq!(result["kerning"]["pairs_only_source"], 1);
    assert_eq!(result["kerning"]["pairs_only_target"], 0);
    assert_eq!(
        result["kerning"]["groups_only_source"],
        json!(["side1:public.kern1.A"])
    );
    let example = &result["kerning"]["examples"][0];
    assert_eq!(example["left"], "@public.kern1.A");
    assert_eq!(example["right"], "B");
    assert_eq!(example["source"], -42.0);
    assert!(example["target"].is_null());
    assert_eq!(result["layout"]["only_source"], json!(["feature:kern"]));
    assert_eq!(result["layout"]["feature_source_equal"], false);
}

#[test]
fn component_differences_are_reported_separately_from_the_evaluated_outline() {
    let target = fixture_font();
    let mut source = fixture_font();
    // Reuse the same drawing through a different reference: only the component
    // record differs, so the evaluated path stays identical.
    let layer = &mut source.glyph_mut("B").unwrap().layers[0];
    layer.components[0].anchor = Some("top".into());
    let (api, _, doc, source_doc) = pair(target, source);

    let result = api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .unwrap();

    let row = row(&result, "B");
    assert_eq!(row["components_changed"], true);
    assert_eq!(row["outline_changed"], false);
    assert_eq!(row["status"], "different");
    assert_eq!(result["counts"]["components"], 1);
    assert_eq!(result["counts"]["outlines"], 0);
}

#[test]
fn family_comparison_covers_every_master_and_marks_mismatched_members() {
    let mut target = fixture_font();
    let extra = fc_model::Master {
        name: "Bold".into(),
        metrics: target.default_master().metrics.clone(),
        ..Default::default()
    };
    let extra_id = extra.id.clone();
    target.masters.push(extra);
    for name in ["A", "B", "C", "D", "E"] {
        let mut layer = Layer::new_master(extra_id.clone());
        layer.width = 600.;
        layer.contours = Contour::from_bezpath(&Rect::new(20., 0., 400., 700.).to_path(0.01));
        if name == "B" {
            layer.contours.clear();
            layer.components.push(Component {
                base: "A".into(),
                transform: Affine::translate((100., 0.)),
                ..Default::default()
            });
        }
        target.glyph_mut(name).unwrap().layers.push(layer);
    }
    // One master really differs: the Bold "C" is wider.
    let bold_layer = target
        .glyph_mut("C")
        .unwrap()
        .layers
        .iter_mut()
        .find(|layer| matches!(&layer.kind, fc_model::LayerKind::Master(id) if *id == extra_id))
        .unwrap();
    bold_layer.width = 800.;
    let (api, _, doc, source_doc) = pair(target, fixture_font());

    let result = api
        .call(
            "font.compare_family",
            json!({"doc":doc,"docs":[source_doc],"all_masters":true}),
        )
        .unwrap();

    let members = result["members"].as_array().unwrap();
    assert_eq!(
        members.len(),
        3,
        "reference, Bold master and the other font"
    );
    assert_eq!(members[0]["reference"], true);
    assert_eq!(members[0]["counts"]["equal"], 0);
    let bold = members
        .iter()
        .find(|member| member["master_name"] == "Bold")
        .unwrap();
    assert_eq!(bold["counts"]["metrics"], 1);
    assert_eq!(bold["examples"], json!(["C"]));
    let other = members
        .iter()
        .find(|member| member["doc"] == json!(source_doc) && member["reference"] == false)
        .unwrap();
    assert_eq!(other["compatibility"]["masters"], false);
    assert_eq!(result["compatible"], false);

    // A document compared with itself, default master only, is compatible.
    let same = api
        .call(
            "font.compare_family",
            json!({"doc":source_doc,"all_masters":false}),
        )
        .unwrap();
    assert_eq!(same["compatible"], true);
    assert_eq!(same["members"].as_array().unwrap().len(), 1);

    // Repeating a member, including the target itself, compares it once.
    let repeated = api
        .call(
            "font.compare_family",
            json!({"doc":doc,"docs":[source_doc,source_doc,doc],"all_masters":true}),
        )
        .unwrap();
    assert_eq!(
        repeated["members"].as_array().unwrap().len(),
        members.len(),
        "duplicate documents must not add duplicate members"
    );
}

#[test]
fn a_reference_binary_opens_read_only_and_the_kernel_rejects_every_edit() {
    let (api, session, doc, _) = pair(fixture_font(), fixture_font());
    let dir = tempfile::tempdir().unwrap();
    api.configure_config_dir(dir.path().join("config")).unwrap();
    let path = dir.path().join("reference.ttf");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Ttf"}}),
    )
    .unwrap();

    let reference = api
        .call("font.open_reference", json!({"path":path}))
        .unwrap();

    assert_eq!(reference["read_only"], true);
    assert_eq!(reference["path"], path.to_string_lossy().as_ref());
    assert!(!reference["masters"].as_array().unwrap().is_empty());
    let source_doc: DocId = serde_json::from_value(reference["doc"].clone()).unwrap();
    let before = session.with_doc(source_doc, |d| d.font.clone()).unwrap();
    let layer = session
        .with_doc(source_doc, |d| {
            d.font.glyph("A").unwrap().layers[0].id.clone()
        })
        .unwrap();
    for (method, params) in [
        (
            "doc.apply",
            json!({"doc":source_doc,"command":{"SetWidth":{"glyph":"A","layer":layer,"width":10.0}},"label":"Edit"}),
        ),
        ("doc.undo", json!({"doc":source_doc})),
        ("doc.redo", json!({"doc":source_doc})),
        (
            "session.save",
            json!({"doc":source_doc,"path":dir.path().join("copy.fcf")}),
        ),
    ] {
        let error = api
            .call(method, params)
            .expect_err("a read-only reference must reject every mutation");
        assert!(
            error.message.contains("read-only reference"),
            "{method}: {}",
            error.message
        );
    }
    assert_eq!(
        session.with_doc(source_doc, |d| d.font.clone()).unwrap(),
        before
    );
    // Comparison itself still works against the read-only document. The compiled
    // reference also carries `.notdef`, which the editable document does not.
    let result = api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .unwrap();
    assert_eq!(result["counts"]["missing_source"], 0);
    assert_eq!(row(&result, ".notdef")["status"], "missing_target");
    assert_eq!(row(&result, "A")["metrics_changed"], false);
}

#[test]
fn a_read_only_reference_refuses_prepared_feature_commits() {
    // `features.apply`, `layout.apply_conversion` and `layout.revert_original`
    // all commit through `Document::commit_prepared_features`, which bypasses
    // `apply`. This drives the reachable one end to end; the kernel guard
    // itself is covered by `fc-doc`'s `script_transaction` suite.
    let (api, session, doc, _) = pair(fixture_font(), fixture_font());
    let dir = tempfile::tempdir().unwrap();
    api.configure_config_dir(dir.path().join("config")).unwrap();
    let path = dir.path().join("reference.ttf");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Ttf"}}),
    )
    .unwrap();
    let reference = api
        .call("font.open_reference", json!({ "path": path }))
        .unwrap();
    assert_eq!(reference["read_only"], true);
    let source_doc: DocId = serde_json::from_value(reference["doc"].clone()).unwrap();
    let before = session.with_doc(source_doc, |d| d.font.clone()).unwrap();

    // Read-only inspection of the same subsystem still works.
    api.call("features.get", json!({ "doc": source_doc }))
        .expect("a reference can still be inspected");
    api.call(
        "features.validate",
        json!({
            "doc": source_doc,
            "expected_generation": 0,
            "expected_compile_revision": 0,
            "expected_draft_revision": 0,
        }),
    )
    .expect("validation does not mutate");

    let error = api
        .call(
            "features.apply",
            json!({
                "doc": source_doc,
                "expected_generation": 0,
                "expected_compile_revision": 0,
                "expected_draft_revision": 0,
            }),
        )
        .expect_err("a reference must refuse a prepared feature commit");
    assert!(
        error.message.contains("read-only reference"),
        "{}",
        error.message
    );
    assert_eq!(
        session.with_doc(source_doc, |d| d.font.clone()).unwrap(),
        before
    );
    assert_eq!(
        api.call("doc.summary", json!({ "doc": source_doc }))
            .unwrap()["undo_label"],
        Value::Null
    );
    // The editable document is unaffected by the refusal.
    assert!(api
        .call("font.compare", json!({"doc":doc,"source_doc":source_doc}))
        .is_ok());
}
