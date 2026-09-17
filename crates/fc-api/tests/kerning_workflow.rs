use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Font, Glyph, KernSide, Master};
use serde_json::{json, Value};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn setup() -> (Api, Arc<Session>, DocId, String) {
    let mut font = Font::new_empty("Kerning workflow", 1000);
    for name in [".notdef", "A", "V", "a", "v"] {
        let mut glyph = Glyph::new(name);
        if name != ".notdef" {
            glyph.unicodes.push(name.chars().next().unwrap() as u32);
        }
        font.insert_glyph(glyph).unwrap();
    }
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].width = 600.0;
    }
    let master = font.masters[0].id.to_string();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session.clone()), session, doc, master)
}

fn apply(api: &Api, doc: DocId, command: Value) {
    api.call(
        "doc.apply",
        json!({"doc": doc, "label": "Kerning", "command": command}),
    )
    .unwrap();
}

fn set(api: &Api, doc: DocId, master: &str, left: Value, right: Value, value: Value) {
    apply(
        api,
        doc,
        json!({"SetHorizontalKerning": {"master": master, "left": left, "right": right, "value": value}}),
    );
}

fn shape(api: &Api, doc: DocId, master: &str, text: &str, kern: bool) -> f64 {
    let start = Instant::now();
    loop {
        let run = api
            .call(
                "text.shape",
                json!({"doc": doc, "master": master, "text": text, "features": [["kern", kern]]}),
            )
            .unwrap();
        if run["stale"] == false {
            return run["glyphs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|glyph| glyph["x_advance"].as_f64().unwrap())
                .sum();
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "shaping never became current: {run}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn exported_advance(api: &Api, doc: DocId, master: &str, text: &str) -> i32 {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kerning.ttf");
    let report = api
        .call(
            "session.export",
            json!({"doc": doc, "path": path, "options": {"format": "Ttf", "master": master}}),
        )
        .unwrap();
    assert_eq!(report["touched_glyphs"], 0);
    fc_compile::shaping_fingerprint(&std::fs::read(path).unwrap(), text)
        .unwrap()
        .iter()
        .map(|(_, advance, _)| *advance)
        .sum()
}

#[test]
fn pair_edit_preview_export_disable_remove_and_undo_agree() {
    let (api, _, doc, master) = setup();
    assert_eq!(shape(&api, doc, &master, "AV", true), 1200.0);
    set(
        &api,
        doc,
        &master,
        json!({"glyph": "A"}),
        json!({"glyph": "V"}),
        json!(-80),
    );
    assert_eq!(shape(&api, doc, &master, "AV", true), 1120.0);
    assert_eq!(shape(&api, doc, &master, "AV", false), 1200.0);
    assert_eq!(exported_advance(&api, doc, &master, "AV"), 1120);
    set(
        &api,
        doc,
        &master,
        json!({"glyph": "A"}),
        json!({"glyph": "V"}),
        Value::Null,
    );
    assert_eq!(shape(&api, doc, &master, "AV", true), 1200.0);
    api.call("doc.undo", json!({"doc": doc})).unwrap();
    assert_eq!(shape(&api, doc, &master, "AV", true), 1120.0);
}

#[test]
fn overlapping_exceptions_and_explicit_zero_match_exported_shaping() {
    let (api, _, doc, master) = setup();
    for (side, name, members) in [
        ("side1", "public.kern1.A", vec!["A", "a"]),
        ("side2", "public.kern2.V", vec!["V", "v"]),
    ] {
        apply(
            &api,
            doc,
            json!({"SetKerningGroup": {"side": side, "name": name, "members": members}}),
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
        set(
            &api,
            doc,
            &master,
            left.clone(),
            right.clone(),
            json!(value),
        );
    }
    for (text, expected) in [("AV", 1200), ("Av", 1140), ("aV", 1160), ("av", 1100)] {
        assert_eq!(
            exported_advance(&api, doc, &master, text),
            expected,
            "{text}"
        );
    }
    for (left, right, value) in candidates {
        let expected = 1200 + value;
        assert_eq!(shape(&api, doc, &master, "AV", true), f64::from(expected));
        assert_eq!(exported_advance(&api, doc, &master, "AV"), expected);
        set(&api, doc, &master, left, right, Value::Null);
    }
}

#[test]
fn multiple_masters_and_group_edits_survive_fcf_and_ufo_reopening() {
    let (api, session, doc, master) = setup();
    let second = session
        .with_doc_mut(doc, |document| {
            let second = Master::default();
            let id = second.id.to_string();
            document.font.add_master(second);
            for glyph in document.font.glyphs.values_mut() {
                glyph.layers[1].width = 600.0;
            }
            id
        })
        .unwrap();
    apply(
        &api,
        doc,
        json!({"SetKerningGroup": {"side": "side1", "name": "public.kern1.A", "members": ["A", "a"]}}),
    );
    for (master, value) in [(&master, -80), (&second, -120)] {
        set(
            &api,
            doc,
            master,
            json!({"group": "public.kern1.A"}),
            json!({"glyph": "V"}),
            json!(value),
        );
    }
    apply(
        &api,
        doc,
        json!({"RenameKerningGroup": {"side": "side1", "from": "public.kern1.A", "to": "public.kern1.New"}}),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip.fcf");
    api.call("session.save", json!({"doc": doc, "path": path}))
        .unwrap();
    let native = fc_doc::fcf::read(&path).unwrap();
    let original = session
        .with_doc(doc, |document| document.font.clone())
        .unwrap();
    assert_eq!(native, original);
    for (master, expected) in [(&master, 1120), (&second, 1080)] {
        assert_eq!(shape(&api, doc, master, "AV", true), f64::from(expected));
        let path = dir.path().join(format!("{expected}.ufo"));
        api.call(
            "session.export",
            json!({"doc": doc, "path": path, "options": {"format": "Ufo", "master": master}}),
        )
        .unwrap();
        let reopened = fc_formats::ufo::read_ufo(&path).unwrap();
        assert_eq!(reopened.kerning_groups, original.kerning_groups);
        assert_eq!(
            reopened.glyph("A").unwrap().kern_group_side1.as_deref(),
            Some("public.kern1.New")
        );
        let output = fc_compile::export(&reopened, &|_| 0, &Default::default()).unwrap();
        assert_eq!(
            fc_compile::shaping_fingerprint(&output.bytes, "AV")
                .unwrap()
                .iter()
                .map(|(_, advance, _)| *advance)
                .sum::<i32>(),
            expected
        );
    }
}

#[test]
fn unresolved_imported_kerning_is_omitted_with_export_diagnostics() {
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
                (
                    KernSide::Glyph("A".into()),
                    KernSide::Group("public.kern2.missing".into()),
                ),
                -100.0,
            );
            document.font.masters[0].kerning_h.0.insert(
                (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
                f64::NAN,
            );
        })
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unresolved.ttf");
    let report = api
        .call(
            "session.export",
            json!({"doc": doc, "path": path, "options": {"format": "Ttf", "master": master}}),
        )
        .unwrap();
    assert!(report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["code"] == "invalid-horizontal-kerning"));
    assert_eq!(shape(&api, doc, &master, "AV", true), 1200.0);
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.masters[0].kerning_h.0.len(), 3)
        })
        .unwrap();
    let run = api
        .call(
            "text.shape",
            json!({"doc": doc, "master": master, "text": "AV"}),
        )
        .unwrap();
    assert!(
        run["warnings"].as_array().unwrap().iter().any(|warning| {
            warning
                .as_str()
                .is_some_and(|warning| warning.starts_with("invalid-horizontal-kerning:"))
        }),
        "preview did not report excluded records: {run}"
    );
}

