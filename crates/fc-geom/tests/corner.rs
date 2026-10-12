//! Open Corner and Reconnect Nodes, the two halves of one reversible edit.

use fc_geom::{open_corner, reconnect_nodes, CornerError};
use fc_model::{Contour, Point, PointKind};

fn node(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        x,
        y,
        kind,
        ..Point::default()
    }
}

/// A closed triangle with its sharp corner at index 1.
fn triangle() -> Contour {
    Contour {
        closed: true,
        points: vec![
            node(0., 0., PointKind::Line),
            node(100., 0., PointKind::Line),
            node(50., 120., PointKind::Line),
        ],
        ..Contour::default()
    }
}

/// Two cubic segments meeting at a corner at index 3.
fn cubic_corner() -> Contour {
    Contour {
        closed: true,
        points: vec![
            node(0., 0., PointKind::Line),
            node(20., 40., PointKind::OffCurve),
            node(60., 80., PointKind::OffCurve),
            node(100., 100., PointKind::Curve),
            node(140., 60., PointKind::OffCurve),
            node(180., 20., PointKind::OffCurve),
            node(200., 0., PointKind::Curve),
        ],
        ..Contour::default()
    }
}

#[test]
fn open_corner_places_both_nodes_on_the_adjacent_tangents() {
    let mut input = triangle();
    input.points[1].id = Some("corner".into());
    input.points[1].smooth = true;
    let before = input.clone();
    let opened = open_corner(&input, 1, 10.).expect("triangle corner opens");
    assert_eq!(opened.points.len(), 4);
    // The kept node moved 10 units further along the incoming direction, which
    // is +x here, and keeps its identity and its incoming segment's kind.
    let kept = &opened.points[1];
    assert_eq!((kept.x, kept.y), (110., 0.));
    assert_eq!(kept.id.as_deref(), Some("corner"));
    assert_eq!(kept.kind, PointKind::Line);
    assert!(!kept.smooth);
    // The fresh node is 10 units back along the outgoing direction, and the
    // mouth between the two is a straight segment.
    let mouth = &opened.points[2];
    let leave = (50. - 100_f64, 120. - 0_f64);
    let length = leave.0.hypot(leave.1);
    assert!((mouth.x - (100. - 10. * leave.0 / length)).abs() < 1e-9);
    assert!((mouth.y - (0. - 10. * leave.1 / length)).abs() < 1e-9);
    assert_eq!(mouth.kind, PointKind::Line);
    assert!(mouth.id.is_none());
    // The two neighbours are untouched, so the tangents are unchanged.
    assert_eq!((opened.points[0].x, opened.points[0].y), (0., 0.));
    assert_eq!((opened.points[3].x, opened.points[3].y), (50., 120.));
    assert_eq!(input, before);
}

#[test]
fn reconnect_nodes_restores_the_original_corner_exactly() {
    let mut input = triangle();
    input.points[1].id = Some("corner".into());
    let opened = open_corner(&input, 1, 12.5).expect("corner opens");
    let closed = reconnect_nodes(&opened, 1, 2).expect("open corner reconnects");
    assert_eq!(closed.points.len(), 3);
    assert!((closed.points[1].x - 100.).abs() < 1e-9);
    assert!((closed.points[1].y - 0.).abs() < 1e-9);
    assert_eq!(closed.points[1].id.as_deref(), Some("corner"));
    assert_eq!(closed.points[1].kind, PointKind::Line);
    assert_eq!(closed.points[0], input.points[0]);
    assert_eq!(closed.points[2], input.points[2]);
}

