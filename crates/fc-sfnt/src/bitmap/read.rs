use super::{checked, error, png_codec};
use crate::Error;
use fc_model::{BdfValue, Bitmap, BitmapGlyph, BitmapRef, BitmapStrike, Font, GlyphName};
use read_fonts::{
    tables::bitmap::{BitmapContent, BitmapData, BitmapDataFormat, BitmapMetrics},
    types::{GlyphId, Tag},
    FontRef, ReadError, TableProvider,
};
use std::collections::BTreeMap;

/// Decode all supported embedded strikes without silently skipping malformed glyphs.
pub fn read_strikes(face: &FontRef<'_>, font: &Font) -> Result<Vec<BitmapStrike>, Error> {
    let mut strikes = Vec::new();
    for color in [false, true] {
        let (location_tag, data_tag) = if color {
            (b"CBLC", b"CBDT")
        } else {
            (b"EBLC", b"EBDT")
        };
        let has_location = face.table_data(Tag::new(location_tag)).is_some();
        let has_data = face.table_data(Tag::new(data_tag)).is_some();
        if has_location != has_data {
            return Err(error("bitmap location/data table pair is incomplete"));
        }
        if has_data {
            strikes.extend(bdt(face, font, color)?);
        }
    }
    if face.table_data(Tag::new(b"sbix")).is_some() {
        strikes.extend(sbix(face, font)?);
    }
    let mut result = Vec::<BitmapStrike>::new();
    let mut bytes = 0usize;
    for strike in strikes {
        checked(strike.validate())?;
        if let Some(existing) = result
            .iter()
            .find(|s| (s.ppem, s.depth) == (strike.ppem, strike.depth))
        {
            if existing.glyphs != strike.glyphs {
                return Err(error(
                    "conflicting embedded strikes have the same size and depth",
                ));
            }
            continue;
        }
        bytes += strike
            .glyphs
            .values()
            .map(|g| g.bitmap.rows.len())
            .sum::<usize>();
        if bytes > fc_model::MAX_FONT_BITMAP_BYTES {
            return Err(error("embedded bitmap memory limit"));
        }
        result.push(strike);
    }
    result.sort_by_key(|s| (s.ppem, s.depth));
    Ok(result)
}

