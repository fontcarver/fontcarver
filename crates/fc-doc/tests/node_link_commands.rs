#[allow(dead_code)]
mod common;

use fc_doc::{Command, DeleteMode, Document, HandleMode, NodeLinkCommand, PointRef, Session};
use fc_model::{Contour, Font, Glyph, Guide, Layer, LayerId, NodeAxis, Point, PointKind};

fn point(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        x,
        y,
        kind,
        ..Point::default()
    }
}

fn point_ref(point: usize) -> PointRef {
    PointRef { contour: 0, point }
}

fn font_with_points(points: Vec<Point>) -> Font {
    let mut font = Font::new_empty("Node links", 1000);
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.contours.push(Contour {
        closed: false,
        points,
        ..Contour::default()
    });
    layer.guides.push(Guide::default());
    let mut glyph = Glyph::new("base");
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

fn plain_font() -> Font {
    font_with_points(vec![
        point(0., 20., PointKind::Move),
        point(50., 40., PointKind::Line),
        point(100., -10., PointKind::Line),
    ])
}

fn layer(doc: &Document) -> &Layer {
    &doc.font.glyph("base").unwrap().layers[0]
}

fn servant(layer: &LayerId, node: usize, first: usize, second: usize) -> Command {
    Command::NodeLinks(NodeLinkCommand::SetServant {
        glyph: "base".into(),
        layer: layer.clone(),
        node: point_ref(node),
        first: point_ref(first),
        second: point_ref(second),
        axis: NodeAxis::X,
    })
}

fn move_point(layer: &LayerId, point: usize, delta: (f64, f64)) -> Command {
    Command::MovePoints {
        glyph: "base".into(),
        layer: layer.clone(),
        points: vec![point_ref(point)],
        delta,
        handles: HandleMode::PointsOnly,
    }
}

#[test]
fn servant_midpoint_tracks_boundary_and_round_trips_history() {
    let session = Session::new();
    let id = session.open_font(plain_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            let original = doc.font.clone();
            doc.apply(servant(&layer_id, 1, 0, 2), "Link midpoint")
                .unwrap();
            let linked = doc.font.clone();
            assert_eq!(layer(doc).node_links.len(), 1);
            assert!(layer(doc).contours[0].points.iter().all(|p| p.id.is_some()));
            doc.apply(move_point(&layer_id, 2, (100., 0.)), "Move boundary")
                .unwrap();
            let points = &layer(doc).contours[0].points;
            assert_eq!((points[1].x, points[1].y), (100., 40.));
            assert_eq!((points[2].x, points[2].y), (200., -10.));
            let moved = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, linked);
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.redo().unwrap();
            doc.redo().unwrap();
            assert_eq!(doc.font, moved);
        })
        .unwrap();
}

#[test]
fn neighboring_handles_scale_with_servant_axis_span() {
    let font = font_with_points(vec![
        point(0., 0., PointKind::Move),
        point(20., 10., PointKind::OffCurve),
        point(40., 20., PointKind::OffCurve),
        point(50., 30., PointKind::Curve),
        point(60., 40., PointKind::OffCurve),
        point(80., 50., PointKind::OffCurve),
        point(100., 60., PointKind::Curve),
    ]);
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            doc.apply(servant(&layer_id, 3, 0, 6), "Link curved node")
                .unwrap();
            let linked = doc.font.clone();
            doc.apply(move_point(&layer_id, 6, (100., 0.)), "Expand span")
                .unwrap();
            let points = &layer(doc).contours[0].points;
            assert_eq!((points[2].x, points[2].y), (80., 20.));
            assert_eq!((points[3].x, points[3].y), (100., 30.));
            assert_eq!((points[4].x, points[4].y), (120., 40.));
            assert_eq!((points[1].x, points[1].y), (20., 10.));
            assert_eq!((points[5].x, points[5].y), (80., 50.));
            doc.undo().unwrap();
            assert_eq!(doc.font, linked);
        })
        .unwrap();
}

