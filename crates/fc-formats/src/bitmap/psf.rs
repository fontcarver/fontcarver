//! Linux console PSF1 and PSF2 bitmap files and Unicode alias tables.
use super::{
    add_glyph,
    binary::{u16le, u32le, Reader},
    checked, error,
};
use crate::Error;
use fc_model::{Bitmap, BitmapGlyph, BitmapStrike, Font};

/// Decode either PSF header, with bounds-checked Unicode tables.
pub fn decode(bytes: &[u8]) -> Result<Font, Error> {
    let mut r = Reader::new(bytes);
    let old = bytes.starts_with(&[0x36, 0x04]);
    let (count, width, height, size, unicode) = if old {
        r.take(2)?;
        let mode = r.u8()?;
        let height = u16::from(r.u8()?);
        if mode & !7 != 0 {
            return Err(error("unsupported PSF1 flags"));
        }
        (
            if mode & 1 != 0 { 512 } else { 256 },
            8,
            height,
            usize::from(height),
            mode & 6 != 0,
        )
    } else {
        if r.u32()? != 0x864ab572 || r.u32()? != 0 {
            return Err(error("invalid PSF2 header/version"));
        }
        let header = r.count(bytes.len())?;
        let flags = r.u32()?;
        let count = r.count(65535)?;
        let size = r.count(fc_model::MAX_BITMAP_BYTES)?;
        let height = r.count(4096)? as u16;
        let width = r.count(4096)? as u16;
        if header < 32 || flags & !1 != 0 {
            return Err(error("invalid PSF2 header size/flags"));
        }
        r.pos = header;
        (count, width, height, size, flags & 1 != 0)
    };
    if count == 0 || height == 0 || width == 0 {
        return Err(error("empty PSF dimensions or glyph set"));
    }
    let expected = usize::from(width).div_ceil(8) * usize::from(height);
    if size != expected || count * size > fc_model::MAX_FONT_BITMAP_BYTES {
        return Err(error("invalid PSF glyph byte count"));
    }
    let pixels = r.take(count * size)?;
    let mut codes = vec![Vec::<u32>::new(); count];
    if unicode {
        for glyph_codes in &mut codes {
            if old {
                loop {
                    let code = r.u16()?;
                    if code == 0xffff {
                        break;
                    }
                    if code == 0xfffe {
                        return Err(error(
                            "PSF Unicode sequences are not editable as scalar aliases",
                        ));
                    }
                    if char::from_u32(u32::from(code)).is_none() {
                        return Err(error("invalid PSF Unicode scalar"));
                    }
                    glyph_codes.push(u32::from(code));
                    if glyph_codes.len() > 1024 {
                        return Err(error("PSF Unicode alias limit"));
                    }
                }
            } else {
                let tail = r
                    .data
                    .get(r.pos..)
                    .ok_or_else(|| error("missing PSF Unicode table"))?;
                let end = tail
                    .iter()
                    .position(|b| *b == 0xff)
                    .ok_or_else(|| error("truncated PSF Unicode table"))?;
                if tail[..end].contains(&0xfe) {
                    return Err(error(
                        "PSF Unicode sequences are not editable as scalar aliases",
                    ));
                }
                let text =
                    std::str::from_utf8(&tail[..end]).map_err(|_| error("invalid PSF UTF-8"))?;
                glyph_codes.extend(text.chars().map(u32::from));
                if glyph_codes.len() > 1024 {
                    return Err(error("PSF Unicode alias limit"));
                }
                r.take(end + 1)?;
            }
        }
    }
    if r.pos != bytes.len() {
        return Err(error("PSF trailing data"));
    }
    let mut font = Font::new_empty("Console", 1000);
    let mut strike = BitmapStrike {
        ppem: height,
        depth: 1,
        ascent: height as i16,
        descent: 0,
        ..Default::default()
    };
    for (index, codes) in codes.into_iter().enumerate() {
        let mut bitmap = checked(Bitmap::new(width, height, 1))?;
        bitmap
            .rows
            .copy_from_slice(&pixels[index * size..(index + 1) * size]);
        // PSF permits arbitrary padding bits; canonicalize unused bits.
        if width % 8 != 0 {
            let stride = usize::from(width).div_ceil(8);
            for row in bitmap.rows.chunks_exact_mut(stride) {
                row[stride - 1] &= 0xff << (8 - width % 8);
            }
        }
        let name = format!("glyph{index:05}");
        add_glyph(
            &mut font,
            &mut strike,
            &name,
            codes,
            BitmapGlyph {
                bitmap,
                advance: width as i16,
                ..Default::default()
            },
        )?;
    }
    font.bitmap_strikes.push(strike);
    checked(font.validate_bitmaps())?;
    Ok(font)
}

