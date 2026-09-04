mod common;

use fw_doc::{
    Command, CurveKind, DeleteMode, Error, HandleMode, PathOp, PointRef, SegmentKind, Session,
    SimplifyOptions, StrokeOptions,
};
use fw_model::{Contour, GlyphName, Point, PointKind};
use kurbo::{Affine, ParamCurve};

use common::test_font;

fn cubic_contour() -> Contour {
    Contour {
        closed: false,
        points: vec![
            Point {
                x: 0.0,
                y: 0.0,
                kind: PointKind::Move,
                ..Point::default()
            },
            Point {
                x: 30.0,
                y: 100.0,
                kind: PointKind::OffCurve,
                ..Point::default()
            },
            Point {
                x: 70.0,
                y: 100.0,
                kind: PointKind::OffCurve,
                ..Point::default()
            },
            Point {
                x: 100.0,
                y: 0.0,
                kind: PointKind::Curve,
                ..Point::default()
            },
        ],
        ..Contour::default()
    }
}

#[test]
fn move_points_carries_unselected_handles_and_undoes_exactly() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![cubic_contour()];
            let before = doc.font.glyph("base").unwrap().clone();
            doc.apply(
                Command::MovePoints {
                    glyph: "base".into(),
                    layer,
                    points: vec![PointRef {
                        contour: 0,
                        point: 3,
                    }],
                    delta: (10.0, 20.0),
                    handles: HandleMode::WithHandles,
                },
                "Move",
            )
            .unwrap();
            let points = &doc.font.glyph("base").unwrap().layers[0].contours[0].points;
            assert_eq!((points[2].x, points[2].y), (80.0, 120.0));
            assert_eq!((points[3].x, points[3].y), (110.0, 20.0));
            assert_eq!((points[1].x, points[1].y), (30.0, 100.0));
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap(), &before);
        })
        .unwrap();
}

#[test]
fn insert_then_keep_shape_delete_round_trips_within_one_unit() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![cubic_contour()];
            let original = cubic_contour().to_bezpath().segments().next().unwrap();
            doc.apply(
                Command::InsertPointOnSegment {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    contour: 0,
                    segment: 0,
                    t: 0.5,
                },
                "Insert",
            )
            .unwrap();
            let inserted = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            let point = inserted
                .points
                .iter()
                .position(|point| {
                    point.kind != PointKind::OffCurve && point.x > 0.0 && point.x < 100.0
                })
                .unwrap();
            doc.apply(
                Command::DeletePoints {
                    glyph: "base".into(),
                    layer,
                    points: vec![PointRef { contour: 0, point }],
                    mode: DeleteMode::KeepShape,
                },
                "Delete",
            )
            .unwrap();
            let result = doc.font.glyph("base").unwrap().layers[0].contours[0]
                .to_bezpath()
                .segments()
                .next()
                .unwrap();
            for step in 0..=100 {
                let t = f64::from(step) / 100.0;
                assert!((original.eval(t) - result.eval(t)).hypot() <= 1.0);
            }
        })
        .unwrap();
}

#[test]
fn basic_outline_commands_apply_and_restore_snapshots() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let commands = vec![
                Command::SetPointPosition {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    point: PointRef {
                        contour: 0,
                        point: 0,
                    },
                    x: -10.0,
                    y: -20.0,
                },
                Command::AddContour {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    contour: cubic_contour(),
                },
                Command::ReverseContours {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    contours: vec![0],
                },
                Command::SetStartPoint {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    point: PointRef {
                        contour: 0,
                        point: 2,
                    },
                },
                Command::TransformSelection {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    points: Vec::new(),
                    components: Vec::new(),
                    anchors: Vec::new(),
                    affine: Affine::translate((5.0, 7.0)).as_coeffs(),
                },
                Command::PathOp {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    op: PathOp::RoundToGrid(10.0),
                },
                Command::DeleteContours {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    contours: vec![0],
                },
            ];
            for command in commands {
                let before = doc.font.glyph(&glyph).unwrap().clone();
                doc.apply(command, "Outline command").unwrap();
                doc.undo().unwrap();
                assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
            }
        })
        .unwrap();
}