#[test]
fn guide_drag_preserves_normal_offsets_identity_and_history() {
    let session = Session::new();
    let id = session.open_font(plain_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            doc.apply(
                Command::NodeLinks(NodeLinkCommand::LinkToGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index: 0,
                    points: vec![point_ref(0), point_ref(2)],
                }),
                "Attach to guide",
            )
            .unwrap();
            let linked = doc.font.clone();
            let guide_id = layer(doc).guides[0].id.clone();
            assert!(guide_id.is_some());
            assert_eq!(layer(doc).linked_guides.len(), 1);
            doc.apply(
                Command::SetGuide {
                    glyph: Some("base".into()),
                    layer: Some(layer_id.clone()),
                    master: None,
                    index: 0,
                    guide: Guide {
                        y: 50.,
                        ..Guide::default()
                    },
                },
                "Drag guide",
            )
            .unwrap();
            let points = &layer(doc).contours[0].points;
            assert_eq!((points[0].x, points[0].y), (0., 70.));
            assert_eq!((points[2].x, points[2].y), (100., 40.));
            assert_eq!((points[1].x, points[1].y), (50., 40.));
            assert_eq!(layer(doc).guides[0].id, guide_id);
            let moved = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, linked);
            doc.redo().unwrap();
            assert_eq!(doc.font, moved);

            let remove = Command::RemoveGuide {
                glyph: Some("base".into()),
                layer: Some(layer_id.clone()),
                master: None,
                index: 0,
            };
            assert!(doc.apply(remove.clone(), "Delete linked guide").is_err());
            assert_eq!(doc.font, moved);
            doc.apply(
                Command::NodeLinks(NodeLinkCommand::UnlinkGuide {
                    glyph: "base".into(),
                    layer: layer_id,
                    guide_index: 0,
                }),
                "Detach guide",
            )
            .unwrap();
            assert!(layer(doc).linked_guides.is_empty());
            doc.apply(remove, "Delete detached guide").unwrap();
            assert!(layer(doc).guides.is_empty());
        })
        .unwrap();
}

#[test]
fn referenced_endpoint_deletion_is_atomic_until_servant_removed() {
    let session = Session::new();
    let id = session.open_font(plain_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            doc.apply(servant(&layer_id, 1, 0, 2), "Link midpoint")
                .unwrap();
            let linked = doc.font.clone();
            let delete = Command::DeletePoints {
                glyph: "base".into(),
                layer: layer_id.clone(),
                points: vec![point_ref(2)],
                mode: DeleteMode::Simple,
            };
            assert!(doc.apply(delete.clone(), "Delete boundary").is_err());
            assert_eq!(doc.font, linked);
            doc.apply(
                Command::NodeLinks(NodeLinkCommand::RemoveServant {
                    glyph: "base".into(),
                    layer: layer_id,
                    node: point_ref(1),
                    axis: NodeAxis::X,
                }),
                "Unlink midpoint",
            )
            .unwrap();
            assert!(layer(doc).node_links.is_empty());
            doc.apply(delete, "Delete detached boundary").unwrap();
            assert_eq!(layer(doc).contours[0].points.len(), 2);
            doc.undo().unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font, linked);
        })
        .unwrap();
}

#[test]
fn cycles_missing_nodes_and_nonfinite_moves_leave_no_partial_edits() {
    let session = Session::new();
    let id = session.open_font(plain_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            let original = doc.font.clone();
            assert!(doc
                .apply(servant(&layer_id, 1, 0, 99), "Missing boundary")
                .is_err());
            assert_eq!(doc.font, original);
            doc.apply(servant(&layer_id, 1, 0, 2), "Link midpoint")
                .unwrap();
            let linked = doc.font.clone();
            assert!(doc
                .apply(servant(&layer_id, 0, 1, 2), "Cyclic boundary")
                .is_err());
            assert_eq!(doc.font, linked);
            for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert!(doc
                    .apply(move_point(&layer_id, 2, (invalid, 0.)), "Invalid move")
                    .is_err());
                assert_eq!(doc.font, linked);
            }
            // Failed commands must not consume an undo step.
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
        })
        .unwrap();
}

