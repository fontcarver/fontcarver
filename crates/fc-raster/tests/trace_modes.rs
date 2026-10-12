use fc_model::{Bitmap, Contour, PointKind};
use fc_raster::{trace_bitmap, trace_bitmap_with_options, BitmapTraceMode, BitmapTraceOptions};
use kurbo::{ParamCurve, Point, Shape};

fn bitmap(rows: &[&str]) -> Bitmap {
    let mut bitmap = Bitmap::new(rows[0].len() as u16, rows.len() as u16, 1).unwrap();
    for (y, row) in rows.iter().enumerate() {
        for (x, pixel) in row.bytes().enumerate() {
            bitmap
                .set_pixel(x as u16, y as u16, u32::from(pixel == b'#'))
                .unwrap();
        }
    }
    bitmap
}

fn trace(bitmap: &Bitmap, mode: BitmapTraceMode, join_diagonals: bool) -> Vec<Contour> {
    trace_bitmap_with_options(
        bitmap,
        16,
        16,
        128,
        BitmapTraceOptions {
            mode,
            join_diagonals,
        },
    )
    .unwrap()
}

fn modes() -> [BitmapTraceMode; 4] {
    [
        BitmapTraceMode::PixelEdges,
        BitmapTraceMode::ChamferedCorners,
        BitmapTraceMode::RoundedCorners,
        BitmapTraceMode::DiagonalSmoothing,
    ]
}

#[test]
fn default_mode_is_exact_and_preserves_origin_scale_and_collinear_removal() {
    let mut input = bitmap(&["###", "###"]);
    input.origin_x = -2;
    input.origin_y = -3;
    let exact = trace_bitmap(&input, 16, 1024, 128).unwrap();
    let options = trace_bitmap_with_options(&input, 16, 1024, 128, Default::default()).unwrap();
    assert_eq!(exact, options);
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].points.len(), 4);
    assert_eq!(exact[0].to_bezpath().bounding_box().x0, -128.0);
    assert_eq!(exact[0].to_bezpath().bounding_box().y0, -192.0);
    assert_eq!(exact[0].to_bezpath().bounding_box().x1, 64.0);
    assert_eq!(exact[0].to_bezpath().bounding_box().y1, -64.0);
    assert_eq!(exact[0].to_bezpath().area(), -24576.0);
}

#[test]
fn chamfer_cuts_quarter_pixel_legs_and_round_uses_tangent_cubic_arcs() {
    let input = bitmap(&["#"]);
    let chamfered = trace(&input, BitmapTraceMode::ChamferedCorners, false);
    assert_eq!(chamfered[0].points.len(), 8);
    assert_eq!(chamfered[0].to_bezpath().area(), -0.875);
    assert!(chamfered[0]
        .points
        .iter()
        .all(|point| point.kind == PointKind::Line));
    let rounded = trace(&input, BitmapTraceMode::RoundedCorners, false);
    assert_eq!(rounded[0].points.len(), 16);
    assert_eq!(
        rounded[0]
            .points
            .iter()
            .filter(|point| point.kind == PointKind::Curve)
            .count(),
        4
    );
    let expected_area = -(0.75 + std::f64::consts::PI / 16.0);
    assert!((rounded[0].to_bezpath().area() - expected_area).abs() < 0.0001);
    let points = &rounded[0].points;
    for corner in points.chunks_exact(4) {
        assert_eq!(corner[0].kind, PointKind::Line);
        assert_eq!(corner[1].kind, PointKind::OffCurve);
        assert_eq!(corner[2].kind, PointKind::OffCurve);
        assert_eq!(corner[3].kind, PointKind::Curve);
        let entry = Point::new(corner[0].x, corner[0].y);
        let control = Point::new(corner[1].x, corner[1].y);
        assert!((entry.distance(control) - 0.13807118745769836).abs() < 1e-12);
    }
}

#[test]
fn diagonal_smoothing_preserves_rectangles_and_cuts_staircase_turns() {
    let rectangle = bitmap(&["###", "###"]);
    assert_eq!(
        trace(&rectangle, BitmapTraceMode::PixelEdges, false),
        trace(&rectangle, BitmapTraceMode::DiagonalSmoothing, false)
    );
    let staircase = bitmap(&["#..", "##.", ".##"]);
    let smoothed = trace(&staircase, BitmapTraceMode::DiagonalSmoothing, false);
    let path = smoothed[0].to_bezpath();
    assert!(path.segments().any(|segment| {
        let delta = segment.end() - segment.start();
        delta.x != 0.0 && delta.y != 0.0
    }));
    assert!(smoothed[0]
        .points
        .iter()
        .any(|point| point.x.fract() == 0.5));
}

#[test]
fn diagonal_join_builds_positive_width_bridge_and_keeps_separate_default() {
    let input = bitmap(&["#.", ".#"]);
    for mode in modes() {
        let separate = trace(&input, mode, false);
        assert_eq!(separate.len(), 2, "{mode:?}");
        let joined = trace(&input, mode, true);
        assert_eq!(joined.len(), 1, "{mode:?}");
        assert!(joined[0].to_bezpath().winding(Point::new(1.0, 1.0)) != 0);
        assert!(joined[0].to_bezpath().area() < -1.0);
        assert!(joined[0].closed);
    }
}

