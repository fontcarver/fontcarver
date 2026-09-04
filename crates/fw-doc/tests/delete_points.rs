mod common;

use fw_doc::{Command, DefaultFontOpener, DeleteMode, FontOpener, PointRef, Session};
use fw_model::{Contour, Font, GlyphName, LayerId, Point, PointKind};
use kurbo::{ParamCurve, ParamCurveNearest, PathSeg, Point as KurboPoint};

use common::test_font;

const CORPUS: [&str; 6] = ["B", "O", "S", "a", "e", "g"];

fn on_curve(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        x,
        y,
        kind,
        ..Point::default()
    }
}

fn handle(x: f64, y: f64) -> Point {
    on_curve(x, y, PointKind::OffCurve)
}

/// A closed square with a redundant on-curve point in the middle of one edge.
fn square_with_midpoint() -> Contour {
    Contour {
        closed: true,
        points: vec![
            on_curve(0.0, 0.0, PointKind::Line),
            on_curve(50.0, 0.0, PointKind::Line),
            on_curve(100.0, 0.0, PointKind::Line),
            on_curve(100.0, 100.0, PointKind::Line),
            on_curve(0.0, 100.0, PointKind::Line),
        ],
        ..Contour::default()
    }
}

/// A closed quadratic contour in TrueType point-list form.
fn quadratic_ring() -> Contour {
    Contour {
        closed: true,
        points: vec![
            on_curve(0.0, 0.0, PointKind::QCurve),
            handle(50.0, -40.0),
            on_curve(100.0, 0.0, PointKind::QCurve),
            handle(140.0, 50.0),
            on_curve(100.0, 100.0, PointKind::QCurve),
            handle(50.0, 140.0),
            on_curve(0.0, 100.0, PointKind::QCurve),
            handle(-40.0, 50.0),
        ],
        ..Contour::default()
    }
}

/// A closed cubic contour with four smooth nodes.
fn cubic_ring() -> Contour {
    Contour {
        closed: true,
        points: vec![
            on_curve(0.0, 0.0, PointKind::Curve),
            handle(30.0, -30.0),
            handle(70.0, -30.0),
            on_curve(100.0, 0.0, PointKind::Curve),
            handle(130.0, 30.0),
            handle(130.0, 70.0),
            on_curve(100.0, 100.0, PointKind::Curve),
            handle(70.0, 130.0),
            handle(30.0, 130.0),
            on_curve(0.0, 100.0, PointKind::Curve),
            handle(-30.0, 70.0),
            handle(-30.0, 30.0),
        ],
        ..Contour::default()
    }
}

fn with_contours(contours: Vec<Contour>) -> (Session, fw_doc::DocId, GlyphName, LayerId) {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    let glyph = GlyphName::from("base");
    let layer = session
        .with_doc_mut(id, |doc| {
            doc.font.glyph_mut("base").unwrap().layers[0].contours = contours;
            doc.font.glyph("base").unwrap().layers[0].id.clone()
        })
        .unwrap();
    (session, id, glyph, layer)
}

fn delete(
    session: &Session,
    id: fw_doc::DocId,
    glyph: &GlyphName,
    layer: &LayerId,
    points: Vec<PointRef>,
    mode: DeleteMode,
) -> Result<Vec<Contour>, fw_doc::Error> {
    session
        .with_doc_mut(id, |doc| {
            doc.apply(
                Command::DeletePoints {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                    points,
                    mode,
                },
                "Delete",
            )?;
            Ok(doc.font.glyph(glyph).unwrap().layers[0].contours.clone())
        })
        .unwrap()
}

#[test]
fn deleting_a_node_between_two_lines_leaves_one_line() {
    let (session, id, glyph, layer) = with_contours(vec![square_with_midpoint()]);
    let contours = delete(
        &session,
        id,
        &glyph,
        &layer,
        vec![PointRef {
            contour: 0,
            point: 1,
        }],
        DeleteMode::KeepShape,
    )
    .unwrap();
    assert_eq!(contours[0].points.len(), 4);
    assert!(contours[0]
        .points
        .iter()
        .all(|point| point.kind == PointKind::Line));
    assert_eq!(
        (contours[0].points[0].x, contours[0].points[0].y),
        (0.0, 0.0)
    );
}

