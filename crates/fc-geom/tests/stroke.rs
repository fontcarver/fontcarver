mod fixtures;

use fc_geom::{
    bbox, expand_stroke, expand_stroke_with_fallback, Cap, CurveKind, FallbackKind, Join, Nib,
    StrokeOptions,
};
use fc_model::{Contour, PointKind};
use kurbo::Shape;

fn options(nib: Nib) -> StrokeOptions {
    StrokeOptions {
        width: 20.0,
        nib,
        cap: Cap::Round,
        join: Join::Round,
        miter_limit: 4.0,
        remove_internal: false,
        remove_external: false,
    }
}

fn closed_options(nib: Nib) -> StrokeOptions {
    StrokeOptions {
        remove_internal: false,
        remove_external: false,
        ..options(nib)
    }
}

#[test]
fn round_straight_stroke_is_a_stadium() {
    let result = expand_stroke(
        &[fixtures::open_line(0.0, 0.0, 100.0, 0.0)],
        &options(Nib::Round),
        CurveKind::Cubic,
    )
    .expect("round stroke");
    let bounds = bbox(&result).expect("bounds");
    assert!((bounds.x0 + 10.0).abs() < 0.1);
    assert!((bounds.x1 - 110.0).abs() < 0.1);
    assert!((bounds.y0 + 10.0).abs() < 0.1);
    assert!((bounds.y1 - 10.0).abs() < 0.1);
}

#[test]
fn rectangular_and_calligraphic_nibs_cover_lines_and_curves() {
    let curve = fc_model::Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, fc_model::PointKind::Move),
            fixtures::point(25.0, 80.0, fc_model::PointKind::OffCurve),
            fixtures::point(75.0, -80.0, fc_model::PointKind::OffCurve),
            fixtures::point(100.0, 0.0, fc_model::PointKind::Curve),
        ],
        name: None,
        id: None,
    };
    for nib in [
        Nib::Rect,
        Nib::Calligraphic {
            width: 24.0,
            height: 8.0,
            angle: 0.4,
        },
    ] {
        assert!(!expand_stroke(
            &[fixtures::open_line(0.0, 0.0, 100.0, 0.0), curve.clone()],
            &options(nib),
            CurveKind::Cubic,
        )
        .expect("convex nib stroke")
        .is_empty());
    }
}

#[test]
fn round_nib_strokes_closed_cornered_contours() {
    let square = fixtures::rect(0.0, 0.0, 200.0, 200.0);
    for (join, expected_area) in [
        (Join::Miter, 16_000.0),
        (Join::Round, 16_000.0 + (std::f64::consts::PI - 4.0) * 100.0),
        (Join::Bevel, 15_800.0),
    ] {
        let opts = StrokeOptions {
            join,
            ..closed_options(Nib::Round)
        };
        let result = expand_stroke(std::slice::from_ref(&square), &opts, CurveKind::Cubic)
            .expect("closed square stroke");
        let area = fixtures::filled_area(&result);
        assert!(
            (area - expected_area).abs() / expected_area < 0.01,
            "{join:?}: area={area}, expected={expected_area}"
        );
    }

    let triangle = Contour {
        closed: true,
        points: vec![
            fixtures::point(0.0, 0.0, PointKind::Line),
            fixtures::point(200.0, 0.0, PointKind::Line),
            fixtures::point(100.0, 180.0, PointKind::Line),
        ],
        name: None,
        id: None,
    };
    assert!(expand_stroke(&[triangle], &closed_options(Nib::Round), CurveKind::Cubic).is_ok());

    let squares = [
        fixtures::rect(0.0, 0.0, 100.0, 100.0),
        fixtures::rect(200.0, 0.0, 300.0, 100.0),
    ];
    assert!(expand_stroke(&squares, &closed_options(Nib::Round), CurveKind::Cubic).is_ok());
}

