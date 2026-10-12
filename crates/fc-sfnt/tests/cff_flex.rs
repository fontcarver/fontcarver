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

/// The cupped serif of figure 8e in the Type 1 specification, raised to y = 690.
fn flex_contour() -> Contour {
    use fc_model::{Point, PointKind};
    let point = |x: f64, y: f64, kind| Point {
        x,
        y,
        kind,
        ..Point::default()
    };
    Contour {
        closed: true,
        points: vec![
            point(100.0, 690.0, PointKind::Line),
            point(115.0, 690.0, PointKind::OffCurve),
            point(125.0, 700.0, PointKind::OffCurve),
            point(150.0, 700.0, PointKind::Curve),
            point(175.0, 700.0, PointKind::OffCurve),
            point(185.0, 690.0, PointKind::OffCurve),
            point(200.0, 690.0, PointKind::Curve),
            point(200.0, 0.0, PointKind::Line),
            point(100.0, 0.0, PointKind::Line),
        ],
        name: None,
        id: None,
    }
}

fn flex_source(depth: Option<f64>) -> (Font, Vec<u8>, Vec<fc_model::GlyphName>) {
    let source = std::fs::read(fc_fixtures::path("SourceSerif4Min.otf")).expect("fixture");
    let mut font = fc_sfnt::read_bytes(&source, &fc_sfnt::ReadOptions::default())
        .expect("source")
        .fonts
        .remove(0);
    let order: Vec<fc_model::GlyphName> = font.glyph_order().cloned().collect();
    let master = font.default_master().id.clone();
    let name = order
        .iter()
        .find(|name| name.as_ref() != ".notdef")
        .expect("an outline glyph")
        .clone();
    let layer = font
        .glyph_mut(&name)
        .expect("glyph")
        .master_layer_mut(&master)
        .expect("layer");
    layer.contours = vec![flex_contour()];
    layer.hints = fc_model::PsHints {
        flex_hints: depth
            .map(|depth| {
                vec![fc_model::FlexHint {
                    contour: 0,
                    point: 0,
                    depth,
                }]
            })
            .unwrap_or_default(),
        ..fc_model::PsHints::default()
    };
    layer
        .hints
        .validate(&layer.contours)
        .expect("valid fixture hints");
    (font, source, order)
}

fn wrap_cff(source: &[u8], cff: Vec<u8>) -> Vec<u8> {
    use read_fonts::{types::Tag, FontRef};
    let source = FontRef::new(source).expect("source face");
    let mut builder = write_fonts::FontBuilder::new();
    for record in source.table_directory().table_records() {
        if record.tag() != Tag::new(b"CFF ") {
            builder.add_raw(
                record.tag(),
                source.table_data(record.tag()).expect("table").as_bytes(),
            );
        }
    }
    builder.add_raw(Tag::new(b"CFF "), cff);
    builder.build()
}

fn build(depth: Option<f64>) -> (Vec<u8>, fc_model::GlyphName) {
    let (font, source, order) = flex_source(depth);
    let cff = write_cff(&font, &font.default_master().id, &order, false).expect("CFF");
    let name = order
        .iter()
        .find(|name| name.as_ref() != ".notdef")
        .expect("glyph")
        .clone();
    (wrap_cff(&source, cff.bytes), name)
}

#[test]
fn an_authored_flex_pair_is_written_and_imported_again() {
    for (depth, expected) in [(50.0, true), (60.0, false)] {
        let (bytes, name) = build(Some(depth));
        let imported = fc_sfnt::read_bytes(&bytes, &fc_sfnt::ReadOptions::default())
            .expect("reimport")
            .fonts
            .remove(0);
        let layer = &imported.glyph(&name).expect("glyph").layers[0];
        assert_eq!(
            layer.hints.flex_hints,
            vec![fc_model::FlexHint {
                contour: 0,
                point: 0,
                depth
            }],
            "depth {depth}"
        );
        // The importer additionally marks the tangent-continuous joint smooth.
        assert_eq!(
            layer.contours[0]
                .points
                .iter()
                .map(|point| (point.x, point.y, point.kind))
                .collect::<Vec<_>>(),
            flex_contour()
                .points
                .iter()
                .map(|point| (point.x, point.y, point.kind))
                .collect::<Vec<_>>(),
            "depth {depth}"
        );
        assert!(!layer.hints.flex, "depth {depth}");
        // The compact form only applies at the implied height of 50.
        let reader = read_fonts::FontRef::new(&bytes).expect("face");
        let charstring = reader
            .table_data(read_fonts::types::Tag::new(b"CFF "))
            .expect("CFF table");
        let has_flex1 = charstring
            .as_bytes()
            .windows(2)
            .any(|pair| pair == [12, 37]);
        let has_flex = charstring
            .as_bytes()
            .windows(2)
            .any(|pair| pair == [12, 35]);
        assert_eq!(has_flex1, expected, "depth {depth}");
        assert_eq!(has_flex, !expected, "depth {depth}");
    }
}

