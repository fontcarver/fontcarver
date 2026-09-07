mod fixtures;

use fc_geom::{
    boolean, remove_overlap, remove_overlap_with_fallback, reverse, BooleanOp, CurveKind,
    FallbackKind, DEFAULT_BOOLEAN_TOLERANCE,
};
use fc_model::TtIndex;
use kurbo::Shape;

#[test]
fn overlapping_rectangles_become_one_eight_point_contour() {
    let contours = [
        fixtures::rect(0.0, 0.0, 100.0, 100.0),
        fixtures::rect(50.0, 50.0, 150.0, 150.0),
    ];
    let result = remove_overlap(&contours, CurveKind::Cubic, DEFAULT_BOOLEAN_TOLERANCE)
        .expect("rectangle union should succeed");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].points.len(), 8);
}

#[test]
fn untouched_contour_keeps_true_type_indices() {
    let mut contour = fixtures::rect(0.0, 0.0, 100.0, 100.0);
    for (index, point) in contour.points.iter_mut().enumerate() {
        point.tt_index = TtIndex::Index(index as u16);
    }
    let result = remove_overlap(std::slice::from_ref(&contour), CurveKind::Cubic, 0.01)
        .expect("single simple contour");
    assert_eq!(result, vec![contour]);
}

#[test]
fn point_touching_contours_take_the_unchanged_fast_path() {
    let mut first = fixtures::rect(0.0, 0.0, 100.0, 100.0);
    let mut second = fixtures::rect(100.0, 100.0, 200.0, 200.0);
    for (index, point) in first
        .points
        .iter_mut()
        .chain(&mut second.points)
        .enumerate()
    {
        point.tt_index = TtIndex::Index(index as u16);
    }
    let contours = vec![first, second];
    let result = remove_overlap(&contours, CurveKind::Cubic, 0.01).expect("point touch");
    assert_eq!(result, contours);
}

#[test]
fn fast_path_cleans_a_zero_width_spike() {
    let spiked = fc_model::Contour {
        closed: true,
        points: vec![
            fixtures::point(0.0, 0.0, fc_model::PointKind::Line),
            fixtures::point(100.0, 0.0, fc_model::PointKind::Line),
            fixtures::point(100.0, 100.0, fc_model::PointKind::Line),
            fixtures::point(50.0, 100.0, fc_model::PointKind::Line),
            fixtures::point(50.0, 120.0, fc_model::PointKind::Line),
            fixtures::point(50.0, 100.0, fc_model::PointKind::Line),
            fixtures::point(0.0, 100.0, fc_model::PointKind::Line),
        ],
        name: None,
        id: None,
    };
    let result = remove_overlap(std::slice::from_ref(&spiked), CurveKind::Cubic, 0.01)
        .expect("clean spiked rectangle");
    assert_ne!(result, vec![spiked]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].points.len(), 4);
    assert!((fixtures::filled_area(&result) - 10_000.0).abs() < 0.01);
}

#[test]
fn tangent_curves_take_the_unchanged_fast_path() {
    let first = fixtures::circle(100.0);
    let mut second = fixtures::circle(100.0);
    for point in &mut second.points {
        point.x += 200.0;
    }
    let contours = vec![first, second];
    let result = remove_overlap(&contours, CurveKind::Cubic, 0.01).expect("tangent circles");
    assert_eq!(result, contours);
}

#[test]
fn booleans_handle_an_empty_side() {
    let a = [fixtures::rect(0.0, 0.0, 100.0, 100.0)];
    assert_eq!(
        boolean(&a, &[], BooleanOp::Union, CurveKind::Cubic, 0.01)
            .expect("union with empty")
            .len(),
        1
    );
    assert!(
        boolean(&a, &[], BooleanOp::Intersect, CurveKind::Cubic, 0.01)
            .expect("intersection with empty")
            .is_empty()
    );
}

#[test]
fn dejavu_o_outer_and_counter_are_unchanged() {
    let outcome = fc_sfnt::read_path(
        &fc_fixtures::path("DejaVuSans.ttf"),
        &fc_sfnt::ReadOptions::default(),
    )
    .expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let glyph = font.glyph("O").expect("O glyph");
    let layer = glyph
        .master_layer(&font.default_master().id)
        .expect("O master layer");
    let result = remove_overlap(
        &layer.contours,
        CurveKind::Quadratic { max_err: 0.01 },
        0.01,
    )
    .expect("remove overlap on O");
    assert_eq!(result, layer.contours);
}

