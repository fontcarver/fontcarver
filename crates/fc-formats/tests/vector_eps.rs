//! Bounded public EPS import performance on a synthetic many-contour outline.

use std::time::{Duration, Instant};

#[test]
fn imports_ten_thousand_contours_within_budget() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("many-contours.eps");
    let mut source = String::from("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 10 10\nnewpath\n");
    for _ in 0..10_000 {
        source.push_str("0 0 moveto 10 0 lineto 0 10 lineto closepath\n");
    }
    source.push_str("fill\n%%EOF\n");
    std::fs::write(&path, source).unwrap();
    let started = Instant::now();
    let font = fc_formats::vector::read_eps(&path).unwrap();
    assert!(started.elapsed() < Duration::from_secs(30));
    assert_eq!(
        font.glyph("glyph").unwrap().layers[0].contours.len(),
        10_000
    );
}