#[test]
fn closed_round_square_preserves_curve_segments_and_compact_rings() {
    let result = expand_stroke(
        &[fixtures::rect(0.0, 0.0, 200.0, 200.0)],
        &closed_options(Nib::Round),
        CurveKind::Cubic,
    )
    .expect("closed square stroke");
    assert_eq!(result.len(), 2);
    let (outer, inner) =
        if result[0].to_bezpath().area().abs() > result[1].to_bezpath().area().abs() {
            (&result[0], &result[1])
        } else {
            (&result[1], &result[0])
        };
    let outer_lines = outer
        .points
        .iter()
        .filter(|point| point.kind == PointKind::Line)
        .count();
    let outer_curves = outer
        .points
        .iter()
        .filter(|point| point.kind == PointKind::Curve)
        .count();
    let outer_on_curve = outer
        .points
        .iter()
        .filter(|point| point.kind != PointKind::OffCurve)
        .count();
    let inner_on_curve = inner
        .points
        .iter()
        .filter(|point| point.kind != PointKind::OffCurve)
        .count();
    assert_eq!(outer_lines, 4, "outer ring: {:?}", outer.points);
    assert_eq!(outer_curves, 4, "outer ring: {:?}", outer.points);
    assert!(outer_on_curve <= 12, "outer on-curve={outer_on_curve}");
    assert_eq!(inner_on_curve, 4, "inner ring: {:?}", inner.points);
}

#[test]
fn closed_dejavu_b_round_stroke_stays_curved_and_compact() {
    let path = fc_fixtures::path("DejaVuSans.ttf");
    if !path.is_file() {
        eprintln!("skipping test: DejaVuSans.ttf is unavailable");
        return;
    }
    let outcome =
        fc_sfnt::read_path(&path, &fc_sfnt::ReadOptions::default()).expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let layer = font
        .glyph("B")
        .and_then(|glyph| glyph.master_layer(&font.default_master().id))
        .expect("B master layer");
    let source_on_curve = layer
        .contours
        .iter()
        .flat_map(|contour| &contour.points)
        .filter(|point| point.kind != PointKind::OffCurve)
        .count();
    let result = expand_stroke(
        &layer.contours,
        &closed_options(Nib::Round),
        CurveKind::Cubic,
    )
    .expect("round stroke B");
    let output_on_curve = result
        .iter()
        .flat_map(|contour| &contour.points)
        .filter(|point| point.kind != PointKind::OffCurve)
        .count();
    let output_off_curve = result
        .iter()
        .flat_map(|contour| &contour.points)
        .filter(|point| point.kind == PointKind::OffCurve)
        .count();
    assert!(
        output_on_curve <= source_on_curve * 3,
        "source on-curve={source_on_curve}, output on-curve={output_on_curve}"
    );
    assert!(output_off_curve > 0);
}

#[test]
fn rectangular_nib_closed_circle_keeps_curved_rings() {
    let result = expand_stroke(
        &[fixtures::circle(100.0)],
        &closed_options(Nib::Rect),
        CurveKind::Cubic,
    )
    .expect("rectangular nib circle");
    assert_eq!(result.len(), 2);
    assert!(result
        .iter()
        .flat_map(|contour| &contour.points)
        .any(|point| point.kind == PointKind::OffCurve));
}

#[test]
fn stroke_fallback_is_additive_and_reports_strict_success() {
    let square = fixtures::rect(0.0, 0.0, 200.0, 200.0);
    let strict = expand_stroke(
        std::slice::from_ref(&square),
        &closed_options(Nib::Round),
        CurveKind::Cubic,
    )
    .expect("strict square stroke");
    let outcome =
        expand_stroke_with_fallback(&[square], &closed_options(Nib::Round), CurveKind::Cubic)
            .expect("square stroke outcome");
    assert_eq!(outcome.fallback, FallbackKind::None);
    assert_eq!(outcome.contours, strict);
}

#[test]
fn dejavu_two_stroke_outcome_is_compact_and_accurate() {
    let path = fc_fixtures::path("DejaVuSans.ttf");
    if !path.is_file() {
        eprintln!("skipping test: DejaVuSans.ttf is unavailable");
        return;
    }
    let outcome =
        fc_sfnt::read_path(&path, &fc_sfnt::ReadOptions::default()).expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let layer = font
        .glyph("two")
        .and_then(|glyph| glyph.master_layer(&font.default_master().id))
        .expect("two master layer");
    let outcome = expand_stroke_with_fallback(
        &layer.contours,
        &closed_options(Nib::Round),
        CurveKind::Cubic,
    )
    .expect("round stroke two with fallback");
    let points = outcome
        .contours
        .iter()
        .map(|contour| contour.points.len())
        .sum::<usize>();
    assert!(points <= 300, "fallback has {points} points");
    let reference = minkowski_reference_area(&layer.contours, &closed_options(Nib::Round), 0.1);
    let area = fixtures::filled_area(&outcome.contours);
    assert!(
        (area - reference).abs() / reference < 0.01,
        "fallback area={area}, reference={reference}"
    );
}

