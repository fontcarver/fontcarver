use fc_model::{Contour, Layer, Point, PointKind};
use kurbo::{Rect, Shape};

#[test]
fn legacy_points_default_to_source_and_nonzero_radius_round_trips() {
    let mut point: Point = serde_json::from_str(r#"{"x":10,"y":20,"kind":"Line"}"#).unwrap();
    assert_eq!(point.corner_radius, 0.);
    assert!(serde_json::to_value(&point)
        .unwrap()
        .get("corner_radius")
        .is_none());
    point.corner_radius = -12.5;
    let saved = serde_json::to_string(&point).unwrap();
    assert_eq!(serde_json::from_str::<Point>(&saved).unwrap(), point);
}

#[test]
fn invalid_roles_nonfinite_values_and_corner_budget_are_rejected() {
    let mut layer = Layer {
        contours: Contour::from_bezpath(&Rect::new(0., 0., 100., 100.).to_path(0.01)),
        ..Default::default()
    };
    for radius in [f64::NAN, f64::INFINITY, 8193., -8193.] {
        layer.contours[0].points[1].corner_radius = radius;
        assert!(layer.validate_effects().is_err());
    }
    layer.contours[0].points[1].corner_radius = 20.;
    layer.contours[0].points[1].kind = PointKind::OffCurve;
    assert!(layer.validate_effects().is_err());
    layer.contours[0].points[1].kind = PointKind::Line;
    layer.contours[0].closed = false;
    layer.contours[0].points[1].corner_radius = -20.;
    assert!(layer.validate_effects().is_err());
    layer.contours[0].closed = true;
    let contour = layer.contours[0].clone();
    layer.contours = vec![contour; 1025];
    assert!(layer.validate_effects().is_err());
    layer.contours.pop();
    layer.validate_effects().unwrap();
}