/// Encode a fixed-cell strike as PSF2, or PSF1 for 256/512 eight-pixel glyphs.
pub fn encode(font: &Font, strike: &BitmapStrike, psf1: bool) -> Result<Vec<u8>, Error> {
    checked(strike.validate())?;
    if strike.depth != 1 {
        return Err(error("PSF requires a monochrome strike"));
    }
    let entries = strike
        .glyphs
        .keys()
        .filter(|name| font.glyph(name).is_some_and(|g| g.export))
        .map(|name| Ok((name, checked(strike.flattened(name))?)))
        .collect::<Result<Vec<_>, Error>>()?;
    let width = entries.first().map_or(0, |(_, g)| g.advance);
    let height = i32::from(strike.ascent) + i32::from(strike.descent);
    if width <= 0 || height <= 0 || height > 4096 || entries.is_empty() {
        return Err(error("PSF requires nonempty fixed cells"));
    }
    let width = width as u16;
    let height = height as u16;
    if psf1 && (width != 8 || height > 255 || !matches!(entries.len(), 256 | 512)) {
        return Err(error(
            "PSF1 needs 256 or 512 glyphs, width 8 and height 1–255",
        ));
    }
    let size = usize::from(width).div_ceil(8) * usize::from(height);
    if size * entries.len() > fc_model::MAX_FONT_BITMAP_BYTES {
        return Err(error("PSF output memory limit"));
    }
    let mut out = Vec::new();
    if psf1 {
        out.extend([0x36, 0x04, 2 | u8::from(entries.len() == 512), height as u8]);
    } else {
        for n in [
            0x864ab572,
            0,
            32,
            1,
            entries.len() as u32,
            size as u32,
            u32::from(height),
            u32::from(width),
        ] {
            u32le(&mut out, n);
        }
    }
    for (_, glyph) in &entries {
        if glyph.advance != width as i16
            || glyph.vertical_advance.is_some()
            || glyph.vertical_origin.is_some()
        {
            return Err(error(
                "PSF requires equal horizontal advances and no vertical advances",
            ));
        }
        let b = &glyph.bitmap;
        let x = i32::from(b.origin_x);
        let y = i32::from(strike.ascent) - i32::from(b.origin_y) - i32::from(b.height);
        if x < 0
            || y < 0
            || x + i32::from(b.width) > i32::from(width)
            || y + i32::from(b.height) > i32::from(height)
        {
            return Err(error("bitmap does not fit the PSF cell without clipping"));
        }
        let mut cell = checked(Bitmap::new(width, height, 1))?;
        for row in 0..b.height {
            for col in 0..b.width {
                checked(cell.set_pixel(x as u16 + col, y as u16 + row, b.pixel(col, row)))?;
            }
        }
        out.extend(cell.rows);
    }
    for (name, _) in &entries {
        let glyph = font
            .glyph(name)
            .ok_or_else(|| error("missing PSF owning glyph"))?;
        for code in &glyph.unicodes {
            let c = char::from_u32(*code).ok_or_else(|| error("invalid Unicode scalar"))?;
            if psf1 {
                if *code >= 0xfffe {
                    return Err(error("PSF1 cannot represent this Unicode alias"));
                }
                u16le(&mut out, *code as u16);
            } else {
                let mut buf = [0; 4];
                out.extend(c.encode_utf8(&mut buf).as_bytes());
            }
        }
        if psf1 {
            u16le(&mut out, 0xffff);
        } else {
            out.push(0xff);
        }
    }
    Ok(out)
}
