mod common;

use fc_doc::{Command, PathOp, PointRef, Session};
use fc_model::{Contour, LayerId, LayerKind, MasterId, Point, PointKind};

fn curved_closed() -> Contour {
    Contour {
        closed: true,
        points: vec![
            Point {
                id: Some("corner".into()),
                kind: PointKind::Curve,
                ..Point::default()
            },
            Point {
                x: 100.,
                kind: PointKind::Line,
                ..Point::default()
            },
            Point {
                x: 100.,
                y: 100.,
                kind: PointKind::Line,
                ..Point::default()
            },
            Point {
                x: 60.,
                y: 120.,
                kind: PointKind::OffCurve,
                ..Point::default()
            },
            Point {
                x: -20.,
                y: 60.,
                kind: PointKind::OffCurve,
                ..Point::default()
            },
        ],
        ..Contour::default()
    }
}

#[test]
fn closed_split_preserves_every_segment_and_reconnects_with_history_and_persistence() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let original = curved_closed();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![original.clone()];
            let before = doc.font.clone();
            doc.apply(
                Command::SplitContour {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    point: PointRef {
                        contour: 0,
                        point: 0,
                    },
                },
                "Break contour",
            )
            .unwrap();
            let split = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            assert!(!split.closed);
            assert_eq!(split.points.len(), original.points.len() + 1);
            assert_eq!(
                split
                    .points
                    .iter()
                    .filter(|point| point.id.is_some())
                    .count(),
                1
            );
            assert_eq!(
                split.to_bezpath().segments().collect::<Vec<_>>(),
                original.to_bezpath().segments().collect::<Vec<_>>()
            );
            let last = split.points.len() - 1;
            let opened = doc.font.clone();
            doc.apply(
                Command::JoinContours {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    a: PointRef {
                        contour: 0,
                        point: last,
                    },
                    b: PointRef {
                        contour: 0,
                        point: 0,
                    },
                },
                "Reconnect nodes",
            )
            .unwrap();
            assert_eq!(doc.font, before);
            doc.undo().unwrap();
            assert_eq!(doc.font, opened);
            doc.redo().unwrap();
            assert_eq!(doc.font, before);
            let directory = common::temp_dir("path-operations");
            let path = directory.join("reconnected.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), before);
            std::fs::remove_dir_all(directory).unwrap();
        })
        .unwrap();
}

#[test]
fn joining_two_starts_reverses_the_first_contour_and_rejects_interior_points() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let contour = |offset: f64| Contour {
                closed: false,
                points: vec![
                    Point {
                        x: offset,
                        kind: PointKind::Move,
                        ..Point::default()
                    },
                    Point {
                        x: offset + 20.,
                        ..Point::default()
                    },
                    Point {
                        x: offset + 40.,
                        ..Point::default()
                    },
                ],
                ..Contour::default()
            };
            doc.font.glyph_mut("base").unwrap().layers[0].contours =
                vec![contour(0.), contour(100.)];
            let before = doc.font.clone();
            let command = |point| Command::JoinContours {
                glyph: "base".into(),
                layer: layer.clone(),
                a: PointRef { contour: 0, point },
                b: PointRef {
                    contour: 1,
                    point: 0,
                },
            };
            assert!(doc.apply(command(1), "Invalid join").is_err());
            assert_eq!(doc.font, before);
            doc.apply(command(0), "Reconnect starts").unwrap();
            let contours = &doc.font.glyph("base").unwrap().layers[0].contours;
            assert_eq!(contours.len(), 1);
            assert_eq!(
                contours[0].points.iter().map(|p| p.x).collect::<Vec<_>>(),
                vec![40., 20., 0., 100., 120., 140.]
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
        })
        .unwrap();
}

#[test]
fn forced_extrema_and_master_batches_are_atomic_and_undoable() {
    let mut font = common::test_font();
    let mut second = font.glyph("base").unwrap().layers[0].clone();
    let mut second_master = font.default_master().clone();
    second_master.id = MasterId::new();
    second.kind = LayerKind::Master(second_master.id.clone());
    font.masters.push(second_master);
    second.id = LayerId::new();
    font.glyph_mut("base").unwrap().layers.push(second);
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            let layers: Vec<_> = doc
                .font
                .glyph("base")
                .unwrap()
                .layers
                .iter()
                .map(|layer| layer.id.clone())
                .collect();
            let commands = layers
                .iter()
                .flat_map(|layer| {
                    [PathOp::CorrectDirection, PathOp::CanonicalOrder].map(|op| Command::PathOp {
                        glyph: "base".into(),
                        layer: layer.clone(),
                        op,
                    })
                })
                .collect();
            doc.apply(Command::Batch(commands), "Correct all masters")
                .unwrap();
            let after = doc.font.clone();
            assert_ne!(before, after);
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
            let mut path = kurbo::BezPath::new();
            path.move_to((0., 0.));
            path.curve_to((-0.001, 0.), (100., 0.), (200., 0.));
            doc.font.glyph_mut("base").unwrap().layers[0].contours = Contour::from_bezpath(&path);
            let before_force = doc.font.clone();
            doc.apply(
                Command::PathOp {
                    glyph: "base".into(),
                    layer: layers[0].clone(),
                    op: PathOp::ForceExtrema,
                },
                "Force extremes",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours[0]
                    .points
                    .len(),
                7
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, before_force);
        })
        .unwrap();
}

