mod common;

use fc_doc::{Command, HandleMode, PointRef, Session};
use fc_model::{Contour, Point, PointKind};

fn fixture() -> Contour {
    let point = |x, y, kind| Point {
        x,
        y,
        kind,
        ..Point::default()
    };
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

fn join() -> Vec<PointRef> {
    vec![PointRef {
        contour: 0,
        point: 3,
    }]
}

fn assert_g2(contour: &Contour) {
    let p = &contour.points;
    let cross = |a: (f64, f64), b: (f64, f64)| a.0 * b.1 - a.1 * b.0;
    let incoming = (p[3].x - p[2].x, p[3].y - p[2].y);
    let outgoing = (p[4].x - p[3].x, p[4].y - p[3].y);
    assert!(cross(incoming, outgoing).abs() < 1e-8);
    let a =
        cross(incoming, (p[1].x - p[3].x, p[1].y - p[3].y)) / incoming.0.hypot(incoming.1).powi(3);
    let b =
        cross(outgoing, (p[5].x - p[3].x, p[5].y - p[3].y)) / outgoing.0.hypot(outgoing.1).powi(3);
    assert!((a - b).abs() < 1e-10);
}

#[test]
fn harmonize_is_one_undoable_operation_without_a_persistent_constraint() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![fixture()];
            let before = doc.font.clone();
            doc.apply(
                Command::HarmonizeNodes {
                    glyph: "base".into(),
                    layer,
                    points: join(),
                },
                "Harmonize",
            )
            .unwrap();
            let contour = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            assert_g2(contour);
            assert!(!contour.points[3].super_smooth);
            let after = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
        })
        .unwrap();
}

#[test]
fn super_smooth_survives_drag_absolute_edits_replace_and_fcf() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![fixture()];
            doc.apply(
                Command::SetSuperSmooth {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    points: join(),
                    enabled: true,
                },
                "Super smooth",
            )
            .unwrap();
            doc.apply(
                Command::MovePoints {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    points: vec![PointRef {
                        contour: 0,
                        point: 1,
                    }],
                    delta: (0., 20.),
                    handles: HandleMode::PointsOnly,
                },
                "Move outer control",
            )
            .unwrap();
            assert_g2(&doc.font.glyph("base").unwrap().layers[0].contours[0]);
            doc.apply(
                Command::SetPointPosition {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    point: PointRef {
                        contour: 0,
                        point: 5,
                    },
                    x: 80.,
                    y: 50.,
                },
                "Position outer control",
            )
            .unwrap();
            let target = &doc.font.glyph("base").unwrap().layers[0];
            assert_g2(&target.contours[0]);
            let mut contours = target.contours.clone();
            contours[0].points[1].y = 120.;
            doc.apply(
                Command::ReplaceLayerOutline {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    contours,
                    components: vec![],
                    anchors: vec![],
                },
                "Replace",
            )
            .unwrap();
            let contour = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            assert_g2(contour);
            assert!(contour.points[3].super_smooth);
            let dir = common::temp_dir("super-smooth-fcf");
            let path = dir.join("roundtrip.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            let read = fc_doc::fcf::read(&path).unwrap();
            std::fs::remove_dir_all(dir).unwrap();
            assert_eq!(read.glyph("base").unwrap().layers[0].contours[0], *contour);
            let coordinates: Vec<_> = contour
                .points
                .iter()
                .map(|point| (point.x, point.y))
                .collect();
            doc.apply(
                Command::SetSmooth {
                    glyph: "base".into(),
                    layer,
                    points: join(),
                    smooth: false,
                    level_handles: false,
                },
                "Corner preserving outline",
            )
            .unwrap();
            let contour = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            assert!(!contour.points[3].super_smooth);
            assert!(!contour.points[3].smooth);
            assert_eq!(
                coordinates,
                contour
                    .points
                    .iter()
                    .map(|point| (point.x, point.y))
                    .collect::<Vec<_>>()
            );
        })
        .unwrap();
}

#[test]
fn invalid_harmonization_is_atomic_and_creates_no_undo_step() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![fixture()];
            let before = doc.font.clone();
            let undo = doc.undo_len();
            assert!(doc
                .apply(
                    Command::SetSuperSmooth {
                        glyph: "base".into(),
                        layer,
                        enabled: true,
                        points: vec![
                            PointRef {
                                contour: 0,
                                point: 3
                            },
                            PointRef {
                                contour: 0,
                                point: 0
                            }
                        ],
                    },
                    "Invalid"
                )
                .is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.undo_len(), undo);
        })
        .unwrap();
}

#[test]
fn ordinary_smooth_keep_outline_only_changes_the_flag() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let mut contour = fixture();
            contour.points[2].y = 10.;
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![contour.clone()];
            doc.apply(
                Command::SetSmooth {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    points: join(),
                    smooth: true,
                    level_handles: false,
                },
                "Smooth preserving outline",
            )
            .unwrap();
            contour.points[3].smooth = true;
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours[0],
                contour
            );
            doc.apply(
                Command::SetSmooth {
                    glyph: "base".into(),
                    layer,
                    points: join(),
                    smooth: false,
                    level_handles: true,
                },
                "Corner",
            )
            .unwrap();
            contour.points[3].smooth = false;
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours[0],
                contour
            );
        })
        .unwrap();
}
