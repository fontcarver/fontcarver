//! X11 PCF: both metric encodings, byte/bit orders and scan-unit padding.
use super::{
    add_glyph,
    binary::{cstr, u16le, u32le, Reader},
    checked, error,
};
use crate::Error;
use fc_model::{BdfValue, Bitmap, BitmapGlyph, BitmapStrike, Font};
use std::collections::BTreeMap;

fn table<'a>(tables: &BTreeMap<u32, &'a [u8]>, tag: u32) -> Result<(u32, Reader<'a>), Error> {
    let mut r = Reader::new(
        tables
            .get(&tag)
            .ok_or_else(|| error(format!("missing PCF table {tag}")))?,
    );
    let format = r.u32()?;
    r.big = format & 4 != 0;
    Ok((format, r))
}
fn metric(r: &mut Reader<'_>, compressed: bool) -> Result<[i16; 6], Error> {
    let mut out = [0; 6];
    if compressed {
        for v in &mut out[..5] {
            *v = i16::from(r.u8()?) - 128;
        }
    } else {
        for v in &mut out {
            *v = r.i16()?;
        }
    }
    Ok(out)
}

/// Decode a PCF file with counts, offsets and bitmap sizes checked before use.
pub fn decode(bytes: &[u8]) -> Result<Font, Error> {
    let mut r = Reader::new(bytes);
    if r.take(4)? != b"\x01fcp" {
        return Err(error("invalid PCF signature"));
    }
    let count = r.count(32)?;
    let header = 8 + count * 16;
    let mut tables = BTreeMap::new();
    let mut ranges = Vec::new();
    for _ in 0..count {
        let tag = r.u32()?;
        let format = r.u32()?;
        let size = r.u32()? as usize;
        let offset = r.u32()? as usize;
        let mut end = offset
            .checked_add(size)
            .ok_or_else(|| error("PCF table overflow"))?;
        // X11 bdftopcf advertises 100 bytes for accelerators but emits only
        // 48/72 at EOF. Accept that documented producer behavior narrowly.
        if matches!(tag, 2 | 256) && size == 100 && end > bytes.len() {
            let actual = if format & 0x100 != 0 { 72 } else { 48 };
            if offset.checked_add(actual) != Some(bytes.len()) {
                return Err(error("truncated PCF accelerators"));
            }
            end = bytes.len();
        }
        if offset < header
            || !offset.is_multiple_of(4)
            || size < 4
            || ranges.iter().any(|(a, b)| offset < *b && end > *a)
        {
            return Err(error("overlapping or invalid PCF table"));
        }
        let data = bytes
            .get(offset..end)
            .ok_or_else(|| error("truncated PCF table"))?;
        if Reader::new(data).u32()? != format || tables.insert(tag, data).is_some() {
            return Err(error("duplicate PCF table or format mismatch"));
        }
        ranges.push((offset, end));
    }
    let (format, mut metrics_reader) = table(&tables, 4)?;
    if format & !0x13f != 0 {
        return Err(error("unsupported PCF metric format"));
    }
    let compressed = format & 0x100 != 0;
    let count = if compressed {
        usize::from(metrics_reader.u16()?)
    } else {
        metrics_reader.count(65535)?
    };
    let metrics = (0..count)
        .map(|_| metric(&mut metrics_reader, compressed))
        .collect::<Result<Vec<_>, _>>()?;
    let mut properties = BTreeMap::new();
    if tables.contains_key(&1) {
        let (_, mut p) = table(&tables, 1)?;
        let n = p.count(65535)?;
        let mut entries = Vec::new();
        for _ in 0..n {
            entries.push((p.u32()? as usize, p.u8()?, p.u32()?));
        }
        p.take((4 - n % 4) % 4)?;
        let size = p.count(super::MAX_INPUT as usize)?;
        let strings = p.take(size)?;
        for (name, is_string, value) in entries {
            let name = cstr(strings, name)?;
            let value = match is_string {
                0 => BdfValue::Int(i64::from(value as i32)),
                1 => BdfValue::Str(cstr(strings, value as usize)?),
                _ => return Err(error("invalid PCF property kind")),
            };
            if properties.insert(name, value).is_some() {
                return Err(error("duplicate PCF property"));
            }
        }
    }
    let prop_int = |key: &str| match properties.get(key) {
        Some(BdfValue::Int(v)) => Some(*v),
        _ => None,
    };
    let mut ascent = prop_int("FONT_ASCENT")
        .unwrap_or_else(|| metrics.iter().map(|m| i64::from(m[3])).max().unwrap_or(0));
    let mut descent = prop_int("FONT_DESCENT")
        .unwrap_or_else(|| metrics.iter().map(|m| i64::from(m[4])).max().unwrap_or(0));
    let accel = if tables.contains_key(&256) { 256 } else { 2 };
    if tables.contains_key(&accel) {
        let (_, mut a) = table(&tables, accel)?;
        a.take(8)?;
        ascent = i64::from(a.u32()? as i32);
        descent = i64::from(a.u32()? as i32);
    }
    let ppem = prop_int("PIXEL_SIZE").unwrap_or(ascent + descent);
    let ppem = u16::try_from(ppem).map_err(|_| error("invalid PCF pixel size"))?;
    let unicode = matches!(
        properties.get("CHARSET_REGISTRY"),
        None | Some(BdfValue::Str(_))
    ) && match properties.get("CHARSET_REGISTRY") {
        Some(BdfValue::Str(s)) => {
            s.eq_ignore_ascii_case("ISO10646")
                || (s.eq_ignore_ascii_case("ISO8859")
                    && matches!(properties.get("CHARSET_ENCODING"),Some(BdfValue::Str(v)) if v=="1"))
        }
        _ => true,
    };
    let family = match properties.get("FAMILY_NAME") {
        Some(BdfValue::Str(s)) => s.clone(),
        _ => "PCF Bitmap".into(),
    };
    let mut font = Font::new_empty(&family, 1000);
    let mut strike = BitmapStrike {
        ppem,
        depth: 1,
        ascent: i16::try_from(ascent).map_err(|_| error("PCF ascent overflow"))?,
        descent: i16::try_from(descent).map_err(|_| error("PCF descent overflow"))?,
        properties,
        ..Default::default()
    };
    if !(1..=4096).contains(&ppem) {
        return Err(error("PCF pixel size exceeds limit"));
    }
    let mut codes = vec![Vec::new(); count];
    if tables.contains_key(&32) {
        let (_, mut e) = table(&tables, 32)?;
        let lo = e.u16()?;
        let hi = e.u16()?;
        let first = e.u16()?;
        let last = e.u16()?;
        let default = e.u16()?;
        strike
            .properties
            .insert("DEFAULT_CHAR".into(), BdfValue::Int(i64::from(default)));
        if lo > hi || first > last || hi > 255 || last > 255 {
            return Err(error("invalid PCF encoding range"));
        }
        for high in first..=last {
            for low in lo..=hi {
                let gid = e.u16()?;
                if gid != 0xffff {
                    let target = codes
                        .get_mut(usize::from(gid))
                        .ok_or_else(|| error("PCF encoding glyph out of range"))?;
                    target.push((u32::from(high) << 8) | u32::from(low));
                }
            }
        }
    }
    let names = if tables.contains_key(&128) {
        let (_, mut n) = table(&tables, 128)?;
        if n.count(65535)? != count {
            return Err(error("PCF glyph-name count mismatch"));
        }
        let offsets = (0..count).map(|_| n.u32()).collect::<Result<Vec<_>, _>>()?;
        let size = n.count(super::MAX_INPUT as usize)?;
        let strings = n.take(size)?;
        offsets
            .into_iter()
            .map(|o| cstr(strings, o as usize))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        (0..count).map(|i| format!("glyph{i:05}")).collect()
    };
    let widths = if tables.contains_key(&64) {
        let (_, mut s) = table(&tables, 64)?;
        if s.count(65535)? != count {
            return Err(error("PCF scalable-width count mismatch"));
        }
        Some(
            (0..count)
                .map(|_| s.u32().map(|v| v as i32))
                .collect::<Result<Vec<_>, _>>()?,
        )
    } else {
        None
    };
    let (format, mut b) = table(&tables, 8)?;
    if format & !0x3f != 0 || b.count(65535)? != count {
        return Err(error("PCF bitmap format/count mismatch"));
    }
    let offsets = (0..count).map(|_| b.u32()).collect::<Result<Vec<_>, _>>()?;
    let sizes = [b.u32()?, b.u32()?, b.u32()?, b.u32()?];
    let size = sizes[(format & 3) as usize] as usize;
    if size > fc_model::MAX_FONT_BITMAP_BYTES {
        return Err(error("PCF pixel buffer limit"));
    }
    let pixels = b.take(size)?;
    let pad = 1usize << (format & 3);
    let unit = 1usize << ((format >> 4) & 3);
    let mut total = 0usize;
    for index in 0..count {
        let m = metrics[index];
        let width = i32::from(m[1]) - i32::from(m[0]);
        let height = i32::from(m[3]) + i32::from(m[4]);
        let mut bitmap = checked(Bitmap::new(
            u16::try_from(width).map_err(|_| error("invalid PCF glyph width"))?,
            u16::try_from(height).map_err(|_| error("invalid PCF glyph height"))?,
            1,
        ))?;
        bitmap.origin_x = m[0];
        bitmap.origin_y = m[4]
            .checked_neg()
            .ok_or_else(|| error("PCF origin overflow"))?;
        total += bitmap.rows.len();
        if total > fc_model::MAX_FONT_BITMAP_BYTES {
            return Err(error("decoded PCF memory limit"));
        }
        let stride = checked(bitmap.stride())?;
        let padded = stride.div_ceil(pad) * pad;
        let start = offsets[index] as usize;
        let len = padded * usize::from(bitmap.height);
        let data = pixels
            .get(
                start
                    ..start
                        .checked_add(len)
                        .ok_or_else(|| error("PCF bitmap offset overflow"))?,
            )
            .ok_or_else(|| error("PCF bitmap outside data"))?;
        let mut normalized = data.to_vec();
        if (format & 4 != 0) != (format & 8 != 0) && unit > 1 {
            if !padded.is_multiple_of(unit) {
                return Err(error("invalid PCF scan-unit padding"));
            }
            for chunk in normalized.chunks_exact_mut(unit) {
                chunk.reverse();
            }
        }
        for y in 0..usize::from(bitmap.height) {
            for x in 0..stride {
                let v = normalized[y * padded + x];
                bitmap.rows[y * stride + x] = if format & 8 != 0 { v } else { v.reverse_bits() };
            }
        }
        if bitmap.width % 8 != 0 && stride > 0 {
            for row in bitmap.rows.chunks_exact_mut(stride) {
                row[stride - 1] &= 0xff << (8 - bitmap.width % 8);
            }
        }
        let glyph_codes = if unicode {
            if codes[index].iter().any(|c| char::from_u32(*c).is_none()) {
                return Err(error("invalid Unicode PCF encoding"));
            }
            codes[index].clone()
        } else {
            Vec::new()
        };
        add_glyph(
            &mut font,
            &mut strike,
            &names[index],
            glyph_codes,
            BitmapGlyph {
                bitmap,
                advance: m[2],
                ..Default::default()
            },
        )?;
        if let Some(g) = font.glyph_mut(&names[index]) {
            g.user_data = serde_json::json!({"org.fontcarver.pcf":{"encodings":codes[index],"attributes":m[5] as u16}});
            if let Some(widths) = &widths {
                if let Some(layer) = g.layers.first_mut() {
                    layer.width = f64::from(widths[index]);
                }
            }
        }
    }
    font.bitmap_strikes.push(strike);
    checked(font.validate_bitmaps())?;
    Ok(font)
}

