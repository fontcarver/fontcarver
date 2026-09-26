//! Bounded COLR v0 / CPAL v0-v1 codecs, implemented from the OpenType specification.

use crate::Error;
use fc_model::{ColorPalette, ColorTable, ColrLayer, GlyphName, PaletteColor, MAX_COLOR_ITEMS};
use std::collections::HashMap;

struct Data<'a> {
    bytes: &'a [u8],
    tag: &'static str,
}

impl Data<'_> {
    fn range(&self, offset: usize, count: usize, size: usize) -> Result<(), Error> {
        if count
            .checked_mul(size)
            .and_then(|n| offset.checked_add(n))
            .is_none_or(|end| end > self.bytes.len())
        {
            return Err(Error::parse(self.tag, "truncated or overflowing array"));
        }
        Ok(())
    }
    fn u16(&self, at: usize) -> Result<u16, Error> {
        self.range(at, 1, 2)?;
        Ok(u16::from_be_bytes([self.bytes[at], self.bytes[at + 1]]))
    }
    fn u32(&self, at: usize) -> Result<u32, Error> {
        self.range(at, 1, 4)?;
        Ok(u32::from_be_bytes([
            self.bytes[at],
            self.bytes[at + 1],
            self.bytes[at + 2],
            self.bytes[at + 3],
        ]))
    }
    fn array(&self, at: usize, count: usize, size: usize, header: usize) -> Result<usize, Error> {
        let offset = self.u32(at)? as usize;
        if count != 0 && offset < header {
            return Err(Error::parse(self.tag, "array overlaps header"));
        }
        self.range(offset, count, size)?;
        Ok(offset)
    }
    fn optional_array(
        &self,
        at: usize,
        count: usize,
        size: usize,
        header: usize,
    ) -> Result<Option<usize>, Error> {
        if self.u32(at)? == 0 {
            return Ok(None);
        }
        self.array(at, count, size, header).map(Some)
    }
}

fn label(id: u16) -> Option<u16> {
    (id != 0xffff).then_some(id)
}

/// Decode both tables atomically. An absent COLR permits palette-only fonts.
pub fn read_tables(
    colr: Option<&[u8]>,
    cpal: &[u8],
    order: &[GlyphName],
) -> Result<ColorTable, Error> {
    // Never take CPAL ownership while an unsupported COLR graph still needs it.
    if let Some(bytes) = colr {
        if (Data { bytes, tag: "COLR" }).u16(0)? != 0 {
            return Err(Error::parse(
                "COLR",
                "only version 0 compositions are editable",
            ));
        }
    }
    let data = Data {
        bytes: cpal,
        tag: "CPAL",
    };
    let version = data.u16(0)?;
    if version > 1 {
        return Err(Error::parse("CPAL", "unsupported version"));
    }
    let entries = usize::from(data.u16(2)?);
    let palettes = usize::from(data.u16(4)?);
    let records = usize::from(data.u16(6)?);
    if palettes == 0 || entries == 0 || palettes.saturating_mul(entries) > MAX_COLOR_ITEMS {
        return Err(Error::parse(
            "CPAL",
            "invalid palette count or expanded palette limit exceeded",
        ));
    }
    let tail = 12 + palettes * 2;
    let header = tail + if version == 1 { 12 } else { 0 };
    data.range(0, header, 1)?;
    let colors = data.array(8, records, 4, header)?;
    let (flags, labels, entry_labels) = if version == 1 {
        (
            data.optional_array(tail, palettes, 4, header)?,
            data.optional_array(tail + 4, palettes, 2, header)?,
            data.optional_array(tail + 8, entries, 2, header)?,
        )
    } else {
        (None, None, None)
    };
    let mut result = ColorTable {
        palette_version: version,
        ..Default::default()
    };
    for i in 0..palettes {
        let start = usize::from(data.u16(12 + i * 2)?);
        if start + entries > records {
            return Err(Error::parse("CPAL", "palette extends beyond color records"));
        }
        let palette = ColorPalette {
            colors: (start..start + entries)
                .map(|record| {
                    let at = colors + record * 4;
                    PaletteColor {
                        blue: cpal[at],
                        green: cpal[at + 1],
                        red: cpal[at + 2],
                        alpha: cpal[at + 3],
                    }
                })
                .collect(),
            flags: flags
                .map(|at| data.u32(at + i * 4))
                .transpose()?
                .unwrap_or(0),
            label: labels
                .map(|at| data.u16(at + i * 2))
                .transpose()?
                .and_then(label),
        };
        result.palettes.push(palette);
    }
    if let Some(at) = entry_labels {
        result.entry_labels = (0..entries)
            .map(|i| data.u16(at + i * 2).map(label))
            .collect::<Result<_, _>>()?;
    }
    if let Some(bytes) = colr {
        read_layers(bytes, order, &mut result)?;
    }
    result
        .validate(order)
        .map_err(|e| Error::parse("COLR/CPAL", e))?;
    Ok(result)
}

