#![allow(dead_code)]

use fc_model::{Contour, Point, PointKind, TtIndex};
use kurbo::{ParamCurve, Shape};

pub fn point(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        corner_radius: 0.,
        x,
        y,
        kind,
        smooth: false,
        name: None,
        tt_index: TtIndex::Unnumbered,
    }
}

pub fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    Contour {
        closed: true,
        points: vec![
            point(x0, y0, PointKind::Line),
            point(x1, y0, PointKind::Line),
            point(x1, y1, PointKind::Line),
            point(x0, y1, PointKind::Line),
        ],
        name: None,
        id: None,
    }
}

pub fn open_line(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    Contour {
        closed: false,
        points: vec![
            point(x0, y0, PointKind::Move),
            point(x1, y1, PointKind::Line),
        ],
        name: None,
        id: None,
    }
}

pub fn circle(radius: f64) -> Contour {
    let k = 0.552_284_749_830_793_6 * radius;
    Contour {
        closed: true,
        points: vec![
            point(radius, 0.0, PointKind::Curve),
            point(radius, k, PointKind::OffCurve),
            point(k, radius, PointKind::OffCurve),
            point(0.0, radius, PointKind::Curve),
            point(-k, radius, PointKind::OffCurve),
            point(-radius, k, PointKind::OffCurve),
            point(-radius, 0.0, PointKind::Curve),
            point(-radius, -k, PointKind::OffCurve),
            point(-k, -radius, PointKind::OffCurve),
            point(0.0, -radius, PointKind::Curve),
            point(k, -radius, PointKind::OffCurve),
            point(radius, -k, PointKind::OffCurve),
        ],
        name: None,
        id: None,
    }
}

pub fn filled_area(contours: &[Contour]) -> f64 {
    contours
        .iter()
        .map(|contour| contour.to_bezpath().area())
        .sum::<f64>()
        .abs()
}

pub fn max_sampled_deviation(source: &Contour, candidate: &Contour) -> f64 {
    let candidate_segments: Vec<_> = candidate.to_bezpath().segments().collect();
    source
        .to_bezpath()
        .segments()
        .flat_map(|segment| (0..=100).map(move |index| segment.eval(index as f64 / 100.0)))
        .map(|point| distance_to_path(point, &candidate_segments))
        .fold(0.0, f64::max)
}

fn distance_to_path(point: kurbo::Point, segments: &[kurbo::PathSeg]) -> f64 {
    use kurbo::ParamCurveNearest;

    segments
        .iter()
        .map(|segment| segment.nearest(point, 1e-6).distance_sq.sqrt())
        .fold(f64::INFINITY, f64::min)
}
