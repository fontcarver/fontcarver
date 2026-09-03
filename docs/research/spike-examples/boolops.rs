//! Boolean path-op evaluation: two overlapping circles.
//! Compares flo_curves (curve-preserving) with i_overlay (polygon/flattening).

use flo_curves::bezier::path::{
    path_add, path_intersect, path_sub, BezierPathBuilder, SimpleBezierPath,
};
use flo_curves::geo::Coord2;
use kurbo::{BezPath, Circle, PathEl, Point, Shape};

/// kurbo Circle -> BezPath made of 4 cubics (kurbo uses cubic arc approximation).
fn circle_path(cx: f64, cy: f64, r: f64) -> BezPath {
    Circle::new((cx, cy), r).to_path(0.01)
}

/// kurbo BezPath (must be all-cubic + closed) -> flo_curves SimpleBezierPath
fn kurbo_to_flo(bp: &BezPath) -> SimpleBezierPath {
    let mut builder: Option<BezierPathBuilder<SimpleBezierPath>> = None;
    for el in bp.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                builder = Some(BezierPathBuilder::<SimpleBezierPath>::start(Coord2(p.x, p.y)));
            }
            PathEl::LineTo(p) => {
                builder = Some(builder.take().unwrap().line_to(Coord2(p.x, p.y)));
            }
            PathEl::CurveTo(c1, c2, p) => {
                builder = Some(builder.take().unwrap().curve_to(
                    (Coord2(c1.x, c1.y), Coord2(c2.x, c2.y)),
                    Coord2(p.x, p.y),
                ));
            }
            PathEl::QuadTo(..) => panic!("quads must be elevated first"),
            PathEl::ClosePath => {}
        }
    }
    builder.unwrap().build()
}

/// flo_curves SimpleBezierPath -> kurbo BezPath (always cubic triples)
fn flo_to_kurbo(p: &SimpleBezierPath) -> BezPath {
    let (start, points) = p;
    let mut bp = BezPath::new();
    bp.move_to(Point::new(start.0, start.1));
    for (c1, c2, e) in points.iter() {
        bp.curve_to(
            Point::new(c1.0, c1.1),
            Point::new(c2.0, c2.1),
            Point::new(e.0, e.1),
        );
    }
    bp.close_path();
    bp
}

/// How far are the control points from the straight chord? ~0 => segment is really a line.
fn curviness(bp: &BezPath) -> (usize, usize, f64) {
    let mut n_cubic = 0usize;
    let mut n_effectively_line = 0usize;
    let mut max_dev: f64 = 0.0;
    let mut cur = Point::ZERO;
    let mut start = Point::ZERO;
    for el in bp.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                cur = p;
                start = p;
            }
            PathEl::LineTo(p) => cur = p,
            PathEl::CurveTo(c1, c2, p) => {
                n_cubic += 1;
                let chord = kurbo::Line::new(cur, p);
                let d1 = dist_point_line(c1, chord);
                let d2 = dist_point_line(c2, chord);
                let dev = d1.max(d2);
                if dev < 1e-6 {
                    n_effectively_line += 1;
                }
                max_dev = max_dev.max(dev);
                cur = p;
            }
            PathEl::QuadTo(_, p) => cur = p,
            PathEl::ClosePath => cur = start,
        }
    }
    (n_cubic, n_effectively_line, max_dev)
}

fn dist_point_line(p: Point, l: kurbo::Line) -> f64 {
    let v = l.p1 - l.p0;
    let len = v.hypot();
    if len < 1e-12 {
        return (p - l.p0).hypot();
    }
    ((p - l.p0).cross(v) / len).abs()
}

fn total_area(paths: &[SimpleBezierPath]) -> f64 {
    paths.iter().map(|p| flo_to_kurbo(p).area().abs()).sum()
}