#[test]
fn a_cubic_corner_keeps_both_handles_and_round_trips() {
    let input = cubic_corner();
    let opened = open_corner(&input, 3, 8.).expect("cubic corner opens");
    assert_eq!(opened.points.len(), input.points.len() + 1);
    // Handles are never moved: the incoming control stays, so the tangent at
    // the moved node is the same direction it arrived in.
    assert_eq!(opened.points[2], input.points[2]);
    let arrive = (100. - 60_f64, 100. - 80_f64);
    let length = arrive.0.hypot(arrive.1);
    assert!((opened.points[3].x - (100. + 8. * arrive.0 / length)).abs() < 1e-9);
    assert!((opened.points[3].y - (100. + 8. * arrive.1 / length)).abs() < 1e-9);
    assert_eq!(opened.points[3].kind, PointKind::Curve);
    assert_eq!(opened.points[4].kind, PointKind::Line);
    assert_eq!(opened.points[5], input.points[4]);
    let closed = reconnect_nodes(&opened, 3, 4).expect("cubic open corner reconnects");
    assert_eq!(closed.points.len(), input.points.len());
    assert!((closed.points[3].x - 100.).abs() < 1e-9);
    assert!((closed.points[3].y - 100.).abs() < 1e-9);
    assert_eq!(closed.points[3].kind, PointKind::Curve);
}

#[test]
fn a_mouth_that_spans_the_closing_seam_still_reconnects() {
    // `Set Start Point` can leave an open corner's mouth across the seam, so
    // the second node is the stored start and the first is the last point.
    let mut input = triangle();
    input.points[1].id = Some("seam".into());
    let mut opened = open_corner(&input, 1, 10.).expect("corner opens");
    opened.points.rotate_left(2);
    assert_eq!(opened.points.len(), 4);
    let closed = reconnect_nodes(&opened, 3, 0).expect("seam corner reconnects");
    assert_eq!(closed.points.len(), 3);
    assert_eq!(closed.points[0].id.as_deref(), Some("seam"));
    assert!((closed.points[0].x - 100.).abs() < 1e-9);
    assert!((closed.points[0].y - 0.).abs() < 1e-9);
    assert_eq!(closed.points[1], input.points[2]);
    assert_eq!(closed.points[2], input.points[0]);
}

#[test]
fn invalid_open_corner_input_is_rejected_without_changing_the_contour() {
    let input = cubic_corner();
    assert_eq!(
        open_corner(&input, 99, 5.),
        Err(CornerError::InvalidNode(99))
    );
    assert_eq!(
        open_corner(&input, 1, 5.),
        Err(CornerError::OffCurveNode(1))
    );
    assert_eq!(open_corner(&input, 3, 0.), Err(CornerError::InvalidDepth));
    assert_eq!(
        open_corner(&input, 3, f64::NAN),
        Err(CornerError::InvalidDepth)
    );
    assert_eq!(
        open_corner(&input, 3, 10_000.),
        Err(CornerError::DepthTooLarge(3))
    );
    let mut smart = input.clone();
    smart.points[3].corner_radius = 12.;
    assert_eq!(open_corner(&smart, 3, 5.), Err(CornerError::SmartCorner(3)));
    let open_path = Contour {
        closed: false,
        points: vec![
            node(0., 0., PointKind::Move),
            node(100., 0., PointKind::Line),
        ],
        ..Contour::default()
    };
    assert_eq!(
        open_corner(&open_path, 1, 5.),
        Err(CornerError::UnsupportedNode(1))
    );
    let degenerate = Contour {
        closed: true,
        points: vec![
            node(0., 0., PointKind::Line),
            node(0., 0., PointKind::Line),
            node(50., 50., PointKind::Line),
        ],
        ..Contour::default()
    };
    assert_eq!(
        open_corner(&degenerate, 1, 5.),
        Err(CornerError::DegenerateNode(1))
    );
}

