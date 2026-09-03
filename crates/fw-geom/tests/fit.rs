use fw_geom::{fit_span, CurveKind};
use kurbo::{CubicBez, ParamCurve, PathSeg, Point, QuadBez, Vec2};

const SAMPLES: usize = 64;

fn sample(segment: PathSeg) -> Vec<Point> {
    (0..=SAMPLES)
        .map(|step| segment.eval(step as f64 / SAMPLES as f64))
        .collect()
}

fn deviation(segment: &PathSeg, samples: &[Point]) -> f64 {
    samples
        .iter()
        .map(|&point| {
            let nearest = kurbo::ParamCurveNearest::nearest(segment, point, 1e-9);
            nearest.distance_sq.sqrt()
        })
        .fold(0.0_f64, f64::max)
}

/// A quarter circle of radius 100 centred on the origin, as a cubic.
fn arc() -> CubicBez {
    let k = 0.552_284_749_830_793_4 * 100.0;
    CubicBez::new(
        Point::new(100.0, 0.0),
        Point::new(100.0, k),
        Point::new(k, 100.0),
        Point::new(0.0, 100.0),
    )
}

#[test]
fn cubic_fit_reproduces_a_synthetic_arc() {
    let arc = arc();
    let samples = sample(PathSeg::Cubic(arc));
    let fitted = fit_span(
        &samples,
        Vec2::new(0.0, 1.0),
        Vec2::new(-1.0, 0.0),
        CurveKind::Cubic,
    );
    assert!(matches!(fitted, PathSeg::Cubic(_)));
    assert!(
        deviation(&fitted, &samples) <= 1e-3,
        "deviation {}",
        deviation(&fitted, &samples)
    );
}

#[test]
fn cubic_fit_joins_two_halves_of_an_arc_within_a_unit() {
    let arc = arc();
    let mut samples = sample(PathSeg::Cubic(arc.subsegment(0.0..0.5)));
    samples.extend(
        sample(PathSeg::Cubic(arc.subsegment(0.5..1.0)))
            .into_iter()
            .skip(1),
    );
    let fitted = fit_span(
        &samples,
        Vec2::new(0.0, 1.0),
        Vec2::new(-1.0, 0.0),
        CurveKind::Cubic,
    );
    assert!(deviation(&fitted, &samples) <= 1.0);
}

#[test]
fn quadratic_fit_uses_the_tangent_intersection() {
    let quad = QuadBez::new(
        Point::new(0.0, 0.0),
        Point::new(50.0, 80.0),
        Point::new(100.0, 0.0),
    );
    let samples = sample(PathSeg::Quad(quad));
    let fitted = fit_span(
        &samples,
        quad.p1 - quad.p0,
        quad.p2 - quad.p1,
        CurveKind::Quadratic { max_err: 0.1 },
    );
    match fitted {
        PathSeg::Quad(fitted) => {
            assert!((fitted.p1 - quad.p1).hypot() <= 1e-9);
        }
        other => panic!("expected a quadratic, got {other:?}"),
    }
}

#[test]
fn parallel_tangents_and_short_input_fall_back_to_a_line() {
    let samples = [
        Point::new(0.0, 0.0),
        Point::new(50.0, 0.0),
        Point::new(100.0, 0.0),
    ];
    let parallel = fit_span(
        &samples,
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 0.0),
        CurveKind::Quadratic { max_err: 0.1 },
    );
    assert!(matches!(parallel, PathSeg::Line(_)));

    let short = fit_span(
        &samples[..2],
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
        CurveKind::Cubic,
    );
    assert!(matches!(short, PathSeg::Line(_)));

    let empty = fit_span(&[], Vec2::ZERO, Vec2::ZERO, CurveKind::Cubic);
    assert!(matches!(empty, PathSeg::Line(_)));
}

#[test]
fn non_finite_samples_are_ignored() {
    let arc = arc();
    let mut samples = sample(PathSeg::Cubic(arc));
    samples.insert(3, Point::new(f64::NAN, 0.0));
    let fitted = fit_span(
        &samples,
        Vec2::new(0.0, 1.0),
        Vec2::new(-1.0, 0.0),
        CurveKind::Cubic,
    );
    assert!(matches!(fitted, PathSeg::Cubic(_)));
}
