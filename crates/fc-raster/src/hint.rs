use crate::{invalid, Error};
use kurbo::BezPath;
use skrifa::{
    instance::{LocationRef, Size},
    outline::{DrawSettings, Engine, HintingInstance, HintingOptions, OutlinePen, Target},
    FontRef, MetadataProvider,
};
use std::collections::HashMap;
/// Draw grid-fitted outlines using skrifa's embedded interpreter or automatic hinter.
/// Glyph IDs must refer to the provided compiled font; paths use pixel coordinates.
pub fn hinted_paths(
    bytes: &[u8],
    glyphs: &[(String, u32)],
    ppem: u16,
    mono: bool,
    automatic: bool,
) -> Result<HashMap<String, BezPath>, Error> {
    if ppem == 0 || ppem > 4096 || glyphs.len() > 65535 {
        return Err(invalid("hinted raster size/count limit"));
    }
    let face = FontRef::new(bytes).map_err(|e| Error::Invalid(e.to_string()))?;
    let outlines = face.outline_glyphs();
    let hint = HintingInstance::new(
        &outlines,
        Size::new(f32::from(ppem)),
        LocationRef::default(),
        HintingOptions {
            engine: if automatic {
                Engine::Auto(None)
            } else {
                Engine::Interpreter
            },
            target: if mono {
                Target::Mono
            } else {
                Target::default()
            },
        },
    )
    .map_err(|e| Error::Invalid(e.to_string()))?;
    let mut result = HashMap::new();
    let mut total = 0;
    for (name, gid) in glyphs {
        let outline = outlines
            .get(skrifa::GlyphId::new(*gid))
            .ok_or_else(|| invalid("missing compiled glyph for hinting"))?;
        let mut pen = Pen(BezPath::new(), false);
        outline
            .draw(DrawSettings::hinted(&hint, true), &mut pen)
            .map_err(|e| Error::Invalid(e.to_string()))?;
        total += pen.0.elements().len();
        if pen.1 || total > 10_000_000 {
            return Err(invalid("hinted outline work limit"));
        }
        result.insert(name.clone(), pen.0);
    }
    Ok(result)
}
struct Pen(BezPath, bool);
impl Pen {
    fn room(&mut self) -> bool {
        if self.0.elements().len() >= 100_000 {
            self.1 = true;
            false
        } else {
            true
        }
    }
}
impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.room() {
            self.0.move_to((f64::from(x), f64::from(y)));
        }
    }
    fn line_to(&mut self, x: f32, y: f32) {
        if self.room() {
            self.0.line_to((f64::from(x), f64::from(y)));
        }
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        if self.room() {
            self.0
                .quad_to((f64::from(cx), f64::from(cy)), (f64::from(x), f64::from(y)));
        }
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        if self.room() {
            self.0.curve_to(
                (f64::from(a), f64::from(b)),
                (f64::from(c), f64::from(d)),
                (f64::from(x), f64::from(y)),
            );
        }
    }
    fn close(&mut self) {
        if self.room() {
            self.0.close_path();
        }
    }
}
