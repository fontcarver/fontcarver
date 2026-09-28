use fc_model::LayerId;
use fc_model::{
    Component, Contour, EffectBoolean, EffectCap, EffectJoin, EffectNib, Font, Glyph, Layer,
    LayerEffect, LayerEffectKind,
};
use kurbo::{Affine, BezPath, Rect, Shape};

fn stroke(width: f64, height: f64) -> LayerEffect {
    LayerEffect {
        enabled: true,
        effect: LayerEffectKind::Stroke {
            nib: EffectNib::Ellipse,
            width,
            height,
            angle: 0.,
            cap: EffectCap::Round,
            join: EffectJoin::Round,
            miter_limit: 4.,
        },
    }
}
fn fixture() -> Font {
    let mut font = Font::new_empty("Effects", 1000);
    let mut glyph = Glyph::new("line");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.contours = Contour::from_bezpath(&BezPath::from_svg("M0 0L100 0").unwrap());
    layer.effects = vec![stroke(20., 40.)];
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}
#[test]
fn evaluates_elliptical_stroke_and_nested_component_without_mutating_sources() {
    let mut font = fixture();
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("composite");
    let mut layer = Layer::new_master(master.clone());
    layer.components.push(Component {
        base: "line".into(),
        transform: Affine::translate((200., 50.)) * Affine::scale(2.),
        ..Default::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let original = font.clone();
    let result = fc_geom::flatten_layer(&font, "composite", &master).unwrap();
    assert_eq!(result.bounding_box(), Rect::new(180., 10., 420., 90.));
    assert_ne!(
        result,
        fc_model::flatten_layer(&font, "composite", &master).unwrap()
    );
    assert_eq!(font, original);
}
#[test]
fn ordered_boolean_masks_can_create_geometry_and_disable_retains_source() {
    let mut font = fixture();
    let master = font.default_master().id.clone();
    let layer = &mut font.glyph_mut("line").unwrap().layers[0];
    layer.effects.push(LayerEffect {
        enabled: true,
        effect: LayerEffectKind::Boolean {
            operation: EffectBoolean::Intersect,
            path: "M0 -50L50 -50L50 50L0 50Z".into(),
        },
    });
    let result = fc_geom::flatten_layer(&font, "line", &master).unwrap();
    assert_eq!(result.bounding_box(), Rect::new(0., -20., 50., 20.));
    let layer = &mut font.glyph_mut("line").unwrap().layers[0];
    layer.effects.swap(0, 1);
    assert!(
        fc_geom::flatten_layer(&font, "line", &master).is_err(),
        "filled booleans reject unclosed source"
    );
    let layer = &mut font.glyph_mut("line").unwrap().layers[0];
    for effect in &mut layer.effects {
        effect.enabled = false;
    }
    assert_eq!(
        fc_geom::flatten_layer(&font, "line", &master).unwrap(),
        fc_model::flatten_layer(&font, "line", &master).unwrap()
    );
    let layer = &mut font.glyph_mut("line").unwrap().layers[0];
    layer.contours.clear();
    layer.effects = vec![LayerEffect {
        enabled: true,
        effect: LayerEffectKind::Boolean {
            operation: EffectBoolean::Union,
            path: "M0 0L50 0L50 50L0 50Z".into(),
        },
    }];
    assert_eq!(
        fc_geom::flatten_layer(&font, "line", &master)
            .unwrap()
            .bounding_box(),
        Rect::new(0., 0., 50., 50.)
    );
}
#[test]
fn baking_font_evaluates_original_components_once_and_clears_derived_hints() {
    let font = fixture();
    let (baked, _) = fc_geom::bake_effects_font(&font).unwrap();
    let layer = &baked.glyph("line").unwrap().layers[0];
    assert!(layer.effects.is_empty());
    assert!(layer.components.is_empty());
    assert!(layer.contours.iter().all(|c| c.closed));
    assert_eq!(
        fc_model::flatten_layer(&baked, "line", &baked.default_master().id).unwrap(),
        fc_geom::flatten_layer(&font, "line", &font.default_master().id).unwrap()
    );
    assert_eq!(font.glyph("line").unwrap().layers[0].effects.len(), 1);
}

#[test]
fn thick_closed_stroke_has_no_inverted_inner_hole() {
    let mut font = fixture();
    let layer = &mut font.glyph_mut("line").unwrap().layers[0];
    layer.contours = Contour::from_bezpath(&Rect::new(0., 0., 10., 10.).to_path(0.01));
    layer.effects = vec![stroke(40., 40.)];
    let path = fc_geom::flatten_layer(&font, "line", &font.default_master().id).unwrap();
    for (x, y) in [(5.3, 5.7), (-10., 5.), (20., 5.)] {
        assert_ne!(
            path.winding(kurbo::Point::new(x, y)),
            0,
            "missing stroke at {x},{y}"
        );
    }
}

#[test]
fn round_stroke_fills_inside_join_without_triangular_hole() {
    for source in [
        "M300 637L187 637L187 410",
        "M300 637L187 637L187 410L300 410Z",
    ] {
        let mut font = fixture();
        let layer = &mut font.glyph_mut("line").unwrap().layers[0];
        layer.contours = Contour::from_bezpath(&BezPath::from_svg(source).unwrap());
        layer.effects = vec![stroke(20., 20.)];
        let path = fc_geom::flatten_layer(&font, "line", &font.default_master().id).unwrap();
        for x in [187.8, 190.8, 194.8] {
            for y in [627.3, 630.3, 634.3] {
                assert_ne!(
                    path.winding((x, y).into()),
                    0,
                    "missing join at {x},{y}: {source}"
                );
            }
        }
    }
}

#[test]
fn global_preview_interpolates_evaluated_sources_and_local_smart_uses_shared_stack() {
    let mut font = fixture();
    let first = font.default_master().id.clone();
    let axis = fc_model::Tag(*b"wght");
    font.axes.push(fc_model::Axis {
        tag: axis,
        name: "Weight".into(),
        min: 0.,
        default: 0.,
        max: 1.,
        ..Default::default()
    });
    font.masters[0].location.insert(axis, 0.);
    let mut second = font.masters[0].clone();
    second.id = fc_model::MasterId::new();
    second.location.insert(axis, 1.);
    let second_id = second.id.clone();
    font.masters.push(second);
    let glyph = font.glyph_mut("line").unwrap();
    let mut heavy = glyph.layers[0].clone();
    heavy.id = LayerId::new();
    heavy.kind = fc_model::LayerKind::Master(second_id);
    heavy.effects = vec![stroke(60., 80.)];
    glyph.layers.push(heavy);
    let middle =
        fc_geom::variation::interpolated_path(&font, "line", &[(axis, 0.5)].into()).unwrap();
    let bounds = middle.bounding_box();
    for (a, b) in [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
        .into_iter()
        .zip([-20., -30., 120., 30.])
    {
        assert!((a - b).abs() < 0.01);
    }
    let glyph = font.glyph_mut("line").unwrap();
    glyph.smart_axes = vec![fc_model::SmartAxis {
        name: "Length".into(),
        min: 0.,
        default: 0.,
        max: 1.,
    }];
    let mut local = glyph.layers[0].clone();
    local.id = LayerId::new();
    local.kind = fc_model::LayerKind::Smart {
        master: first.clone(),
        location: [("Length".into(), 1.)].into(),
    };
    local.contours[0].points[1].x = 200.;
    glyph.layers.push(local);
    let local =
        fc_geom::flatten_smart_component(&font, "line", &first, &[("Length".into(), 0.5)].into())
            .unwrap();
    assert_eq!(local.bounding_box(), Rect::new(-10., -20., 160., 20.));
    font.glyph_mut("line")
        .unwrap()
        .layers
        .last_mut()
        .unwrap()
        .effects
        .clear();
    assert!(fc_geom::flatten_smart_component(
        &font,
        "line",
        &first,
        &[("Length".into(), 0.5)].into()
    )
    .unwrap_err()
    .to_string()
    .contains("identical"));
}
