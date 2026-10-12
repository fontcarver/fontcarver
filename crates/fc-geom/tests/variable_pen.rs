use fc_model::{
    Contour, EffectCap, EffectJoin, EffectNib, EffectProgression, Font, Glyph, Layer, LayerEffect,
    LayerEffectKind, PenPoint,
};
use kurbo::{BezPath, Point, Rect, Shape};

fn fixture(source: &str, cap: EffectCap, join: EffectJoin, nib: EffectNib) -> Font {
    let mut font = Font::new_empty("Variable pen", 1000);
    let mut glyph = Glyph::new("pen");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.contours = Contour::from_bezpath(&BezPath::from_svg(source).unwrap());
    layer.effects.push(LayerEffect {
        enabled: true,
        effect: LayerEffectKind::VariableStroke {
            nib,
            cap,
            join,
            miter_limit: 4.,
            progression: EffectProgression::Linear,
            points: vec![
                PenPoint {
                    position: 0.,
                    width: 20.,
                    height: 20.,
                    angle: 0.,
                },
                PenPoint {
                    position: 1.,
                    width: 60.,
                    height: 60.,
                    angle: 0.,
                },
            ],
        },
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

fn path(font: &Font) -> BezPath {
    fc_geom::flatten_layer(font, "pen", &font.default_master().id).unwrap()
}
fn parameters(font: &mut Font) -> (&mut Vec<PenPoint>, &mut EffectProgression) {
    let LayerEffectKind::VariableStroke {
        points,
        progression,
        ..
    } = &mut font.glyph_mut("pen").unwrap().layers[0].effects[0].effect
    else {
        unreachable!()
    };
    (points, progression)
}

#[test]
fn taper_uses_normalized_length_and_exact_butt_bounds_for_both_nibs() {
    for nib in [EffectNib::Ellipse, EffectNib::Rectangle] {
        let font = fixture("M0 0L200 0", EffectCap::Butt, EffectJoin::Miter, nib);
        let original = font.clone();
        let result = path(&font);
        assert_eq!(result.bounding_box(), Rect::new(0., -30., 200., 30.));
        assert_eq!(result.winding(Point::new(25., 20.)), 0);
        assert_ne!(result.winding(Point::new(175., 20.)), 0);
        assert_eq!(font, original);
        let evaluated =
            fc_geom::evaluate_layer(&font, "pen", &font.glyph("pen").unwrap().layers[0].id)
                .unwrap();
        assert_eq!(evaluated.approximated_layers.len(), 1);
    }
}

#[test]
fn smooth_progression_and_interior_positions_change_the_envelope() {
    let mut font = fixture(
        "M0 0L200 0",
        EffectCap::Butt,
        EffectJoin::Round,
        EffectNib::Ellipse,
    );
    assert_ne!(path(&font).winding(Point::new(50., 14.)), 0);
    *parameters(&mut font).1 = EffectProgression::Smooth;
    assert_eq!(path(&font).winding(Point::new(50., 14.)), 0);
    let (points, _) = parameters(&mut font);
    points.insert(
        1,
        PenPoint {
            position: 0.5,
            width: 100.,
            height: 100.,
            angle: 0.,
        },
    );
    assert_ne!(path(&font).winding(Point::new(100., 45.)), 0);
}

#[test]
fn caps_and_join_limits_have_distinct_geometry() {
    for cap in [EffectCap::Butt, EffectCap::Round, EffectCap::Square] {
        let font = fixture("M0 0L200 0", cap, EffectJoin::Round, EffectNib::Ellipse);
        let result = path(&font);
        let bounds = result.bounding_box();
        assert!(
            (bounds.x0 - if cap == EffectCap::Butt { 0. } else { -10. }).abs() < 0.06,
            "{bounds:?}"
        );
        assert!(
            (bounds.x1 - if cap == EffectCap::Butt { 200. } else { 230. }).abs() < 0.06,
            "{bounds:?}"
        );
        assert_eq!(
            result.winding(Point::new(224., 24.)) != 0,
            cap == EffectCap::Square
        );
    }
    for join in [EffectJoin::Bevel, EffectJoin::Round, EffectJoin::Miter] {
        let mut font = fixture(
            "M0 0L100 0L100 100",
            EffectCap::Butt,
            join,
            EffectNib::Ellipse,
        );
        for point in parameters(&mut font).0 {
            point.width = 40.;
            point.height = 40.;
        }
        let result = path(&font);
        assert_eq!(
            result.winding(Point::new(115., -15.)) != 0,
            join == EffectJoin::Miter
        );
        assert_eq!(
            result.winding(Point::new(113., -13.)) != 0,
            join != EffectJoin::Bevel
        );
        assert_ne!(result.winding(Point::new(90., 10.)), 0, "inner join hole");
        if let LayerEffectKind::VariableStroke { miter_limit, .. } =
            &mut font.glyph_mut("pen").unwrap().layers[0].effects[0].effect
        {
            *miter_limit = 1.;
        }
        if join == EffectJoin::Miter {
            assert_eq!(path(&font).winding(Point::new(115., -15.)), 0);
        }
    }
}

#[test]
fn closed_curves_keep_counters_and_thick_curves_fill_without_inverted_holes() {
    for source in ["M0 0L100 0L100 100L0 100Z", "M0 0L0 100L100 100L100 0Z"] {
        let mut font = fixture(
            source,
            EffectCap::Round,
            EffectJoin::Round,
            EffectNib::Ellipse,
        );
        let (points, _) = parameters(&mut font);
        points[1] = PenPoint {
            position: 1.,
            ..points[0]
        };
        let result = path(&font);
        assert_eq!(result.winding(Point::new(50., 50.)), 0);
        assert_ne!(result.winding(Point::new(5., 50.)), 0);
        for point in parameters(&mut font).0 {
            point.width = 240.;
            point.height = 240.;
        }
        assert_ne!(path(&font).winding(Point::new(50., 50.)), 0);
    }
}

#[test]
fn rotated_nibs_curves_multicontours_and_bake_are_finite_and_deterministic() {
    let started = std::time::Instant::now();
    for nib in [EffectNib::Ellipse, EffectNib::Rectangle] {
        let mut font = fixture(
            "M0 0C0 100 200 100 200 0 M300 0Q400 200 500 0",
            EffectCap::Round,
            EffectJoin::Round,
            nib,
        );
        let (points, progression) = parameters(&mut font);
        points[0].height = 10.;
        points[0].angle = 170.;
        points[1].angle = -170.;
        *progression = EffectProgression::Smooth;
        let result = path(&font);
        assert!(result.is_finite());
        assert!(!result.is_empty());
        assert_ne!(result.winding(Point::new(100., 75.)), 0);
        let (baked, approximated) = fc_geom::bake_effects_font(&font).unwrap();
        assert_eq!(approximated.len(), 1);
        assert_eq!(
            result,
            fc_model::flatten_layer(&baked, "pen", &baked.default_master().id).unwrap()
        );
        assert!(baked.glyph("pen").unwrap().layers[0].effects.is_empty());
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "small multi-contour variable pen evaluation exceeded ten seconds"
    );
}

#[test]
fn round_reversal_join_covers_the_turnaround() {
    let mut font = fixture(
        "M0 0L100 0L0 0",
        EffectCap::Butt,
        EffectJoin::Round,
        EffectNib::Ellipse,
    );
    for point in parameters(&mut font).0 {
        point.width = 40.;
        point.height = 40.;
    }
    assert_ne!(path(&font).winding(Point::new(115., 0.)), 0);
}

#[test]
fn incompatible_seams_and_font_grid_overflow_fail_explicitly() {
    let font = fixture(
        "M0 0L100 0L100 100Z",
        EffectCap::Round,
        EffectJoin::Round,
        EffectNib::Ellipse,
    );
    assert!(
        fc_geom::flatten_layer(&font, "pen", &font.default_master().id)
            .unwrap_err()
            .to_string()
            .contains("matching")
    );
    let font = fixture(
        "M32760 0L32767 0",
        EffectCap::Round,
        EffectJoin::Round,
        EffectNib::Ellipse,
    );
    assert!(fc_geom::flatten_layer(&font, "pen", &font.default_master().id).is_err());
}

#[test]
fn rotated_rectangle_square_caps_extend_only_along_tangent() {
    let mut font = fixture(
        "M0 0L100 0",
        EffectCap::Square,
        EffectJoin::Round,
        EffectNib::Rectangle,
    );
    for point in parameters(&mut font).0 {
        point.width = 40.;
        point.height = 20.;
        point.angle = 45.;
    }
    let bounds = path(&font).bounding_box();
    let half_height = 30. / 2f64.sqrt();
    assert!((bounds.y0 + half_height).abs() < 0.06, "{bounds:?}");
    assert!((bounds.y1 - half_height).abs() < 0.06, "{bounds:?}");
}

#[test]
fn unrepresentable_pen_spacing_is_rejected_without_nonfinite_geometry() {
    let mut font = fixture(
        "M100 0L200 0",
        EffectCap::Butt,
        EffectJoin::Round,
        EffectNib::Ellipse,
    );
    parameters(&mut font).0.insert(
        1,
        PenPoint {
            position: 1e-20,
            width: 30.,
            height: 30.,
            angle: 0.,
        },
    );
    font.validate_effects().unwrap();
    let error = fc_geom::flatten_layer(&font, "pen", &font.default_master().id).unwrap_err();
    assert!(error.to_string().contains("too close"));
}
