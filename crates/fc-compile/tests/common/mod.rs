use fc_model::{Contour, Font, Glyph, GlyphName, KernSide, Layer};
use kurbo::BezPath;

pub fn scratch_font() -> Font {
    let mut font = Font::new_empty("Compile Test", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicode, width, rect) in [
        (".notdef", None, 600.0, None),
        ("space", Some(0x20), 250.0, None),
        ("A", Some(0x41), 600.0, Some((50.0, 0.0, 500.0, 700.0))),
        ("V", Some(0x56), 600.0, Some((60.0, 0.0, 480.0, 700.0))),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        if let Some((x, y, w, h)) = rect {
            let mut path = BezPath::new();
            path.move_to((x, y));
            path.line_to((x, y + h));
            path.line_to((x + w, y + h));
            path.line_to((x + w, y));
            path.close_path();
            layer.contours = Contour::from_bezpath(&path);
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert scratch glyph");
    }
    font.masters[0].kerning_h.0.insert(
        (
            KernSide::Glyph(GlyphName::from("A")),
            KernSide::Glyph(GlyphName::from("V")),
        ),
        -50.0,
    );
    font
}