#[test]
fn unresolved_source_records_survive_ufo_and_fcf_without_becoming_compiled_rules() {
    let (_, session, doc, _) = setup();
    let font = session
        .with_doc_mut(doc, |document| {
            document.font.kerning_groups.side1.insert(
                "public.kern1.A".into(),
                ["A".into(), "missing".into()].into(),
            );
            let pairs = &mut document.font.masters[0].kerning_h.0;
            pairs.insert(
                (
                    KernSide::Group("public.kern1.A".into()),
                    KernSide::Glyph("V".into()),
                ),
                -100.0,
            );
            pairs.insert(
                (
                    KernSide::Glyph("missing".into()),
                    KernSide::Glyph("V".into()),
                ),
                -80.0,
            );
            pairs.insert(
                (
                    KernSide::Glyph("A".into()),
                    KernSide::Group("public.kern2.missing".into()),
                ),
                -90.0,
            );
            document.font.clone()
        })
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ufo = dir.path().join("unresolved.ufo");
    fc_formats::ufo::write_ufo(&font, &font.masters[0].id, &ufo, &Default::default()).unwrap();
    let reopened = fc_formats::ufo::read_ufo(&ufo).unwrap();
    assert_eq!(reopened.kerning_groups, font.kerning_groups);
    assert_eq!(reopened.masters[0].kerning_h, font.masters[0].kerning_h);
    let native = dir.path().join("unresolved.fcf");
    fc_doc::fcf::write(&native, &reopened).unwrap();
    let native = fc_doc::fcf::read(&native).unwrap();
    assert_eq!(native, reopened);
    let output = fc_compile::export(&native, &|_| 0, &Default::default()).unwrap();
    assert!(output
        .warnings
        .iter()
        .any(|warning| warning.code == "invalid-horizontal-kerning"));
    assert_eq!(
        fc_compile::shaping_fingerprint(&output.bytes, "AV")
            .unwrap()
            .iter()
            .map(|(_, advance, _)| *advance)
            .sum::<i32>(),
        1200
    );
}

#[test]
fn reopening_exported_ttf_preserves_imported_layout_and_new_model_metrics_in_preview() {
    let (api, session, doc, master) = setup();
    set(
        &api,
        doc,
        &master,
        json!({"glyph": "A"}),
        json!({"glyph": "V"}),
        json!(-80),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("authored.ttf");
    api.call(
        "session.export",
        json!({"doc": doc, "path": path, "options": {"format": "Ttf", "master": master}}),
    )
    .unwrap();
    let opened = api.call("session.open", json!({"path": path})).unwrap();
    let reopened: DocId = serde_json::from_value(opened["doc"].clone()).unwrap();
    let (master, layer) = session
        .with_doc(reopened, |document| {
            (
                document.font.masters[0].id.to_string(),
                document.font.glyph("A").unwrap().layers[0].id.to_string(),
            )
        })
        .unwrap();
    assert_eq!(
        api.call("kerning.status", json!({"doc": reopened}))
            .unwrap()["editable"],
        false
    );
    assert_eq!(shape(&api, reopened, &master, "AV", true), 1120.0);
    assert_eq!(shape(&api, reopened, &master, "AV", false), 1200.0);
    apply(
        &api,
        reopened,
        json!({"SetMetrics": {"glyph": "A", "layer": layer, "width": 650, "lsb": null, "rsb": null}}),
    );
    assert_eq!(shape(&api, reopened, &master, "AV", true), 1170.0);
    let native = dir.path().join("imported.fcf");
    api.call("session.save", json!({"doc": reopened, "path": native}))
        .unwrap();
    let native = api.call("session.open", json!({"path": native})).unwrap();
    let native_doc = serde_json::from_value(native["doc"].clone()).unwrap();
    assert_eq!(shape(&api, native_doc, &master, "AV", true), 1170.0);
}