fn read_layers(bytes: &[u8], order: &[GlyphName], result: &mut ColorTable) -> Result<(), Error> {
    let data = Data { bytes, tag: "COLR" };
    data.range(0, 14, 1)?;
    let base_count = usize::from(data.u16(2)?);
    let layer_count = usize::from(data.u16(12)?);
    let bases = data.array(4, base_count, 6, 14)?;
    let layers = data.array(8, layer_count, 4, 14)?;
    let name = |id: u16| {
        order
            .get(usize::from(id))
            .cloned()
            .ok_or_else(|| Error::parse("COLR", "glyph ID outside glyph order"))
    };
    let mut previous = None;
    let mut expanded = 0;
    for i in 0..base_count {
        let at = bases + i * 6;
        let id = data.u16(at)?;
        if previous.is_some_and(|last| last >= id) {
            return Err(Error::parse(
                "COLR",
                "base records must have unique ascending glyph IDs",
            ));
        }
        previous = Some(id);
        let first = usize::from(data.u16(at + 2)?);
        let count = usize::from(data.u16(at + 4)?);
        expanded += count;
        if first + count > layer_count || count == 0 || expanded > MAX_COLOR_ITEMS {
            return Err(Error::parse("COLR", "invalid or excessive layer range"));
        }
        let stack = (first..first + count)
            .map(|i| {
                let at = layers + i * 4;
                Ok(ColrLayer {
                    glyph: name(data.u16(at)?)?,
                    palette_index: label(data.u16(at + 2)?),
                })
            })
            .collect::<Result<_, Error>>()?;
        result.glyphs.insert(name(id)?, stack);
    }
    Ok(())
}

fn u16out(bytes: &mut Vec<u8>, value: usize, tag: &'static str) -> Result<(), Error> {
    let value = u16::try_from(value)
        .map_err(|_| Error::parse(tag, "packed array exceeds 65535 records"))?;
    bytes.extend(value.to_be_bytes());
    Ok(())
}
fn u32out(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend((value as u32).to_be_bytes());
}

/// Encode palettes and ordered layers against the actual exported glyph order.
/// Returns `(COLR, CPAL)`; palette-only data has no COLR table.
pub fn write_tables(
    color: &ColorTable,
    order: &[GlyphName],
) -> Result<(Option<Vec<u8>>, Vec<u8>), Error> {
    color
        .validate(order)
        .map_err(|e| Error::parse("COLR/CPAL", e))?;
    let cpal = write_palettes(color)?;
    if color.glyphs.is_empty() {
        return Ok((None, cpal));
    }
    let ids: HashMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect();
    let mut records: Vec<_> = color.glyphs.iter().collect();
    records.sort_by_key(|(name, _)| ids[name]);
    let mut bases = Vec::new();
    let mut layers = Vec::new();
    let mut shared = HashMap::new();
    for (name, stack) in &records {
        let start = if let Some(start) = shared.get(*stack) {
            *start
        } else {
            let start = layers.len() / 4;
            for layer in *stack {
                u16out(&mut layers, ids[&layer.glyph], "COLR")?;
                layers.extend(layer.palette_index.unwrap_or(0xffff).to_be_bytes());
            }
            shared.insert(*stack, start);
            start
        };
        u16out(&mut bases, ids[name], "COLR")?;
        u16out(&mut bases, start, "COLR")?;
        u16out(&mut bases, stack.len(), "COLR")?;
    }
    let mut colr = vec![0, 0];
    u16out(&mut colr, records.len(), "COLR")?;
    u32out(&mut colr, 14);
    u32out(&mut colr, 14 + bases.len());
    u16out(&mut colr, layers.len() / 4, "COLR")?;
    colr.extend(bases);
    colr.extend(layers);
    Ok((Some(colr), cpal))
}

fn write_palettes(color: &ColorTable) -> Result<Vec<u8>, Error> {
    let entries = color.palettes[0].colors.len();
    let count = color.palettes.len();
    let header = 12 + count * 2 + if color.palette_version == 1 { 12 } else { 0 };
    let mut indices = Vec::new();
    let mut colors = Vec::new();
    let mut shared = HashMap::new();
    for palette in &color.palettes {
        let start = if let Some(start) = shared.get(&palette.colors) {
            *start
        } else {
            let start = colors.len() / 4;
            for c in &palette.colors {
                colors.extend([c.blue, c.green, c.red, c.alpha]);
            }
            shared.insert(&palette.colors, start);
            start
        };
        u16out(&mut indices, start, "CPAL")?;
    }
    let mut bytes = Vec::new();
    bytes.extend(color.palette_version.to_be_bytes());
    u16out(&mut bytes, entries, "CPAL")?;
    u16out(&mut bytes, count, "CPAL")?;
    u16out(&mut bytes, colors.len() / 4, "CPAL")?;
    u32out(&mut bytes, header);
    bytes.extend(indices);
    if color.palette_version == 1 {
        let start = header + colors.len();
        u32out(&mut bytes, start);
        u32out(&mut bytes, start + count * 4);
        u32out(
            &mut bytes,
            if color.entry_labels.is_empty() {
                0
            } else {
                start + count * 6
            },
        );
    }
    bytes.extend(colors);
    if color.palette_version == 1 {
        for palette in &color.palettes {
            bytes.extend(palette.flags.to_be_bytes());
        }
        for palette in &color.palettes {
            bytes.extend(palette.label.unwrap_or(0xffff).to_be_bytes());
        }
        for label in &color.entry_labels {
            bytes.extend(label.unwrap_or(0xffff).to_be_bytes());
        }
    }
    Ok(bytes)
}
