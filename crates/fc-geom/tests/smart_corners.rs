use fc_model::{Contour, PointKind};
use kurbo::{BezPath, ParamCurve, Point, Shape};

fn corner(source: &str, node: (f64, f64), radius: f64) -> Contour {
    let mut contour = Contour::from_bezpath(&BezPath::from_svg(source).unwrap()).remove(0);
    contour
        .points
        .iter_mut()
        .find(|p| p.kind != PointKind::OffCurve && p.x == node.0 && p.y == node.1)
        .unwrap()
        .corner_radius = radius;
    contour
}

#[test]
fn positive_corner_matches_analytic_circle_and_keeps_source() {
    let contour = corner("M0 0L100 0L100 100L0 100Z", (100., 0.), 20.);
    let original = contour.clone();
    let path = fc_geom::smart_corner_path(&contour).unwrap();
    assert_eq!(contour, original);
    assert_eq!(path.bounding_box(), original.to_bezpath().bounding_box());
    let center = Point::new(80., 20.);
    let arc = path
        .segments()
        .find(|s| s.start() == Point::new(80., 0.))
        .unwrap();
    for i in 0..=100 {
        let p = arc.eval(f64::from(i) / 100.);
        assert!((p.distance(center) - 20.).abs() < 0.01);
    }
    assert_eq!(path.winding((99., 1.).into()), 0);
    assert_ne!(path.winding((90., 10.).into()), 0);
}

#[test]
fn ink_trap_cuts_into_fill_with_either_winding() {
    for source in ["M0 0L100 0L100 100L0 100Z", "M0 100L100 100L100 0L0 0Z"] {
        let contour = corner(source, (100., 0.), -20.);
        let path = fc_geom::smart_corner_path(&contour).unwrap();
        assert_eq!(path.winding((90., 10.).into()), 0, "{source}");
        assert_ne!(path.winding((70., 30.).into()), 0, "{source}");
        assert!(path.is_finite());
    }
}

#[test]
fn neighboring_corners_and_invalid_nodes_fail_explicitly() {
    let mut contour = corner("M0 0L100 0L100 100L0 100Z", (100., 0.), 60.);
    contour
        .points
        .iter_mut()
        .find(|p| p.x == 0. && p.y == 0.)
        .unwrap()
        .corner_radius = 60.;
    assert!(fc_geom::smart_corner_path(&contour)
        .unwrap_err()
        .to_string()
        .contains("overlap"));
    let endpoint = corner("M0 0L100 0L100 100", (0., 0.), 10.);
    assert!(fc_geom::smart_corner_path(&endpoint).is_err());
    let nonfinite = corner("M0 0L100 0L100 100", (100., 0.), f64::NAN);
    assert!(fc_geom::smart_corner_path(&nonfinite).is_err());
}

#[test]
fn curved_join_is_trimmed_with_a_tangent_circular_fillet() {
    let contour = corner("M0 0C30 -10 70 -10 100 0L100 100", (100., 0.), 10.);
    let path = fc_geom::smart_corner_path(&contour).unwrap();
    let segments: Vec<_> = path.segments().collect();
    assert!(segments.len() >= 3);
    for pair in segments.windows(2) {
        let left = pair[0].to_cubic();
        let right = pair[1].to_cubic();
        let a = if left.p3 == left.p2 {
            left.p3 - left.p0
        } else {
            left.p3 - left.p2
        };
        let b = if right.p1 == right.p0 {
            right.p3 - right.p0
        } else {
            right.p1 - right.p0
        };
        assert!(a.cross(b).abs() / (a.hypot() * b.hypot()) < 0.001);
    }
    assert_eq!(segments[0].start(), Point::new(0., 0.));
    assert_eq!(segments.last().unwrap().end(), Point::new(100., 100.));
}

#[test]
fn trap_on_a_counter_cuts_into_surrounding_ink() {
    let mut font = fc_model::Font::new_empty("Counter", 1000);
    let master = font.default_master().id.clone();
    let mut glyph = fc_model::Glyph::new("counter");
    let mut layer = fc_model::Layer::new_master(master.clone());
    layer.contours = vec![
        corner("M0 0L200 0L200 200L0 200Z", (200., 0.), 0.),
        corner("M50 50L50 150L150 150L150 50Z", (150., 50.), -20.),
    ];
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let before = fc_model::flatten_layer(&font, "counter", &master).unwrap();
    let after = fc_geom::flatten_layer(&font, "counter", &master).unwrap();
    assert_ne!(before.winding((155., 45.).into()), 0);
    assert_eq!(after.winding((155., 45.).into()), 0);
    assert_ne!(after.winding((180., 20.).into()), 0);
}

