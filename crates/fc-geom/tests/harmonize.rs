use fc_geom::harmonize_nodes;
use fc_model::{Contour, Point, PointKind};

fn point(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        x,
        y,
        kind,
        ..Point::default()
    }
}

fn two_cubics() -> Contour {
    Contour {
        points: vec![
            point(-100., 100., PointKind::Move),
            point(-60., 80., PointKind::OffCurve),
            point(-20., 0., PointKind::OffCurve),
            point(0., 0., PointKind::Curve),
            point(40., 0., PointKind::OffCurve),
            point(80., 20., PointKind::OffCurve),
            point(100., 100., PointKind::Curve),
        ],
        ..Contour::default()
    }
}

fn curvature(contour: &Contour, node: usize) -> (f64, f64) {
    let len = contour.points.len();
    let p = &contour.points[node];
    let before = &contour.points[(node + len - 1) % len];
    let outer_before = &contour.points[(node + len - 2) % len];
    let after = &contour.points[(node + 1) % len];
    let outer_after = &contour.points[(node + 2) % len];
    let incoming = (p.x - before.x, p.y - before.y);
    let outgoing = (after.x - p.x, after.y - p.y);
    let cross = |a: (f64, f64), b: (f64, f64)| a.0 * b.1 - a.1 * b.0;
    (
        2. / 3. * cross(incoming, (outer_before.x - p.x, outer_before.y - p.y))
            / incoming.0.hypot(incoming.1).powi(3),
        2. / 3. * cross(outgoing, (outer_after.x - p.x, outer_after.y - p.y))
            / outgoing.0.hypot(outgoing.1).powi(3),
    )
}

fn assert_g2(contour: &Contour, node: usize) {
    let (a, b) = curvature(contour, node);
    assert!(
        (a - b).abs() <= 1e-8 * a.abs().max(b.abs()).max(1e-10),
        "{a} != {b}"
    );
}

#[test]
fn equalizes_signed_curvature_and_preserves_nodes_and_metadata() {
    let mut contour = two_cubics();
    contour.name = Some("source".into());
    contour.points[3].name = Some("join".into());
    let source = contour.clone();
    harmonize_nodes(&mut contour, &[3, 3]).unwrap();
    assert_g2(&contour, 3);
    assert!((contour.points[2].x + 40.).abs() < 1e-8);
    assert!((contour.points[4].x - 20.).abs() < 1e-8);
    assert!(contour.points[3].smooth);
    assert_eq!(contour.name, source.name);
    assert_eq!(contour.points[3].name, source.points[3].name);
    for index in [0, 1, 5, 6] {
        assert_eq!(contour.points[index], source.points[index]);
    }
    assert_eq!((contour.points[3].x, contour.points[3].y), (0., 0.));
}

#[test]
fn aligns_handles_and_supports_closed_seams() {
    let mut contour = two_cubics();
    contour.points[2].y = -5.;
    contour.points[4].y = -5.;
    contour.closed = true;
    contour.points[0].kind = PointKind::Curve;
    contour.points.push(point(100., 200., PointKind::OffCurve));
    contour.points.push(point(-100., 200., PointKind::OffCurve));
    contour.points.rotate_left(3);
    harmonize_nodes(&mut contour, &[0]).unwrap();
    assert_g2(&contour, 0);
    let a = &contour.points[contour.points.len() - 1];
    let b = &contour.points[1];
    assert!((a.x * b.y - a.y * b.x).abs() < 1e-8);
    assert!(a.x * b.x + a.y * b.y < 0.);
    let accepted = contour.clone();
    harmonize_nodes(&mut contour, &[0]).unwrap();
    assert_eq!(contour, accepted);
}

#[test]
fn neighboring_selected_joins_remain_g2_together() {
    let mut contour = two_cubics();
    contour.points[1] = point(-80., 100., PointKind::OffCurve);
    contour.points[5] = point(100., 40., PointKind::OffCurve);
    contour.points.extend([
        point(100., 120., PointKind::OffCurve),
        point(20., 200., PointKind::OffCurve),
        point(0., 200., PointKind::Curve),
    ]);
    harmonize_nodes(&mut contour, &[3, 6]).unwrap();
    assert_g2(&contour, 3);
    assert_g2(&contour, 6);
}

#[test]
fn rejects_invalid_non_cubic_degenerate_and_inflexion_joins_atomically() {
    for case in 0..6 {
        let mut contour = two_cubics();
        let nodes = match case {
            0 => vec![3, 99],
            1 => vec![3, 0],
            2 => {
                contour.points[3].kind = PointKind::QCurve;
                vec![3]
            }
            3 => {
                contour.points[2].x = 0.;
                vec![3]
            }
            4 => {
                contour.points[5].y = -20.;
                vec![3]
            }
            _ => {
                contour.points[2].x = f64::INFINITY;
                vec![3]
            }
        };
        let source = contour.clone();
        assert!(
            harmonize_nodes(&mut contour, &nodes).is_err(),
            "case {case}"
        );
        assert_eq!(contour, source);
    }
}

#[test]
fn straight_joins_and_empty_selection_are_stable() {
    let mut contour = two_cubics();
    contour.points[1].y = 0.;
    contour.points[5].y = 0.;
    harmonize_nodes(&mut contour, &[3]).unwrap();
    assert_g2(&contour, 3);
    let source = contour.clone();
    harmonize_nodes(&mut contour, &[]).unwrap();
    assert_eq!(source, contour);
}