#[test]
fn reconnect_rejects_nodes_that_are_not_one_open_corner() {
    let opened = open_corner(&triangle(), 1, 10.).expect("corner opens");
    assert_eq!(
        reconnect_nodes(&opened, 1, 1),
        Err(CornerError::NotOneSegment(1, 1))
    );
    assert_eq!(
        reconnect_nodes(&opened, 1, 3),
        Err(CornerError::NotOneSegment(1, 3))
    );
    // Reversed order is not the same edit: 2 is not followed by 1.
    assert_eq!(
        reconnect_nodes(&opened, 2, 1),
        Err(CornerError::NotOneSegment(2, 1))
    );
    let straight = Contour {
        closed: true,
        points: vec![
            node(0., 0., PointKind::Line),
            node(100., 0., PointKind::Line),
            node(200., 0., PointKind::Line),
            node(300., 0., PointKind::Line),
        ],
        ..Contour::default()
    };
    // 1 and 2 lie on one straight run, so their tangents never meet.
    assert_eq!(
        reconnect_nodes(&straight, 1, 2),
        Err(CornerError::ParallelTangents(1, 2))
    );
    let wedge = Contour {
        closed: true,
        points: vec![
            node(0., 0., PointKind::Line),
            node(100., 0., PointKind::Line),
            node(200., 0., PointKind::Line),
            node(200., 100., PointKind::Line),
        ],
        ..Contour::default()
    };
    // The tangents of 1 and 2 meet ahead of 1, which is the wrong topology.
    assert_eq!(
        reconnect_nodes(&wedge, 1, 2),
        Err(CornerError::NoCornerAhead(1, 2))
    );
    // A closed triangle's own corners would merge onto their shared neighbour.
    assert_eq!(
        reconnect_nodes(&triangle(), 0, 1),
        Err(CornerError::DegenerateResult(0, 1))
    );
}

/// A chevron whose vertex at index 2 points into the ink (a reflex corner).
fn chevron() -> Contour {
    Contour {
        closed: true,
        points: vec![
            node(0., 0., PointKind::Line),
            node(100., 0., PointKind::Line),
            node(50., 60., PointKind::Line),
            node(100., 120., PointKind::Line),
            node(0., 120., PointKind::Line),
        ],
        ..Contour::default()
    }
}

#[test]
fn removing_the_overlap_of_an_open_reflex_corner_restores_the_sharp_corner() {
    let opened = open_corner(&chevron(), 2, 10.).expect("reflex corner opens");
    assert_eq!(opened.points.len(), 6);
    let merged = fc_geom::remove_overlap(
        std::slice::from_ref(&opened),
        fc_geom::CurveKind::Cubic,
        fc_geom::DEFAULT_BOOLEAN_TOLERANCE,
    )
    .expect("the overlapping construction is a valid filled shape");
    assert_eq!(merged.len(), 1, "expected one region, got {merged:?}");
    let points = &merged[0].points;
    assert_eq!(
        points.len(),
        5,
        "expected the original chevron, got {points:?}"
    );
    assert!(
        points
            .iter()
            .any(|point| (point.x - 50.).abs() < 0.05 && (point.y - 60.).abs() < 0.05),
        "expected the original corner at (50, 60), got {points:?}"
    );
    // And the edit is still exactly reversible without any boolean operation.
    let closed = reconnect_nodes(&opened, 2, 3).expect("reflex corner reconnects");
    assert_eq!(closed.points.len(), 5);
    assert!((closed.points[2].x - 50.).abs() < 1e-9);
    assert!((closed.points[2].y - 60.).abs() < 1e-9);
}

#[test]
fn a_convex_corner_opens_and_reconnects_but_its_mouth_stays_outside_the_ink() {
    // Documented limit: only a reflex corner can cross its own contour, so the
    // triangle of a convex open corner survives overlap removal as an ear. The
    // construction is still exactly reversible, which is what Reconnect is for.
    let opened = open_corner(&triangle(), 1, 15.).expect("convex corner opens");
    let merged = fc_geom::remove_overlap(
        std::slice::from_ref(&opened),
        fc_geom::CurveKind::Cubic,
        fc_geom::DEFAULT_BOOLEAN_TOLERANCE,
    )
    .expect("the construction is a valid filled shape");
    assert_eq!(merged.len(), 2, "expected the triangle plus its ear");
    let closed = reconnect_nodes(&opened, 1, 2).expect("convex corner reconnects");
    assert_eq!(closed.points, triangle().points);
}