#[test]
fn deleting_a_run_of_nodes_joins_the_span_once() {
    let (session, id, glyph, layer) = with_contours(vec![cubic_ring()]);
    let contours = delete(
        &session,
        id,
        &glyph,
        &layer,
        vec![
            PointRef {
                contour: 0,
                point: 3,
            },
            PointRef {
                contour: 0,
                point: 6,
            },
        ],
        DeleteMode::KeepShape,
    )
    .unwrap();
    let contour = &contours[0];
    let nodes: Vec<_> = contour
        .points
        .iter()
        .filter(|point| point.kind != PointKind::OffCurve)
        .collect();
    assert_eq!(nodes.len(), 2);
    assert_eq!(contour.points.len(), 6);
    assert_eq!(contour.to_bezpath().segments().count(), 2);
    contour.try_to_bezpath().unwrap();
}

#[test]
fn deleting_every_node_removes_the_contour() {
    for contour in [quadratic_ring(), cubic_ring(), square_with_midpoint()] {
        let points: Vec<_> = (0..contour.points.len())
            .map(|point| PointRef { contour: 0, point })
            .collect();
        let (session, id, glyph, layer) = with_contours(vec![contour]);
        let contours = delete(&session, id, &glyph, &layer, points, DeleteMode::KeepShape).unwrap();
        assert!(contours.is_empty());
    }
}

#[test]
fn a_quadratic_contour_stays_quadratic_after_a_deletion() {
    let original = quadratic_ring();
    let (session, id, glyph, layer) = with_contours(vec![original]);
    let contours = delete(
        &session,
        id,
        &glyph,
        &layer,
        vec![PointRef {
            contour: 0,
            point: 2,
        }],
        DeleteMode::KeepShape,
    )
    .unwrap();
    let contour = &contours[0];
    assert!(!contour
        .points
        .iter()
        .any(|point| point.kind == PointKind::Curve));
    assert!(contour.is_quadratic());
    contour.try_to_bezpath().unwrap();
    assert_eq!(
        (contour.points[0].x, contour.points[0].y),
        (0.0, 0.0),
        "the start point is preserved"
    );
}

#[test]
fn deleting_the_start_node_promotes_the_next_surviving_node() {
    let (session, id, glyph, layer) = with_contours(vec![cubic_ring()]);
    let contours = delete(
        &session,
        id,
        &glyph,
        &layer,
        vec![PointRef {
            contour: 0,
            point: 0,
        }],
        DeleteMode::KeepShape,
    )
    .unwrap();
    let contour = &contours[0];
    let first = contour
        .points
        .iter()
        .find(|point| point.kind != PointKind::OffCurve)
        .unwrap();
    assert_eq!((first.x, first.y), (100.0, 0.0));
    contour.try_to_bezpath().unwrap();
}

#[test]
fn deleting_an_open_contour_endpoint_drops_it_without_fitting() {
    let mut open = cubic_ring();
    open.closed = false;
    open.points.truncate(10);
    open.points[0].kind = PointKind::Move;
    let expected = open.points.len() - 3;
    let (session, id, glyph, layer) = with_contours(vec![open]);
    let contours = delete(
        &session,
        id,
        &glyph,
        &layer,
        vec![PointRef {
            contour: 0,
            point: 9,
        }],
        DeleteMode::KeepShape,
    )
    .unwrap();
    assert_eq!(contours[0].points.len(), expected);
    assert_eq!(contours[0].points[0].kind, PointKind::Move);
    contours[0].try_to_bezpath().unwrap();
}

#[test]
fn simple_mode_drops_points_and_handles_without_fitting() {
    let (session, id, glyph, layer) = with_contours(vec![cubic_ring()]);
    let contours = delete(
        &session,
        id,
        &glyph,
        &layer,
        vec![PointRef {
            contour: 0,
            point: 3,
        }],
        DeleteMode::Simple,
    )
    .unwrap();
    let contour = &contours[0];
    assert_eq!(contour.points.len(), 9);
    contour.try_to_bezpath().unwrap();
}

#[test]
fn deleting_a_handle_turns_its_segment_into_a_line() {
    let (session, id, glyph, layer) = with_contours(vec![cubic_ring()]);
    let contours = delete(
        &session,
        id,
        &glyph,
        &layer,
        vec![PointRef {
            contour: 0,
            point: 1,
        }],
        DeleteMode::KeepShape,
    )
    .unwrap();
    let contour = &contours[0];
    assert_eq!(contour.points.len(), 10);
    assert_eq!(contour.points[1].kind, PointKind::Line);
    contour.try_to_bezpath().unwrap();
}