#[test]
fn stable_node_ids_survive_reordering_and_native_round_trip() {
    let session = Session::new();
    let id = session.open_font(plain_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            let ensure = Command::NodeLinks(NodeLinkCommand::EnsureNodeIds {
                glyph: "base".into(),
                layer: layer_id.clone(),
                points: vec![point_ref(0), point_ref(1), point_ref(2)],
            });
            doc.apply(ensure.clone(), "Assign node ids").unwrap();
            let identified = doc.font.clone();
            doc.apply(ensure, "Keep node ids").unwrap();
            assert_eq!(doc.font, identified);
            let servant_id = layer(doc).contours[0].points[1].id.clone();
            let boundary_id = layer(doc).contours[0].points[2].id.clone();
            doc.apply(servant(&layer_id, 1, 0, 2), "Link midpoint")
                .unwrap();
            doc.apply(
                Command::NodeLinks(NodeLinkCommand::LinkToGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index: 0,
                    points: vec![point_ref(0)],
                }),
                "Attach boundary to guide",
            )
            .unwrap();
            doc.apply(
                Command::ReverseContours {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    contours: vec![0],
                },
                "Reverse linked contour",
            )
            .unwrap();
            let boundary_index = layer(doc).contours[0]
                .points
                .iter()
                .position(|point| point.id == boundary_id)
                .unwrap();
            doc.apply(
                move_point(&layer_id, boundary_index, (100., 0.)),
                "Move reordered boundary",
            )
            .unwrap();
            let midpoint = layer(doc).contours[0]
                .points
                .iter()
                .find(|point| point.id == servant_id)
                .unwrap();
            assert_eq!((midpoint.x, midpoint.y), (100., 40.));

            struct TestDirectory(std::path::PathBuf);
            impl Drop for TestDirectory {
                fn drop(&mut self) {
                    let _ = std::fs::remove_dir_all(&self.0);
                }
            }
            let directory = TestDirectory(common::temp_dir("node-links"));
            let path = directory.0.join("linked.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), doc.font);

            let linked = doc.font.clone();
            doc.apply(
                Command::NodeLinks(NodeLinkCommand::Clear {
                    glyph: "base".into(),
                    layer: layer_id,
                }),
                "Clear node relationships",
            )
            .unwrap();
            assert!(layer(doc).node_links.is_empty());
            assert!(layer(doc).linked_guides.is_empty());
            doc.undo().unwrap();
            assert_eq!(doc.font, linked);
        })
        .unwrap();
}

#[test]
fn two_nonparallel_guides_move_a_corner_and_parallel_owner_fails_atomically() {
    let mut font = plain_font();
    font.glyph_mut("base").unwrap().layers[0].guides.extend([
        Guide {
            x: 10.,
            angle: 90.,
            ..Guide::default()
        },
        Guide {
            y: 10.,
            ..Guide::default()
        },
    ]);
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            let link = |guide_index| {
                Command::NodeLinks(NodeLinkCommand::LinkToGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index,
                    points: vec![point_ref(1)],
                })
            };
            doc.apply(link(0), "Link horizontal guide").unwrap();
            let horizontal = doc.font.clone();
            assert!(doc.apply(link(2), "Link parallel guide").is_err());
            assert_eq!(doc.font, horizontal);
            doc.apply(link(1), "Link vertical guide").unwrap();
            assert_eq!(layer(doc).linked_guides.len(), 2);
            let linked = doc.font.clone();

            for (index, guide) in [
                (
                    0,
                    Guide {
                        y: 30.,
                        ..Guide::default()
                    },
                ),
                (
                    1,
                    Guide {
                        x: 35.,
                        angle: 90.,
                        ..Guide::default()
                    },
                ),
            ] {
                doc.apply(
                    Command::SetGuide {
                        glyph: Some("base".into()),
                        layer: Some(layer_id.clone()),
                        master: None,
                        index,
                        guide,
                    },
                    "Move corner guide",
                )
                .unwrap();
            }
            let corner = &layer(doc).contours[0].points[1];
            assert!((corner.x - 75.).abs() < 1e-9);
            assert!((corner.y - 70.).abs() < 1e-9);
            let moved = doc.font.clone();
            doc.undo().unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font, linked);
            doc.redo().unwrap();
            doc.redo().unwrap();
            assert_eq!(doc.font, moved);
        })
        .unwrap();
}

#[test]
fn snap_and_link_batch_projects_unlinked_node_and_undoes_as_one_edit() {
    let session = Session::new();
    let id = session.open_font(plain_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            let original = doc.font.clone();
            doc.apply(
                Command::Batch(vec![
                    Command::NodeLinks(NodeLinkCommand::ProjectToGuide {
                        glyph: "base".into(),
                        layer: layer_id.clone(),
                        guide_index: 0,
                        points: vec![point_ref(1)],
                    }),
                    Command::NodeLinks(NodeLinkCommand::LinkToGuide {
                        glyph: "base".into(),
                        layer: layer_id.clone(),
                        guide_index: 0,
                        points: vec![point_ref(1)],
                    }),
                ]),
                "Snap and link",
            )
            .unwrap();
            let midpoint = &layer(doc).contours[0].points[1];
            assert_eq!((midpoint.x, midpoint.y), (50., 0.));
            assert_eq!(layer(doc).linked_guides.len(), 1);
            let snapped = doc.font.clone();
            doc.apply(
                Command::SetGuide {
                    glyph: Some("base".into()),
                    layer: Some(layer_id),
                    master: None,
                    index: 0,
                    guide: Guide {
                        y: 80.,
                        ..Guide::default()
                    },
                },
                "Move snapped guide",
            )
            .unwrap();
            let midpoint = &layer(doc).contours[0].points[1];
            assert_eq!((midpoint.x, midpoint.y), (50., 80.));
            doc.undo().unwrap();
            assert_eq!(doc.font, snapped);
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.redo().unwrap();
            assert_eq!(doc.font, snapped);
        })
        .unwrap();
}