#[test]
#[cfg(not(debug_assertions))]
fn dejavu_stroke_corpus_meets_strict_fallback_and_accuracy_targets() {
    let path = fc_fixtures::path("DejaVuSans.ttf");
    if !path.is_file() {
        eprintln!("skipping test: DejaVuSans.ttf is unavailable");
        return;
    }
    let outcome =
        fc_sfnt::read_path(&path, &fc_sfnt::ReadOptions::default()).expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let nibs = [
        Nib::Round,
        Nib::Rect,
        Nib::Calligraphic {
            width: 24.0,
            height: 8.0,
            angle: 0.4,
        },
    ];
    for nib in nibs {
        let opts = closed_options(nib);
        let mut successful = 0_usize;
        let mut curved = 0_usize;
        let mut fallback = 0_usize;
        let mut flattened = 0_usize;
        let mut total = 0_usize;
        for glyph in font.glyphs.values() {
            let Some(layer) = glyph.master_layer(&font.default_master().id) else {
                continue;
            };
            if !layer.contours.iter().any(|contour| contour.closed) {
                continue;
            }
            total += 1;
            if let Ok(result) = expand_stroke(&layer.contours, &opts, CurveKind::Cubic) {
                successful += 1;
                curved += usize::from(
                    result
                        .iter()
                        .flat_map(|contour| &contour.points)
                        .any(|point| point.kind == PointKind::OffCurve),
                );
            }
            if let Ok(outcome) =
                expand_stroke_with_fallback(&layer.contours, &opts, CurveKind::Cubic)
            {
                fallback += 1;
                flattened += usize::from(outcome.fallback == FallbackKind::Flattened);
            }
        }
        let mut sample_total = 0_usize;
        let mut sample_successful = 0_usize;
        let mut accurate = 0_usize;
        let mut worst_error = 0.0_f64;
        let mut worst_name = "";
        let mut worst_areas = (0.0, 0.0);
        // A deterministic cross-script sample of 300 alphabetic glyphs,
        // excluding the IPA modifier and combining-mark range.
        for (name, layer) in font
            .glyphs
            .iter()
            .filter_map(|(name, glyph)| {
                if !glyph
                    .unicodes
                    .iter()
                    .filter_map(|value| char::from_u32(*value))
                    .any(|value| {
                        value.is_alphabetic() && !('\u{0180}'..='\u{036f}').contains(&value)
                    })
                {
                    return None;
                }
                glyph
                    .master_layer(&font.default_master().id)
                    .filter(|layer| layer.contours.iter().any(|contour| contour.closed))
                    .map(|layer| (name, layer))
            })
            .take(300)
        {
            sample_total += 1;
            let Ok(result) = expand_stroke(&layer.contours, &opts, CurveKind::Cubic) else {
                continue;
            };
            sample_successful += 1;
            let reference = minkowski_reference_area(&layer.contours, &opts, 0.1);
            if reference <= 1e-9 {
                continue;
            }
            let area = nonzero_contour_area(&result, 0.1);
            let error = (area - reference).abs() / reference;
            if error > worst_error {
                worst_error = error;
                worst_name = name;
                worst_areas = (area, reference);
            }
            accurate += usize::from(error <= 0.02);
        }
        eprintln!(
            "DejaVu {nib:?} strokes: total={total}, strict={successful}, curved={curved}, fallback={fallback}, flattened={flattened}, accuracy={accurate}/{sample_successful} successes in {sample_total}-glyph sample, worst_error={worst_error:.6}, worst={worst_name} area={} reference={}",
            worst_areas.0,
            worst_areas.1,
        );
        let minimum = if nib == Nib::Round { 3_550 } else { 3_450 };
        assert!(successful >= minimum, "{nib:?}: strict={successful}");
        if nib == Nib::Round {
            assert_eq!(curved, successful);
        }
        assert_eq!(fallback, total, "{nib:?}");
        assert_eq!(sample_total, 300, "{nib:?}");
        assert_eq!(accurate, sample_successful, "{nib:?}: worst={worst_error}");
    }
}