fn span_of(contour: &Contour, previous: usize, node: usize, next: usize) -> Vec<PathSeg> {
    contour
        .to_bezpath()
        .segments()
        .enumerate()
        .filter(|(index, _)| {
            let pair = fw_geom::segment_to_points(contour, *index);
            pair == (previous, node) || pair == (node, next)
        })
        .map(|(_, segment)| segment)
        .collect()
}

/// Whether two points coincide to within a rounding error.
fn coincident(left: KurboPoint, right: KurboPoint) -> bool {
    (left - right).hypot() <= 1.0e-9
}

/// The segments of a rewritten contour that run from `from` round to `to`.
///
/// Both ends survive a deletion unchanged, so the run between them is exactly
/// the joined span the deletion produced.
fn joined_span(updated: &Contour, from: KurboPoint, to: KurboPoint) -> Vec<PathSeg> {
    let segments: Vec<PathSeg> = updated.to_bezpath().segments().collect();
    let Some(start) = segments
        .iter()
        .position(|segment| coincident(segment.start(), from))
    else {
        return Vec::new();
    };
    let mut span = Vec::new();
    for offset in 0..segments.len() {
        let Some(&segment) = segments.get((start + offset) % segments.len()) else {
            break;
        };
        span.push(segment);
        if coincident(segment.end(), to) {
            return span;
        }
    }
    Vec::new()
}

/// The largest distance from any sample of `from` to the nearest point of `to`.
fn one_way_deviation(from: &[PathSeg], to: &[PathSeg]) -> f64 {
    let mut worst = 0.0_f64;
    for segment in from {
        for step in 0..=16 {
            let sample = segment.eval(f64::from(step) / 16.0);
            let nearest = to
                .iter()
                .map(|candidate| candidate.nearest(sample, 1.0e-6).distance_sq)
                .fold(f64::INFINITY, f64::min);
            worst = worst.max(nearest.sqrt());
        }
    }
    worst
}

fn neighbours(contour: &Contour, node: usize) -> (usize, usize) {
    let len = contour.points.len();
    let previous = (1..len)
        .map(|offset| (node + len - offset) % len)
        .find(|&index| contour.points[index].kind != PointKind::OffCurve)
        .unwrap();
    let next = (1..len)
        .map(|offset| (node + offset) % len)
        .find(|&index| contour.points[index].kind != PointKind::OffCurve)
        .unwrap();
    (previous, next)
}

fn dejavu() -> Font {
    let path = fw_fixtures::path("DejaVuSans.ttf");
    DefaultFontOpener
        .open_path(&path, Some(0))
        .expect("DejaVu Sans opens")
        .into_iter()
        .next()
        .expect("DejaVu Sans has one face")
}