fn bdt(face: &FontRef<'_>, font: &Font, color: bool) -> Result<Vec<BitmapStrike>, Error> {
    let (sizes, offset_data) = if color {
        let loc = face.cblc().map_err(|e| error(e.to_string()))?;
        (loc.bitmap_sizes().to_vec(), loc.offset_data())
    } else {
        let loc = face.eblc().map_err(|e| error(e.to_string()))?;
        (loc.bitmap_sizes().to_vec(), loc.offset_data())
    };
    if sizes.len() > 256 {
        return Err(error("embedded strike count exceeds 256"));
    }
    let names: Vec<_> = font.glyphs.keys().cloned().collect();
    let mut out = Vec::new();
    let mut total = 0usize;
    for size in sizes {
        if size.ppem_x() != size.ppem_y() {
            return Err(error("anisotropic embedded strikes are not supported"));
        }
        let mut strike = BitmapStrike {
            ppem: u16::from(size.ppem_y()),
            depth: size.bit_depth(),
            ascent: i16::from(size.hori().ascender()).max(0),
            descent: i16::from(size.hori().descender()).abs(),
            ..Default::default()
        };
        strike.properties.insert(
            "FONTCARVER_BITMAP_TABLE".into(),
            BdfValue::Str(if color { "CBDT" } else { "EBDT" }.into()),
        );
        let list = size
            .index_subtable_list(offset_data)
            .map_err(|e| error(e.to_string()))?;
        let records = list.index_subtable_records();
        if records.len() != size.number_of_index_subtables() as usize || records.len() > 65535 {
            return Err(error("truncated bitmap subtable list"));
        }
        let mut components = BTreeMap::<GlyphName, Vec<(GlyphName, i16, i16)>>::new();
        for record in records {
            record
                .index_subtable(list.offset_data())
                .map_err(|e| error(e.to_string()))?;
            let first = record.first_glyph_index().to_u32();
            let last = record.last_glyph_index().to_u32();
            if first > last || last as usize >= names.len() {
                return Err(error("bitmap glyph range is invalid"));
            }
            for gid in first..=last {
                let location = match size.location(offset_data, GlyphId::new(gid)) {
                    Ok(value) => value,
                    Err(ReadError::InvalidCollectionIndex(_)) => continue,
                    Err(e) => return Err(error(e.to_string())),
                };
                if location.is_empty() {
                    continue;
                }
                let data = if color {
                    face.cbdt()
                        .map_err(|e| error(e.to_string()))?
                        .data(&location)
                } else {
                    face.ebdt()
                        .map_err(|e| error(e.to_string()))?
                        .data(&location)
                }
                .map_err(|e| error(e.to_string()))?;
                let (glyph, refs) = decode_bdt(data, size.bit_depth(), size.flags().bits())?;
                total += glyph.bitmap.rows.len();
                if total > fc_model::MAX_FONT_BITMAP_BYTES {
                    return Err(error("embedded pixel memory limit"));
                }
                let name = names[gid as usize].clone();
                if strike.glyphs.insert(name.clone(), glyph).is_some() {
                    return Err(error("overlapping bitmap glyph ranges"));
                }
                if !refs.is_empty() {
                    components.insert(
                        name,
                        refs.into_iter()
                            .map(|(gid, x, y)| {
                                Ok((
                                    names
                                        .get(gid as usize)
                                        .ok_or_else(|| {
                                            error("bitmap component glyph out of range")
                                        })?
                                        .clone(),
                                    x,
                                    y,
                                ))
                            })
                            .collect::<Result<_, Error>>()?,
                    );
                }
            }
        }
        for (name, refs) in components {
            let parent = &strike.glyphs[&name].bitmap;
            let x = i32::from(parent.origin_x);
            let top = i32::from(parent.origin_y) + i32::from(parent.height);
            let references = refs
                .into_iter()
                .map(|(base, dx, dy)| {
                    let child = &strike
                        .glyphs
                        .get(&base)
                        .ok_or_else(|| error("missing bitmap component image"))?
                        .bitmap;
                    let dx = i16::try_from(x + i32::from(dx) - i32::from(child.origin_x))
                        .map_err(|_| error("bitmap component x overflow"))?;
                    let dy = i16::try_from(
                        top - i32::from(dy) - i32::from(child.origin_y) - i32::from(child.height),
                    )
                    .map_err(|_| error("bitmap component y overflow"))?;
                    Ok(BitmapRef { base, dx, dy })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            if let Some(g) = strike.glyphs.get_mut(&name) {
                g.refs = references;
            }
        }
        out.push(strike);
    }
    Ok(out)
}

type Components = Vec<(u16, i16, i16)>;
fn decode_bdt(
    data: BitmapData<'_>,
    depth: u8,
    flags: u8,
) -> Result<(BitmapGlyph, Components), Error> {
    let (width, height, x, top, advance, vertical, vertical_origin) = match data.metrics {
        BitmapMetrics::Small(m) => {
            if flags & 1 == 0 {
                return Err(error(
                    "vertical-only small bitmap metrics require a horizontal origin",
                ));
            }
            (
                m.width(),
                m.height(),
                i16::from(m.bearing_x()),
                i16::from(m.bearing_y()),
                i16::from(m.advance()),
                None,
                None,
            )
        }
        BitmapMetrics::Big(m) => {
            let origin = (
                i16::from(m.hori_bearing_x()) - i16::from(m.vert_bearing_x()),
                i16::from(m.hori_bearing_y()) - i16::from(m.vert_bearing_y()),
            );
            (
                m.width(),
                m.height(),
                i16::from(m.hori_bearing_x()),
                i16::from(m.hori_bearing_y()),
                i16::from(m.hori_advance()),
                Some(i16::from(m.vert_advance())),
                Some(origin),
            )
        }
    };
    let mut bitmap = checked(Bitmap::new(u16::from(width), u16::from(height), depth))?;
    let mut refs = Vec::new();
    match data.content {
        BitmapContent::Composite(components) => {
            for c in components {
                refs.push((
                    c.glyph_id().to_u16(),
                    i16::from(c.x_offset()),
                    i16::from(c.y_offset()),
                ));
            }
        }
        BitmapContent::Data(BitmapDataFormat::Png, bytes) => {
            if depth != 32 {
                return Err(error("PNG bitmap depth must be 32"));
            }
            bitmap = png_codec::decode(bytes)?;
            if (bitmap.width, bitmap.height) != (u16::from(width), u16::from(height)) {
                return Err(error("PNG dimensions differ from bitmap metrics"));
            }
        }
        BitmapContent::Data(format, bytes) => {
            let stride = checked(bitmap.stride())?;
            if depth == 32 {
                if bytes.len() != bitmap.rows.len() {
                    return Err(error("invalid BGRA byte count"));
                }
                for (dst, src) in bitmap.rows.chunks_exact_mut(4).zip(bytes.chunks_exact(4)) {
                    let a = u32::from(src[3]);
                    for i in 0..3 {
                        dst[i] = if a == 0 {
                            0
                        } else {
                            (u32::from(src[2 - i]) * 255 / a).min(255) as u8
                        };
                    }
                    dst[3] = src[3];
                }
            } else {
                for row in 0..bitmap.height {
                    for col in 0..bitmap.width {
                        let bit = if format == BitmapDataFormat::BitAligned {
                            (usize::from(row) * usize::from(bitmap.width) + usize::from(col))
                                * usize::from(depth)
                        } else {
                            usize::from(row) * stride * 8 + usize::from(col) * usize::from(depth)
                        };
                        let mut sample = 0;
                        for i in 0..usize::from(depth) {
                            let at = bit + i;
                            sample = (sample << 1)
                                | u32::from(
                                    (bytes
                                        .get(at / 8)
                                        .ok_or_else(|| error("truncated bitmap samples"))?
                                        >> (7 - at % 8))
                                        & 1,
                                );
                        }
                        checked(bitmap.set_pixel(col, row, sample))?;
                    }
                }
            }
        }
    }
    bitmap.origin_x = x;
    bitmap.origin_y = top - i16::from(height);
    Ok((
        BitmapGlyph {
            bitmap,
            advance,
            vertical_advance: vertical,
            vertical_origin,
            refs: Vec::new(),
        },
        refs,
    ))
}

fn sbix(face: &FontRef<'_>, font: &Font) -> Result<Vec<BitmapStrike>, Error> {
    let table = face.sbix().map_err(|e| error(e.to_string()))?;
    if table.num_strikes() > 256 || table.strikes().len() != table.num_strikes() as usize {
        return Err(error("invalid sbix strike count"));
    }
    let names: Vec<_> = font.glyphs.keys().cloned().collect();
    let mut out = Vec::new();
    let mut total = 0usize;
    for index in 0..table.strikes().len() {
        let source = table
            .strikes()
            .get(index)
            .map_err(|e| error(e.to_string()))?;
        let ppem = source.ppem();
        let scale = f64::from(ppem) / f64::from(font.info.units_per_em);
        let metric = |v: f64| -> Result<i16, Error> {
            let n = (v * scale).round();
            if !n.is_finite() || n < i16::MIN as f64 || n > i16::MAX as f64 {
                Err(error("sbix metric overflow"))
            } else {
                Ok(n as i16)
            }
        };
        let mut strike = BitmapStrike {
            ppem,
            depth: 32,
            ascent: metric(font.default_master().metrics.ascender)?.max(0),
            descent: metric(-font.default_master().metrics.descender)?.max(0),
            ..Default::default()
        };
        strike.properties.insert(
            "FONTCARVER_BITMAP_TABLE".into(),
            BdfValue::Str("sbix".into()),
        );
        for (gid, name) in names.iter().enumerate() {
            let Some(data) = source
                .glyph_data(GlyphId::new(gid as u32))
                .map_err(|e| error(e.to_string()))?
            else {
                continue;
            };
            let origin = (data.origin_offset_x(), data.origin_offset_y());
            let mut data = data;
            let mut seen = vec![gid];
            while data.graphic_type() == Tag::new(b"dupe") {
                let bytes = data.data();
                if bytes.len() != 2 {
                    return Err(error("invalid sbix duplicate"));
                }
                let target = usize::from(u16::from_be_bytes([bytes[0], bytes[1]]));
                if seen.len() >= 64 || seen.contains(&target) {
                    return Err(error("sbix duplicate cycle/depth"));
                }
                seen.push(target);
                data = source
                    .glyph_data(GlyphId::new(target as u32))
                    .map_err(|e| error(e.to_string()))?
                    .ok_or_else(|| error("missing sbix duplicate image"))?;
            }
            if data.graphic_type() != Tag::new(b"png ") {
                return Err(error(
                    "sbix supports editable PNG images; this graphic type is unsupported",
                ));
            }
            let mut bitmap = png_codec::decode(data.data())?;
            bitmap.origin_x = origin.0;
            bitmap.origin_y = origin.1;
            total += bitmap.rows.len();
            if total > fc_model::MAX_FONT_BITMAP_BYTES {
                return Err(error("sbix decoded memory limit"));
            }
            let layer = font
                .glyph(name)
                .and_then(|g| g.layers.first())
                .ok_or_else(|| error("missing sbix glyph metrics"))?;
            strike.glyphs.insert(
                name.clone(),
                BitmapGlyph {
                    bitmap,
                    advance: metric(layer.width)?,
                    ..Default::default()
                },
            );
        }
        out.push(strike);
    }
    Ok(out)
}
