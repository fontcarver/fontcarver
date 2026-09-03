mod fixtures;

use fw_geom::{simplify, SimplifyOptions};
use fw_model::{Contour, PointKind};

#[test]
fn noisy_circle_is_reduced() {
    let points = (0..200)
        .map(|i| {
            let angle = i as f64 / 200.0 * std::f64::consts::TAU;
            let radius = 100.0 + (i as f64 * 1.7).sin() * 0.08;
            fixtures::point(radius * angle.cos(), radius * angle.sin(), PointKind::Line)
        })
        .collect();
    let contour = Contour {
        closed: true,
        points,
        name: None,
        id: None,
    };
    let result = simplify(
        &[contour],
        &SimplifyOptions {
            error: 1.0,
            ignore_slopes: false,
            ignore_extrema: false,
            smooth_curves: true,
            force_lines: false,
            nearly_hv_lines: false,
            start_at_extrema: false,
        },
    );
    assert!(result[0].points.len() < 20, "{:?}", result[0].points);
}

#[test]
fn extrema_are_explicit_unless_ignored() {
    let contour = Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, PointKind::Move),
            fixtures::point(25.0, 100.0, PointKind::OffCurve),
            fixtures::point(75.0, 100.0, PointKind::OffCurve),
            fixtures::point(100.0, 0.0, PointKind::Curve),
        ],
        name: None,
        id: None,
    };
    let result = simplify(
        &[contour],
        &SimplifyOptions {
            error: 0.1,
            ..SimplifyOptions::default()
        },
    );
    assert!(result[0]
        .points
        .iter()
        .filter(|point| point.kind != PointKind::OffCurve)
        .any(|point| (point.x - 50.0).abs() < 1e-6 && (point.y - 75.0).abs() < 1e-6));
}