#[test]
fn concave_quadratic_and_small_interpolated_radii_evaluate() {
    let contour = corner("M0 0L100 0L100 50L50 50L50 100L0 100Z", (50., 50.), 10.);
    let path = fc_geom::smart_corner_path(&contour).unwrap();
    assert_ne!(path.winding((51., 51.).into()), 0);
    assert_eq!(path.winding((59., 59.).into()), 0);
    let quad = corner("M0 0Q50 -20 100 0L100 100", (100., 0.), 10.);
    assert!(fc_geom::smart_corner_path(&quad).unwrap().is_finite());
    for radius in [0.05, 0.0001, 0.000001, 0.000000001, -0.000001] {
        let contour = corner("M0 0L100 0L100 100L0 100Z", (100., 0.), radius);
        let path = fc_geom::smart_corner_path(&contour).unwrap();
        assert!(path.is_finite());
        assert!((path.area() - 10000.).abs() <= radius * radius * 2. + 1e-8);
    }
}

#[test]
fn nested_local_smart_radii_survive_global_resolution_and_export_baking() {
    use fc_model::{Component, Font, Glyph, Layer, LayerKind, SmartAxis};
    let mut font = Font::new_empty("Local corner", 1000);
    let master = font.default_master().id.clone();
    let mut part = Glyph::new("part");
    part.smart_axes.push(SmartAxis {
        name: "Round".into(),
        min: 0.,
        default: 0.,
        max: 1.,
    });
    let mut base = Layer::new_master(master.clone());
    base.contours
        .push(corner("M0 0L100 0L100 100L0 100Z", (100., 0.), 0.));
    let mut alternate = base.clone();
    alternate.id = fc_model::LayerId::new();
    alternate.kind = LayerKind::Smart {
        master: master.clone(),
        location: [("Round".into(), 1.)].into(),
    };
    alternate.contours[0].points[1].corner_radius = 20.;
    part.layers = vec![base, alternate];
    font.insert_glyph(part).unwrap();
    for (name, base, smart) in [
        ("middle", "part", Some([("Round".into(), 0.5)].into())),
        ("outer", "middle", None),
    ] {
        let mut glyph = Glyph::new(name);
        let mut layer = Layer::new_master(master.clone());
        layer.components.push(Component {
            base: base.into(),
            smart,
            ..Default::default()
        });
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    for name in ["middle", "outer"] {
        let glyph = font.glyph(name).unwrap();
        let resolved =
            fc_geom::variation::interpolate_glyph(&font, glyph, &Default::default()).unwrap();
        assert!(resolved.components.is_empty());
        let path = resolved
            .contours
            .iter()
            .fold(BezPath::new(), |mut path, c| {
                path.extend(c.to_bezpath());
                path
            });
        assert_eq!(path.winding((99., 1.).into()), 0);
        assert_ne!(path.winding((90., 10.).into()), 0);
    }
    let resolved =
        fc_geom::resolve_smart_layer(&font, &font.glyph("middle").unwrap().layers[0]).unwrap();
    assert_eq!(
        resolved.contours[0].to_bezpath().winding((99., 1.).into()),
        0
    );
    let (baked, _) = fc_geom::bake_effects_font(&font).unwrap();
    assert!(!baked.glyph("outer").unwrap().layers[0].has_smart_corners());
    assert_eq!(
        fc_model::flatten_layer(&baked, "outer", &master)
            .unwrap()
            .winding((99., 1.).into()),
        0
    );
}

#[test]
fn global_interpolation_rejects_changed_corner_topology() {
    use fc_model::{Axis, Font, Glyph, Layer, MasterId, Tag};
    let mut font = Font::new_empty("Corner topology", 1000);
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 0.,
        default: 0.,
        max: 1.,
        ..Default::default()
    });
    let mut second = font.default_master().clone();
    second.id = MasterId::new();
    second.location.insert(Tag(*b"wght"), 1.);
    font.masters.push(second);
    let mut glyph = Glyph::new("A");
    for (i, master) in font.masters.iter().enumerate() {
        let mut layer = Layer::new_master(master.id.clone());
        layer.contours.push(corner(
            "M0 0L100 0L100 100L0 100Z",
            (100., 0.),
            if i == 0 { 0. } else { 20. },
        ));
        glyph.layers.push(layer);
    }
    font.insert_glyph(glyph).unwrap();
    let error = fc_geom::variation::interpolate_glyph(
        &font,
        font.glyph("A").unwrap(),
        &[(Tag(*b"wght"), 0.5)].into(),
    )
    .unwrap_err();
    let fc_geom::Error::Incompatible(issues) = error else {
        panic!("expected a topology incompatibility, got {error}");
    };
    assert!(issues
        .iter()
        .any(|issue| issue.reason.contains("point count")));
}