#[test]
fn convert_join_split_and_break_commands_are_structurally_valid() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            doc.font.glyph_mut(&glyph).unwrap().layers[0].contours = vec![
                Contour {
                    closed: false,
                    points: vec![
                        Point {
                            x: 0.0,
                            y: 0.0,
                            kind: PointKind::Move,
                            ..Point::default()
                        },
                        Point {
                            x: 50.0,
                            y: 0.0,
                            kind: PointKind::Line,
                            ..Point::default()
                        },
                    ],
                    ..Contour::default()
                },
                Contour {
                    closed: false,
                    points: vec![
                        Point {
                            x: 50.0,
                            y: 0.0,
                            kind: PointKind::Move,
                            ..Point::default()
                        },
                        Point {
                            x: 100.0,
                            y: 0.0,
                            kind: PointKind::Line,
                            ..Point::default()
                        },
                    ],
                    ..Contour::default()
                },
            ];
            doc.apply(
                Command::JoinContours {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    a: PointRef {
                        contour: 0,
                        point: 1,
                    },
                    b: PointRef {
                        contour: 1,
                        point: 0,
                    },
                },
                "Join",
            )
            .unwrap();
            assert_eq!(doc.font.glyph(&glyph).unwrap().layers[0].contours.len(), 1);
            doc.apply(
                Command::SplitContour {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    point: PointRef {
                        contour: 0,
                        point: 1,
                    },
                },
                "Split",
            )
            .unwrap();
            assert_eq!(doc.font.glyph(&glyph).unwrap().layers[0].contours.len(), 2);
            doc.undo().unwrap();
            doc.apply(
                Command::ConvertSegment {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    contour: 0,
                    segment: 0,
                    to: SegmentKind::Curve,
                },
                "Curve",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph(&glyph).unwrap().layers[0].contours[0]
                    .points
                    .len(),
                5
            );
            doc.apply(
                Command::ConvertSegment {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    contour: 0,
                    segment: 0,
                    to: SegmentKind::Line,
                },
                "Line",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph(&glyph).unwrap().layers[0].contours[0]
                    .points
                    .len(),
                3
            );
            doc.font.glyph_mut(&glyph).unwrap().layers[0].contours[0].closed = true;
            doc.apply(
                Command::DeletePoints {
                    glyph,
                    layer,
                    points: vec![PointRef {
                        contour: 0,
                        point: 0,
                    }],
                    mode: DeleteMode::BreakContour,
                },
                "Break",
            )
            .unwrap();
        })
        .unwrap();
}

#[test]
fn smooth_leveling_and_handle_deletion_follow_node_semantics() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let mut contour = cubic_contour();
            contour.closed = true;
            contour.points.push(Point {
                x: 130.0,
                y: -50.0,
                kind: PointKind::OffCurve,
                ..Point::default()
            });
            contour.points.push(Point {
                x: -30.0,
                y: -50.0,
                kind: PointKind::OffCurve,
                ..Point::default()
            });
            doc.font.glyph_mut(&glyph).unwrap().layers[0].contours = vec![contour];
            doc.apply(
                Command::SetSmooth {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    points: vec![PointRef {
                        contour: 0,
                        point: 0,
                    }],
                    smooth: true,
                    level_handles: true,
                },
                "Smooth",
            )
            .unwrap();
            let contour = &doc.font.glyph(&glyph).unwrap().layers[0].contours[0];
            assert!(contour.points[0].smooth);
            let before = (contour.points[5].x, contour.points[5].y);
            doc.apply(
                Command::DeletePoints {
                    glyph,
                    layer,
                    points: vec![PointRef {
                        contour: 0,
                        point: 5,
                    }],
                    mode: DeleteMode::KeepShape,
                },
                "Delete handle",
            )
            .unwrap();
            let contour = &doc.font.glyph("base").unwrap().layers[0].contours[0];
            assert!(!contour
                .points
                .iter()
                .any(|point| (point.x, point.y) == before));
        })
        .unwrap();
}

#[test]
fn every_path_operation_is_undoable_and_geometry_errors_are_atomic() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let operations = vec![
                PathOp::RemoveOverlap,
                PathOp::CorrectDirection,
                PathOp::AddExtrema,
                PathOp::AddInflections,
                PathOp::Simplify(SimplifyOptions::default()),
                PathOp::RoundToGrid(5.0),
                PathOp::CanonicalStart,
                PathOp::CanonicalOrder,
                PathOp::ExpandStroke(StrokeOptions {
                    width: 10.0,
                    ..StrokeOptions::default()
                }),
                PathOp::ConvertCurves(CurveKind::Quadratic { max_err: 0.1 }),
            ];
            for op in operations {
                let before = doc.font.glyph(&glyph).unwrap().clone();
                doc.apply(
                    Command::PathOp {
                        glyph: glyph.clone(),
                        layer: layer.clone(),
                        op,
                    },
                    "Path",
                )
                .unwrap();
                doc.undo().unwrap();
                assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
            }

            let before = doc.font.glyph(&glyph).unwrap().clone();
            let error = doc
                .apply(
                    Command::PathOp {
                        glyph: glyph.clone(),
                        layer,
                        op: PathOp::ExpandStroke(StrokeOptions {
                            width: 0.0,
                            ..StrokeOptions::default()
                        }),
                    },
                    "Invalid stroke",
                )
                .unwrap_err();
            assert!(matches!(error, Error::Geometry(_)));
            assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
        })
        .unwrap();
}

