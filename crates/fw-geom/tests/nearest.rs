mod fixtures;

use fw_geom::{bbox, intersections_with_line, nearest_on_path, segment_to_points, split_segment};
use fw_model::{Contour, PointKind};
use kurbo::{Line, ParamCurve, Point};
use proptest::prelude::*;

fn cubic() -> Contour {
    Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, PointKind::Move),
            fixtures::point(20.0, 100.0, PointKind::OffCurve),
            fixtures::point(80.0, -100.0, PointKind::OffCurve),
            fixtures::point(100.0, 0.0, PointKind::Curve),
        ],
        name: None,
        id: None,
    }
}

#[test]
fn nearest_reports_segment_and_parameter() {
    let hit = nearest_on_path(
        &[fixtures::open_line(0.0, 0.0, 100.0, 0.0)],
        Point::new(25.0, 5.0),
        10.0,
    )
    .expect("near line");
    assert_eq!(hit.segment, 0);
    assert!((hit.t - 0.25).abs() < 1e-9);
    assert!((hit.distance - 5.0).abs() < 1e-9);
}

#[test]
fn segment_indices_map_to_model_point_spans() {
    let contour = Contour {
        closed: true,
        points: vec![
            fixtures::point(25.0, 100.0, PointKind::OffCurve),
            fixtures::point(75.0, 100.0, PointKind::OffCurve),
            fixtures::point(100.0, 0.0, PointKind::QCurve),
            fixtures::point(50.0, -50.0, PointKind::Line),
            fixtures::point(0.0, 0.0, PointKind::QCurve),
        ],
        name: None,
        id: None,
    };
    assert_eq!(segment_to_points(&contour, 0), (2, 3));
    assert_eq!(segment_to_points(&contour, 1), (3, 4));
    assert_eq!(segment_to_points(&contour, 2), (4, 2));
    assert_eq!(segment_to_points(&contour, 3), (4, 2));
}

#[test]
fn line_intersections_report_path_parameter() {
    let hits = intersections_with_line(
        &[fixtures::open_line(0.0, 0.0, 100.0, 0.0)],
        Line::new((50.0, -10.0), (50.0, 10.0)),
    );
    assert_eq!(hits.len(), 1);
    assert!((hits[0].3 - 0.5).abs() < 1e-9);
}

#[test]
fn quadratic_split_preserves_the_curve() {
    let contour = Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, PointKind::Move),
            fixtures::point(50.0, 100.0, PointKind::OffCurve),
            fixtures::point(100.0, 0.0, PointKind::QCurve),
        ],
        name: None,
        id: None,
    };
    let original = contour.to_bezpath().segments().next().expect("quadratic");
    let result = split_segment(&contour, 0, 0.3);
    let segments: Vec<_> = result.to_bezpath().segments().collect();
    assert_eq!(segments.len(), 2);
    for index in 0..101 {
        let t = index as f64 / 100.0;
        let point = if t <= 0.3 {
            segments[0].eval(t / 0.3)
        } else {
            segments[1].eval((t - 0.3) / 0.7)
        };
        assert!((point - original.eval(t)).hypot() < 1e-8);
    }
}

#[test]
fn one_point_contour_has_degenerate_bounds() {
    let contour = Contour {
        closed: false,
        points: vec![fixtures::point(12.0, 34.0, PointKind::Move)],
        name: None,
        id: None,
    };
    assert_eq!(
        bbox(&[contour]),
        Some(kurbo::Rect::new(12.0, 34.0, 12.0, 34.0))
    );
}

#[test]
fn nearest_finds_the_selected_dejavu_g_segment() {
    let outcome = fw_sfnt::read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &fw_sfnt::ReadOptions::default(),
    )
    .expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let glyph = font.glyph("g").expect("g glyph");
    let layer = glyph
        .master_layer(&font.default_master().id)
        .expect("g master layer");
    let (contour_index, segment_index, segment) = layer
        .contours
        .iter()
        .enumerate()
        .flat_map(|(contour_index, contour)| {
            contour
                .to_bezpath()
                .segments()
                .enumerate()
                .map(move |(segment_index, segment)| (contour_index, segment_index, segment))
                .collect::<Vec<_>>()
        })
        .max_by(|left, right| {
            (left.2.end() - left.2.start())
                .hypot2()
                .total_cmp(&(right.2.end() - right.2.start()).hypot2())
        })
        .expect("g segment");
    let target = segment.eval(0.37);
    let hit = nearest_on_path(&layer.contours, target, 0.01).expect("point lies on g");
    assert_eq!((hit.contour, hit.segment), (contour_index, segment_index));
    assert!((hit.t - 0.37).abs() < 1e-6);
    assert!(hit.distance < 1e-7);
}

proptest! {
    #[test]
    fn split_preserves_cubic_shape(t in 0.001f64..0.999) {
        let input = cubic();
        let original = input.to_bezpath().segments().next().expect("segment");
        let split = split_segment(&input, 0, t);
        let segments: Vec<_> = split.to_bezpath().segments().collect();
        prop_assert_eq!(segments.len(), 2);
        for i in 0..101 {
            let u = i as f64 / 100.0;
            let actual = if u <= t {
                segments[0].eval(u / t)
            } else {
                segments[1].eval((u - t) / (1.0 - t))
            };
            prop_assert!((actual - original.eval(u)).hypot() < 1e-6);
        }
    }
}
