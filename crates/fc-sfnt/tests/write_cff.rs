use fc_model::{Component, Contour, Font, Glyph, GlyphName, Layer};
use fc_sfnt::write::write_cff;
use kurbo::{Affine, BezPath};
use read_fonts::{
    ps::{cff::CffFontRef, cs::CommandSink},
    types::{Fixed, GlyphId},
};

#[derive(Default)]
struct Sink {
    points: Vec<(f64, f64)>,
    closes: usize,
}
impl CommandSink for Sink {
    fn move_to(&mut self, x: Fixed, y: Fixed) {
        self.points.push((x.to_f64(), y.to_f64()));
    }
    fn line_to(&mut self, x: Fixed, y: Fixed) {
        self.points.push((x.to_f64(), y.to_f64()));
    }
    fn curve_to(&mut self, ax: Fixed, ay: Fixed, bx: Fixed, by: Fixed, x: Fixed, y: Fixed) {
        self.points.extend([
            (ax.to_f64(), ay.to_f64()),
            (bx.to_f64(), by.to_f64()),
            (x.to_f64(), y.to_f64()),
        ]);
    }
    fn close(&mut self) {
        self.closes += 1;
    }
}

fn font_with_path(path: &BezPath, width: f64) -> Font {
    let mut font = Font::new_empty("CFF Test", 1000);
    let master = font.default_master().id.clone();
    for name in [".notdef", "A"] {
        let mut glyph = Glyph::new(name);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        if name == "A" {
            layer.contours = Contour::from_bezpath(path);
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert glyph");
    }
    font
}

fn read(font: &Font, gid: u32) -> (Sink, f64) {
    let order: Vec<_> = font.glyph_order().cloned().collect();
    let cff = write_cff(font, &font.default_master().id, &order, false).expect("write CFF");
    let reader = CffFontRef::new_cff(&cff.bytes, 0, Some(i32::from(font.info.units_per_em)))
        .expect("read CFF");
    let subfont = reader.subfont(0, &[]).expect("subfont");
    let mut sink = Sink::default();
    let width = reader
        .evaluate_charstring(&subfont, GlyphId::new(gid), &[], &mut sink)
        .expect("read charstring")
        .expect("width")
        .to_f64();
    (sink, width)
}

#[test]
fn charstring_preserves_fractional_cubics_multiple_contours_and_large_deltas() {
    let mut path = BezPath::new();
    path.move_to((-30000.0, -1000.25));
    path.curve_to((-10000.125, 500.5), (20000.75, 500.5), (30000.0, -1000.25));
    path.close_path();
    path.move_to((20.125, 30.25));
    path.line_to((40.5, 50.75));
    path.close_path();
    let font = font_with_path(&path, 32767.0);
    let (sink, width) = read(&font, 1);
    assert_eq!(width, 32767.0);
    assert_eq!(sink.closes, 2);
    assert!(sink.points.contains(&(-10000.125, 500.5)));
    assert!(sink.points.contains(&(20000.75, 500.5)));
    assert!(sink.points.contains(&(20.125, 30.25)));
}

#[test]
fn quadratic_components_are_raised_and_transformed() {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.0));
    path.quad_to((30.0, 60.0), (60.0, 0.0));
    path.close_path();
    let mut font = font_with_path(&path, 600.0);
    let mut glyph = Glyph::new("component");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 700.0;
    layer.components.push(Component {
        base: "A".into(),
        transform: Affine::translate((100.0, 200.0)),
        ..Component::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).expect("insert component");
    let (sink, width) = read(&font, 2);
    assert_eq!(width, 700.0);
    assert!(sink.points.contains(&(120.0, 240.0)));
    assert!(sink.points.contains(&(140.0, 240.0)));
    assert!(sink.points.contains(&(160.0, 200.0)));
}

#[test]
fn empty_glyph_keeps_large_width() {
    let font = font_with_path(&BezPath::new(), 32767.0);
    let (sink, width) = read(&font, 0);
    assert_eq!(width, 32767.0);
    assert!(sink.points.is_empty());
}

