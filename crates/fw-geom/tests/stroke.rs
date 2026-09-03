mod fixtures;

use fw_geom::{bbox, expand_stroke, Cap, CurveKind, Join, Nib, StrokeOptions};
use fw_model::{Contour, PointKind};
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
    let curve = fw_model::Contour {
        closed: false,
        points: vec![
            fixtures::point(0.0, 0.0, fw_model::PointKind::Move),
            fixtures::point(25.0, 80.0, fw_model::PointKind::OffCurve),
            fixtures::point(75.0, -80.0, fw_model::PointKind::OffCurve),
            fixtures::point(100.0, 0.0, fw_model::PointKind::Curve),
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
    let path = fw_fixtures::path("DejaVuSans.ttf");
    if !path.is_file() {
        eprintln!("skipping test: DejaVuSans.ttf is unavailable");
        return;
    }
    let outcome =
        fw_sfnt::read_path(&path, &fw_sfnt::ReadOptions::default()).expect("read DejaVu Sans");
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
#[cfg(not(debug_assertions))]
fn dejavu_round_stroke_corpus_is_at_least_ninety_percent_curved() {
    let path = fw_fixtures::path("DejaVuSans.ttf");
    if !path.is_file() {
        eprintln!("skipping test: DejaVuSans.ttf is unavailable");
        return;
    }
    let outcome =
        fw_sfnt::read_path(&path, &fw_sfnt::ReadOptions::default()).expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let mut total = 0_usize;
    let mut successful = 0_usize;
    let mut curved = 0_usize;
    for glyph in font.glyphs.values() {
        let Some(layer) = glyph.master_layer(&font.default_master().id) else {
            continue;
        };
        if !layer.contours.iter().any(|contour| contour.closed) {
            continue;
        }
        total += 1;
        let Ok(result) = expand_stroke(
            &layer.contours,
            &closed_options(Nib::Round),
            CurveKind::Cubic,
        ) else {
            continue;
        };
        successful += 1;
        if result
            .iter()
            .flat_map(|contour| &contour.points)
            .any(|point| point.kind == PointKind::OffCurve)
        {
            curved += 1;
        }
    }
    eprintln!(
        "DejaVu round strokes: total={total}, successful={successful}, failed={}, curved={curved}",
        total - successful
    );
    assert!(successful > 0);
    assert!(curved * 10 >= successful * 9);
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
