mod fixtures;

use fw_geom::{canonical_order, canonical_start_points, correct_direction, reverse};
use fw_model::{Contour, PointKind, TtIndex};
use kurbo::Shape;

#[test]
fn outer_contour_is_made_clockwise() {
    let ccw = fixtures::rect(0.0, 0.0, 100.0, 100.0);
    let fixed = correct_direction(&[ccw], true);
    assert!(fixed[0].to_bezpath().area() < 0.0);
}

#[test]
fn nested_contours_alternate_direction() {
    let contours = [
        fixtures::rect(0.0, 0.0, 100.0, 100.0),
        fixtures::rect(20.0, 20.0, 80.0, 80.0),
    ];
    let fixed = correct_direction(&contours, true);
    assert!(fixed[0].to_bezpath().area() < 0.0);
    assert!(fixed[1].to_bezpath().area() > 0.0);
}

#[test]
fn three_level_nesting_alternates_outer_hole_and_island() {
    let contours = [
        fixtures::rect(0.0, 0.0, 100.0, 100.0),
        fixtures::rect(20.0, 20.0, 80.0, 80.0),
        fixtures::rect(40.0, 40.0, 60.0, 60.0),
    ];
    let fixed = correct_direction(&contours, true);
    assert!(fixed[0].to_bezpath().area() < 0.0);
    assert!(fixed[1].to_bezpath().area() > 0.0);
    assert!(fixed[2].to_bezpath().area() < 0.0);
}

#[test]
fn canonicalization_is_stable() {
    let contours = [
        fixtures::rect(20.0, 20.0, 30.0, 30.0),
        fixtures::rect(0.0, 0.0, 10.0, 10.0),
    ];
    let once = canonical_order(&canonical_start_points(&contours));
    let twice = canonical_order(&canonical_start_points(&once));
    assert_eq!(once, twice);
}

#[test]
fn reversing_twice_preserves_mixed_segments_and_metadata() {
    let mut named = fixtures::point(0.0, 0.0, PointKind::Line);
    named.name = Some("origin".into());
    named.tt_index = TtIndex::Index(9);
    let contour = Contour {
        closed: true,
        points: vec![
            named,
            fixtures::point(30.0, 80.0, PointKind::OffCurve),
            fixtures::point(70.0, 80.0, PointKind::OffCurve),
            fixtures::point(100.0, 0.0, PointKind::Curve),
            fixtures::point(50.0, -50.0, PointKind::Line),
        ],
        name: Some("mixed".into()),
        id: Some("contour-id".into()),
    };
    assert_eq!(reverse(&reverse(&contour)), contour);
}