#[test]
fn independent_tools_decode_flex_and_render_the_same_outline() {
    use std::process::Command;
    if !Command::new("python3")
        .args(["-c", "import fontTools, freetype"])
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("skipping independent CFF flex check: fontTools/freetype unavailable");
        return;
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let (flexed, name) = build(Some(50.0));
    let (plain, _) = build(None);
    let flexed_path = directory.path().join("flexed.otf");
    let plain_path = directory.path().join("plain.otf");
    std::fs::write(&flexed_path, &flexed).expect("write");
    std::fs::write(&plain_path, &plain).expect("write");
    let script = r#"
import sys, freetype
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import RecordingPen

flexed, plain, name = sys.argv[1], sys.argv[2], sys.argv[3]

def charstring(path):
    font = TTFont(path)
    cs = font['CFF '].cff.topDictIndex[0].CharStrings[name]
    cs.decompile()
    return cs

program = charstring(flexed).program
assert 'flex1' in program, program
start = program.index('flex1') - 11
assert program[start:start + 12] == [15, 0, 10, 10, 25, 0, 25, 0, 10, -10, 15, 'flex1'], program
assert 'flex' not in program, program

def outline(path):
    pen = RecordingPen()
    charstring(path).draw(pen)
    return pen.value

assert outline(flexed) == outline(plain), (outline(flexed), outline(plain))

renders = 0
for path in (flexed, plain):
    face = freetype.Face(path)
    index = face.get_name_index(name.encode())
    for size in (9, 12, 16, 24, 36, 48):
        face.set_pixel_sizes(0, size)
        face.load_glyph(index, freetype.FT_LOAD_RENDER)
        renders += 1
print('renders', renders)
"#;
    let file = directory.path().join("flex.py");
    std::fs::write(&file, script).expect("write script");
    let output = Command::new("python3")
        .arg(&file)
        .arg(&flexed_path)
        .arg(&plain_path)
        .arg(name.as_ref())
        .output()
        .expect("python3");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("renders 12"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[derive(Default)]
struct Order(Vec<&'static str>);

impl CommandSink for Order {
    fn move_to(&mut self, _: Fixed, _: Fixed) {
        self.0.push("move");
    }
    fn line_to(&mut self, _: Fixed, _: Fixed) {
        self.0.push("line");
    }
    fn curve_to(&mut self, _: Fixed, _: Fixed, _: Fixed, _: Fixed, _: Fixed, _: Fixed) {
        self.0.push("curve");
    }
    fn close(&mut self) {}
    fn vstem(&mut self, _: Fixed, _: Fixed) {
        self.0.push("vstem");
    }
    fn hint_mask(&mut self, _: &[u8]) {
        self.0.push("mask");
    }
}

#[test]
fn a_mask_bound_to_the_flex_joint_is_activated_before_the_flex_operator() {
    let (font, _, order) = flex_source(Some(50.0));
    let mut font = font;
    let master = font.default_master().id.clone();
    let name = order
        .iter()
        .find(|name| name.as_ref() != ".notdef")
        .expect("glyph")
        .clone();
    let layer = font
        .glyph_mut(&name)
        .expect("glyph")
        .master_layer_mut(&master)
        .expect("layer");
    layer.hints.v_stems = vec![
        fc_model::StemHint {
            start: 100.0,
            width: 40.0,
            ranges: Vec::new(),
        },
        fc_model::StemHint {
            start: 160.0,
            width: 40.0,
            ranges: Vec::new(),
        },
    ];
    // The joint is the on-curve point the first of the two curves ends at.
    layer.hints.hint_masks = vec![fc_model::HintMask {
        counter: false,
        closing: false,
        contour: 0,
        point: 3,
        mask: vec![0x40],
    }];
    layer
        .hints
        .validate(&layer.contours)
        .expect("a joint mask is representable");
    let cff = write_cff(&font, &master, &order, false).expect("CFF");
    let reader = CffFontRef::new_cff(&cff.bytes, 0, Some(1000)).expect("CFF reader");
    let subfont = reader.subfont(0, &[]).expect("subfont");
    let mut sink = Order::default();
    reader
        .evaluate_charstring(
            &subfont,
            GlyphId::new(u32::from(
                order.iter().position(|entry| *entry == name).expect("gid") as u16,
            )),
            &[],
            &mut sink,
        )
        .expect("valid charstring");
    // Two stems, the implicit initial mask, the move, then the joint's mask
    // immediately before the two curves the flex operator draws.
    assert_eq!(
        sink.0,
        ["vstem", "vstem", "mask", "move", "mask", "curve", "curve", "line", "line"]
    );
}
