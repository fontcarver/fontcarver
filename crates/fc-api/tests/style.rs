//! Batch styles must preview the same original sources they eventually mutate.
use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Anchor, Component, Contour, Font, Glyph, Layer, LayerKind, MasterId};
use kurbo::{Affine, Rect, Shape};
use serde_json::{json, Value};
use std::sync::Arc;

fn fixture() -> (Api, Arc<Session>, DocId, Value) {
    let mut font = Font::new_empty("Style fixture", 1000);
    let mut second = font.masters[0].clone();
    second.id = MasterId::new();
    second.name = "Second".into();
    font.masters.push(second);
    for (name, composite) in [("A", false), ("B", true)] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = vec![if composite { 66 } else { 65 }];
        for master in &font.masters {
            let mut layer = Layer::new_master(master.id.clone());
            layer.width = 600.;
            layer.anchors = vec![Anchor {
                name: "top".into(),
                x: 200.,
                y: 500.,
            }];
            if composite {
                layer.components.push(Component {
                    base: "A".into(),
                    transform: Affine::translate((100., 0.)),
                    ..Default::default()
                });
            } else {
                layer.contours =
                    Contour::from_bezpath(&Rect::new(50., 0., 350., 500.).to_path(0.01));
            }
            glyph.layers.push(layer);
        }
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let glyphs = session
        .with_doc(doc, |doc| {
            [doc.glyph_id("A").unwrap(), doc.glyph_id("B").unwrap()]
        })
        .unwrap();
    let params = json!({"doc":doc,"glyphs":glyphs,"master":null,"options":{"scale":[0.5,0.7],"translate":[10.,200.],"oblique":0.,"weight":[0.,0.]},"scale_advance":true,"center":false,"backup":true,"dry_run":true,"expected_generation":null,"label":"Small caps"});
    (Api::new(session.clone()), session, doc, params)
}
fn font(session: &Session, doc: DocId) -> Font {
    session.with_doc(doc, |doc| doc.font.clone()).unwrap()
}
fn apply(api: &Api, params: &Value) -> Value {
    let preview = api.call("style.batch", params.clone()).unwrap();
    let mut edit = params.clone();
    edit["dry_run"] = json!(false);
    edit["expected_generation"] = preview["generation"].clone();
    api.call("style.batch", edit).unwrap()
}

#[test]
fn frozen_sources_backups_anchors_and_advances_survive_undo_native_and_export() {
    let (api, session, doc, params) = fixture();
    let before = font(&session, doc);
    let preview = api.call("style.batch", params.clone()).unwrap();
    assert_eq!(preview["layer_count"], 4);
    assert_eq!(preview["decomposed_layers"], 2);
    assert_eq!(font(&session, doc), before);
    apply(&api, &params);
    let after = font(&session, doc);
    for (name, x0) in [("A", 35.), ("B", 85.)] {
        let glyph = after.glyph(name).unwrap();
        assert_eq!(glyph.layers.len(), 4);
        for master in &after.masters {
            let layer = glyph.master_layer(&master.id).unwrap();
            assert_eq!(layer.width, 300.);
            assert_eq!(layer.anchors[0].x, 110.);
            assert_eq!(layer.anchors[0].y, 550.);
            assert!(layer.components.is_empty());
            assert_eq!(
                fc_model::flatten_layer(&after, name, &master.id)
                    .unwrap()
                    .bounding_box(),
                Rect::new(x0, 200., x0 + 150., 550.)
            );
            let backup = glyph
                .layers
                .iter()
                .find(|l| matches!(&l.kind, LayerKind::Backup {of,..} if *of == master.id))
                .unwrap();
            assert!(backup.components.is_empty());
            assert_eq!(
                fc_model::query::flatten_layer_by_id(&after, name, &backup.id).unwrap(),
                fc_model::flatten_layer(&before, name, &master.id).unwrap()
            );
        }
    }
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(font(&session, doc), before);
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    assert_eq!(font(&session, doc), after);
    let dir = tempfile::tempdir().unwrap();
    for ext in ["fcf", "fcf.gz"] {
        let path = dir.path().join(format!("styled.{ext}"));
        api.call("session.save", json!({"doc":doc,"path":path}))
            .unwrap();
        assert_eq!(fc_doc::fcf::read(&path).unwrap(), after);
    }
    for format in ["Ttf", "Otf", "Woff2"] {
        let path = dir.path().join(format!("styled.{format}"));
        api.call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":format}}),
        )
        .unwrap();
        let compiled = fc_sfnt::read_path(&path, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        for (name, x0) in [("A", 35.), ("B", 85.)] {
            let layer = compiled
                .glyph(name)
                .unwrap()
                .master_layer(&compiled.default_master().id)
                .unwrap();
            assert_eq!(layer.width, 300.);
            assert_eq!(
                fc_model::flatten_layer(&compiled, name, &compiled.default_master().id)
                    .unwrap()
                    .bounding_box(),
                Rect::new(x0, 200., x0 + 150., 550.)
            );
        }
    }
}

