//! A reviewed merge resolves each conflict explicitly and commits atomically.
use std::sync::Arc;

use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Contour, Font, Glyph, KernSide, Layer};
use kurbo::{Rect, Shape};
use serde_json::{json, Value};

fn master_mut<'a>(font: &'a mut Font, id: &fc_model::MasterId) -> &'a mut fc_model::Master {
    font.masters
        .iter_mut()
        .find(|master| &master.id == id)
        .expect("master")
}

fn glyph(font: &mut Font, name: &str, width: f64, height: f64) {
    let mut glyph = Glyph::new(name);
    glyph.unicodes = vec![u32::from(name.as_bytes()[0])];
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = width;
    layer.contours = Contour::from_bezpath(&Rect::new(20., 0., 400., height).to_path(0.01));
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
}

/// A destination with A/B and a reference with a different B plus a new C.
fn fixture() -> (Api, Arc<Session>, DocId, DocId) {
    let mut target = Font::new_empty("Target", 1000);
    glyph(&mut target, "A", 600., 700.);
    glyph(&mut target, "B", 600., 700.);
    let master = target.default_master().id.clone();
    master_mut(&mut target, &master).kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("B".into())),
        -10.,
    );

    let mut source = Font::new_empty("Reference", 1000);
    glyph(&mut source, "A", 600., 700.);
    glyph(&mut source, "B", 640., 520.);
    glyph(&mut source, "C", 610., 680.);
    source.info.open_type_name_designer = Some("Reference designer".into());
    source
        .kerning_groups
        .side1
        .insert("public.kern1.A".into(), ["A".into()].into_iter().collect());
    source.glyph_mut("A").unwrap().kern_group_side1 = Some("public.kern1.A".into());
    let source_master = source.default_master().id.clone();
    source
        .masters
        .iter_mut()
        .find(|candidate| candidate.id == source_master)
        .unwrap()
        .kerning_h
        .0
        .insert(
            (
                KernSide::Group("public.kern1.A".into()),
                KernSide::Glyph("C".into()),
            ),
            -55.,
        );

    let session = Arc::new(Session::new());
    let doc = session.open_font(target, None);
    let source_doc = session.open_font(source, None);
    (Api::new(session.clone()), session, doc, source_doc)
}

fn width(session: &Session, doc: DocId, name: &str) -> Option<f64> {
    session
        .with_doc(doc, |document| {
            document.font.glyph(name).map(|glyph| glyph.layers[0].width)
        })
        .unwrap()
}

fn request(doc: DocId, source_doc: DocId) -> Value {
    json!({
        "doc": doc,
        "source_doc": source_doc,
        "glyphs": [
            { "name": "A", "choice": "target" },
            { "name": "B", "choice": "source" },
            { "name": "C", "choice": "source" },
        ],
        "adopt": { "metadata": true, "master_metrics": false, "kerning": true },
        "missing_bases": "reject",
        "dry_run": true,
    })
}

#[test]
fn a_reviewed_merge_applies_choices_and_adoptions_as_one_undo_entry() {
    let (api, session, doc, source_doc) = fixture();
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();

    let preview = api.call("font.merge", request(doc, source_doc)).unwrap();
    assert!(preview["change"].is_null(), "a preview never mutates");
    assert_eq!(preview["replaced"], json!(["B", "C"]));
    assert_eq!(preview["kept"], json!(["A"]));
    assert_eq!(preview["adopted"], json!(["metadata", "kerning"]));
    assert_eq!(preview["label"], "Merge fonts");
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);

    let mut apply = request(doc, source_doc);
    apply["dry_run"] = json!(false);
    apply["expected_generation"] = preview["generation"].clone();
    apply["expected_source_generation"] = preview["source_generation"].clone();
    let applied = api.call("font.merge", apply.clone()).unwrap();
    assert!(applied["change"].is_object());

    // The chosen reference glyphs win, the kept glyph is untouched, and the
    // adopted font-level resources are present.
    assert_eq!(width(&session, doc, "A"), Some(600.));
    assert_eq!(width(&session, doc, "B"), Some(640.));
    assert_eq!(width(&session, doc, "C"), Some(610.));
    let font = session.with_doc(doc, |d| d.font.clone()).unwrap();
    assert_eq!(
        font.info.open_type_name_designer.as_deref(),
        Some("Reference designer")
    );
    assert_eq!(font.info.family_name, "Reference");
    assert!(font.kerning_groups.side1.contains_key("public.kern1.A"));
    let master = font.default_master().id.clone();
    let kerning = &font.master(&master).unwrap().kerning_h;
    assert_eq!(
        kerning.0.get(&(
            KernSide::Group("public.kern1.A".into()),
            KernSide::Glyph("C".into())
        )),
        Some(&-55.)
    );
    assert!(
        !kerning
            .0
            .contains_key(&(KernSide::Glyph("A".into()), KernSide::Glyph("B".into()))),
        "adopting kerning replaces the destination table"
    );

    // One undo entry restores the document exactly.
    let summary = api.call("doc.summary", json!({ "doc": doc })).unwrap();
    assert_eq!(summary["undo_label"], "Merge fonts");
    api.call("doc.undo", json!({ "doc": doc })).unwrap();
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
    api.call("doc.redo", json!({ "doc": doc })).unwrap();
    assert_eq!(width(&session, doc, "B"), Some(640.));

    // The reference document is never modified.
    assert_eq!(width(&session, source_doc, "B"), Some(640.));
    assert_eq!(
        api.call("doc.summary", json!({ "doc": source_doc }))
            .unwrap()["undo_label"],
        Value::Null
    );
}

