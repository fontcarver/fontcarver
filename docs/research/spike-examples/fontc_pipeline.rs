//! End-to-end: in-memory .glyphs source -> fontc::generate_font -> read-fonts/skrifa/harfrust.
use fontc::{Input, Options};
use read_fonts::{FontRef, TableProvider};

const GLYPHS: &str = r#"{
.formatVersion = 3;
familyName = "SpikeSans";
unitsPerEm = 1000;
metrics = (
{type = ascender;},
{type = "cap height";},
{type = baseline;},
{type = descender;}
);
fontMaster = (
{
id = "m01";
name = Regular;
metricValues = (
{pos = 800;},
{pos = 700;},
{},
{pos = -200;}
);
}
);
glyphs = (
{
glyphname = A;
unicode = 65;
layers = (
{
layerId = "m01";
width = 600;
shapes = (
{
closed = 1;
nodes = (
(50,0,l),
(550,0,l),
(550,700,l),
(50,700,l)
);
}
);
}
);
},
{
glyphname = B;
unicode = 66;
layers = (
{
layerId = "m01";
width = 500;
shapes = (
{
closed = 1;
nodes = (
(50,0,l),
(450,0,l),
(450,700,l),
(50,700,l)
);
}
);
}
);
},
{
glyphname = space;
unicode = 32;
layers = (
{
layerId = "m01";
width = 250;
}
);
}
);
kerningLTR = {
m01 = {
A = {
B = -40;
};
};
};
featurePrefixes = (
{
code = "";
name = prefix;
}
);
features = (
{
code = "sub A by B;";
tag = ss01;
}
);
}"#;

fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    let input = Input::from_glyphs(GLYPHS.to_string());
    let source = input.create_source().expect("create_source");
    let opts = Options::default();
    let bytes = match fontc::generate_font(source, opts) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("fontc::generate_font FAILED: {e}");
            std::process::exit(1);
        }
    };
    println!("fontc::generate_font -> {} bytes", bytes.len());
    std::fs::write("/tmp/spike.ttf", &bytes).unwrap();

    // ---- read back with read-fonts ----
    let font = FontRef::new(&bytes).expect("FontRef::new on fontc output");
    let tags: Vec<String> = font
        .table_directory()
        .table_records()
        .iter()
        .map(|r| r.tag().to_string())
        .collect();
    println!("tables produced ({}): {}", tags.len(), tags.join(" "));
    println!("num_glyphs = {}", font.maxp().unwrap().num_glyphs());
    println!("upem = {}", font.head().unwrap().units_per_em());
    println!("has GSUB: {}, GPOS: {}, GDEF: {}", font.gsub().is_ok(), font.gpos().is_ok(), font.gdef().is_ok());

    // ---- skrifa outline ----
    use skrifa::{instance::{LocationRef, Size}, outline::DrawSettings, MetadataProvider};
    let outlines = font.outline_glyphs();
    println!("outline format = {:?}", outlines.format());
    let gid = font.charmap().map('A').expect("cmap A");
    let glyph = outlines.get(gid).unwrap();
    let mut pen = SvgPen(String::new());
    glyph
        .draw(DrawSettings::unhinted(Size::unscaled(), LocationRef::default()), &mut pen)
        .unwrap();
    println!("glyph 'A' (gid {gid:?}) outline = {}", pen.0);
    let gm = font.glyph_metrics(Size::unscaled(), LocationRef::default());
    println!("advance A = {:?}, B = {:?}", gm.advance_width(gid), gm.advance_width(font.charmap().map('B').unwrap()));

    // ---- skrifa hinting instance (grid-fit preview) ----
    use skrifa::outline::{Engine, HintingInstance, HintingOptions, SmoothMode};
    match HintingInstance::new(
        &outlines,
        Size::new(16.0),
        LocationRef::default(),
        HintingOptions { engine: Engine::AutoFallback, target: SmoothMode::Normal.into() },
    ) {
        Ok(h) => {
            let mut p = SvgPen(String::new());
            glyph.draw(DrawSettings::hinted(&h, false), &mut p).unwrap();
            println!("HintingInstance OK (enabled={}), hinted@16ppem = {}", h.is_enabled(), p.0);
        }
        Err(e) => println!("HintingInstance failed: {e:?}"),
    }

    // ---- harfrust shaping of the in-memory font ----
    use harfrust::{FontRef as HrFontRef, ShapeOptions, ShaperData, UnicodeBuffer};
    let hr_font = HrFontRef::from_index(&bytes, 0).expect("harfrust FontRef");
    let data = ShaperData::new(&hr_font);
    let shaper = data.shaper(&hr_font).build();
    let mut buf = UnicodeBuffer::new();
    buf.push_str("AB");
    buf.guess_segment_properties();
    let out = shaper.shape(buf, ShapeOptions::new());
    let infos = out.glyph_infos();
    let poss = out.glyph_positions();
    println!("harfrust shaped \"AB\": {} glyphs", infos.len());
    for (i, p) in infos.iter().zip(poss.iter()) {
        println!("  gid={} cluster={} x_adv={} x_off={}", i.glyph_id, i.cluster, p.x_advance, p.x_offset);
    }
    println!("(expect kern A/B = -40 applied via GPOS)");
}

struct SvgPen(String);
impl skrifa::outline::OutlinePen for SvgPen {
    fn move_to(&mut self, x: f32, y: f32) { self.0 += &format!("M{x} {y} "); }
    fn line_to(&mut self, x: f32, y: f32) { self.0 += &format!("L{x} {y} "); }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) { self.0 += &format!("Q{a} {b} {x} {y} "); }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) { self.0 += &format!("C{a} {b} {c} {d} {x} {y} "); }
    fn close(&mut self) { self.0 += "Z "; }
}