#[test]
fn move_points_moves_selected_handles_in_both_modes() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.font.glyph_mut("base").unwrap().layers[0].contours = vec![cubic_contour()];
            let before = doc.font.glyph("base").unwrap().clone();

            doc.apply(
                Command::MovePoints {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    points: vec![PointRef {
                        contour: 0,
                        point: 1,
                    }],
                    delta: (5.0, -7.0),
                    handles: HandleMode::PointsOnly,
                },
                "Drag handle",
            )
            .unwrap();
            let points = &doc.font.glyph("base").unwrap().layers[0].contours[0].points;
            assert_eq!((points[1].x, points[1].y), (35.0, 93.0));
            assert_eq!((points[0].x, points[0].y), (0.0, 0.0));
            assert_eq!((points[2].x, points[2].y), (70.0, 100.0));
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap(), &before);

            doc.apply(
                Command::MovePoints {
                    glyph: "base".into(),
                    layer,
                    points: vec![
                        PointRef {
                            contour: 0,
                            point: 1,
                        },
                        PointRef {
                            contour: 0,
                            point: 3,
                        },
                    ],
                    delta: (10.0, 0.0),
                    handles: HandleMode::WithHandles,
                },
                "Drag node and handle",
            )
            .unwrap();
            let points = &doc.font.glyph("base").unwrap().layers[0].contours[0].points;
            assert_eq!((points[0].x, points[0].y), (0.0, 0.0));
            assert_eq!((points[1].x, points[1].y), (40.0, 100.0));
            assert_eq!((points[2].x, points[2].y), (80.0, 100.0));
            assert_eq!((points[3].x, points[3].y), (110.0, 0.0));
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap(), &before);
        })
        .unwrap();
}

/// A closed TrueType-style contour: three nodes, one handle per segment.
fn quadratic_triangle() -> Contour {
    Contour {
        closed: true,
        points: vec![
            Point {
                x: 0.0,
                y: 0.0,
                kind: PointKind::QCurve,
                ..Point::default()
            },
            Point {
                x: 50.0,
                y: -40.0,
                kind: PointKind::OffCurve,
                ..Point::default()
            },
            Point {
                x: 100.0,
                y: 0.0,
                kind: PointKind::QCurve,
                ..Point::default()
            },
            Point {
                x: 60.0,
                y: 60.0,
                kind: PointKind::OffCurve,
                ..Point::default()
            },
            Point {
                x: 50.0,
                y: 100.0,
                kind: PointKind::QCurve,
                ..Point::default()
            },
            Point {
                x: -20.0,
                y: 50.0,
                kind: PointKind::OffCurve,
                ..Point::default()
            },
        ],
        ..Contour::default()
    }
}

#[test]
fn convert_segment_keeps_the_start_point_and_honours_the_layer_curve_kind() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            doc.font.glyph_mut(&glyph).unwrap().layers[0].contours = vec![quadratic_triangle()];

            doc.apply(
                Command::ConvertSegment {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    contour: 0,
                    segment: 2,
                    to: SegmentKind::Line,
                },
                "Line",
            )
            .unwrap();
            let contour = &doc.font.glyph(&glyph).unwrap().layers[0].contours[0];
            assert_eq!((contour.points[0].x, contour.points[0].y), (0.0, 0.0));
            assert_eq!(contour.points.len(), 5);
            assert_eq!(contour.points[0].kind, PointKind::Line);

            doc.apply(
                Command::ConvertSegment {
                    glyph: glyph.clone(),
                    layer,
                    contour: 0,
                    segment: 2,
                    to: SegmentKind::Curve,
                },
                "Curve",
            )
            .unwrap();
            let contour = &doc.font.glyph(&glyph).unwrap().layers[0].contours[0];
            assert_eq!((contour.points[0].x, contour.points[0].y), (0.0, 0.0));
            assert_eq!(contour.points.len(), 6);
            assert_eq!(contour.points[0].kind, PointKind::QCurve);
            assert_eq!(
                (contour.points[5].x, contour.points[5].y),
                (25.0, 50.0),
                "a quadratic layer gets one handle at the chord midpoint"
            );
            assert!(!contour
                .points
                .iter()
                .any(|point| point.kind == PointKind::Curve));
            contour.try_to_bezpath().unwrap();
        })
        .unwrap();
}