#[test]
fn a_locked_guide_refuses_every_change_until_it_is_explicitly_unlocked() {
    let session = Session::new();
    let id = session.open_font(plain_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            // Link first, so the lock has both an owner and a binding to protect.
            doc.apply(
                Command::NodeLinks(NodeLinkCommand::LinkToGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index: 0,
                    points: vec![point_ref(0)],
                }),
                "Attach to guide",
            )
            .unwrap();
            let stored = layer(doc).guides[0].clone();
            doc.apply(
                Command::SetGuide {
                    glyph: Some("base".into()),
                    layer: Some(layer_id.clone()),
                    master: None,
                    index: 0,
                    guide: Guide {
                        locked: true,
                        ..stored.clone()
                    },
                },
                "Lock the guide",
            )
            .unwrap();
            let locked = doc.font.clone();
            assert!(layer(doc).guides[0].locked);

            let refused: Vec<Command> = vec![
                Command::NodeLinks(NodeLinkCommand::LinkToGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index: 0,
                    points: vec![point_ref(2)],
                }),
                Command::NodeLinks(NodeLinkCommand::ProjectToGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index: 0,
                    points: vec![point_ref(2)],
                }),
                Command::NodeLinks(NodeLinkCommand::UnlinkGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index: 0,
                }),
                Command::SetGuide {
                    glyph: Some("base".into()),
                    layer: Some(layer_id.clone()),
                    master: None,
                    index: 0,
                    guide: Guide {
                        y: 120.,
                        locked: true,
                        ..stored.clone()
                    },
                },
                Command::RemoveGuide {
                    glyph: Some("base".into()),
                    layer: Some(layer_id.clone()),
                    master: None,
                    index: 0,
                },
            ];
            for command in refused {
                let error = doc.apply(command, "Refused").unwrap_err().to_string();
                assert!(error.contains("locked"), "unexpected error: {error}");
                assert_eq!(doc.font, locked, "a refused command changed the document");
            }

            // The one permitted write is the explicit unlock, and afterwards the
            // ordinary edits work again.
            doc.apply(
                Command::SetGuide {
                    glyph: Some("base".into()),
                    layer: Some(layer_id.clone()),
                    master: None,
                    index: 0,
                    guide: stored.clone(),
                },
                "Unlock the guide",
            )
            .unwrap();
            assert!(!layer(doc).guides[0].locked);
            doc.apply(
                Command::NodeLinks(NodeLinkCommand::LinkToGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index: 0,
                    points: vec![point_ref(2)],
                }),
                "Attach after unlocking",
            )
            .unwrap();
            assert_eq!(layer(doc).linked_guides[0].nodes.len(), 2);
            // Undo walks back over the unlock without resurrecting a changed guide.
            doc.undo().unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font, locked);
        })
        .unwrap();
}

#[test]
fn a_locked_guide_still_moves_the_nodes_it_already_owns() {
    let session = Session::new();
    let id = session.open_font(plain_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer_id = layer(doc).id.clone();
            doc.apply(
                Command::NodeLinks(NodeLinkCommand::LinkToGuide {
                    glyph: "base".into(),
                    layer: layer_id.clone(),
                    guide_index: 0,
                    points: vec![point_ref(0)],
                }),
                "Attach to guide",
            )
            .unwrap();
            let stored = layer(doc).guides[0].clone();
            // Locking and moving in one command is still a move of a guide that
            // ends up locked, which an unlocked guide is allowed to do.
            doc.apply(
                Command::SetGuide {
                    glyph: Some("base".into()),
                    layer: Some(layer_id.clone()),
                    master: None,
                    index: 0,
                    guide: Guide {
                        y: 40.,
                        locked: true,
                        ..stored.clone()
                    },
                },
                "Move and lock",
            )
            .unwrap();
            assert_eq!(layer(doc).contours[0].points[0].y, 60.);
            doc.undo().unwrap();
            // An unlocked guide moves its nodes as before.
            doc.apply(
                Command::SetGuide {
                    glyph: Some("base".into()),
                    layer: Some(layer_id.clone()),
                    master: None,
                    index: 0,
                    guide: Guide {
                        y: 60.,
                        ..stored.clone()
                    },
                },
                "Move the guide",
            )
            .unwrap();
            // The node keeps its stored offset of twenty units from the guide.
            assert_eq!(layer(doc).contours[0].points[0].y, 80.);
        })
        .unwrap();
}
