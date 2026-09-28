//! Bounded static COLRv1 decoding. Variable tables retain raw ownership atomically.
mod write;
use super::Data;
use crate::Error;
use fc_model::{ColrColorLine, ColrColorStop, ColrGraph, ColrPaint, ColrTransform, GlyphName};
use std::collections::HashMap;
pub(super) use write::upgrade;

fn error(message: impl std::fmt::Display) -> Error {
    Error::parse("COLRv1", message)
}
impl Data<'_> {
    fn byte(&self, at: usize) -> Result<u8, Error> {
        self.range(at, 1, 1)?;
        Ok(self.bytes[at])
    }
    fn signed(&self, at: usize) -> Result<i16, Error> {
        Ok(self.u16(at)? as i16)
    }
    fn f2(&self, at: usize) -> Result<f64, Error> {
        Ok(f64::from(self.signed(at)?) / 16384.)
    }
    fn point(&self, at: usize) -> Result<[i16; 2], Error> {
        Ok([self.signed(at)?, self.signed(at + 2)?])
    }
    fn offset24(&self, base: usize, field: usize, minimum: usize) -> Result<usize, Error> {
        self.range(field, 3, 1)?;
        let b = &self.bytes[field..field + 3];
        let offset = (usize::from(b[0]) << 16) | (usize::from(b[1]) << 8) | usize::from(b[2]);
        self.relative(base, offset, minimum)
    }
    fn relative(&self, base: usize, offset: usize, minimum: usize) -> Result<usize, Error> {
        if offset < minimum {
            return Err(error("offset overlaps parent record"));
        }
        let target = base
            .checked_add(offset)
            .ok_or_else(|| error("offset overflow"))?;
        self.range(target, 1, 1)?;
        Ok(target)
    }
}
struct Reader<'a> {
    data: Data<'a>,
    order: &'a [GlyphName],
    graph: ColrGraph,
    offsets: HashMap<usize, u32>,
    layers: Vec<usize>,
    items: usize,
}

pub(super) fn read(bytes: &[u8], order: &[GlyphName]) -> Result<ColrGraph, Error> {
    let data = Data {
        bytes,
        tag: "COLRv1",
    };
    data.range(0, 34, 1)?;
    if bytes.len() > 32 * 1024 * 1024 || data.u32(26)? != 0 || data.u32(30)? != 0 {
        return Err(error("oversized or variable COLRv1 table is not editable"));
    }
    let mut reader = Reader {
        data,
        order,
        graph: ColrGraph::default(),
        offsets: HashMap::new(),
        layers: Vec::new(),
        items: 0,
    };
    let layer_list = reader.data.u32(18)? as usize;
    if layer_list != 0 {
        if layer_list < 34 {
            return Err(error("layer list overlaps header"));
        }
        let count = reader.data.u32(layer_list)? as usize;
        if count > 1_000_000 {
            return Err(error("layer list budget exceeded"));
        }
        reader.data.range(layer_list + 4, count, 4)?;
        for i in 0..count {
            let offset = reader.data.u32(layer_list + 4 + i * 4)? as usize;
            reader
                .layers
                .push(reader.data.relative(layer_list, offset, 4 + count * 4)?);
        }
    }
    let bases = reader.data.u32(14)? as usize;
    if bases != 0 {
        if bases < 34 {
            return Err(error("base glyph list overlaps header"));
        }
        let count = reader.data.u32(bases)? as usize;
        if count > 65535 {
            return Err(error("too many color glyph roots"));
        }
        reader.data.range(bases + 4, count, 6)?;
        let mut previous = None;
        for i in 0..count {
            let record = bases + 4 + i * 6;
            let gid = reader.data.u16(record)?;
            if previous.is_some_and(|last| gid <= last) {
                return Err(error("base glyphs are not sorted and unique"));
            }
            previous = Some(gid);
            let name = reader.name(gid)?;
            let offset = reader.data.u32(record + 2)? as usize;
            let target = reader.data.relative(bases, offset, 4 + count * 6)?;
            let paint = reader.paint(target, 0)?;
            reader.graph.glyphs.insert(name, paint);
        }
    }
    let clips = reader.data.u32(22)? as usize;
    if clips != 0 {
        reader.clips(clips)?;
    }
    Ok(reader.graph)
}