fn minkowski_reference_area(contours: &[Contour], opts: &StrokeOptions, tolerance: f64) -> f64 {
    use kurbo::{flatten, PathEl};

    let nib = reference_nib_vertices(opts.nib);
    let mut sweeps = Vec::<Vec<[f64; 2]>>::new();
    for contour in contours {
        let mut points = Vec::new();
        flatten(contour.to_bezpath().iter(), tolerance, |element| {
            if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                points.push([point.x, point.y]);
            }
        });
        if contour.closed && points.first() != points.last() {
            if let Some(first) = points.first().copied() {
                points.push(first);
            }
        }
        for edge in points.windows(2) {
            let cloud: Vec<_> = edge
                .iter()
                .flat_map(|point| {
                    nib.iter()
                        .map(move |offset| [point[0] + offset[0], point[1] + offset[1]])
                })
                .collect();
            if let Some(hull) = convex_hull(&cloud) {
                sweeps.push(hull);
            }
        }
    }
    overlay_area(&sweeps)
}

#[cfg(not(debug_assertions))]
fn nonzero_contour_area(contours: &[Contour], tolerance: f64) -> f64 {
    use kurbo::{flatten, PathEl};

    let polygons: Vec<_> = contours
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
    overlay_area(&polygons)
}

fn overlay_area(polygons: &Vec<Vec<[f64; 2]>>) -> f64 {
    use i_overlay::{
        core::{fill_rule::FillRule, overlay_rule::OverlayRule},
        float::single::SingleFloatOverlay,
    };

    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    polygons
        .overlay(&empty, OverlayRule::Union, FillRule::NonZero)
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

fn reference_nib_vertices(nib: Nib) -> Vec<[f64; 2]> {
    match nib {
        Nib::Round => (0..64)
            .map(|index| {
                let angle = std::f64::consts::TAU * index as f64 / 64.0;
                [10.0 * angle.cos(), 10.0 * angle.sin()]
            })
            .collect(),
        Nib::Rect => vec![[-10.0, -10.0], [10.0, -10.0], [10.0, 10.0], [-10.0, 10.0]],
        Nib::Calligraphic {
            width,
            height,
            angle,
        } => {
            let (sin, cos) = angle.sin_cos();
            [
                [-width * 0.5, -height * 0.5],
                [width * 0.5, -height * 0.5],
                [width * 0.5, height * 0.5],
                [-width * 0.5, height * 0.5],
            ]
            .into_iter()
            .map(|point| {
                [
                    point[0] * cos - point[1] * sin,
                    point[0] * sin + point[1] * cos,
                ]
            })
            .collect()
        }
    }
}

fn convex_hull(points: &[[f64; 2]]) -> Option<Vec<[f64; 2]>> {
    let mut points = points.to_vec();
    points.sort_by(|left, right| {
        left[0]
            .total_cmp(&right[0])
            .then(left[1].total_cmp(&right[1]))
    });
    points.dedup();
    if points.len() < 3 {
        return None;
    }
    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let mut lower = Vec::new();
    for &point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper = Vec::new();
    for &point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    Some(lower)
}

#[test]
fn closed_ring_removal_flags_are_independent_for_every_nib() {
    for nib in [
        Nib::Round,
        Nib::Rect,
        Nib::Calligraphic {
            width: 24.0,
            height: 8.0,
            angle: 0.4,
        },
    ] {
        let circle = fixtures::circle(100.0);
        let keep = expand_stroke(
            std::slice::from_ref(&circle),
            &StrokeOptions {
                remove_internal: false,
                remove_external: false,
                ..options(nib)
            },
            CurveKind::Cubic,
        )
        .expect("keep both rings");
        let no_inner = expand_stroke(
            std::slice::from_ref(&circle),
            &StrokeOptions {
                remove_internal: true,
                remove_external: false,
                ..options(nib)
            },
            CurveKind::Cubic,
        )
        .expect("drop inner ring");
        let no_outer = expand_stroke(
            std::slice::from_ref(&circle),
            &StrokeOptions {
                remove_internal: false,
                remove_external: true,
                ..options(nib)
            },
            CurveKind::Cubic,
        )
        .expect("drop outer ring");
        let neither = expand_stroke(
            &[circle],
            &StrokeOptions {
                remove_internal: true,
                remove_external: true,
                ..options(nib)
            },
            CurveKind::Cubic,
        )
        .expect("drop both rings");
        assert_eq!(keep.len(), 2, "{nib:?}");
        assert_eq!(no_inner.len(), 1, "{nib:?}");
        assert_eq!(no_outer.len(), 1, "{nib:?}");
        assert!(neither.is_empty(), "{nib:?}");
        assert!(fixtures::filled_area(&no_inner) > fixtures::filled_area(&no_outer));
    }
}

#[test]
fn calligraphic_open_cubic_stays_curved_and_compact() {
    let curve = Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, PointKind::Move),
            fixtures::point(25.0, 80.0, PointKind::OffCurve),
            fixtures::point(75.0, -80.0, PointKind::OffCurve),
            fixtures::point(100.0, 0.0, PointKind::Curve),
        ],
        name: None,
        id: None,
    };
    let result = expand_stroke(
        &[curve],
        &StrokeOptions {
            cap: Cap::Butt,
            join: Join::Bevel,
            remove_internal: false,
            remove_external: false,
            ..options(Nib::Calligraphic {
                width: 24.0,
                height: 8.0,
                angle: 0.4,
            })
        },
        CurveKind::Cubic,
    )
    .expect("curved calligraphic stroke");
    assert_eq!(result.len(), 1);
    let on_curve = result[0]
        .points
        .iter()
        .filter(|point| point.kind != PointKind::OffCurve)
        .count();
    assert!(
        on_curve <= 26,
        "whole outline has {on_curve} on-curve points"
    );
    assert!(result[0]
        .points
        .iter()
        .any(|point| point.kind == PointKind::Curve));
}