fn describe(label: &str, paths: &[SimpleBezierPath], expected: f64) {
    let area: f64 = total_area(paths);
    let mut cubics = 0;
    let mut lines = 0;
    let mut dev: f64 = 0.0;
    for p in paths {
        let (c, l, d) = curviness(&flo_to_kurbo(p));
        cubics += c;
        lines += l;
        dev = dev.max(d);
    }
    println!(
        "{label:<22} subpaths={:<2} cubic_segs={:<4} degenerate(line-like)={:<3} max_ctrl_dev={:<8.3} area={:<12.3} expected={:<12.3} err={:.4}%",
        paths.len(),
        cubics,
        lines,
        dev,
        area,
        expected,
        ((area - expected) / expected * 100.0).abs()
    );
}

fn svg_of(paths: &[SimpleBezierPath]) -> String {
    paths
        .iter()
        .map(|p| flo_to_kurbo(p).to_svg())
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let r = 100.0_f64;
    let d = 120.0_f64;
    let a = circle_path(0.0, 0.0, r);
    let b = circle_path(d, 0.0, r);

    println!("input circle: {} elements, svg = {}", a.elements().len(), a.to_svg());
    let (c, l, dev) = curviness(&a);
    println!("input circle cubic segs = {c}, line-like = {l}, max ctrl dev = {dev:.3}\n");

    let fa = vec![kurbo_to_flo(&a)];
    let fb = vec![kurbo_to_flo(&b)];

    // Analytic expectations
    let lens = 2.0 * r * r * (d / (2.0 * r)).acos() - (d / 2.0) * (4.0 * r * r - d * d).sqrt();
    let circle_area = std::f64::consts::PI * r * r;
    let union = 2.0 * circle_area - lens;
    let diff = circle_area - lens;

    let acc = 0.01;
    let u: Vec<SimpleBezierPath> = path_add(&fa, &fb, acc);
    let i: Vec<SimpleBezierPath> = path_intersect(&fa, &fb, acc);
    let s: Vec<SimpleBezierPath> = path_sub(&fa, &fb, acc);

    println!("--- flo_curves 0.8.1 (accuracy = {acc}) ---");
    describe("path_add (union)", &u, union);
    describe("path_intersect", &i, lens);
    describe("path_sub (A - B)", &s, diff);

    println!("\nunion  svg: {}", svg_of(&u));
    println!("inter  svg: {}", svg_of(&i));
    println!("sub    svg: {}", svg_of(&s));

    // remove-overlap style: self-union of a single path containing both circles
    let both = vec![kurbo_to_flo(&a), kurbo_to_flo(&b)];
    let ro: Vec<SimpleBezierPath> =
        flo_curves::bezier::path::path_remove_overlapped_points(&both, acc);
    println!();
    describe("remove_overlapped_pts", &ro, union);
    let ri: Vec<SimpleBezierPath> =
        flo_curves::bezier::path::path_remove_interior_points(&both, acc);
    describe("remove_interior_pts", &ri, union);

    // --- render for visual check ---
    render(&u, "/tmp/boolops_union.png");
    render(&i, "/tmp/boolops_intersect.png");
    render(&s, "/tmp/boolops_sub.png");
    println!("\nwrote /tmp/boolops_{{union,intersect,sub}}.png");

    // --- i_overlay comparison (polygon boolean, flattening required) ---
    i_overlay_compare(&a, &b, union, lens, diff);

    // --- timing on a realistic remove-overlap workload ---
    bench_remove_overlap();
}

fn bench_remove_overlap() {
    use std::time::Instant;
    // 12 overlapping circles arranged in a ring (a stress-y "remove overlap" case)
    let mut paths: Vec<SimpleBezierPath> = Vec::new();
    for k in 0..12 {
        let t = k as f64 / 12.0 * std::f64::consts::TAU;
        paths.push(kurbo_to_flo(&circle_path(120.0 * t.cos(), 120.0 * t.sin(), 90.0)));
    }
    let t0 = Instant::now();
    let ro: Vec<SimpleBezierPath> = flo_curves::bezier::path::path_remove_interior_points(&paths, 0.01);
    let dt = t0.elapsed();
    let segs: usize = ro.iter().map(|p| p.1.len()).sum();
    println!(
        "\nremove-overlap of 12 overlapping circles: {} subpaths / {segs} cubic segs in {:?}",
        ro.len(), dt
    );

    // same via i_overlay (flattened)
    use i_overlay::core::fill_rule::FillRule;
    use i_overlay::core::overlay_rule::OverlayRule;
    use i_overlay::float::single::SingleFloatOverlay;
    let shapes: Vec<Vec<[f64; 2]>> = (0..12).map(|k| {
        let t = k as f64 / 12.0 * std::f64::consts::TAU;
        let bp = circle_path(120.0 * t.cos(), 120.0 * t.sin(), 90.0);
        let mut v = Vec::new();
        kurbo::flatten(bp.elements().iter().copied(), 0.05, |el| match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => v.push([p.x, p.y]),
            _ => {}
        });
        v
    }).collect();
    let t0 = Instant::now();
    let out = shapes.overlay(&Vec::<Vec<[f64;2]>>::new(), OverlayRule::Subject, FillRule::NonZero);
    let dt = t0.elapsed();
    let pts: usize = out.iter().flat_map(|s| s.iter()).map(|c| c.len()).sum();
    println!("i_overlay same case: {} shapes / {pts} points in {:?}", out.len(), dt);
}