#[test]
fn splitting_a_live_smart_corner_fails_without_creating_invalid_open_endpoints() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours[0].points[0].corner_radius = 10.;
            let before = doc.font.clone();
            let result = doc.apply(
                Command::SplitContour {
                    glyph: "base".into(),
                    layer,
                    point: PointRef {
                        contour: 0,
                        point: 0,
                    },
                },
                "Break live corner",
            );
            assert!(result.unwrap_err().to_string().contains("smart corner"));
            assert_eq!(doc.font, before);
        })
        .unwrap();
}

/// A chevron whose node 2 points into the ink, which is where an open corner
/// actually overlaps: the tangents extend past the corner and cross.
fn reflex_chevron() -> Contour {
    let node = |x: f64, y: f64| Point {
        x,
        y,
        kind: PointKind::Line,
        ..Point::default()
    };
    Contour {
        closed: true,
        points: vec![
            node(0., 0.),
            node(100., 0.),
            Point {
                id: Some("corner".into()),
                ..node(50., 60.)
            },
            node(100., 120.),
            node(0., 120.),
        ],
        ..Contour::default()
    }
}

#[test]
fn opening_and_reconnecting_a_corner_round_trips_through_history_and_fcf() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![reflex_chevron()];
            let before = doc.font.clone();
            doc.apply(
                Command::OpenCorner {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    point: PointRef {
                        contour: 0,
                        point: 2,
                    },
                    depth: 10.,
                },
                "Open corner",
            )
            .unwrap();
            let opened = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            assert_eq!(opened.points.len(), 6);
            // The moved node keeps its identity; the mouth's second node is new.
            assert_eq!(opened.points[2].id.as_deref(), Some("corner"));
            assert!(opened.points[3].id.is_none());
            assert_eq!(opened.points[3].kind, PointKind::Line);
            let opened_font = doc.font.clone();

            // One undo unit per gesture, and the native file round trips.
            let directory = common::temp_dir("open-corner");
            let path = directory.join("open-corner.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), opened_font);
            std::fs::remove_dir_all(directory).unwrap();

            doc.apply(
                Command::ReconnectNodes {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    a: PointRef {
                        contour: 0,
                        point: 2,
                    },
                    b: PointRef {
                        contour: 0,
                        point: 3,
                    },
                },
                "Reconnect nodes",
            )
            .unwrap();
            let closed = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            assert_eq!(closed.points.len(), 5);
            assert!((closed.points[2].x - 50.).abs() < 1e-9);
            assert!((closed.points[2].y - 60.).abs() < 1e-9);
            assert_eq!(closed.points[2].id.as_deref(), Some("corner"));
            doc.undo().unwrap();
            assert_eq!(doc.font, opened_font);
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, opened_font);
        })
        .unwrap();
}

#[test]
fn an_invalid_corner_edit_leaves_the_layer_untouched() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![reflex_chevron()];
            let before = doc.font.clone();
            let open = |point: usize, depth: f64| Command::OpenCorner {
                glyph: "base".into(),
                layer: layer.clone(),
                point: PointRef { contour: 0, point },
                depth,
            };
            // A setback longer than an adjacent segment, a non-finite setback
            // and an out-of-range node are all refused atomically.
            assert!(doc.apply(open(2, 10_000.), "Open corner").is_err());
            assert!(doc.apply(open(2, f64::INFINITY), "Open corner").is_err());
            assert!(doc.apply(open(2, -5.), "Open corner").is_err());
            assert!(doc.apply(open(9, 10.), "Open corner").is_err());
            assert_eq!(doc.font, before);

            // A live smart corner owns the join, so it has to be resolved first.
            doc.font.glyph_mut("base").unwrap().layers[0].contours[0].points[2].corner_radius = 8.;
            let with_smart = doc.font.clone();
            assert!(doc.apply(open(2, 10.), "Open corner").is_err());
            assert_eq!(doc.font, with_smart);
            doc.font.glyph_mut("base").unwrap().layers[0].contours[0].points[2].corner_radius = 0.;

            // Reconnecting needs the two ends of one segment of one contour.
            let reconnect = |a: (usize, usize), b: (usize, usize)| Command::ReconnectNodes {
                glyph: "base".into(),
                layer: layer.clone(),
                a: PointRef {
                    contour: a.0,
                    point: a.1,
                },
                b: PointRef {
                    contour: b.0,
                    point: b.1,
                },
            };
            assert!(doc.apply(reconnect((0, 1), (0, 3)), "Reconnect").is_err());
            assert!(doc.apply(reconnect((0, 0), (1, 0)), "Reconnect").is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.undo_label(), None);
        })
        .unwrap();
}

#[test]
fn every_master_can_be_opened_in_one_undo_unit() {
    let mut font = common::test_font();
    let mut second = font.glyph("base").unwrap().layers[0].clone();
    let mut second_master = font.default_master().clone();
    second_master.id = MasterId::new();
    second.kind = LayerKind::Master(second_master.id.clone());
    font.masters.push(second_master);
    second.id = LayerId::new();
    font.glyph_mut("base").unwrap().layers.push(second);
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            for layer in &mut doc.font.glyph_mut("base").unwrap().layers {
                layer.contours = vec![reflex_chevron()];
            }
            let before = doc.font.clone();
            let layers: Vec<LayerId> = doc
                .font
                .glyph("base")
                .unwrap()
                .layers
                .iter()
                .map(|layer| layer.id.clone())
                .collect();
            let commands = layers
                .iter()
                .map(|layer| Command::OpenCorner {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    point: PointRef {
                        contour: 0,
                        point: 2,
                    },
                    depth: 12.,
                })
                .collect();
            doc.apply(Command::Batch(commands), "Open corner in all masters")
                .unwrap();
            for layer in &doc.font.glyph("base").unwrap().layers {
                assert_eq!(layer.contours[0].points.len(), 6);
            }
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
        })
        .unwrap();
}