#[test]
fn rectangular_nib_closed_circle_has_analytic_swept_area() {
    let result = expand_stroke(
        &[fixtures::circle(100.0)],
        &closed_options(Nib::Rect),
        CurveKind::Cubic,
    )
    .expect("rectangular nib circle");
    let expected = 16_000.0;
    assert!((fixtures::filled_area(&result) - expected).abs() / expected < 0.02);
}

#[test]
fn convex_nib_caps_and_joins_change_geometry() {
    let line = fixtures::open_line(0.0, 0.0, 100.0, 0.0);
    let mut bounds = Vec::new();
    for cap in [Cap::Butt, Cap::Round, Cap::Square] {
        let result = expand_stroke(
            std::slice::from_ref(&line),
            &StrokeOptions {
                cap,
                remove_internal: false,
                remove_external: false,
                ..options(Nib::Rect)
            },
            CurveKind::Cubic,
        )
        .expect("rect cap");
        bounds.push(bbox(&result).expect("stroke bounds"));
    }
    assert!(bounds[0].x0 >= -0.01 && bounds[0].x1 <= 100.01);
    assert!(bounds[1].x0 < bounds[0].x0 && bounds[1].x1 > bounds[0].x1);
    assert!(bounds[2].x0 < bounds[0].x0 && bounds[2].x1 > bounds[0].x1);

    let corner = Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, PointKind::Move),
            fixtures::point(100.0, 0.0, PointKind::Line),
            fixtures::point(100.0, 100.0, PointKind::Line),
        ],
        name: None,
        id: None,
    };
    let areas: Vec<_> = [Join::Bevel, Join::Miter, Join::Round]
        .into_iter()
        .map(|join| {
            expand_stroke(
                std::slice::from_ref(&corner),
                &StrokeOptions {
                    join,
                    remove_internal: false,
                    remove_external: false,
                    ..options(Nib::Rect)
                },
                CurveKind::Cubic,
            )
            .map(|result| fixtures::filled_area(&result))
            .expect("rect join")
        })
        .collect();
    assert!(areas[0] != areas[1] || areas[1] != areas[2]);
}
