//! Windows 2/3 monochrome raster FNT and NE FON font resources.
use super::{
    add_glyph,
    binary::{cstr, u16le, u32le, Reader},
    checked, error,
};
use crate::Error;
use fc_model::{BdfValue, Bitmap, BitmapGlyph, BitmapStrike, Font};
use std::collections::BTreeMap;

const CP1252: [u32; 32] = [
    0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039,
    0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x2dc,
    0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
];
fn codepoint(code: u8) -> u32 {
    if (128..160).contains(&code) {
        CP1252[usize::from(code - 128)]
    } else {
        u32::from(code)
    }
}
fn byte_code(code: u32) -> Option<u8> {
    if code < 128 || (160..=255).contains(&code) {
        Some(code as u8)
    } else {
        CP1252
            .iter()
            .position(|v| *v == code)
            .map(|i| 128 + i as u8)
    }
}
fn word(data: &[u8], offset: usize) -> Result<u16, Error> {
    let mut r = Reader::new(data);
    r.pos = offset;
    r.u16()
}
fn long(data: &[u8], offset: usize) -> Result<u32, Error> {
    let mut r = Reader::new(data);
    r.pos = offset;
    r.u32()
}
fn put16(data: &mut [u8], offset: usize, v: u16) {
    data[offset..offset + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(data: &mut [u8], offset: usize, v: u32) {
    data[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
}

/// Decode a Windows 2.x/3.x raster FNT. Vector/color/ABC variants fail explicitly.
pub fn decode(bytes: &[u8]) -> Result<Font, Error> {
    let version = word(bytes, 0)?;
    let header = match version {
        0x200 => 118,
        0x300 => 148,
        _ => return Err(error("unsupported FNT version")),
    };
    let size = long(bytes, 2)? as usize;
    if size < header || size > bytes.len() || word(bytes, 66)? & 1 != 0 {
        return Err(error("truncated or vector FNT"));
    }
    let bytes = &bytes[..size];
    if version == 0x300 {
        let flags = long(bytes, 118)?;
        if flags & !0x13 != 0 || word(bytes, 122)? != 0 || word(bytes, 126)? != 0 {
            return Err(error("color or ABC FNT is not supported"));
        }
    }
    let height = word(bytes, 88)?;
    let ascent = word(bytes, 74)?;
    let internal = word(bytes, 76)?;
    let ppem = height
        .checked_sub(internal)
        .filter(|v| *v > 0)
        .ok_or_else(|| error("invalid FNT internal leading"))?;
    if height == 0 || height > 4096 || ascent > height || ppem > 4096 {
        return Err(error("invalid FNT cell metrics"));
    }
    let first = *bytes.get(95).ok_or_else(|| error("truncated FNT header"))?;
    let last = bytes[96];
    let charset = bytes[85];
    if first > last {
        return Err(error("invalid FNT character range"));
    }
    let face = long(bytes, 105)? as usize;
    let family = if face == 0 {
        "Windows Bitmap".into()
    } else {
        cstr(bytes, face)?
    };
    let mut font = Font::new_empty(&family, 1000);
    let mut strike = BitmapStrike {
        ppem,
        depth: 1,
        ascent: ascent as i16,
        descent: (height - ascent) as i16,
        ..Default::default()
    };
    strike
        .properties
        .insert("WINDOWS_CHARSET".into(), BdfValue::Int(i64::from(charset)));
    strike.properties.insert(
        "FNT_INTERNAL_LEADING".into(),
        BdfValue::Int(i64::from(internal)),
    );
    let entry_size = if version == 0x300 { 6 } else { 4 };
    let count = usize::from(last - first) + 1;
    let table_end = header + (count + 1) * entry_size;
    if table_end > bytes.len() {
        return Err(error("truncated FNT character table"));
    }
    for index in 0..count {
        let position = header + index * entry_size;
        let width = word(bytes, position)?;
        let offset = if version == 0x300 {
            long(bytes, position + 2)? as usize
        } else {
            usize::from(word(bytes, position + 2)?)
        };
        let mut bitmap = checked(Bitmap::new(width, height, 1))?;
        bitmap.origin_y = -strike.descent;
        let stride = checked(bitmap.stride())?;
        if offset < table_end {
            return Err(error("FNT bitmap overlaps header/table"));
        }
        let data = bytes
            .get(offset..offset + bitmap.rows.len())
            .ok_or_else(|| error("truncated FNT bitmap"))?;
        for row in 0..usize::from(height) {
            for col in 0..stride {
                bitmap.rows[row * stride + col] = data[col * usize::from(height) + row];
            }
        }
        if width % 8 != 0 {
            for row in bitmap.rows.chunks_exact_mut(stride) {
                row[stride - 1] &= 0xff << (8 - width % 8);
            }
        }
        let code = first + index as u8;
        let name = format!("char{code:03}");
        let codes = if charset == 0 {
            vec![codepoint(code)]
        } else {
            Vec::new()
        };
        add_glyph(
            &mut font,
            &mut strike,
            &name,
            codes,
            BitmapGlyph {
                bitmap,
                advance: i16::try_from(width).map_err(|_| error("FNT advance overflow"))?,
                ..Default::default()
            },
        )?;
        if let Some(g) = font.glyph_mut(&name) {
            g.user_data = serde_json::json!({"org.fontcarver.fnt":{"encoding":code}});
        }
    }
    font.user_data = serde_json::json!({"org.fontcarver.fnt":{"copyright":String::from_utf8_lossy(&bytes[6..66]).trim_end_matches('\0'),"weight":word(bytes,83)?,"italic":bytes[80],"underline":bytes[81],"strikeout":bytes[82]}});
    font.bitmap_strikes.push(strike);
    checked(font.validate_bitmaps())?;
    Ok(font)
}

/// Encode a monochrome strike as a native Windows 3.0 FNT.
pub fn encode(font: &Font, strike: &BitmapStrike) -> Result<Vec<u8>, Error> {
    checked(strike.validate())?;
    if strike.depth != 1 {
        return Err(error("FNT requires monochrome pixels"));
    }
    let charset = match strike.properties.get("WINDOWS_CHARSET") {
        Some(BdfValue::Int(v)) => u8::try_from(*v).map_err(|_| error("invalid FNT charset"))?,
        _ => 0,
    };
    let mut entries = BTreeMap::new();
    for name in strike.glyphs.keys() {
        let g = font
            .glyph(name)
            .ok_or_else(|| error("missing FNT owning glyph"))?;
        if !g.export {
            continue;
        }
        let code = if charset == 0 {
            if g.unicodes.len() != 1 {
                return Err(error("FNT needs exactly one CP1252 character per glyph"));
            }
            byte_code(g.unicodes[0])
                .ok_or_else(|| error("glyph cannot be represented in Windows CP1252 FNT"))?
        } else {
            g.user_data
                .pointer("/org.fontcarver.fnt/encoding")
                .and_then(|v| v.as_u64())
                .and_then(|v| u8::try_from(v).ok())
                .ok_or_else(|| error("FNT glyph lacks its original byte encoding"))?
        };
        if entries
            .insert(code, checked(strike.flattened(name))?)
            .is_some()
        {
            return Err(error("duplicate FNT character encoding"));
        }
    }
    let first = *entries.keys().next().ok_or_else(|| error("empty FNT"))?;
    let last = *entries
        .keys()
        .next_back()
        .ok_or_else(|| error("empty FNT"))?;
    let height = i32::from(strike.ascent) + i32::from(strike.descent);
    if height <= 0 || height > 4096 || i32::from(strike.ppem) > height {
        return Err(error("FNT ppem must fit the strike cell height"));
    }
    let height = height as u16;
    let count = usize::from(last - first) + 1;
    let mut output = vec![0; 148 + (count + 1) * 6];
    put16(&mut output, 0, 0x300);
    put16(&mut output, 68, strike.ppem);
    put16(&mut output, 70, 72);
    put16(&mut output, 72, 72);
    put16(&mut output, 74, strike.ascent as u16);
    put16(&mut output, 76, height - strike.ppem);
    put16(&mut output, 83, 400);
    output[85] = charset;
    put16(&mut output, 88, height);
    output[90] = 1;
    output[95] = first;
    output[96] = last;
    output[98] = if first <= 32 && last >= 32 {
        32 - first
    } else {
        0
    };
    put32(&mut output, 118, 0x12);
    let start = output.len() as u32;
    put32(&mut output, 113, start);
    let mut max_width = 0u16;
    let mut sum_width = 0u32;
    for index in 0..=count {
        let code = first as usize + index;
        let glyph = if index < count {
            entries.get(&(code as u8))
        } else {
            None
        };
        let width = match glyph {
            Some(g) => u16::try_from(g.advance).map_err(|_| error("negative FNT advance"))?,
            None => 0,
        };
        let offset = output.len() as u32;
        put16(&mut output, 148 + index * 6, width);
        put32(&mut output, 150 + index * 6, offset);
        let Some(g) = glyph else {
            continue;
        };
        if g.vertical_advance.is_some() || g.vertical_origin.is_some() {
            return Err(error("FNT cannot represent vertical advances"));
        }
        let b = &g.bitmap;
        let x = i32::from(b.origin_x);
        let y = i32::from(strike.ascent) - i32::from(b.origin_y) - i32::from(b.height);
        if x < 0
            || y < 0
            || x + i32::from(b.width) > i32::from(width)
            || y + i32::from(b.height) > i32::from(height)
        {
            return Err(error("bitmap does not fit FNT cell without clipping"));
        }
        let mut cell = checked(Bitmap::new(width, height, 1))?;
        for row in 0..b.height {
            for col in 0..b.width {
                checked(cell.set_pixel(x as u16 + col, y as u16 + row, b.pixel(col, row)))?;
            }
        }
        let stride = checked(cell.stride())?;
        for col in 0..stride {
            for row in 0..usize::from(height) {
                output.push(cell.rows[row * stride + col]);
            }
        }
        max_width = max_width.max(width);
        sum_width += u32::from(width);
    }
    put16(&mut output, 91, (sum_width / count as u32) as u16);
    put16(&mut output, 93, max_width);
    put16(&mut output, 99, max_width.div_ceil(16) * 2);
    if !font.info.family_name.is_ascii() || font.info.family_name.contains('\0') {
        return Err(error("FNT face names must be ASCII"));
    }
    let face = output.len() as u32;
    put32(&mut output, 105, face);
    output.extend(font.info.family_name.as_bytes());
    output.push(0);
    if let Some(c) = font
        .user_data
        .pointer("/org.fontcarver.fnt/copyright")
        .and_then(|v| v.as_str())
    {
        for (dst, src) in output[6..66].iter_mut().zip(c.bytes()) {
            *dst = src;
        }
    }
    let size = output.len() as u32;
    put32(&mut output, 2, size);
    Ok(output)
}

/// Decode all same-family strikes in an NE FON container.
pub fn decode_fon(bytes: &[u8]) -> Result<Font, Error> {
    if !bytes.starts_with(b"MZ") {
        return Err(error("invalid FON DOS signature"));
    }
    let ne = long(bytes, 60)? as usize;
    if bytes.get(ne..ne + 2) != Some(b"NE".as_slice()) {
        return Err(error("FON requires an NE resource container"));
    }
    let offset = ne + usize::from(word(bytes, ne + 36)?);
    let mut r = Reader::new(bytes);
    r.pos = offset;
    let shift = r.u16()?;
    if shift > 16 {
        return Err(error("FON alignment exceeds limit"));
    }
    let mut fonts = Vec::new();
    let mut resources = 0usize;
    let mut decoded_bytes = 0usize;
    loop {
        let kind = r.u16()?;
        if kind == 0 {
            break;
        }
        let count = r.u16()? as usize;
        resources += count;
        if resources > 4096 {
            return Err(error("FON resource count limit"));
        }
        r.take(4)?;
        for _ in 0..count {
            let offset = usize::from(r.u16()?) << shift;
            let length = usize::from(r.u16()?) << shift;
            r.take(8)?;
            let data = bytes
                .get(offset..offset + length)
                .ok_or_else(|| error("FON resource out of bounds"))?;
            if kind == 0x8008 {
                if fonts.len() >= 256 {
                    return Err(error("FON exceeds 256 strikes"));
                }
                let decoded = decode(data)?;
                decoded_bytes += decoded
                    .bitmap_strikes
                    .iter()
                    .flat_map(|s| s.glyphs.values())
                    .map(|g| g.bitmap.rows.len())
                    .sum::<usize>();
                if decoded_bytes > fc_model::MAX_FONT_BITMAP_BYTES {
                    return Err(error("FON decoded pixel memory limit"));
                }
                fonts.push(decoded);
            }
        }
    }
    let mut iter = fonts.into_iter();
    let mut font = iter
        .next()
        .ok_or_else(|| error("FON contains no raster fonts"))?;
    for other in iter {
        if other.info.family_name != font.info.family_name {
            return Err(error(
                "FON contains multiple families; import separate FNT resources",
            ));
        }
        for glyph in other.glyphs.values() {
            if let Some(existing) = font.glyph(&glyph.name) {
                if existing.unicodes != glyph.unicodes {
                    return Err(error("FON strikes have conflicting encodings"));
                }
            } else {
                let mut g = glyph.clone();
                g.layers.clear();
                checked(font.insert_glyph(g))?;
            }
        }
        font.bitmap_strikes.extend(other.bitmap_strikes);
    }
    font.bitmap_strikes.sort_by_key(|s| (s.ppem, s.depth));
    checked(font.validate_bitmaps())?;
    Ok(font)
}

/// Package every eligible strike into a Windows NE FON with a FONTDIR resource.
pub fn encode_fon(font: &Font) -> Result<Vec<u8>, Error> {
    checked(font.validate_bitmaps())?;
    if font.bitmap_strikes.is_empty() || font.bitmap_strikes.len() > 256 {
        return Err(error("FON requires 1–256 strikes"));
    }
    let fonts = font
        .bitmap_strikes
        .iter()
        .map(|s| encode(font, s))
        .collect::<Result<Vec<_>, _>>()?;
    let mut directory = Vec::new();
    u16le(&mut directory, fonts.len() as u16);
    for (i, fnt) in fonts.iter().enumerate() {
        u16le(&mut directory, i as u16 + 1);
        directory.extend(&fnt[..113]);
        directory.push(0);
        directory.extend(font.info.family_name.as_bytes());
        directory.push(0);
    }
    let mut resources = vec![(0x8007, 1, directory)];
    resources.extend(
        fonts
            .into_iter()
            .enumerate()
            .map(|(i, data)| (0x8008, i as u16 + 1, data)),
    );
    let table_len = 2 + 8 + 12 + 8 + (resources.len() - 1) * 12 + 2;
    let resident = 64 + table_len;
    let data_start = (128 + table_len + 1).next_multiple_of(16);
    let mut out = vec![0; 128];
    out[..2].copy_from_slice(b"MZ");
    put32(&mut out, 60, 64);
    out[64..66].copy_from_slice(b"NE");
    put16(&mut out, 64 + 12, 0x8000);
    put16(&mut out, 64 + 36, 64);
    put16(&mut out, 64 + 38, resident as u16);
    out[64 + 54] = 2;
    put16(&mut out, 64 + 62, 0x300);
    u16le(&mut out, 4);
    let mut offset = data_start;
    for kind in [0x8007, 0x8008] {
        let selected: Vec<_> = resources.iter().filter(|r| r.0 == kind).collect();
        u16le(&mut out, kind);
        u16le(&mut out, selected.len() as u16);
        u32le(&mut out, 0);
        for (_, id, data) in selected {
            let padded = data.len().next_multiple_of(16);
            u16le(
                &mut out,
                u16::try_from(offset / 16).map_err(|_| error("FON resource offset too large"))?,
            );
            u16le(
                &mut out,
                u16::try_from(padded / 16).map_err(|_| error("FON resource too large"))?,
            );
            u16le(&mut out, 0x30);
            u16le(&mut out, 0x8000 | id);
            u32le(&mut out, 0);
            offset += padded;
        }
    }
    u16le(&mut out, 0);
    out.resize(data_start, 0);
    for (_, _, data) in resources {
        out.extend(data);
        out.resize(out.len().next_multiple_of(16), 0);
    }
    Ok(out)
}
