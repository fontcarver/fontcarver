mod fixtures;
use fc_geom::{bbox, style_effect, CurveKind, StyleOptions};
use kurbo::Shape;

#[test]
fn anisotropic_weight_expands_and_contracts_both_boundaries_of_a_counter() {
    let outer = fixtures::rect(0., 0., 300., 500.);
    let inner = fc_geom::reverse(&fixtures::rect(100., 100., 200., 400.));
    let input = vec![outer, inner];
    for sign in [1., -1.] {
        let result = style_effect(
            &input,
            &StyleOptions {
                weight: [10. * sign, 20. * sign],
                ..Default::default()
            },
            CurveKind::Cubic,
        )
        .unwrap();
        let bounds = bbox(&result.contours).unwrap();
        assert!((bounds.x0 + sign * 10.).abs() < 0.1);
        assert!((bounds.y1 - (500. + sign * 20.)).abs() < 0.1);
        let paths = fc_geom::to_bezpaths(&result.contours);
        let filled = |x, y| {
            paths
                .iter()
                .map(|p| p.winding(kurbo::Point::new(x, y)))
                .sum::<i32>()
                != 0
        };
        assert!(!filled(150., 250.));
        assert_eq!(filled(105., 250.), sign > 0.);
    }
}
#[test]
fn affine_style_preserves_open_curves_and_reports_the_anchor_transform() {
    let input = vec![fixtures::open_line(0., 0., 100., 100.)];
    let result = style_effect(
        &input,
        &StyleOptions {
            scale: [0.5, 0.7],
            translate: [10., 200.],
            oblique: 0.,
            ..Default::default()
        },
        CurveKind::Cubic,
    )
    .unwrap();
    assert!(!result.contours[0].closed);
    assert_eq!(
        result.transform * kurbo::Point::new(100., 100.),
        kurbo::Point::new(60., 270.)
    );
    let b = bbox(&result.contours).unwrap();
    assert_eq!((b.x0, b.y0, b.x1, b.y1), (10., 200., 60., 270.));
}
#[test]
fn invalid_styles_and_open_weight_paths_are_rejected_without_mutation() {
    let input = vec![fixtures::open_line(0., 0., 100., 100.)];
    let original = input.clone();
    for options in [
        StyleOptions {
            scale: [0., 1.],
            ..Default::default()
        },
        StyleOptions {
            oblique: 90.,
            ..Default::default()
        },
        StyleOptions {
            weight: [10., -10.],
            ..Default::default()
        },
        StyleOptions {
            weight: [10., 10.],
            ..Default::default()
        },
        StyleOptions {
            translate: [f64::NAN, 0.],
            ..Default::default()
        },
    ] {
        assert!(style_effect(&input, &options, CurveKind::Cubic).is_err());
    }
    assert_eq!(input, original);
}

#[test]
fn heavy_contraction_removes_thin_shapes_and_quadratic_output_stays_quadratic() {
    let input = vec![fixtures::rect(0., 0., 10., 500.)];
    let removed = style_effect(
        &input,
        &StyleOptions {
            weight: [-20., -20.],
            ..Default::default()
        },
        CurveKind::Cubic,
    )
    .unwrap();
    assert!(removed.contours.is_empty(), "{:?}", removed.contours);
    let round = style_effect(
        &input,
        &StyleOptions {
            weight: [100., 50.],
            ..Default::default()
        },
        CurveKind::Quadratic { max_err: 0.1 },
    )
    .unwrap();
    assert!(round.contours.iter().any(|c| c.is_quadratic()));
    assert!(!round
        .contours
        .iter()
        .flat_map(|c| &c.points)
        .any(|p| p.kind == fc_model::PointKind::Curve));
    let bounds = bbox(&round.contours).unwrap();
    assert!((bounds.x0 + 100.).abs() < 0.1 && (bounds.y1 - 550.).abs() < 0.1);
}

#[test]
fn curved_offsets_and_overlapping_sources_use_the_filled_boundary() {
    for weight in [10., -10.] {
        let result = style_effect(
            &[fixtures::circle(100.)],
            &StyleOptions {
                weight: [weight, weight],
                ..Default::default()
            },
            CurveKind::Cubic,
        )
        .unwrap();
        let bounds = bbox(&result.contours).unwrap();
        assert!((bounds.x1 - 100. - weight).abs() < 0.1, "{bounds:?}");
        assert!(result
            .contours
            .iter()
            .flat_map(|c| &c.points)
            .any(|p| p.kind == fc_model::PointKind::Curve));
    }
    let sources = [
        fixtures::rect(0., 0., 200., 300.),
        fixtures::rect(100., 0., 300., 300.),
    ];
    let result = style_effect(
        &sources,
        &StyleOptions {
            weight: [-20., -20.],
            ..Default::default()
        },
        CurveKind::Cubic,
    )
    .unwrap();
    let paths = fc_geom::to_bezpaths(&result.contours);
    for x in [99., 100., 101., 199., 200., 201.] {
        assert_ne!(
            paths
                .iter()
                .map(|p| p.winding(kurbo::Point::new(x, 150.)))
                .sum::<i32>(),
            0,
            "internal overlap eroded at x={x}"
        );
    }
}

#[test]
fn noto_curves_report_refitting_and_emboldening_keeps_the_original_stem_filled() {
    let font = fc_sfnt::read_path(
        &fc_fixtures::path("NotoSans-Regular.ttf"),
        &Default::default(),
    )
    .unwrap()
    .fonts
    .remove(0);
    for name in ["B", "S"] {
        let source = &font
            .glyph(name)
            .unwrap()
            .master_layer(&font.default_master().id)
            .unwrap()
            .contours;
        let output = style_effect(
            source,
            &StyleOptions {
                weight: [10., 10.],
                ..Default::default()
            },
            CurveKind::Quadratic { max_err: 0.1 },
        )
        .unwrap();
        if name == "S" {
            assert_eq!(output.fallback, fc_geom::FallbackKind::Flattened, "{name}");
        }
        let original: Vec<_> = source.iter().map(|c| c.to_bezpath()).collect();
        let result: Vec<_> = output.contours.iter().map(|c| c.to_bezpath()).collect();
        if name == "B" {
            let point = kurbo::Point::new(109.34, 357.);
            assert_ne!(
                result.iter().map(|p| p.winding(point)).sum::<i32>(),
                0,
                "lost B stem at probe point"
            );
        }
        // Emboldening cannot remove an originally filled point.
        for x in (100..=500).step_by(20) {
            for y in (0..=700).step_by(20) {
                let point = kurbo::Point::new(x as f64 + 0.37, y as f64 + 0.29);
                if original.iter().map(|p| p.winding(point)).sum::<i32>() != 0 {
                    assert_ne!(
                        result.iter().map(|p| p.winding(point)).sum::<i32>(),
                        0,
                        "{name} lost ink at {point:?}"
                    );
                }
            }
        }
    }
}