impl Reader<'_> {
    fn name(&self, gid: u16) -> Result<GlyphName, Error> {
        self.order
            .get(usize::from(gid))
            .cloned()
            .ok_or_else(|| error("glyph ID outside glyph order"))
    }
    fn line(&mut self, at: usize) -> Result<ColrColorLine, Error> {
        let extend = self.data.byte(at)?;
        let count = usize::from(self.data.u16(at + 1)?);
        self.data.range(at + 3, count, 6)?;
        self.items = self.items.saturating_add(count);
        if self.items > 1_000_000 {
            return Err(error("color stop budget exceeded"));
        }
        let mut stops = Vec::with_capacity(count);
        for i in 0..count {
            let at = at + 3 + i * 6;
            stops.push(ColrColorStop {
                offset: self.data.f2(at)?,
                palette_index: super::label(self.data.u16(at + 2)?),
                alpha: self.data.f2(at + 4)?,
            });
        }
        Ok(ColrColorLine { extend, stops })
    }
    fn paint(&mut self, at: usize, depth: usize) -> Result<u32, Error> {
        if depth > 64 {
            return Err(error("paint nesting limit exceeded"));
        }
        if let Some(index) = self.offsets.get(&at) {
            return Ok(*index);
        }
        if self.graph.paints.len() >= fc_model::colrv1::MAX_COLR_PAINTS {
            return Err(error("paint node budget exceeded"));
        }
        let index = self.graph.paints.len() as u32;
        self.offsets.insert(at, index);
        self.graph.paints.push(ColrPaint::Solid {
            palette_index: None,
            alpha: 0.,
        });
        let format = self.data.byte(at)?;
        let paint = match format {
            1 => {
                if self.data.u32(18)? == 0 {
                    return Err(error("PaintColrLayers requires LayerList"));
                }
                let count = usize::from(self.data.byte(at + 1)?);
                let first = self.data.u32(at + 2)? as usize;
                let end = first
                    .checked_add(count)
                    .ok_or_else(|| error("layer range overflow"))?;
                let offsets = self
                    .layers
                    .get(first..end)
                    .ok_or_else(|| error("layer range outside LayerList"))?
                    .to_vec();
                self.items = self.items.saturating_add(count);
                if self.items > 1_000_000 {
                    return Err(error("paint layer budget exceeded"));
                }
                let paints = offsets
                    .into_iter()
                    .map(|offset| self.paint(offset, depth + 1))
                    .collect::<Result<_, _>>()?;
                ColrPaint::Layers { paints }
            }
            2 => ColrPaint::Solid {
                palette_index: super::label(self.data.u16(at + 1)?),
                alpha: self.data.f2(at + 3)?,
            },
            4 | 6 | 8 => {
                let header = if format == 8 { 12 } else { 16 };
                self.data.range(at, header, 1)?;
                let target = self.data.offset24(at, at + 1, header)?;
                let color_line = self.line(target)?;
                match format {
                    4 => ColrPaint::Linear {
                        color_line,
                        points: [
                            self.data.point(at + 4)?,
                            self.data.point(at + 8)?,
                            self.data.point(at + 12)?,
                        ],
                    },
                    6 => ColrPaint::Radial {
                        color_line,
                        centers: [self.data.point(at + 4)?, self.data.point(at + 10)?],
                        radii: [self.data.u16(at + 8)?, self.data.u16(at + 14)?],
                    },
                    _ => ColrPaint::Sweep {
                        color_line,
                        center: self.data.point(at + 4)?,
                        angles: [
                            (self.data.f2(at + 8)? + 1.) * 180.,
                            (self.data.f2(at + 10)? + 1.) * 180.,
                        ],
                    },
                }
            }
            10 => {
                let glyph = self.name(self.data.u16(at + 4)?)?;
                let target = self.data.offset24(at, at + 1, 6)?;
                ColrPaint::Glyph {
                    glyph,
                    paint: self.paint(target, depth + 1)?,
                }
            }
            11 => ColrPaint::ColrGlyph {
                glyph: self.name(self.data.u16(at + 1)?)?,
            },
            12 | 14 | 16 | 18 | 20 | 22 | 24 | 26 | 28 | 30 => {
                let (transform, len) = self.transform(at, format)?;
                let target = self.data.offset24(at, at + 1, len)?;
                ColrPaint::Transform {
                    paint: self.paint(target, depth + 1)?,
                    transform,
                }
            }
            32 => {
                let source = self.data.offset24(at, at + 1, 8)?;
                let backdrop = self.data.offset24(at, at + 5, 8)?;
                ColrPaint::Composite {
                    mode: self.data.byte(at + 4)?,
                    source: self.paint(source, depth + 1)?,
                    backdrop: self.paint(backdrop, depth + 1)?,
                }
            }
            _ => {
                return Err(error(format!(
                    "paint format {format} is variable or unsupported"
                )))
            }
        };
        self.graph.paints[index as usize] = paint;
        Ok(index)
    }
    fn transform(&self, at: usize, format: u8) -> Result<(ColrTransform, usize), Error> {
        let p = |offset| self.data.point(at + offset);
        let f = |offset| self.data.f2(at + offset);
        Ok(match format {
            12 => {
                let affine = self.data.offset24(at, at + 4, 7)?;
                let mut matrix = [0.; 6];
                for (i, v) in matrix.iter_mut().enumerate() {
                    *v = (self.data.u32(affine + i * 4)? as i32) as f64 / 65536.;
                }
                (ColrTransform::Affine { matrix }, 7)
            }
            14 => (ColrTransform::Translate { offset: p(4)? }, 8),
            16 | 18 => (
                ColrTransform::Scale {
                    scale: [f(4)?, f(6)?],
                    center: if format == 18 { p(8)? } else { [0, 0] },
                },
                if format == 18 { 12 } else { 8 },
            ),
            20 | 22 => (
                ColrTransform::Scale {
                    scale: [f(4)?, f(4)?],
                    center: if format == 22 { p(6)? } else { [0, 0] },
                },
                if format == 22 { 10 } else { 6 },
            ),
            24 | 26 => (
                ColrTransform::Rotate {
                    angle: f(4)? * 180.,
                    center: if format == 26 { p(6)? } else { [0, 0] },
                },
                if format == 26 { 10 } else { 6 },
            ),
            _ => (
                ColrTransform::Skew {
                    angles: [f(4)? * 180., f(6)? * 180.],
                    center: if format == 30 { p(8)? } else { [0, 0] },
                },
                if format == 30 { 12 } else { 8 },
            ),
        })
    }
    fn clips(&mut self, at: usize) -> Result<(), Error> {
        if at < 34 || self.data.byte(at)? != 1 {
            return Err(error("unsupported clip list"));
        }
        let count = self.data.u32(at + 1)? as usize;
        if count > 65535 {
            return Err(error("clip range budget exceeded"));
        }
        self.data.range(at + 5, count, 7)?;
        let mut previous = None;
        for i in 0..count {
            let record = at + 5 + i * 7;
            let start = self.data.u16(record)?;
            let end = self.data.u16(record + 2)?;
            if start > end || previous.is_some_and(|last| start <= last) {
                return Err(error("unsorted or overlapping clip ranges"));
            }
            previous = Some(end);
            let target = self.data.offset24(at, record + 4, 5 + count * 7)?;
            if self.data.byte(target)? != 1 {
                return Err(error("variable clip boxes are not editable"));
            }
            let clip = [
                self.data.signed(target + 1)?,
                self.data.signed(target + 3)?,
                self.data.signed(target + 5)?,
                self.data.signed(target + 7)?,
            ];
            for gid in start..=end {
                self.graph.clips.insert(self.name(gid)?, clip);
            }
        }
        Ok(())
    }
}
