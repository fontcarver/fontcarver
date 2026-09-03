//! Does read-fonts 0.43.3 really parse Type 1 (.pfb/.pfa)?
use read_fonts::ps::type1::Type1Font;

struct P(String);
impl read_fonts::model::pen::OutlinePen for P {
    fn move_to(&mut self, x: f32, y: f32) { self.0 += &format!("M{x:.0} {y:.0} "); }
    fn line_to(&mut self, x: f32, y: f32) { self.0 += &format!("L{x:.0} {y:.0} "); }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) { self.0 += &format!("Q{a:.0} {b:.0} {x:.0} {y:.0} "); }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) { self.0 += &format!("C{a:.0} {b:.0} {c:.0} {d:.0} {x:.0} {y:.0} "); }
    fn close(&mut self) { self.0 += "Z "; }
}

fn main() {
    for path in ["/usr/share/fonts/X11/Type1/P052-BoldItalic.pfb",
                 "/usr/share/fonts/X11/Type1/NimbusSansNarrow-Oblique.pfb"] {
        let Ok(data) = std::fs::read(path) else { continue };
        match Type1Font::new(&data) {
            Ok(f) => {
                println!("{path}\n  name={:?} full={:?} family={:?} weight={:?}",
                         f.name(), f.full_name(), f.family_name(), f.weight());
                println!("  num_glyphs={} upem={} italic_angle={} bbox={:?} fixed_pitch={}",
                         f.num_glyphs(), f.upem(), f.italic_angle(), f.bbox(), f.is_fixed_pitch());
                let names: Vec<&str> = f.glyph_names().take(8).map(|(_, n)| n).collect();
                println!("  first glyph names: {names:?}");
                let gid = f.unicode_charmap().map('A' as u32).unwrap_or(read_fonts::types::GlyphId::new(1));
                let mut pen = P(String::new());
                match f.draw(gid, None, &mut pen) {
                    Ok(adv) => println!("  draw({gid:?}) advance={adv:?}\n    {}", &pen.0[..pen.0.len().min(180)]),
                    Err(e) => println!("  draw failed: {e:?}"),
                }
            }
            Err(e) => println!("{path}: Type1Font::new FAILED {e:?}"),
        }
    }
}
