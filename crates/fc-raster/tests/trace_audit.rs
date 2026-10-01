//! Every tracing mode must emit outlines the rest of the editor accepts:
//! closed contours, outer shapes clockwise with holes the other way, and no
//! self-crossing. `fc-api`'s `bitmap_parity` suite runs the model-level
//! geometry scan on the same outputs, where `fc-doc` is available.

use fc_model::{Bitmap, Contour};
use fc_raster::{trace_bitmap_with_options, BitmapTraceMode, BitmapTraceOptions};
use kurbo::{BezPath, ParamCurve, PathEl, Point};

fn mask(rows: &[&str]) -> Bitmap {
    let height = rows.len() as u16;
    let width = rows.first().map_or(0, |row| row.len()) as u16;
    let mut bitmap = Bitmap::new(width, height, 1).expect("bitmap");
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.bytes().enumerate() {
            if cell == b'#' {
                bitmap.set_pixel(x as u16, y as u16, 1).expect("set pixel");
            }
        }
    }
    bitmap
}

/// Flatten one contour into the closed polyline the checks below use.
fn polyline(contour: &Contour) -> Vec<Point> {
    let path: BezPath = contour.to_bezpath();
    let mut points = Vec::new();
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => points.push(point),
            PathEl::LineTo(point) => points.push(point),
            PathEl::QuadTo(..) | PathEl::CurveTo(..) => {
                let segments: Vec<_> = path.segments().collect();
                points.clear();
                for segment in &segments {
                    for step in 0..16 {
                        points.push(segment.eval(f64::from(step) / 16.0));
                    }
                }
                break;
            }
            PathEl::ClosePath => {}
        }
    }
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

