mod fixtures;

use fw_geom::{bbox, expand_stroke, Cap, CurveKind, Join, Nib, StrokeOptions};

fn options(nib: Nib) -> StrokeOptions {
    StrokeOptions {
        width: 20.0,
        nib,
        cap: Cap::Round,
        join: Join::Round,
        miter_limit: 4.0,
        remove_internal: true,
        remove_external: true,
    }
}

#[test]
fn round_straight_stroke_is_a_stadium() {
    let result = expand_stroke(
        &[fixtures::open_line(0.0, 0.0, 100.0, 0.0)],
        &options(Nib::Round),
        CurveKind::Cubic,
    )
    .expect("round stroke");
    let bounds = bbox(&result).expect("bounds");
    assert!((bounds.x0 + 10.0).abs() < 0.1);
    assert!((bounds.x1 - 110.0).abs() < 0.1);
    assert!((bounds.y0 + 10.0).abs() < 0.1);
    assert!((bounds.y1 - 10.0).abs() < 0.1);
}

#[test]
fn rectangular_and_calligraphic_nibs_cover_lines_and_curves() {
    let curve = fw_model::Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, fw_model::PointKind::Move),
            fixtures::point(25.0, 80.0, fw_model::PointKind::OffCurve),
            fixtures::point(75.0, -80.0, fw_model::PointKind::OffCurve),
            fixtures::point(100.0, 0.0, fw_model::PointKind::Curve),
        ],
        name: None,
        id: None,
    };
    for nib in [
        Nib::Rect,
        Nib::Calligraphic {
            width: 24.0,
            height: 8.0,
            angle: 0.4,
        },
    ] {
        assert!(!expand_stroke(
            &[fixtures::open_line(0.0, 0.0, 100.0, 0.0), curve.clone()],
            &options(nib),
            CurveKind::Cubic,
        )
        .expect("convex nib stroke")
        .is_empty());
    }
}
