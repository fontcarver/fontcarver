mod fixtures;

use fc_geom::{add_extrema, add_inflections, force_extrema, from_bezpaths, to_bezpaths, CurveKind};
use fc_model::{Contour, PointKind};
use kurbo::{Affine, Circle, Shape};

#[test]
fn rotated_ellipse_gets_four_new_extrema() {
    let mut path = Circle::new((0.0, 0.0), 100.0).to_path(0.01);
    path.apply_affine(Affine::scale_non_uniform(1.5, 0.7).then_rotate(0.4));
    let mut input = Contour::from_bezpath(&path);
    input[0].points[0].id = Some("original-start".into());
    input[0].points[0].super_smooth = true;
    let before = input[0]
        .points
        .iter()
        .filter(|p| p.kind != PointKind::OffCurve)
        .count();
    let output = add_extrema(&input);
    let after = output[0]
        .points
        .iter()
        .filter(|p| p.kind != PointKind::OffCurve)
        .count();
    assert_eq!(after - before, 4);
    assert_eq!(
        output[0]
            .points
            .iter()
            .filter(|point| point.id.is_some())
            .count(),
        1
    );
    assert!(output[0].points.iter().any(|point| point.super_smooth));
}

#[test]
fn force_extrema_inserts_near_endpoint_extreme_without_moving_curve() {
    let mut path = kurbo::BezPath::new();
    path.move_to((0., 0.));
    path.curve_to((-0.001, 0.), (100., 0.), (200., 0.));
    let mut input = Contour::from_bezpath(&path);
    input[0].points[0].id = Some("original-start".into());
    input[0].points[0].super_smooth = true;
    assert_eq!(add_extrema(&input), input);
    let forced = force_extrema(&input);
    assert_eq!(
        forced[0]
            .points
            .iter()
            .filter(|point| point.kind != PointKind::OffCurve)
            .count(),
        3
    );
    let inserted = &forced[0].points[3];
    assert!(inserted.x < 0. && inserted.x > -0.01);
    assert_eq!(inserted.y, 0.);
    assert_eq!(force_extrema(&forced), forced);
    assert_eq!(forced[0].points[0].id.as_deref(), Some("original-start"));
    assert!(forced[0].points[0].super_smooth);
    assert_eq!(
        forced[0]
            .points
            .iter()
            .filter(|point| point.id.is_some())
            .count(),
        1
    );
    let original_bounds = path.bounding_box();
    let forced_bounds = forced[0].to_bezpath().bounding_box();
    assert!((original_bounds.x0 - forced_bounds.x0).abs() < 1e-10);
    assert!((original_bounds.x1 - forced_bounds.x1).abs() < 1e-10);
}

#[test]
fn quadratic_conversion_has_no_implied_points() {
    let contour = Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, PointKind::Move),
            fixtures::point(50.0, 100.0, PointKind::OffCurve),
            fixtures::point(100.0, 0.0, PointKind::QCurve),
        ],
        name: None,
        id: None,
    };
    let paths = to_bezpaths(std::slice::from_ref(&contour));
    assert!(paths[0]
        .elements()
        .iter()
        .all(|element| !matches!(element, kurbo::PathEl::QuadTo(..))));
    let converted = from_bezpaths(&paths, CurveKind::Quadratic { max_err: 1e-6 });
    assert!(converted[0]
        .points
        .iter()
        .all(|point| point.tt_index != fc_model::TtIndex::Implied));
    assert!(converted[0]
        .points
        .iter()
        .any(|point| point.kind == PointKind::QCurve));
}

#[test]
fn invalid_quadratic_accuracy_is_rejected_without_a_fallback() {
    let paths = to_bezpaths(&[fixtures::rect(0.0, 0.0, 100.0, 100.0)]);
    assert!(from_bezpaths(&paths, CurveKind::Quadratic { max_err: f64::NAN }).is_empty());
}

#[test]
fn an_s_curve_gets_its_inflection() {
    let contour = Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, PointKind::Move),
            fixtures::point(25.0, 100.0, PointKind::OffCurve),
            fixtures::point(75.0, -100.0, PointKind::OffCurve),
            fixtures::point(100.0, 0.0, PointKind::Curve),
        ],
        name: None,
        id: None,
    };
    let result = add_inflections(&[contour]);
    assert_eq!(
        result[0]
            .points
            .iter()
            .filter(|p| p.kind != PointKind::OffCurve)
            .count(),
        3
    );
}
