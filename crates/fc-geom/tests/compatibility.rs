use fc_geom::{canonical_source, compatible, repair_compatibility, reverse};
use fc_model::{Contour, HintMask, Layer, Point, PointKind};
use kurbo::{Affine, Shape};

fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Contour {
    Contour {
        closed: true,
        points: [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
            .into_iter()
            .map(|(x, y)| Point {
                x,
                y,
                kind: PointKind::Line,
                tt_index: fc_model::TtIndex::Index(1),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn repair_matches_reordered_reversed_shifted_contours_without_moving_geometry() {
    let reference = canonical_source(&Layer {
        contours: vec![
            rectangle(0., 0., 100., 200.),
            rectangle(300., 20., 50., 60.),
        ],
        ..Default::default()
    });
    let mut source = reference.clone();
    for contour in &mut source.contours {
        for point in &mut contour.points {
            point.x = point.x * 1.2 + 15.;
            point.y = point.y * 1.2 + 25.;
        }
        *contour = reverse(contour);
        contour.points.rotate_left(2);
    }
    source.contours.reverse();
    source.tt_instructions = Some(vec![0]);
    source.hints.hint_masks.push(HintMask::default());
    let before: f64 = source
        .contours
        .iter()
        .map(|c| c.to_bezpath().area().abs())
        .sum();
    let repaired = repair_compatibility(&reference, &source).unwrap();
    compatible(&reference, &repaired).unwrap();
    assert_eq!(
        repaired
            .contours
            .iter()
            .map(|c| c.to_bezpath().area().abs())
            .sum::<f64>(),
        before
    );
    for (left, right) in reference.contours.iter().zip(&repaired.contours) {
        let expected = Affine::new([1.2, 0., 0., 1.2, 15., 25.]) * left.to_bezpath();
        assert_eq!(expected, right.to_bezpath());
    }
    assert!(repaired.tt_instructions.is_none());
    assert!(repaired.hints.hint_masks.is_empty());
    assert!(repaired
        .contours
        .iter()
        .flat_map(|c| &c.points)
        .all(|p| p.tt_index == fc_model::TtIndex::Unnumbered));
}

#[test]
fn incompatible_topology_fails_without_mutating_input() {
    let reference = Layer {
        contours: vec![rectangle(0., 0., 10., 10.)],
        ..Default::default()
    };
    let mut source = reference.clone();
    source.contours[0].points.pop();
    let snapshot = source.clone();
    assert!(repair_compatibility(&reference, &source).is_err());
    assert_eq!(source, snapshot);
}
