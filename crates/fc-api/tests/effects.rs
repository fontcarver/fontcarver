use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{Contour, Font, Glyph, Layer};
use kurbo::{BezPath, Rect, Shape};
use serde_json::{json, Value};
use std::sync::Arc;

fn fixture() -> (Api, Arc<Session>, DocId, Value) {
    let mut font = Font::new_empty("Live effects", 1000);
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![65];
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 600.;
    layer.contours = Contour::from_bezpath(&BezPath::from_svg("M100 0L300 700").unwrap());
    let id = layer.id.clone();
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let params = json!({"doc":doc,"glyph":"A","layer":id,"effects":[{"enabled":true,"effect":{"kind":"stroke","nib":"ellipse","width":40,"height":80,"angle":0,"cap":"round","join":"round","miter_limit":4}}],"dry_run":true,"bake":false,"expected_generation":null});
    (Api::new(session.clone()), session, doc, params)
}
fn font(session: &Session, doc: DocId) -> Font {
    session.with_doc(doc, |d| d.font.clone()).unwrap()
}
fn commit(api: &Api, params: &Value, bake: bool) -> Value {
    let mut params = params.clone();
    let preview = api.call("effects.apply", params.clone()).unwrap();
    params["expected_generation"] = preview["generation"].clone();
    params["dry_run"] = json!(false);
    params["bake"] = json!(bake);
    api.call("effects.apply", params).unwrap()
}

#[test]
fn preview_apply_source_edits_bake_undo_and_mask_use_one_geometry() {
    let (api, session, doc, params) = fixture();
    let original = font(&session, doc);
    let preview = api.call("effects.apply", params.clone()).unwrap();
    assert_eq!(font(&session, doc), original);
    let applied = commit(&api, &params, false);
    assert_eq!(applied["example"], preview["example"]);
    let after = font(&session, doc);
    assert_eq!(
        after.glyph("A").unwrap().layers[0].contours,
        original.glyph("A").unwrap().layers[0].contours
    );
    let (gid, master) = session
        .with_doc(doc, |d| {
            (d.glyph_id("A").unwrap(), d.font.default_master().id.clone())
        })
        .unwrap();
    let cell = api
        .call("glyphs.cell", json!({"doc":doc,"gid":gid,"master":master}))
        .unwrap();
    let detail = api
        .call("glyphs.detail", json!({"doc":doc,"gid":gid}))
        .unwrap();
    assert_eq!(cell["path"], detail["paths"][0]["flattened"]);
    let mask = api
        .call("effects.mask", json!({"doc":doc,"glyph":"A","layer":null}))
        .unwrap();
    assert_eq!(mask, preview["example"]["after"]);
    let mut stale = params.clone();
    stale["dry_run"] = json!(false);
    stale["expected_generation"] = preview["generation"].clone();
    assert!(api
        .call("effects.apply", stale)
        .unwrap_err()
        .message
        .contains("stale"));
    session
        .with_doc_mut(doc, |d| {
            d.apply(
                fc_doc::Command::MovePoints {
                    glyph: "A".into(),
                    layer: after.glyph("A").unwrap().layers[0].id.clone(),
                    points: vec![fc_doc::PointRef {
                        contour: 0,
                        point: 1,
                    }],
                    delta: (100., 0.),
                    handles: fc_doc::HandleMode::PointsOnly,
                },
                "Edit source",
            )
        })
        .unwrap()
        .unwrap();
    let moved = font(&session, doc);
    let path = fc_geom::flatten_layer(&moved, "A", &master).unwrap();
    assert!(path.bounding_box().x1 > 419.);
    assert_eq!(moved.glyph("A").unwrap().layers[0].effects.len(), 1);
    commit(&api, &params, true);
    let baked = font(&session, doc);
    assert!(baked.glyph("A").unwrap().layers[0].effects.is_empty());
    assert!(baked.glyph("A").unwrap().layers[0]
        .contours
        .iter()
        .all(|c| c.closed));
    let baked_path = fc_model::flatten_layer(&baked, "A", &master).unwrap();
    assert_eq!(path, baked_path);
    session.with_doc_mut(doc, |d| d.undo()).unwrap().unwrap();
    assert_eq!(font(&session, doc), moved);
}

#[test]
fn malformed_stacks_and_locked_sources_are_atomic() {
    let (api, session, doc, params) = fixture();
    let original = font(&session, doc);
    let mut invalid = params.clone();
    invalid["effects"][0]["effect"]["width"] = json!(0);
    assert!(api.call("effects.apply", invalid).is_err());
    assert_eq!(font(&session, doc), original);
    session
        .with_doc_mut(doc, |d| d.font.glyph_mut("A").unwrap().locked = true)
        .unwrap();
    assert!(api
        .call("effects.apply", params)
        .unwrap_err()
        .message
        .contains("locked"));
}

#[test]
fn source_formats_reject_live_stacks_before_replacing_files() {
    let (api, session, doc, params) = fixture();
    commit(&api, &params, false);
    let font = font(&session, doc);
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("sentinel");
    std::fs::write(&output, b"retain me").unwrap();
    let master = &font.default_master().id;
    assert!(fc_formats::glyphs::write_glyphs(&font, &output).is_err());
    assert!(fc_formats::sfd::write_sfd(&font, master, &output).is_err());
    assert!(fc_formats::type1::write_type1(&font, master, &output, true).is_err());
    assert!(fc_formats::ufo::write_ufo(&font, master, &output, &Default::default()).is_err());
    assert_eq!(std::fs::read(&output).unwrap(), b"retain me");
}

#[test]
fn compiled_ttf_and_otf_include_effect_geometry_and_keep_source() {
    let (api, session, doc, params) = fixture();
    commit(&api, &params, false);
    let original = font(&session, doc);
    let dir = tempfile::tempdir().unwrap();
    for (format, extension) in [
        (fc_compile::ExportFormat::Ttf, "ttf"),
        (fc_compile::ExportFormat::Otf, "otf"),
        (fc_compile::ExportFormat::Woff, "woff"),
        (fc_compile::ExportFormat::Woff2, "woff2"),
    ] {
        let path = dir.path().join(format!("effect.{extension}"));
        let opts = fc_compile::ExportOptions {
            format,
            ..Default::default()
        };
        let report = fc_compile::export(&original, &|_| 0, &opts).unwrap();
        assert!(report
            .warnings
            .iter()
            .any(|w| w.code == "layer-effects-baked"));
        std::fs::write(&path, &report.bytes).unwrap();
        let read = fc_sfnt::read_path(&path, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let bounds = fc_model::layer_bbox(&read, "A", &read.default_master().id)
            .unwrap()
            .unwrap();
        let expected = Rect::new(80., -40., 320., 740.);
        for (a, b) in [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
            .into_iter()
            .zip([expected.x0, expected.y0, expected.x1, expected.y1])
        {
            assert!((a - b).abs() <= 1.0, "{extension}: {bounds:?}");
        }
    }
    assert_eq!(font(&session, doc), original);
}