#[test]
fn all_boolean_operations_are_available() {
    let a = [fixtures::rect(0.0, 0.0, 100.0, 100.0)];
    let b = [fixtures::rect(50.0, 0.0, 150.0, 100.0)];
    for (op, expected_area) in [
        (BooleanOp::Union, 15_000.0),
        (BooleanOp::Intersect, 5_000.0),
        (BooleanOp::Exclude, 5_000.0),
        (BooleanOp::Xor, 10_000.0),
    ] {
        let result = boolean(&a, &b, op, CurveKind::Cubic, 0.01).expect("boolean operation");
        assert!((fixtures::filled_area(&result) - expected_area).abs() < 0.01);
        if op == BooleanOp::Intersect {
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].points.len(), 4);
            let bounds = result[0].to_bezpath().bounding_box();
            assert!((bounds.width() - 50.0).abs() < 0.01);
            assert!((bounds.height() - 100.0).abs() < 0.01);
        }
    }
}

#[test]
fn same_winding_nested_rectangles_reduce_to_the_outer_boundary() {
    let contours = [
        fixtures::rect(0.0, 0.0, 100.0, 100.0),
        fixtures::rect(10.0, 10.0, 90.0, 90.0),
    ];
    let result = remove_overlap(&contours, CurveKind::Cubic, 0.01)
        .expect("same-winding nesting is a NonZero union");
    assert_eq!(result.len(), 1);
    assert!((fixtures::filled_area(&result) - 10_000.0).abs() < 0.01);
}

#[test]
fn dejavu_b_counters_are_preserved_and_same_winding_is_nonzero_union() {
    let outcome = fc_sfnt::read_path(
        &fc_fixtures::path("DejaVuSans.ttf"),
        &fc_sfnt::ReadOptions::default(),
    )
    .expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let glyph = font.glyph("B").expect("B glyph");
    let layer = glyph
        .master_layer(&font.default_master().id)
        .expect("B master layer");
    let outer = layer
        .contours
        .iter()
        .enumerate()
        .max_by(|left, right| {
            left.1
                .to_bezpath()
                .area()
                .abs()
                .total_cmp(&right.1.to_bezpath().area().abs())
        })
        .map(|(index, _)| index)
        .expect("B has contours");
    let outer_clockwise = layer.contours[outer].to_bezpath().area() < 0.0;
    let preserved = remove_overlap(
        &layer.contours,
        CurveKind::Quadratic { max_err: 0.01 },
        0.01,
    )
    .expect("correctly wound B counters");
    assert_eq!(preserved, layer.contours);
    let mut flipped = layer.contours.clone();
    for (index, contour) in flipped.iter_mut().enumerate() {
        if index != outer && (contour.to_bezpath().area() < 0.0) != outer_clockwise {
            *contour = reverse(contour);
        }
    }
    let result = remove_overlap(&flipped, CurveKind::Quadratic { max_err: 0.01 }, 0.01)
        .expect("same-winding B is accepted as a NonZero union");
    assert_eq!(result.len(), 1);
    assert!(
        (fixtures::filled_area(&result) - layer.contours[outer].to_bezpath().area().abs()).abs()
            < 1.0
    );
}

#[test]
fn open_contours_retain_their_relative_slots() {
    let first_open = fixtures::open_line(-20.0, 0.0, -10.0, 0.0);
    let last_open = fixtures::open_line(160.0, 0.0, 170.0, 0.0);
    let contours = vec![
        first_open.clone(),
        fixtures::rect(0.0, 0.0, 100.0, 100.0),
        fixtures::rect(50.0, 0.0, 150.0, 100.0),
        last_open.clone(),
    ];
    let result = remove_overlap(&contours, CurveKind::Cubic, 0.01).expect("mixed paths");
    assert_eq!(result.first(), Some(&first_open));
    assert_eq!(result.last(), Some(&last_open));
}

#[test]
fn self_touching_input_never_panics() {
    let bow = fc_model::Contour {
        closed: true,
        points: vec![
            fixtures::point(0.0, 0.0, fc_model::PointKind::Line),
            fixtures::point(100.0, 100.0, fc_model::PointKind::Line),
            fixtures::point(0.0, 100.0, fc_model::PointKind::Line),
            fixtures::point(100.0, 0.0, fc_model::PointKind::Line),
        ],
        name: None,
        id: None,
    };
    let _ = remove_overlap(&[bow], CurveKind::Cubic, 0.01);
}

#[test]
fn fallback_api_reports_when_it_preserves_an_untouched_outline() {
    let contours = [fixtures::rect(0.0, 0.0, 100.0, 100.0)];
    let outcome =
        remove_overlap_with_fallback(&contours, CurveKind::Cubic, 0.01).expect("simple outline");
    assert_eq!(outcome.contours, contours);
    assert_eq!(outcome.fallback, FallbackKind::None);
}