fn render(paths: &[SimpleBezierPath], out: &str) {
    use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};
    let mut pm = Pixmap::new(440, 280).unwrap();
    pm.fill(tiny_skia::Color::WHITE);
    let mut pb = PathBuilder::new();
    for p in paths {
        let bp = flo_to_kurbo(p);
        for el in bp.elements() {
            match *el {
                PathEl::MoveTo(q) => pb.move_to(q.x as f32, q.y as f32),
                PathEl::LineTo(q) => pb.line_to(q.x as f32, q.y as f32),
                PathEl::CurveTo(c1, c2, q) => pb.cubic_to(
                    c1.x as f32,
                    c1.y as f32,
                    c2.x as f32,
                    c2.y as f32,
                    q.x as f32,
                    q.y as f32,
                ),
                PathEl::QuadTo(c, q) => pb.quad_to(c.x as f32, c.y as f32, q.x as f32, q.y as f32),
                PathEl::ClosePath => pb.close(),
            }
        }
    }
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color_rgba8(30, 90, 200, 255);
        paint.anti_alias = true;
        pm.fill_path(
            &path,
            &paint,
            FillRule::EvenOdd,
            Transform::from_translate(160.0, 140.0),
            None,
        );
    }
    pm.save_png(out).unwrap();
}

fn i_overlay_compare(a: &BezPath, b: &BezPath, union: f64, lens: f64, diff: f64) {
    use i_overlay::core::fill_rule::FillRule;
    use i_overlay::core::overlay_rule::OverlayRule;
    use i_overlay::float::single::SingleFloatOverlay;

    let tol = 0.05; // flattening tolerance in font units
    let flat = |bp: &BezPath| -> Vec<[f64; 2]> {
        let mut v: Vec<[f64; 2]> = Vec::new();
        kurbo::flatten(bp.elements().iter().copied(), tol, |el| match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => v.push([p.x, p.y]),
            _ => {}
        });
        v
    };
    let sa = flat(a);
    let sb = flat(b);
    println!(
        "\n--- i_overlay 8.1.0 (flattened at tol={tol}) --- subj pts={} clip pts={}",
        sa.len(),
        sb.len()
    );
    for (name, rule, exp) in [
        ("Union", OverlayRule::Union, union),
        ("Intersect", OverlayRule::Intersect, lens),
        ("Difference", OverlayRule::Difference, diff),
    ] {
        let shapes = sa.overlay(&sb, rule, FillRule::NonZero);
        let mut pts = 0usize;
        let mut area = 0.0;
        for shape in &shapes {
            for contour in shape {
                pts += contour.len();
                let mut acc = 0.0;
                for k in 0..contour.len() {
                    let p = contour[k];
                    let q = contour[(k + 1) % contour.len()];
                    acc += p[0] * q[1] - q[0] * p[1];
                }
                area += acc / 2.0;
            }
        }
        println!(
            "{name:<12} shapes={:<2} total_pts={pts:<5} area={:<12.3} expected={exp:<12.3} err={:.4}%",
            shapes.len(),
            area.abs(),
            ((area.abs() - exp) / exp * 100.0).abs()
        );
    }
}

#[allow(dead_code)]
fn bench() {}
