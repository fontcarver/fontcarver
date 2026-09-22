use fc_api::Api;
use fc_doc::Session;
use fc_model::{Axis, Font, Glyph, Layer, MasterId, Tag};
use serde_json::json;
use std::sync::Arc;

#[test]
fn design_space_api_roundtrips_metadata_rejects_stale_edits_and_previews_metrics() {
    let mut font = Font::new_empty("Variation API", 1000);
    let master = font.masters[0].id.clone();
    let mut glyph = Glyph::new("A");
    let mut layer = Layer::new_master(master);
    layer.width = 500.0;
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    let state = api.call("variation.state", json!({"doc":doc})).unwrap();
    let mut masters: Vec<fc_model::Master> =
        serde_json::from_value(state["masters"].clone()).unwrap();
    masters[0].location.insert(Tag(*b"wght"), 100.0);
    let mut heavy = masters[0].clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(Tag(*b"wght"), 900.0);
    masters.push(heavy);
    let axis = Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.0,
        default: 100.0,
        max: 900.0,
        ..Default::default()
    };
    let params = json!({"doc":doc,"expected_generation":0,"label":"Design space","command":{"Set":{"axes":[axis],"masters":masters,"instances":[],"copy_from":masters[0].id}}});
    api.call("variation.apply", params.clone()).unwrap();
    assert!(api.call("variation.apply", params).is_err());
    let result = api
        .call(
            "variation.preview",
            json!({"doc":doc,"glyph":"A","location":{"wght":500}}),
        )
        .unwrap();
    assert_eq!(result["width"], 500.0);
    assert_eq!(result["sources"].as_array().unwrap().len(), 2);
    assert!(result["issues"].as_array().unwrap().is_empty());
    assert!(api
        .call(
            "variation.preview",
            json!({"doc":doc,"glyph":"A","location":{"wght":1000}})
        )
        .is_err());
    session
        .with_doc_mut(doc, |doc| {
            doc.undo().unwrap();
            assert!(doc.font.axes.is_empty());
            assert_eq!(doc.font.masters.len(), 1);
            doc.redo().unwrap();
            assert_eq!(doc.font.masters.len(), 2);
        })
        .unwrap();
}

#[test]
fn smart_parameters_decompose_undo_and_native_save_preserve_evaluated_geometry() {
    use fc_model::{Component, Contour, LayerId, LayerKind, SmartAxis};
    use kurbo::{Affine, Shape};
    let mut font = Font::new_empty("Smart API", 1000);
    let master = font.default_master().id.clone();
    let mut base = Glyph::new("part");
    base.smart_axes.push(SmartAxis {
        name: "Width".into(),
        min: 0.,
        default: 0.,
        max: 100.,
    });
    for width in [100., 200.] {
        let mut layer = Layer::new_master(master.clone());
        layer.width = width + 100.;
        layer.contours = Contour::from_bezpath(&kurbo::Rect::new(0., 0., width, 400.).to_path(0.1));
        if width == 200. {
            layer.kind = LayerKind::Smart {
                master: master.clone(),
                location: [("Width".into(), 100.)].into(),
            };
        }
        base.layers.push(layer);
    }
    font.insert_glyph(base).unwrap();
    let mut composite = Glyph::new("A");
    let mut layer = Layer::new_master(master.clone());
    let layer_id = layer.id.clone();
    layer.components.push(Component {
        base: "part".into(),
        transform: Affine::translate((20., 30.)),
        smart: Some([("Width".into(), 50.)].into()),
        ..Default::default()
    });
    composite.layers.push(layer);
    let mut background = Layer::new_master(master.clone());
    background.id = LayerId::new();
    background.kind = LayerKind::Background(master.clone());
    background.components.push(Component {
        base: "deletedBackgroundReference".into(),
        ..Default::default()
    });
    composite.layers.push(background);
    font.insert_glyph(composite).unwrap();
    // Compilation must not resolve unused background drawings.
    assert!(fc_geom::resolve_smart_font(&font).is_ok());
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    let gid = session.with_doc(doc, |d| d.glyph_id("A").unwrap()).unwrap();
    let outline = api
        .call(
            "glyphs.outline",
            json!({"doc":doc,"gid":gid,"layer":layer_id}),
        )
        .unwrap();
    assert_eq!(
        outline["components"][0]["bbox"],
        json!([20., 30., 170., 430.])
    );
    assert_eq!(outline["components"][0]["smart_axes"][0]["name"], "Width");
    let mut component = outline["components"][0]["component"].clone();
    component["smart"]["Width"] = json!(75.);
    api.call("doc.apply", json!({"doc":doc,"label":"Smart width","command":{"SetComponent":{"glyph":"A","layer":layer_id,"index":0,"component":component}}})).unwrap();
    let before = session.with_doc(doc, |d| d.font.clone()).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("smart.fcf");
    fc_doc::fcf::write(&path, &before).unwrap();
    assert_eq!(fc_doc::fcf::read(&path).unwrap(), before);
    api.call("doc.apply", json!({"doc":doc,"label":"Decompose","command":{"DecomposeComponents":{"glyph":"A","layer":layer_id,"indices":[0]}}})).unwrap();
    session
        .with_doc(doc, |d| {
            let layer = d.font.glyph("A").unwrap().layer(&layer_id).unwrap();
            assert!(layer.components.is_empty());
            assert_eq!(
                layer.contours[0].to_bezpath().bounding_box(),
                kurbo::Rect::new(20., 30., 195., 430.)
            );
        })
        .unwrap();
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(session.with_doc(doc, |d| d.font.clone()).unwrap(), before);
}
