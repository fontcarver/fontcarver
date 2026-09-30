use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Axis, Contour, Font, Glyph, Instance, KernSide, Layer, MasterId, Tag};
use kurbo::Shape;
use serde_json::json;

fn setup() -> (Api, Arc<Session>, fc_doc::DocId) {
    let mut font = Font::new_empty("Proof", 1000);
    let tag = Tag(*b"wght");
    font.axes.push(Axis {
        tag,
        name: "Weight".into(),
        min: 100.,
        default: 100.,
        max: 900.,
        map: vec![(100., 0.), (500., 300.), (900., 1000.)],
        ..Default::default()
    });
    font.masters[0].location.insert(tag, 0.);
    let mut heavy = font.masters[0].clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(tag, 1000.);
    font.masters.push(heavy);
    for (name, cp) in [("A", 65), ("V", 86)] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(cp);
        for (index, master) in font.masters.iter().enumerate() {
            let mut layer = Layer::new_master(master.id.clone());
            layer.width = 500. + index as f64 * 200.;
            layer.contours = Contour::from_bezpath(
                &kurbo::Rect::new(0., 0., 100. + index as f64 * 200., 400.).to_path(0.1),
            );
            glyph.layers.push(layer);
        }
        font.insert_glyph(glyph).unwrap();
    }
    for (index, master) in font.masters.iter_mut().enumerate() {
        master.kerning_h.0.insert(
            (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
            -40. - index as f64 * 80.,
        );
    }
    font.instances.push(Instance {
        name: "Midpoint".into(),
        location: [(tag, 500.)].into(),
        active: true,
        ..Default::default()
    });
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session.clone()), session, doc)
}

#[test]
fn instance_preview_shapes_interpolated_geometry_advances_and_kerning() {
    let (api, session, doc) = setup();
    let before = session.with_doc(doc, |doc| doc.font.clone()).unwrap();
    let result = api
        .call(
            "text.variable_preview",
            json!({"doc":doc,"text":"AV","instance":0,"location":{}}),
        )
        .unwrap();
    assert_eq!(result["generation"], 0);
    assert_eq!(result["glyphs"][0]["x_advance"], 520.);
    assert_eq!(result["glyphs"][1]["x"], 520.);
    assert_eq!(result["glyphs"][1]["x_advance"], 600.);
    let path = kurbo::BezPath::from_svg(result["glyphs"][0]["path"].as_str().unwrap()).unwrap();
    assert_eq!(path.bounding_box().width(), 200.);
    assert!((result["location"]["wght"].as_f64().unwrap() - 614.2857142857143).abs() < 0.001);
    assert_eq!(
        session.with_doc(doc, |doc| doc.font.clone()).unwrap(),
        before
    );
    let plain = api
        .call(
            "text.variable_preview",
            json!({"doc":doc,"text":"AV","instance":0,"location":{},"features":[["kern",false]]}),
        )
        .unwrap();
    assert_eq!(plain["glyphs"][0]["x_advance"], 600.);
    let maximum = api
        .call(
            "text.variable_preview",
            json!({"doc":doc,"text":"A","location":{"wght":900}}),
        )
        .unwrap();
    assert_eq!(maximum["glyphs"][0]["x_advance"], 700.);
}

#[test]
fn instance_preview_rejects_unknown_axes_out_of_range_and_stale_instance() {
    let (api, _, doc) = setup();
    for params in [
        json!({"doc":doc,"text":"A","location":{"wdth":100}}),
        json!({"doc":doc,"text":"A","location":{"wght":1000}}),
        json!({"doc":doc,"text":"A","location":{},"instance":1}),
    ] {
        assert!(api.call("text.variable_preview", params).is_err());
    }
}
