use fw_geom::{fit_span, CurveKind, SpanFit};
use kurbo::{
    CubicBez, Line, ParamCurve, ParamCurveNearest, PathSeg, Point, QuadBez, QuadSpline, Vec2,
};

const SAMPLES: u32 = 64;

fn sample(segment: PathSeg) -> Vec<Point> {
    (0..=SAMPLES)
        .map(|step| segment.eval(f64::from(step) / f64::from(SAMPLES)))
        .collect()
}

/// Maximum distance from the samples to the segments the fit stands for.
fn deviation(fit: &SpanFit, samples: &[Point]) -> f64 {
    let (Some(&start), Some(&end)) = (samples.first(), samples.last()) else {
        return f64::INFINITY;
    };
    let segments: Vec<PathSeg> = match fit {
        SpanFit::Line => vec![PathSeg::Line(Line::new(start, end))],
        SpanFit::Cubic(first, second) => {
            vec![PathSeg::Cubic(CubicBez::new(start, *first, *second, end))]
        }
        SpanFit::Quadratic(controls) => {
            let mut points = vec![start];
            points.extend(controls.iter().copied());
            points.push(end);
            QuadSpline::new(points)
                .to_quads()
                .map(PathSeg::Quad)
                .collect()
        }
    };
    samples
        .iter()
        .map(|&point| {
            segments
                .iter()
                .map(|segment| segment.nearest(point, 1e-9).distance_sq)
                .fold(f64::INFINITY, f64::min)
                .sqrt()
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
    let samples = sample(PathSeg::Cubic(arc()));
    let fitted = fit_span(
        &samples,
        Vec2::new(0.0, 1.0),
        Vec2::new(-1.0, 0.0),
        CurveKind::Cubic,
    );
    assert!(matches!(fitted, SpanFit::Cubic(_, _)));
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
        SpanFit::Quadratic(controls) => {
            assert_eq!(controls.len(), 1);
            assert!((controls[0] - quad.p1).hypot() <= 1e-9);
        }
        other => panic!("expected a quadratic, got {other:?}"),
    }
}

#[test]
fn quadratic_fit_subdivides_when_one_control_cannot_hold_the_span() {
    let half = CubicBez::new(
        Point::new(100.0, 0.0),
        Point::new(100.0, 55.2),
        Point::new(-100.0, 55.2),
        Point::new(-100.0, 0.0),
    );
    let samples = sample(PathSeg::Cubic(half));
    let fitted = fit_span(
        &samples,
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
        CurveKind::Quadratic { max_err: 0.5 },
    );
    match &fitted {
        SpanFit::Quadratic(controls) => assert!(controls.len() > 1),
        other => panic!("expected a quadratic chain, got {other:?}"),
    }
    assert!(deviation(&fitted, &samples) <= 0.5);
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
    assert_eq!(parallel, SpanFit::Line);

    let short = fit_span(
        &samples[..2],
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
        CurveKind::Cubic,
    );
    assert_eq!(short, SpanFit::Line);

    assert_eq!(
        fit_span(&[], Vec2::ZERO, Vec2::ZERO, CurveKind::Cubic),
        SpanFit::Line
    );
}

#[test]
fn non_finite_samples_are_ignored() {
    let mut samples = sample(PathSeg::Cubic(arc()));
    samples.insert(3, Point::new(f64::NAN, 0.0));
    let fitted = fit_span(
        &samples,
        Vec2::new(0.0, 1.0),
        Vec2::new(-1.0, 0.0),
        CurveKind::Cubic,
    );
    assert!(matches!(fitted, SpanFit::Cubic(_, _)));
}