#[test]
fn degenerate_contours_are_rejected_instead_of_panicking() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let single = Contour {
                closed: true,
                points: vec![Point {
                    x: 10.0,
                    y: 20.0,
                    kind: PointKind::Line,
                    ..Point::default()
                }],
                ..Contour::default()
            };
            let handles_only = Contour {
                closed: true,
                points: vec![
                    Point {
                        x: 0.0,
                        y: 0.0,
                        kind: PointKind::OffCurve,
                        ..Point::default()
                    },
                    Point {
                        x: 50.0,
                        y: 80.0,
                        kind: PointKind::OffCurve,
                        ..Point::default()
                    },
                    Point {
                        x: 100.0,
                        y: 0.0,
                        kind: PointKind::OffCurve,
                        ..Point::default()
                    },
                ],
                ..Contour::default()
            };
            doc.font.glyph_mut(&glyph).unwrap().layers[0].contours =
                vec![single, handles_only.clone()];
            let before = doc.font.glyph(&glyph).unwrap().clone();
            for contour in 0..2 {
                for to in [SegmentKind::Curve, SegmentKind::Line] {
                    let error = doc
                        .apply(
                            Command::ConvertSegment {
                                glyph: glyph.clone(),
                                layer: layer.clone(),
                                contour,
                                segment: 0,
                                to,
                            },
                            "Convert",
                        )
                        .unwrap_err();
                    assert!(matches!(error, Error::InvalidCommand(_)), "{error}");
                    assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
                }
            }

            doc.apply(
                Command::DeletePoints {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    points: (0..3).map(|point| PointRef { contour: 1, point }).collect(),
                    mode: DeleteMode::KeepShape,
                },
                "Delete every point",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph(&glyph).unwrap().layers[0].contours.len(),
                1,
                "an emptied contour is removed"
            );
        })
        .unwrap();
}

#[test]
fn a_malformed_contour_is_rejected_and_leaves_the_layer_unchanged() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let before = doc.font.glyph(&glyph).unwrap().clone();
            let malformed = Contour {
                closed: false,
                points: vec![
                    Point {
                        x: 0.0,
                        y: 0.0,
                        kind: PointKind::Move,
                        ..Point::default()
                    },
                    Point {
                        x: 10.0,
                        y: 10.0,
                        kind: PointKind::OffCurve,
                        ..Point::default()
                    },
                    Point {
                        x: 20.0,
                        y: 0.0,
                        kind: PointKind::Curve,
                        ..Point::default()
                    },
                ],
                ..Contour::default()
            };
            let error = doc
                .apply(
                    Command::AddContour {
                        glyph: glyph.clone(),
                        layer,
                        contour: malformed,
                    },
                    "Add malformed contour",
                )
                .unwrap_err();
            assert!(matches!(error, Error::InvalidCommand(_)), "{error}");
            assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
        })
        .unwrap();
}

/// A font whose provenance declares TrueType (quadratic) outlines.
fn quadratic_font() -> fw_model::Font {
    let mut font = test_font();
    font.extra.source = Some(fw_model::SourceProvenance {
        format: "sfnt".to_owned(),
        table_tags: vec![fw_model::Tag(*b"head"), fw_model::Tag(*b"glyf")],
        ..fw_model::SourceProvenance::default()
    });
    font
}

#[test]
fn a_quadratic_font_gets_quadratic_handles_on_a_straight_glyph() {
    let session = Session::new();
    let id = session.open_font(quadratic_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            assert!(
                doc.font.glyph(&glyph).unwrap().layers[0].contours[0]
                    .points
                    .iter()
                    .all(|point| point.kind == PointKind::Line),
                "the fixture glyph holds no curve to infer a kind from"
            );
            doc.apply(
                Command::ConvertSegment {
                    glyph: glyph.clone(),
                    layer,
                    contour: 0,
                    segment: 0,
                    to: SegmentKind::Curve,
                },
                "Curve",
            )
            .unwrap();
            let contour = &doc.font.glyph(&glyph).unwrap().layers[0].contours[0];
            assert_eq!(contour.points.len(), 5, "one control point is inserted");
            assert!(
                !contour
                    .points
                    .iter()
                    .any(|point| point.kind == PointKind::Curve),
                "a TrueType font must not receive cubic handles"
            );
            assert_eq!(contour.points[1].kind, PointKind::OffCurve);
            assert_eq!((contour.points[1].x, contour.points[1].y), (50.0, 0.0));
            assert_eq!(contour.points[2].kind, PointKind::QCurve);
            contour.try_to_bezpath().unwrap();
        })
        .unwrap();
}

#[test]
fn a_font_without_a_declaration_still_infers_the_kind_from_the_outlines() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            doc.apply(
                Command::ConvertSegment {
                    glyph: glyph.clone(),
                    layer,
                    contour: 0,
                    segment: 0,
                    to: SegmentKind::Curve,
                },
                "Curve",
            )
            .unwrap();
            let contour = &doc.font.glyph(&glyph).unwrap().layers[0].contours[0];
            assert_eq!(contour.points.len(), 6, "two handles are inserted");
            assert_eq!(contour.points[3].kind, PointKind::Curve);
            contour.try_to_bezpath().unwrap();
        })
        .unwrap();
}