fn metric_bytes(out: &mut Vec<u8>, m: &[i16; 6]) {
    for v in m {
        u16le(out, *v as u16);
    }
}
fn string_offset(out: &mut Vec<u8>, text: &str) -> Result<u32, Error> {
    if text.contains('\0') {
        return Err(error("NUL in PCF string"));
    }
    let offset = u32::try_from(out.len()).map_err(|_| error("PCF strings too large"))?;
    out.extend(text.as_bytes());
    out.push(0);
    Ok(offset)
}

/// Write byte-aligned monochrome PCF, retaining names, properties and metrics.
pub fn encode(font: &Font, strike: &BitmapStrike) -> Result<Vec<u8>, Error> {
    checked(strike.validate())?;
    if strike.depth != 1 {
        return Err(error("PCF requires monochrome pixels"));
    }
    let entries = strike
        .glyphs
        .keys()
        .filter(|name| font.glyph(name).is_some_and(|g| g.export))
        .map(|n| Ok((n, checked(strike.flattened(n))?)))
        .collect::<Result<Vec<_>, Error>>()?;
    if entries.is_empty() {
        return Err(error("PCF requires at least one glyph"));
    }
    let count = entries.len() as u32;
    let mut props = strike.properties.clone();
    for (key, value) in [
        ("PIXEL_SIZE", strike.ppem as i64),
        ("FONT_ASCENT", strike.ascent as i64),
        ("FONT_DESCENT", strike.descent as i64),
    ] {
        props.insert(key.into(), BdfValue::Int(value));
    }
    props
        .entry("FAMILY_NAME".into())
        .or_insert_with(|| BdfValue::Str(font.info.family_name.clone()));
    props
        .entry("CHARSET_REGISTRY".into())
        .or_insert_with(|| BdfValue::Str("ISO10646".into()));
    props
        .entry("CHARSET_ENCODING".into())
        .or_insert_with(|| BdfValue::Str("1".into()));
    let unicode = matches!(props.get("CHARSET_REGISTRY"),Some(BdfValue::Str(s)) if s.eq_ignore_ascii_case("ISO10646") || s.eq_ignore_ascii_case("ISO8859"));
    let mut properties = vec![0; 4];
    u32le(&mut properties, props.len() as u32);
    let mut strings = Vec::new();
    for (name, value) in &props {
        u32le(&mut properties, string_offset(&mut strings, name)?);
        match value {
            BdfValue::Str(s) => {
                properties.push(1);
                u32le(&mut properties, string_offset(&mut strings, s)?);
            }
            BdfValue::Int(v) => {
                properties.push(0);
                u32le(
                    &mut properties,
                    i32::try_from(*v).map_err(|_| error("PCF property integer overflow"))? as u32,
                );
            }
        }
    }
    properties.resize(properties.len().next_multiple_of(4), 0);
    u32le(&mut properties, strings.len() as u32);
    properties.extend(strings);
    let mut metrics = vec![0; 4];
    u32le(&mut metrics, count);
    let mut all_metrics = Vec::new();
    let mut bitmaps = Vec::new();
    u32le(&mut bitmaps, 8);
    u32le(&mut bitmaps, count);
    let mut pixels = Vec::new();
    let mut sizes = [0u32; 4];
    let mut names = vec![0; 4];
    u32le(&mut names, count);
    let mut name_strings = Vec::new();
    let mut widths = vec![0; 4];
    u32le(&mut widths, count);
    let mut encoding = BTreeMap::new();
    for (index, (name, g)) in entries.iter().enumerate() {
        if g.vertical_advance.is_some() || g.vertical_origin.is_some() {
            return Err(error("PCF does not represent vertical advances"));
        }
        let b = &g.bitmap;
        let number = |v: i32| i16::try_from(v).map_err(|_| error("PCF glyph metric overflow"));
        let source = font
            .glyph(name)
            .ok_or_else(|| error("missing PCF glyph owner"))?;
        let attributes = source
            .user_data
            .pointer("/org.fontcarver.pcf/attributes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16;
        let m = [
            b.origin_x,
            number(i32::from(b.origin_x) + i32::from(b.width))?,
            g.advance,
            number(i32::from(b.origin_y) + i32::from(b.height))?,
            number(-i32::from(b.origin_y))?,
            attributes as i16,
        ];
        metric_bytes(&mut metrics, &m);
        all_metrics.push(m);
        u32le(&mut bitmaps, pixels.len() as u32);
        pixels.extend_from_slice(&b.rows);
        for (power, size) in sizes.iter_mut().enumerate() {
            *size += (usize::from(b.width).div_ceil(8).div_ceil(1 << power)
                * (1 << power)
                * usize::from(b.height)) as u32;
        }
        u32le(&mut names, string_offset(&mut name_strings, name)?);
        let width = source.layers.first().map_or(
            f64::from(g.advance) * 1000.0 / f64::from(strike.ppem),
            |l| l.width * 1000.0 / f64::from(font.info.units_per_em),
        );
        if !width.is_finite() || width < i32::MIN as f64 || width > i32::MAX as f64 {
            return Err(error("PCF scalable width overflow"));
        }
        u32le(&mut widths, width.round() as i32 as u32);
        let codes = if unicode {
            source.unicodes.clone()
        } else {
            source
                .user_data
                .pointer("/org.fontcarver.pcf/encodings")
                .and_then(|v| v.as_array())
                .map(|v| {
                    v.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u32))
                        .collect()
                })
                .unwrap_or_default()
        };
        for code in codes {
            if code > 65535 || encoding.insert(code, index as u16).is_some() {
                return Err(error("PCF requires unique 16-bit character encodings"));
            }
        }
    }
    for size in sizes {
        u32le(&mut bitmaps, size);
    }
    bitmaps.extend(pixels);
    u32le(&mut names, name_strings.len() as u32);
    names.extend(name_strings);
    let lo = encoding.keys().map(|v| v & 255).min().unwrap_or(0) as u16;
    let hi = encoding.keys().map(|v| v & 255).max().unwrap_or(0) as u16;
    let first = encoding.keys().map(|v| v >> 8).min().unwrap_or(0) as u16;
    let last = encoding.keys().map(|v| v >> 8).max().unwrap_or(0) as u16;
    let mut enc = vec![0; 4];
    for n in [lo, hi, first, last, 0xffff] {
        u16le(&mut enc, n);
    }
    for a in first..=last {
        for b in lo..=hi {
            u16le(
                &mut enc,
                *encoding
                    .get(&((u32::from(a) << 8) | u32::from(b)))
                    .unwrap_or(&0xffff),
            );
        }
    }
    let mut accel = vec![0; 12];
    u32le(&mut accel, strike.ascent as u32);
    u32le(&mut accel, strike.descent as u32);
    u32le(&mut accel, 0);
    let mut min = [i16::MAX; 6];
    let mut max = [i16::MIN; 6];
    for m in all_metrics {
        for i in 0..6 {
            min[i] = min[i].min(m[i]);
            max[i] = max[i].max(m[i]);
        }
    }
    metric_bytes(&mut accel, &min);
    metric_bytes(&mut accel, &max);
    let tables = [
        (1, 0, properties),
        (2, 0, accel),
        (4, 0, metrics),
        (8, 8, bitmaps),
        (32, 0, enc),
        (64, 0, widths),
        (128, 0, names),
    ];
    let mut out = b"\x01fcp".to_vec();
    u32le(&mut out, tables.len() as u32);
    let mut offset = 8 + tables.len() * 16;
    for (tag, format, data) in &tables {
        u32le(&mut out, *tag);
        u32le(&mut out, *format);
        u32le(&mut out, data.len() as u32);
        u32le(&mut out, offset as u32);
        offset += (data.len() + 3) & !3;
    }
    for (_, _, data) in tables {
        out.extend(data);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    Ok(out)
}