fn area(points: &[Point]) -> f64 {
    let count = points.len();
    (0..count)
        .map(|index| {
            let a = points[index];
            let b = points[(index + 1) % count];
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

/// Whether two closed segments properly cross (shared endpoints do not count).
fn crosses(a0: Point, a1: Point, b0: Point, b1: Point) -> bool {
    let side = |p: Point, q: Point, r: Point| {
        let value = (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
        if value.abs() < 1e-9 {
            0
        } else if value > 0.0 {
            1
        } else {
            -1
        }
    };
    let (d1, d2, d3, d4) = (
        side(a0, a1, b0),
        side(a0, a1, b1),
        side(b0, b1, a0),
        side(b0, b1, a1),
    );
    d1 * d2 < 0 && d3 * d4 < 0
}

fn assert_no_self_crossing(points: &[Point], label: &str) {
    let count = points.len();
    for i in 0..count {
        for j in (i + 1)..count {
            // Segments sharing an endpoint touch by construction.
            if j == i + 1 || (i == 0 && j + 1 == count) {
                continue;
            }
            assert!(
                !crosses(
                    points[i],
                    points[(i + 1) % count],
                    points[j],
                    points[(j + 1) % count],
                ),
                "{label}: segments {i} and {j} cross"
            );
        }
    }
}

/// A point just inside the contour, found from the first edge's inward normal.
fn interior_point(points: &[Point], signed: f64) -> Point {
    let a = points[0];
    let b = points[1 % points.len()];
    let delta = b - a;
    let length = delta.hypot();
    let normal = if length > 0.0 {
        kurbo::Vec2::new(-delta.y / length, delta.x / length)
    } else {
        kurbo::Vec2::new(0.0, 1.0)
    };
    // The interior lies to the left of a counterclockwise edge (positive area).
    let direction = if signed > 0.0 { 1.0 } else { -1.0 };
    Point::new(
        (a.x + b.x) / 2.0 + normal.x * direction * 0.01,
        (a.y + b.y) / 2.0 + normal.y * direction * 0.01,
    )
}

/// Even-odd ray cast, used only to establish nesting depth.
fn contains(points: &[Point], probe: Point) -> bool {
    let count = points.len();
    let mut inside = false;
    for index in 0..count {
        let a = points[index];
        let b = points[(index + 1) % count];
        if (a.y > probe.y) != (b.y > probe.y) {
            let x = a.x + (probe.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if probe.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

fn audit(contours: &[Contour], label: &str) {
    assert!(!contours.is_empty(), "{label}: no contours");
    let mut outlines = Vec::new();
    for (index, contour) in contours.iter().enumerate() {
        assert!(contour.closed, "{label}: contour {index} is open");
        assert!(
            contour.points.len() >= 3,
            "{label}: contour {index} is degenerate"
        );
        assert!(
            contour
                .points
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite()),
            "{label}: contour {index} has a non-finite point"
        );
        let points = polyline(contour);
        let signed = area(&points);
        assert!(
            signed.abs() > 1e-6,
            "{label}: contour {index} encloses no area"
        );
        assert_no_self_crossing(&points, &format!("{label} contour {index}"));
        outlines.push((points, signed));
    }
    // y-up clockwise is a negative shoelace area. An outer contour (even
    // nesting depth) must be clockwise and every hole must run the other way,
    // which is what the compiler's non-zero fill expects.
    for (index, (points, signed)) in outlines.iter().enumerate() {
        let probe = interior_point(points, *signed);
        let depth = outlines
            .iter()
            .enumerate()
            .filter(|(other, (candidate, _))| *other != index && contains(candidate, probe))
            .count();
        let expected_clockwise = depth % 2 == 0;
        assert_eq!(
            *signed < 0.0,
            expected_clockwise,
            "{label}: contour {index} at nesting depth {depth} has area {signed}"
        );
    }
}

const FIXTURES: [(&str, &[&str]); 4] = [
    ("staircase", &["#...", "##..", ".##.", "..##"]),
    ("diagonal", &["#..", ".#.", "..#"]),
    ("hole", &["####", "#..#", "#..#", "####"]),
    (
        "nested",
        &["######", "#....#", "#.##.#", "#.##.#", "#....#", "######"],
    ),
];

#[test]
fn every_mode_and_join_setting_passes_the_outline_audit() {
    let modes = [
        BitmapTraceMode::PixelEdges,
        BitmapTraceMode::ChamferedCorners,
        BitmapTraceMode::RoundedCorners,
        BitmapTraceMode::DiagonalSmoothing,
    ];
    for (name, rows) in FIXTURES {
        let bitmap = mask(rows);
        for mode in modes {
            for join in [false, true] {
                let contours = trace_bitmap_with_options(
                    &bitmap,
                    8,
                    1000,
                    128,
                    BitmapTraceOptions {
                        mode,
                        join_diagonals: join,
                    },
                )
                .expect("trace");
                let label = format!("{name} {mode:?} join={join}");
                audit(&contours, &label);
            }
        }
    }
}

#[test]
fn exact_mode_merges_rectangles_into_four_points() {
    // Rectangle merging and collinear removal: a solid block is one rectangle.
    let contours = trace_bitmap_with_options(
        &mask(&["###", "###"]),
        8,
        1000,
        128,
        BitmapTraceOptions::default(),
    )
    .expect("trace");
    assert_eq!(contours.len(), 1);
    assert_eq!(contours[0].points.len(), 4);
    let xs: Vec<f64> = contours[0].points.iter().map(|point| point.x).collect();
    let ys: Vec<f64> = contours[0].points.iter().map(|point| point.y).collect();
    let unit = 1000.0 / 8.0;
    assert!(xs.iter().all(|x| *x == 0.0 || *x == 3.0 * unit));
    assert!(ys.iter().all(|y| *y == 0.0 || *y == 2.0 * unit));
}

#[test]
fn diagonal_contacts_stay_separate_until_joining_is_requested() {
    let bitmap = mask(&["#.", ".#"]);
    let separate = trace_bitmap_with_options(&bitmap, 8, 1000, 128, BitmapTraceOptions::default())
        .expect("trace");
    assert_eq!(separate.len(), 2);
    let joined = trace_bitmap_with_options(
        &bitmap,
        8,
        1000,
        128,
        BitmapTraceOptions {
            mode: BitmapTraceMode::PixelEdges,
            join_diagonals: true,
        },
    )
    .expect("trace");
    assert_eq!(joined.len(), 1);
    audit(&joined, "joined diagonal");
}
