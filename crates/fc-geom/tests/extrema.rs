mod fixtures;

use fc_geom::{add_extrema, add_inflections, from_bezpaths, to_bezpaths, CurveKind};
use fc_model::{Contour, PointKind};
use kurbo::{Affine, Circle, Shape};

#[test]
fn rotated_ellipse_gets_four_new_extrema() {
    let mut path = Circle::new((0.0, 0.0), 100.0).to_path(0.01);
    path.apply_affine(Affine::scale_non_uniform(1.5, 0.7).then_rotate(0.4));
    let input = Contour::from_bezpath(&path);
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
