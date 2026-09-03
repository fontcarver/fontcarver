//! Rasterize a real glyph to an 8-bit alpha mask at small ppem, with tiny-skia and zeno.
//! This is the primitive we would use for bitmap-strike (EBDT/CBDT/sbix) generation.
use read_fonts::FontRef;
use skrifa::{instance::{LocationRef, Size}, outline::{DrawSettings, OutlinePen}, MetadataProvider};

#[derive(Default)]
struct Collect(Vec<kurbo::PathEl>);
impl OutlinePen for Collect {
    fn move_to(&mut self, x: f32, y: f32) { self.0.push(kurbo::PathEl::MoveTo((x as f64, y as f64).into())); }
    fn line_to(&mut self, x: f32, y: f32) { self.0.push(kurbo::PathEl::LineTo((x as f64, y as f64).into())); }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
        self.0.push(kurbo::PathEl::QuadTo((a as f64, b as f64).into(), (x as f64, y as f64).into()));
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        self.0.push(kurbo::PathEl::CurveTo((a as f64, b as f64).into(), (c as f64, d as f64).into(), (x as f64, y as f64).into()));
    }
    fn close(&mut self) { self.0.push(kurbo::PathEl::ClosePath); }
}

fn main() {
    let bytes = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf").unwrap();
    let font = FontRef::new(&bytes).unwrap();
    let ppem = 16.0_f32;
    let gid = font.charmap().map('g').unwrap();
    let outlines = font.outline_glyphs();
    let glyph = outlines.get(gid).unwrap();

    // hinted, scaled to ppem
    use skrifa::outline::{Engine, HintingInstance, HintingOptions, SmoothMode};
    let hinter = HintingInstance::new(
        &outlines, Size::new(ppem), LocationRef::default(),
        HintingOptions { engine: Engine::Interpreter, target: SmoothMode::Normal.into() },
    ).unwrap();
    let mut pen = Collect::default();
    glyph.draw(DrawSettings::hinted(&hinter, false), &mut pen).unwrap();
    let path = kurbo::BezPath::from_vec(pen.0);
    let bb = kurbo::Shape::bounding_box(&path);
    println!("glyph 'g' @ {ppem}ppem hinted bbox = {bb:?}");

    let w = bb.width().ceil() as u32 + 2;
    let h = bb.height().ceil() as u32 + 2;

    // ---- tiny-skia Mask -> plain 8-bit coverage ----
    use tiny_skia::{FillRule, Mask, PathBuilder, Transform};
    let mut pb = PathBuilder::new();
    for el in path.elements() {
        match *el {
            kurbo::PathEl::MoveTo(p) => pb.move_to(p.x as f32, p.y as f32),
            kurbo::PathEl::LineTo(p) => pb.line_to(p.x as f32, p.y as f32),
            kurbo::PathEl::QuadTo(c, p) => pb.quad_to(c.x as f32, c.y as f32, p.x as f32, p.y as f32),
            kurbo::PathEl::CurveTo(a, b, p) => pb.cubic_to(a.x as f32, a.y as f32, b.x as f32, b.y as f32, p.x as f32, p.y as f32),
            kurbo::PathEl::ClosePath => pb.close(),
        }
    }
    let tsp = pb.finish().unwrap();
    let mut mask = Mask::new(w, h).unwrap();
    // flip y and translate so bbox top-left is at (1,1)
    mask.fill_path(&tsp, FillRule::Winding, true,
        Transform::from_row(1.0, 0.0, 0.0, -1.0, 1.0 - bb.x0 as f32, 1.0 + bb.y1 as f32));
    println!("\ntiny-skia Mask {w}x{h}, {} bytes of u8 coverage:", mask.data().len());
    print_ascii(mask.data(), w, h);
    // 1-bit threshold, as an EBDT format-1/2 strike would need
    let bits: usize = mask.data().iter().filter(|&&v| v >= 128).count();
    println!("thresholded >=128: {bits} set pixels of {}", w * h);

    // ---- zeno ----
    use zeno::{Command, Format, Mask as ZMask, Vector};
    let mut cmds: Vec<Command> = Vec::new();
    for el in path.elements() {
        match *el {
            kurbo::PathEl::MoveTo(p) => cmds.push(Command::MoveTo(Vector::new(p.x as f32, p.y as f32))),
            kurbo::PathEl::LineTo(p) => cmds.push(Command::LineTo(Vector::new(p.x as f32, p.y as f32))),
            kurbo::PathEl::QuadTo(c, p) => cmds.push(Command::QuadTo(Vector::new(c.x as f32, c.y as f32), Vector::new(p.x as f32, p.y as f32))),
            kurbo::PathEl::CurveTo(a, b, p) => cmds.push(Command::CurveTo(
                Vector::new(a.x as f32, a.y as f32), Vector::new(b.x as f32, b.y as f32), Vector::new(p.x as f32, p.y as f32))),
            kurbo::PathEl::ClosePath => cmds.push(Command::Close),
        }
    }
    let (zbuf, placement) = ZMask::new(&cmds[..])
        .format(Format::Alpha)
        .origin(zeno::Origin::BottomLeft)
        .render();
    println!("\nzeno Mask::render -> placement {placement:?}, {} bytes", zbuf.len());
    print_ascii(&zbuf, placement.width, placement.height);
}

fn print_ascii(data: &[u8], w: u32, h: u32) {
    const RAMP: &[u8] = b" .:-=+*#%@";
    for y in 0..h {
        let mut line = String::new();
        for x in 0..w {
            let v = data[(y * w + x) as usize];
            line.push(RAMP[(v as usize * (RAMP.len() - 1)) / 255] as char);
        }
        println!("  |{line}|");
    }
}