#[test]
fn dejavu_o_with_overlapping_blob_has_a_compact_accurate_fallback() {
    let path = fc_fixtures::path("DejaVuSans.ttf");
    if !path.is_file() {
        eprintln!("skipping test: DejaVuSans.ttf is unavailable");
        return;
    }
    let outcome =
        fc_sfnt::read_path(&path, &fc_sfnt::ReadOptions::default()).expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let layer = font
        .glyph("O")
        .and_then(|glyph| glyph.master_layer(&font.default_master().id))
        .expect("O master layer");
    let mut contours = layer.contours.clone();
    contours.push(fixtures::circle(250.0));
    for point in &mut contours.last_mut().expect("blob").points {
        point.x += 1_350.0;
        point.y += 750.0;
    }
    let outcome = remove_overlap_with_fallback(&contours, CurveKind::Cubic, 0.01)
        .expect("O plus overlapping blob fallback");
    assert_eq!(outcome.fallback, FallbackKind::Flattened);
    let points = outcome
        .contours
        .iter()
        .map(|contour| contour.points.len())
        .sum::<usize>();
    assert!(points <= 200, "fallback has {points} points");
    let input_area = nonzero_area(&contours, 0.01);
    let output_area = nonzero_area(&outcome.contours, 0.01);
    assert!(
        (output_area - input_area).abs() / input_area < 0.01,
        "input area={input_area}, output area={output_area}"
    );
}

fn nonzero_area(contours: &[fc_model::Contour], tolerance: f64) -> f64 {
    use i_overlay::{
        core::{fill_rule::FillRule, overlay_rule::OverlayRule},
        float::single::SingleFloatOverlay,
    };
    use kurbo::{flatten, PathEl};

    let polygons: Vec<Vec<[f64; 2]>> = contours
        .iter()
        .filter(|contour| contour.closed)
        .filter_map(|contour| {
            let mut polygon = Vec::new();
            flatten(contour.to_bezpath().iter(), tolerance, |element| {
                if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                    polygon.push([point.x, point.y]);
                }
            });
            (polygon.len() >= 3).then_some(polygon)
        })
        .collect();
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    polygons
        .overlay(&empty, OverlayRule::Subject, FillRule::NonZero)
        .iter()
        .map(|shape| {
            shape
                .iter()
                .map(|polygon| {
                    polygon
                        .iter()
                        .zip(polygon.iter().cycle().skip(1))
                        .map(|(left, right)| left[0] * right[1] - right[0] * left[1])
                        .sum::<f64>()
                        * 0.5
                })
                .sum::<f64>()
                .abs()
        })
        .sum()
}

#[test]
#[cfg(not(debug_assertions))]
fn dejavu_remove_overlap_corpus_meets_strict_and_fallback_targets() {
    let path = fc_fixtures::path("DejaVuSans.ttf");
    if !path.is_file() {
        eprintln!("skipping test: DejaVuSans.ttf is unavailable");
        return;
    }
    let outcome =
        fc_sfnt::read_path(&path, &fc_sfnt::ReadOptions::default()).expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let mut total = 0_usize;
    let mut strict = 0_usize;
    let mut strict_unchanged = 0_usize;
    let mut changed_names = Vec::new();
    let mut fallback = 0_usize;
    let mut flattened = 0_usize;
    for (name, glyph) in &font.glyphs {
        let Some(layer) = glyph.master_layer(&font.default_master().id) else {
            continue;
        };
        if !layer.contours.iter().any(|contour| contour.closed) {
            continue;
        }
        total += 1;
        if let Ok(result) = remove_overlap(
            &layer.contours,
            CurveKind::Quadratic { max_err: 0.01 },
            0.01,
        ) {
            strict += 1;
            strict_unchanged += usize::from(result == layer.contours);
            if result != layer.contours {
                changed_names.push(name.to_string());
            }
        }
        let result = remove_overlap_with_fallback(
            &layer.contours,
            CurveKind::Quadratic { max_err: 0.01 },
            0.01,
        );
        if let Ok(outcome) = result {
            fallback += 1;
            flattened += usize::from(outcome.fallback == FallbackKind::Flattened);
        }
    }
    eprintln!(
        "DejaVu remove overlap: total={total}, strict={strict}, strict_unchanged={strict_unchanged}, changed={changed_names:?}, fallback={fallback}, flattened={flattened}"
    );
    assert!(total > 0);
    assert_eq!(fallback, total);
    assert!(strict * 100 >= total * 97);
    assert!(strict_unchanged * 100 >= total * 99);
}
