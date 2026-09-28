//! Post-resolution callbacks let geometry consumers evaluate live layer data.
use fc_model::{Component, Contour, Font, Glyph, Layer, LayerKind};
use kurbo::{Affine, BezPath, Rect, Shape};
use std::cell::RefCell;

fn fixture() -> Font {
    let mut font = Font::new_empty("Filtered outlines", 1000);
    let master = font.default_master().id.clone();
    let mut base = Glyph::new("base");
    let mut layer = Layer::new_master(master.clone());
    layer.contours = Contour::from_bezpath(&Rect::new(0., 0., 100., 100.).to_path(0.01));
    layer.name = "base".into();
    base.layers.push(layer);
    font.insert_glyph(base).unwrap();
    let mut composite = Glyph::new("composite");
    let mut layer = Layer::new_master(master);
    layer.name = "composite".into();
    layer.components.push(Component {
        base: "base".into(),
        transform: Affine::translate((100., 0.)) * Affine::scale(2.),
        ..Default::default()
    });
    composite.layers.push(layer);
    font.insert_glyph(composite).unwrap();
    font
}

#[test]
fn filter_evaluates_children_before_parent_transforms_and_parent_effects() {
    let font = fixture();
    let original = font.clone();
    let calls = RefCell::new(Vec::new());
    let filter = |layer: &Layer, path: BezPath| {
        calls.borrow_mut().push(layer.name.clone());
        Ok(Affine::translate((10., 0.)) * path)
    };
    let path =
        fc_model::query::flatten_layer_with(&font, "composite", &font.default_master().id, &filter)
            .unwrap();
    assert_eq!(*calls.borrow(), ["base", "composite"]);
    assert_eq!(path.bounding_box(), Rect::new(130., 0., 330., 200.));
    assert_eq!(
        fc_model::flatten_layer(&font, "composite", &font.default_master().id)
            .unwrap()
            .bounding_box(),
        Rect::new(100., 0., 300., 200.)
    );
    assert_eq!(font, original);
}

#[test]
fn selected_layer_filter_keeps_its_master_and_rejects_invalid_results() {
    let mut font = fixture();
    let master = font.default_master().id.clone();
    let glyph = font.glyph_mut("composite").unwrap();
    let mut background = glyph.layers[0].clone();
    background.id = fc_model::LayerId::new();
    background.kind = LayerKind::Background(master);
    background.name = "background".into();
    let layer = background.id.clone();
    glyph.layers.push(background);
    let path =
        fc_model::query::flatten_layer_by_id_with(&font, "composite", &layer, &|layer, path| {
            Ok(if layer.name == "background" {
                Affine::translate((0., 50.)) * path
            } else {
                path
            })
        })
        .unwrap();
    assert_eq!(path.bounding_box(), Rect::new(100., 50., 300., 250.));
    let result = fc_model::query::flatten_layer_with(
        &font,
        "composite",
        &font.default_master().id,
        &|_, path| Ok(Affine::scale(f64::INFINITY) * path),
    );
    assert!(result.is_err());
    let result = fc_model::query::flatten_layer_with(
        &font,
        "composite",
        &font.default_master().id,
        &|_, _| Err(fc_model::Error::InvalidDesignSpace("filter failed".into())),
    );
    assert!(result.unwrap_err().to_string().contains("filter failed"));
}
