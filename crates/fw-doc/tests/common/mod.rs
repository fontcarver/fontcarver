use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use fw_model::{Component, Contour, Font, Glyph, GlyphName, Layer, Point, PointKind};

pub fn test_font() -> Font {
    let mut font = Font::new_empty("Document Test", 1_000);
    let master = font.default_master().id.clone();

    let mut base = Glyph::new("base");
    let mut base_layer = Layer::new_master(master.clone());
    base_layer.width = 500.0;
    base_layer.contours.push(Contour {
        closed: true,
        points: vec![
            point(0.0, 0.0),
            point(100.0, 0.0),
            point(100.0, 100.0),
            point(0.0, 100.0),
        ],
        ..Contour::default()
    });
    base.layers.push(base_layer);
    font.insert_glyph(base).expect("base glyph is valid");

    let mut composite = Glyph::new("composite");
    let mut composite_layer = Layer::new_master(master);
    composite_layer.width = 600.0;
    composite_layer.components.push(Component {
        base: GlyphName::from("base"),
        ..Component::default()
    });
    composite.layers.push(composite_layer);
    font.insert_glyph(composite)
        .expect("composite glyph is valid");
    font
}

#[allow(dead_code)]
pub fn synthetic_font(glyph_count: usize) -> Font {
    let mut font = Font::new_empty("Synthetic", 1_000);
    for index in 0..glyph_count {
        font.insert_glyph(Glyph::new(format!("glyph-{index}")))
            .expect("synthetic glyph is valid");
    }
    font
}

pub fn point(x: f64, y: f64) -> Point {
    Point {
        x,
        y,
        kind: PointKind::Line,
        ..Point::default()
    }
}

#[allow(dead_code)]
pub fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "fontworkshop-fw-doc-{label}-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temporary test directory can be created");
    dir
}
