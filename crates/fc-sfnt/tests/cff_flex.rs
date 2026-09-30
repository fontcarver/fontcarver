use fc_model::{Component, Contour, Font, Glyph, Layer};
use fc_sfnt::write::write_cff;
use kurbo::BezPath;
use read_fonts::{
    ps::{cff::CffFontRef, cs::CommandSink},
    types::{Fixed, GlyphId},
};

fn font_with_cubics(flex: bool) -> Font {
    let mut font = Font::new_empty("CFF Flex Test", 1000);
    let mut path = BezPath::new();
    path.move_to((0.0, 0.0));
    path.curve_to((20.0, 1.0), (40.0, 2.0), (60.0, 2.0));
    path.curve_to((80.0, 2.0), (100.0, 1.0), (120.0, 0.0));
    path.close_path();
    let mut glyph = Glyph::new("A");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 500.0;
    layer.contours = Contour::from_bezpath(&path);
    layer.hints.flex = flex;
    glyph.layers.push(layer);
    font.insert_glyph(glyph).expect("insert A");
    font
}

#[test]
fn authored_flex_is_rejected_instead_of_silently_lost() {
    let font = font_with_cubics(true);
    let order = vec![".notdef".into(), "A".into()];
    let error = write_cff(&font, &font.default_master().id, &order, false)
        .err()
        .expect("unsupported authored flex must be rejected");
    let message = error.to_string();
    assert!(message.contains("A:"), "{message}");
    assert!(message.contains("flex"), "{message}");
    assert!(message.contains("unsupported"), "{message}");
}

#[test]
fn flex_in_component_base_omitted_from_output_order_is_rejected() {
    let mut font = font_with_cubics(true);
    let mut component = Glyph::new("component");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 500.0;
    layer.components.push(Component {
        base: "A".into(),
        ..Component::default()
    });
    component.layers.push(layer);
    font.insert_glyph(component).expect("insert component");
    let order = vec![".notdef".into(), "component".into()];
    let error = write_cff(&font, &font.default_master().id, &order, false)
        .err()
        .expect("component-base flex must be rejected during expansion");
    let message = error.to_string();
    assert!(message.contains("A"), "{message}");
    assert!(message.contains("flex"), "{message}");
}

#[derive(Default)]
struct CurveSink {
    curves: usize,
}

impl CommandSink for CurveSink {
    fn move_to(&mut self, _: Fixed, _: Fixed) {}
    fn line_to(&mut self, _: Fixed, _: Fixed) {}
    fn curve_to(&mut self, _: Fixed, _: Fixed, _: Fixed, _: Fixed, _: Fixed, _: Fixed) {
        self.curves += 1;
    }
    fn close(&mut self) {}
}

#[test]
fn ordinary_cubic_pair_remains_exportable() {
    let font = font_with_cubics(false);
    let order = vec![".notdef".into(), "A".into()];
    let cff = write_cff(&font, &font.default_master().id, &order, false).expect("CFF");
    let reader = CffFontRef::new_cff(&cff.bytes, 0, Some(1000)).expect("CFF reader");
    let subfont = reader.subfont(0, &[]).expect("subfont");
    let mut sink = CurveSink::default();
    reader
        .evaluate_charstring(&subfont, GlyphId::new(1), &[], &mut sink)
        .expect("valid charstring");
    assert_eq!(sink.curves, 2);
}