#[test]
fn modes_keep_holes_and_opposite_winding() {
    let input = bitmap(&["###", "#.#", "###"]);
    for mode in modes() {
        for join in [false, true] {
            let result = trace(&input, mode, join);
            assert_eq!(result.len(), 2);
            assert_eq!(
                result
                    .iter()
                    .filter(|c| c.to_bezpath().area() < 0.0)
                    .count(),
                1
            );
            assert_eq!(
                result
                    .iter()
                    .filter(|c| c.to_bezpath().area() > 0.0)
                    .count(),
                1
            );
            assert_eq!(
                result
                    .iter()
                    .map(|c| c.to_bezpath().winding(Point::new(1.5, 1.5)))
                    .sum::<i32>(),
                0
            );
        }
    }
}

fn components(mask: u16, ink: bool, diagonals: bool, enclosed_only: bool) -> usize {
    let mut seen = [false; 9];
    let mut count = 0;
    for start in 0..9 {
        if seen[start] || (mask & (1 << start) != 0) != ink {
            continue;
        }
        let mut stack = vec![start];
        let mut touches_edge = false;
        seen[start] = true;
        while let Some(index) = stack.pop() {
            let x = index as i32 % 3;
            let y = index as i32 / 3;
            touches_edge |= x == 0 || x == 2 || y == 0 || y == 2;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 || !diagonals && dx != 0 && dy != 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if !(0..3).contains(&nx) || !(0..3).contains(&ny) {
                        continue;
                    }
                    let neighbor = (ny * 3 + nx) as usize;
                    if !seen[neighbor] && (mask & (1 << neighbor) != 0) == ink {
                        seen[neighbor] = true;
                        stack.push(neighbor);
                    }
                }
            }
        }
        count += usize::from(!enclosed_only || !touches_edge);
    }
    count
}

#[test]
fn every_three_by_three_mask_preserves_connectivity_holes_and_pixel_centers() {
    for mask in 0..512 {
        let mut input = Bitmap::new(3, 3, 1).unwrap();
        for index in 0..9 {
            input
                .set_pixel(index % 3, index / 3, u32::from(mask & (1 << index) != 0))
                .unwrap();
        }
        for mode in modes() {
            for join in [false, true] {
                let contours = trace(&input, mode, join);
                let paths: Vec<_> = contours
                    .iter()
                    .map(|contour| {
                        assert!(contour.closed);
                        assert!(contour.points.iter().all(|p| p.x.is_finite()
                            && p.y.is_finite()
                            && (0.0..=3.0).contains(&p.x)
                            && (0.0..=3.0).contains(&p.y)));
                        let path = contour.try_to_bezpath().unwrap();
                        assert_ne!(path.area(), 0.0, "mask {mask} mode {mode:?} join {join}");
                        path
                    })
                    .collect();
                assert_eq!(
                    paths.iter().filter(|path| path.area() < 0.0).count(),
                    components(mask, true, join, false),
                    "foreground mask {mask} mode {mode:?} join {join}"
                );
                assert_eq!(
                    paths.iter().filter(|path| path.area() > 0.0).count(),
                    components(mask, false, !join, true),
                    "holes mask {mask} mode {mode:?} join {join}"
                );
                for index in 0..9 {
                    let point = Point::new(f64::from(index % 3) + 0.5, 2.5 - f64::from(index / 3));
                    let winding: i32 = paths.iter().map(|path| path.winding(point)).sum();
                    assert_eq!(
                        winding != 0,
                        mask & (1 << index) != 0,
                        "pixel {index} mask {mask} mode {mode:?} join {join}"
                    );
                }
            }
        }
    }
}

#[test]
fn modes_validate_threshold_scale_and_work_limits() {
    let mut input = Bitmap::new(2, 1, 8).unwrap();
    input.set_pixel(0, 0, 127).unwrap();
    input.set_pixel(1, 0, 128).unwrap();
    let oversized = Bitmap::new(1001, 1000, 1).unwrap();
    let mut fragmented = Bitmap::new(320, 320, 1).unwrap();
    for y in 0..320 {
        for x in 0..320 {
            fragmented
                .set_pixel(x, y, u32::from((x + y) % 2 == 0))
                .unwrap();
        }
    }
    for mode in modes() {
        let options = BitmapTraceOptions {
            mode,
            join_diagonals: true,
        };
        assert_eq!(trace(&input, mode, false).len(), 1);
        assert!(trace(&Bitmap::new(0, 0, 1).unwrap(), mode, false).is_empty());
        for (ppem, upem, threshold) in [(0, 1000, 1), (16, 0, 1), (16, 1000, 0)] {
            assert!(trace_bitmap_with_options(&input, ppem, upem, threshold, options).is_err());
        }
        assert!(trace_bitmap_with_options(&oversized, 16, 1000, 128, options).is_err());
        assert!(trace_bitmap_with_options(&fragmented, 16, 1000, 128, options).is_err());
    }
}
