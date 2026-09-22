use crate::{invalid, Error};
use kurbo::BezPath;
use skrifa::{
    instance::{LocationRef, Size},
    outline::{DrawSettings, Engine, HintingInstance, HintingOptions, OutlinePen, Target},
    FontRef, MetadataProvider,
};
use std::collections::HashMap;

/// One compiled outline and its horizontal advance in the requested coordinate space.
pub struct GlyphDrawing {
    /// Bounded outline path.
    pub path: BezPath,
    /// Scaled or interpreter-adjusted horizontal advance.
    pub advance: f64,
}

/// Read the exact unhinted compiled outline in font units.
pub fn compiled_outline(bytes: &[u8], glyph_id: u16) -> Result<GlyphDrawing, Error> {
    draw_one(bytes, glyph_id, Size::unscaled(), None)
}

/// Draw a single glyph at a pixel size, optionally using the pedantic TT interpreter.
/// Skrifa bounds VM instruction execution and backward control flow; errors propagate.
pub fn preview_glyph(
    bytes: &[u8],
    glyph_id: u16,
    ppem: u16,
    hinted: bool,
    mono: bool,
) -> Result<GlyphDrawing, Error> {
    if ppem == 0 || ppem > 128 {
        return Err(invalid("instruction preview size must be in 1..=128"));
    }
    draw_one(
        bytes,
        glyph_id,
        Size::new(f32::from(ppem)),
        hinted.then_some(mono),
    )
}

fn draw_one(
    bytes: &[u8],
    glyph_id: u16,
    size: Size,
    hinted: Option<bool>,
) -> Result<GlyphDrawing, Error> {
    let face = FontRef::new(bytes).map_err(|e| Error::Invalid(e.to_string()))?;
    let outlines = face.outline_glyphs();
    let outline = outlines
        .get(skrifa::GlyphId::new(u32::from(glyph_id)))
        .ok_or_else(|| invalid("missing compiled preview glyph"))?;
    let hint = hinted
        .map(|mono| {
            HintingInstance::new(
                &outlines,
                size,
                LocationRef::default(),
                HintingOptions {
                    engine: Engine::Interpreter,
                    target: if mono {
                        Target::Mono
                    } else {
                        Target::default()
                    },
                },
            )
        })
        .transpose()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let settings = hint.as_ref().map_or_else(
        || DrawSettings::unhinted(size, LocationRef::default()),
        |hint| DrawSettings::hinted(hint, true),
    );
    let mut pen = Pen(BezPath::new(), false);
    let metrics = outline
        .draw(settings, &mut pen)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    if pen.1 {
        return Err(invalid("instruction preview outline work limit"));
    }
    let advance = metrics
        .advance_width
        .or_else(|| {
            face.glyph_metrics(size, LocationRef::default())
                .advance_width(skrifa::GlyphId::new(u32::from(glyph_id)))
        })
        .ok_or_else(|| invalid("missing preview advance width"))?;
    Ok(GlyphDrawing {
        path: pen.0,
        advance: f64::from(advance),
    })
}

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
