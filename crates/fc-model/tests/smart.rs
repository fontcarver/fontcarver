use fc_model::{Component, Contour, Font, Glyph, Layer, LayerKind, Point, PointKind, SmartAxis};
use kurbo::Shape;

fn glyph(master: &fc_model::MasterId) -> Glyph {
    let mut glyph = Glyph::new("_part.stem");
    glyph.smart_axes = vec![
        SmartAxis {
            name: "Width".into(),
            min: 0.,
            default: 0.,
            max: 100.,
        },
        SmartAxis {
            name: "Height".into(),
            min: 0.,
            default: 0.,
            max: 100.,
        },
    ];
    for (x, y) in [(0., 0.), (100., 0.), (0., 100.), (100., 100.)] {
        let mut layer = Layer::new_master(master.clone());
        if x != 0. || y != 0. {
            layer.kind = LayerKind::Smart {
                master: master.clone(),
                location: [("Width".into(), x), ("Height".into(), y)].into(),
            };
        }
        layer.width = 200. + x;
        layer.contours.push(Contour {
            closed: true,
            points: [
                (0., 0.),
                (100. + x, 0.),
                (100. + x, 200. + y),
                (0., 200. + y),
            ]
            .into_iter()
            .map(|(x, y)| Point {
                x,
                y,
                kind: PointKind::Line,
                ..Default::default()
            })
            .collect(),
            ..Default::default()
        });
        glyph.layers.push(layer);
    }
    glyph
}
#[test]
fn local_axes_resolve_two_dimensions_nested_components_and_metrics() {
    let mut font = Font::new_empty("Smart", 1000);
    let master = font.masters[0].id.clone();
    let part = glyph(&master);
    let values = [("Width".into(), 50.), ("Height".into(), 25.)].into();
    let resolved = fc_model::smart_layer(&part, &master, &values).unwrap();
    assert_eq!(resolved.width, 250.);
    assert_eq!(resolved.contours[0].points[2].x, 150.);
    assert_eq!(resolved.contours[0].points[2].y, 225.);
    font.insert_glyph(part).unwrap();
    let mut composite = Glyph::new("A");
    let mut layer = Layer::new_master(master.clone());
    layer.components.push(Component {
        base: "_part.stem".into(),
        smart: Some(values),
        transform: kurbo::Affine::translate((20., 30.)),
        ..Default::default()
    });
    composite.layers.push(layer);
    font.insert_glyph(composite).unwrap();
    assert_eq!(
        fc_model::flatten_layer(&font, "A", &master)
            .unwrap()
            .bounding_box(),
        kurbo::Rect::new(20., 30., 170., 255.)
    );
}
#[test]
fn malformed_local_axes_missing_corners_and_cycles_fail() {
    let mut font = Font::new_empty("Smart", 1000);
    let master = font.masters[0].id.clone();
    let mut part = glyph(&master);
    assert!(fc_model::smart_layer(&part, &master, &[("Width".into(), 101.)].into()).is_err());
    assert!(fc_model::smart_layer(&part, &master, &[("Unknown".into(), 1.)].into()).is_err());
    part.layers.pop();
    assert!(fc_model::smart_layer(
        &part,
        &master,
        &[("Width".into(), 50.), ("Height".into(), 50.)].into()
    )
    .is_err());
    font.insert_glyph(part).unwrap();
    font.glyph_mut("_part.stem").unwrap().layers[0]
        .components
        .push(Component {
            base: "_part.stem".into(),
            ..Default::default()
        });
    assert!(fc_model::flatten_layer(&font, "_part.stem", &master).is_err());
}

#[test]
fn finite_component_transforms_cannot_overflow_resolved_coordinates() {
    let mut font = Font::new_empty("Overflow", 1000);
    let master = font.default_master().id.clone();
    font.insert_glyph(glyph(&master)).unwrap();
    let mut composite = Glyph::new("A");
    let mut layer = Layer::new_master(master.clone());
    layer.components.push(Component {
        base: "_part.stem".into(),
        transform: kurbo::Affine::scale(f64::MAX),
        ..Default::default()
    });
    composite.layers.push(layer);
    font.insert_glyph(composite).unwrap();
    assert!(fc_model::flatten_layer(&font, "A", &master)
        .unwrap_err()
        .to_string()
        .contains("overflows"));
}

#[test]
fn local_axes_blend_corner_intent_including_a_zero_crossing() {
    let font = Font::new_empty("Smart corners", 1000);
    let master = &font.default_master().id;
    let mut part = glyph(master);
    for layer in &mut part.layers {
        layer.contours[0].points[1].corner_radius = layer.width - 250.;
    }
    for (width, radius) in [(25., -25.), (50., 0.), (75., 25.)] {
        let resolved = fc_model::smart_layer(
            &part,
            master,
            &[("Width".into(), width), ("Height".into(), 25.)].into(),
        )
        .unwrap();
        assert_eq!(resolved.contours[0].points[1].corner_radius, radius);
        resolved.validate_effects().unwrap();
    }
}