#[test]
fn cjk_centering_preserves_cell_and_bad_or_stale_batches_are_atomic() {
    let (api, session, doc, mut params) = fixture();
    params["center"] = json!(true);
    params["scale_advance"] = json!(false);
    params["options"]["weight"] = json!([10., 20.]);
    let stale = api.call("style.batch", params.clone()).unwrap();
    apply(&api, &params);
    let after = font(&session, doc);
    for name in ["A", "B"] {
        let layer = after
            .glyph(name)
            .unwrap()
            .master_layer(&after.default_master().id)
            .unwrap();
        let bounds = fc_model::flatten_layer(&after, name, &after.default_master().id)
            .unwrap()
            .bounding_box();
        assert_eq!(layer.width, 600.);
        assert!((bounds.x0 - (600. - bounds.x1)).abs() < 0.01);
        assert!((bounds.width() - 170.).abs() < 0.01, "{bounds:?}");
    }
    params["dry_run"] = json!(false);
    params["expected_generation"] = stale["generation"].clone();
    assert!(api
        .call("style.batch", params.clone())
        .unwrap_err()
        .message
        .contains("stale"));
    params["dry_run"] = json!(true);
    for (field, value) in [
        ("glyphs", json!([])),
        ("glyphs", json!([u32::MAX])),
        ("master", json!("missing")),
    ] {
        let mut bad = params.clone();
        bad[field] = value;
        assert!(api.call("style.batch", bad).is_err());
    }
    let mut bad = params.clone();
    bad["options"]["weight"] = json!([20., -20.]);
    assert!(api.call("style.batch", bad).is_err());
    assert_eq!(font(&session, doc), after);
    session
        .with_doc_mut(doc, |doc| doc.font.glyph_mut("B").unwrap().locked = true)
        .unwrap();
    assert!(api
        .call("style.batch", params)
        .unwrap_err()
        .message
        .contains("locked"));
}

#[test]
fn a_late_geometry_failure_does_not_modify_earlier_sources_or_add_backups() {
    let (api, session, doc, mut params) = fixture();
    params["options"]["weight"] = json!([10., 10.]);
    let generation = session
        .with_doc_mut(doc, |doc| {
            let layer = &mut doc.font.glyph_mut("B").unwrap().layers[1];
            layer.components.clear();
            let mut path = kurbo::BezPath::new();
            path.move_to((0., 0.));
            path.line_to((100., 100.));
            layer.contours = Contour::from_bezpath(&path);
            doc.generation
        })
        .unwrap();
    let before = font(&session, doc);
    params["dry_run"] = json!(false);
    params["expected_generation"] = json!(generation);
    assert!(api
        .call("style.batch", params)
        .unwrap_err()
        .message
        .contains("closed contours"));
    assert_eq!(font(&session, doc), before);
    assert_eq!(
        session.with_doc(doc, |doc| doc.generation).unwrap(),
        generation
    );
}