#[test]
fn a_merged_document_saves_reopens_and_exports_with_the_merged_glyphs() {
    let (api, _session, doc, source_doc) = fixture();
    let dir = tempfile::tempdir().unwrap();
    api.configure_config_dir(dir.path().join("config")).unwrap();
    let preview = api.call("font.merge", request(doc, source_doc)).unwrap();
    let mut apply = request(doc, source_doc);
    apply["dry_run"] = json!(false);
    apply["expected_generation"] = preview["generation"].clone();
    apply["expected_source_generation"] = preview["source_generation"].clone();
    api.call("font.merge", apply).unwrap();

    let native = dir.path().join("merged.fcf");
    api.call("session.save", json!({ "doc": doc, "path": native }))
        .expect("save the merged document");
    let reopened = api
        .call("session.open", json!({ "path": native }))
        .expect("reopen the merged document");
    let glyphs = api
        .call(
            "glyphs.list",
            json!({ "doc": reopened["doc"], "offset": 0, "limit": 10, "include_paths": false }),
        )
        .unwrap();
    let names: Vec<&str> = glyphs["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["A", "B", "C"]);
    assert_eq!(reopened["family_name"], "Reference");

    let exported = dir.path().join("merged.ttf");
    api.call(
        "session.export",
        json!({"doc": doc, "path": exported, "options": {"format": "Ttf"}}),
    )
    .expect("export the merged document");
    let binary = api
        .call("font.open_reference", json!({ "path": exported }))
        .expect("reopen the exported binary as a reference");
    let comparison = api
        .call(
            "font.compare",
            json!({"doc": doc, "source_doc": binary["doc"], "differing_only": false, "limit": 50}),
        )
        .unwrap();
    // The compiled font adds `.notdef`; every merged glyph round-trips unchanged.
    assert_eq!(comparison["counts"]["missing_source"], 0);
    assert_eq!(comparison["counts"]["missing_target"], 1);
    assert_eq!(comparison["counts"]["metrics"], 0);
    // Compilation rounds coordinates, so exact outline equality is not asserted
    // here: unicodes, metrics and the glyph inventory must round-trip.
    assert_eq!(comparison["counts"]["unicodes"], 0);
    for name in ["A", "B", "C"] {
        let row = comparison["glyphs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"] == name)
            .unwrap_or_else(|| panic!("{name} is missing from the exported font"));
        assert_eq!(row["metrics_changed"], false, "{name}");
        assert_eq!(row["unicode_changed"], false, "{name}");
    }
}

#[test]
fn a_merge_rejects_stale_revisions_empty_decisions_and_one_document() {
    let (api, session, doc, source_doc) = fixture();
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let preview = api.call("font.merge", request(doc, source_doc)).unwrap();

    // Change the destination after the review.
    api.call("glyphs.add", json!({ "doc": doc, "recipe": "Z" }))
        .unwrap();
    let mut stale = request(doc, source_doc);
    stale["dry_run"] = json!(false);
    stale["expected_generation"] = preview["generation"].clone();
    let error = api
        .call("font.merge", stale)
        .expect_err("a stale destination revision must reject the merge");
    assert!(error.message.contains("stale"));

    for params in [
        json!({"doc": doc, "source_doc": source_doc, "glyphs": [], "missing_bases": "reject"}),
        json!({"doc": doc, "source_doc": doc, "glyphs": [{"name":"A","choice":"source"}], "missing_bases": "reject"}),
        json!({"doc": doc, "source_doc": source_doc, "glyphs": [{"name":"A","choice":"source"},{"name":"A","choice":"target"}], "missing_bases": "reject"}),
        json!({"doc": doc, "source_doc": source_doc, "glyphs": [{"name":"absent","choice":"source"}], "missing_bases": "reject"}),
    ] {
        assert!(api.call("font.merge", params).is_err());
    }
    api.call("doc.undo", json!({ "doc": doc })).unwrap();
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
}