#[test]
fn rejects_open_nonfinite_and_cyclic_outlines() {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.0));
    path.line_to((10.0, 20.0));
    let mut font = font_with_path(&path, 600.0);
    let master = font.default_master().id.clone();
    let order = vec![GlyphName::from(".notdef"), GlyphName::from("A")];
    assert!(write_cff(&font, &master, &order, false).is_err());
    let layer = font
        .glyph_mut("A")
        .expect("A")
        .master_layer_mut(&master)
        .expect("layer");
    layer.contours[0].closed = true;
    layer.contours[0].points[0].x = f64::NAN;
    assert!(write_cff(&font, &master, &order, false).is_err());
    let layer = font
        .glyph_mut("A")
        .expect("A")
        .master_layer_mut(&master)
        .expect("layer");
    layer.contours.clear();
    layer.components.push(Component {
        base: "A".into(),
        ..Component::default()
    });
    assert!(write_cff(&font, &master, &order, false).is_err());
}

#[test]
fn production_names_are_unique_and_native_charset_names() {
    let mut font = font_with_path(&BezPath::new(), 600.0);
    let master = font.default_master().id.clone();
    let order = vec![GlyphName::from(".notdef"), GlyphName::from("A")];
    font.glyph_mut("A").expect("A").production_name = Some("uni0041".into());
    let cff = write_cff(&font, &master, &order, true).expect("CFF");
    let reader = CffFontRef::new_cff(&cff.bytes, 0, Some(1000)).expect("reader");
    let sid = reader
        .charset()
        .expect("charset")
        .string_id(GlyphId::new(1))
        .expect("SID");
    assert_eq!(reader.string(sid), Some(b"uni0041".as_slice()));
    font.glyph_mut("A").expect("A").production_name = Some(".notdef".into());
    assert!(write_cff(&font, &master, &order, true).is_err());
    font.glyph_mut(".notdef").expect(".notdef").production_name = Some("foo".into());
    assert!(write_cff(&font, &master, &order, true).is_err());
    font.glyph_mut("A").expect("A").production_name = Some("foo".into());
    assert!(write_cff(&font, &master, &order, true).is_ok());
}

#[test]
fn unrepresentable_widths_and_relative_coordinates_are_rejected() {
    let font = font_with_path(&BezPath::new(), 50000.0);
    let order: Vec<_> = font.glyph_order().cloned().collect();
    assert!(write_cff(&font, &font.default_master().id, &order, false).is_err());
    let mut path = BezPath::new();
    path.move_to((-30000.0, 0.0));
    path.line_to((30000.0, 0.0));
    path.close_path();
    let font = font_with_path(&path, 600.0);
    assert!(write_cff(&font, &font.default_master().id, &order, false).is_err());
}

#[test]
fn control_points_cannot_overflow_even_when_curve_bounds_fit() {
    let mut path = BezPath::new();
    path.move_to((20000.0, 0.0));
    path.curve_to((40000.0, 100.0), (20000.0, 100.0), (0.0, 0.0));
    path.close_path();
    let font = font_with_path(&path, 600.0);
    let order: Vec<_> = font.glyph_order().cloned().collect();
    assert!(write_cff(&font, &font.default_master().id, &order, false).is_err());
}

#[test]
fn oversized_charstrings_are_rejected_before_emitting_invalid_cff() {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.0));
    for index in 0..30_000 {
        path.line_to((f64::from(index % 2), 0.0));
    }
    path.close_path();
    let font = font_with_path(&path, 600.0);
    let order: Vec<_> = font.glyph_order().cloned().collect();
    let error = write_cff(&font, &font.default_master().id, &order, false)
        .err()
        .expect("overlong charstring rejected");
    assert!(error.to_string().contains("65,535-byte"));
}

#[test]
fn bounds_describe_quantized_cubic_coordinates() {
    let a = 0.000023;
    let b = (4.0 - a) / 3.0;
    let mut path = BezPath::new();
    path.move_to((0.0, a));
    path.curve_to((100.0, b), (200.0, b), (300.0, a));
    path.close_path();
    let font = font_with_path(&path, 600.0);
    let order: Vec<_> = font.glyph_order().cloned().collect();
    let cff = write_cff(&font, &font.default_master().id, &order, false).expect("CFF");
    let bounds = cff.bounds[1].expect("bounds");
    assert!(bounds.y1 > 1.0);
    assert_eq!(bounds.y1.ceil(), 2.0);
}