#[test]
fn keep_shape_deletion_sweeps_a_real_corpus_without_errors() {
    let font = dejavu();
    let master = font.default_master().id.clone();
    let mut fitted = Vec::new();
    let mut straight = Vec::new();
    let mut all = Vec::new();
    let mut deletions = 0_usize;
    let mut forward_only = 0.0_f64;
    let mut backward_only = 0.0_f64;

    for name in CORPUS {
        let glyph = font.glyph(name).expect("corpus glyph exists");
        let source = glyph
            .master_layer(&master)
            .expect("corpus glyph has a master layer")
            .contours
            .clone();
        for (contour_index, contour) in source.iter().enumerate() {
            for node in 0..contour.points.len() {
                if contour.points[node].kind == PointKind::OffCurve {
                    continue;
                }
                deletions += 1;
                let (previous, next) = neighbours(contour, node);
                let chord = (KurboPoint::new(contour.points[next].x, contour.points[next].y)
                    - KurboPoint::new(contour.points[previous].x, contour.points[previous].y))
                .hypot();
                let span = span_of(contour, previous, node, next);
                let is_straight = span
                    .iter()
                    .all(|segment| matches!(segment, PathSeg::Line(_)));

                let (session, id, glyph_name, layer) = with_contours(source.clone());
                let contours = delete(
                    &session,
                    id,
                    &glyph_name,
                    &layer,
                    vec![PointRef {
                        contour: contour_index,
                        point: node,
                    }],
                    DeleteMode::KeepShape,
                )
                .unwrap_or_else(|error| {
                    panic!("deleting {name}[{contour_index}] point {node} failed: {error}")
                });
                for contour in &contours {
                    contour.try_to_bezpath().unwrap_or_else(|error| {
                        panic!("{name}[{contour_index}] point {node} is malformed: {error}")
                    });
                }
                let Some(updated) = contours.get(contour_index) else {
                    continue;
                };
                // Deviation is measured in both directions and gated on the
                // larger one: sampling only the original span would miss a
                // fitted segment that bulges away from the original between the
                // samples of the new curve.
                let mut forward = 0.0_f64;
                for segment in &span {
                    for step in 0..=16 {
                        let sample = segment.eval(f64::from(step) / 16.0);
                        let hit =
                            fw_geom::nearest_on_path(std::slice::from_ref(updated), sample, 1.0e9);
                        forward = forward.max(hit.map_or(f64::INFINITY, |hit| hit.distance));
                    }
                }
                let new_span = joined_span(
                    updated,
                    KurboPoint::new(contour.points[previous].x, contour.points[previous].y),
                    KurboPoint::new(contour.points[next].x, contour.points[next].y),
                );
                assert!(
                    !new_span.is_empty(),
                    "{name}[{contour_index}] point {node}: the joined span was not found"
                );
                let backward = one_way_deviation(&new_span, &span);
                forward_only = forward_only.max(if chord > 0.0 { forward / chord } else { 0.0 });
                backward_only = backward_only.max(if chord > 0.0 { backward / chord } else { 0.0 });
                let deviation = forward.max(backward);
                let relative = if chord > 0.0 { deviation / chord } else { 0.0 };
                all.push(relative);
                if is_straight {
                    straight.push(relative);
                } else {
                    fitted.push(relative);
                }
            }
        }
    }

    let percentile = |values: &mut Vec<f64>, fraction: f64| {
        values.sort_by(f64::total_cmp);
        let index = ((values.len() as f64 - 1.0) * fraction).round().max(0.0) as usize;
        values.get(index).copied().unwrap_or(0.0)
    };
    let fitted_median = percentile(&mut fitted, 0.5);
    let fitted_ninetieth = percentile(&mut fitted, 0.9);
    let fitted_max = percentile(&mut fitted, 1.0);
    let straight_median = percentile(&mut straight, 0.5);
    let straight_max = percentile(&mut straight, 1.0);
    let overall_median = percentile(&mut all, 0.5);
    let overall_max = percentile(&mut all, 1.0);
    println!(
        "corpus deletions: {deletions} ({} fitted, {} straight)",
        fitted.len(),
        straight.len()
    );
    println!(
        "fitted joins: median {:.3} %, p90 {:.3} %, max {:.3} % of the joined chord",
        fitted_median * 100.0,
        fitted_ninetieth * 100.0,
        fitted_max * 100.0
    );
    println!(
        "line-to-line joins: median {:.3} %, max {:.3} % of the joined chord",
        straight_median * 100.0,
        straight_max * 100.0
    );
    println!(
        "all joins: median {:.3} %, max {:.3} % of the joined chord",
        overall_median * 100.0,
        overall_max * 100.0
    );
    println!(
        "per direction: original samples max {:.3} %, new-span samples max {:.3} %",
        forward_only * 100.0,
        backward_only * 100.0
    );

    assert!(deletions >= 80, "corpus is smaller than expected");
    assert!(
        overall_median <= 0.02,
        "median deviation {overall_median} exceeds 2 % of the joined chord"
    );
    assert!(
        fitted_median <= 0.02,
        "median fitted deviation {fitted_median} exceeds 2 % of the joined chord"
    );
    assert!(
        fitted_ninetieth <= 0.15,
        "ninetieth-percentile fitted deviation {fitted_ninetieth} exceeds 15 % of the joined chord"
    );
    // Removing a corner between two straight segments, or a node at the waist of
    // `B`, changes the outline by construction: one segment cannot reproduce a
    // corner or an inflected span. Those joins stay bounded by half the chord.
    assert!(
        overall_max <= 0.5,
        "worst deviation {overall_max} exceeds half the joined chord"
    );
}
