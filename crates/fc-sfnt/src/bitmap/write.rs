use super::{checked, error, png_codec};
use crate::Error;
use fc_model::{BdfValue, BitmapGlyph, BitmapStrike, Font, GlyphName};
use read_fonts::{types::Tag, FontRef, TableProvider};
use write_fonts::FontBuilder;

const TAGS: [&[u8; 4]; 5] = [b"EBLC", b"EBDT", b"CBLC", b"CBDT", b"sbix"];

/// Replace owned embedded bitmap tables using the compiler's actual glyph order.
/// Coverage strikes use EBDT; RGBA strikes use CBDT or their imported sbix flavor.
pub fn replace_tables(sfnt: &[u8], font: &Font, order: &[GlyphName]) -> Result<Vec<u8>, Error> {
    if !font.bitmap_tables_owned && font.bitmap_strikes.is_empty() {
        return Ok(sfnt.to_vec());
    }
    checked(font.validate_bitmaps())?;
    let face = FontRef::new(sfnt).map_err(|e| error(e.to_string()))?;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        let tag = record.tag();
        if TAGS.iter().any(|t| tag == Tag::new(t)) || tag == Tag::new(b"EBSC") {
            continue;
        }
        let data = face
            .table_data(tag)
            .ok_or_else(|| error("missing output table"))?;
        builder.add_raw(tag, data.as_bytes());
    }
    let mut mono = Vec::new();
    let mut color = Vec::new();
    let mut apple = Vec::new();
    for strike in &font.bitmap_strikes {
        if strike.depth != 32 {
            mono.push(strike);
        } else if matches!(strike.properties.get("FONTCARVER_BITMAP_TABLE"), Some(BdfValue::Str(s)) if s == "sbix")
        {
            apple.push(strike);
        } else {
            color.push(strike);
        }
    }
    for (strikes, color) in [(mono, false), (color, true)] {
        if strikes.is_empty() {
            continue;
        }
        let (location, data) = bdt(&strikes, order, color)?;
        builder.add_raw(Tag::new(if color { b"CBLC" } else { b"EBLC" }), location);
        builder.add_raw(Tag::new(if color { b"CBDT" } else { b"EBDT" }), data);
    }
    if !apple.is_empty() {
        builder.add_raw(Tag::new(b"sbix"), sbix(&apple, order, &face)?);
    }
    Ok(builder.build())
}
fn u16be(v: u16, out: &mut Vec<u8>) {
    out.extend(v.to_be_bytes());
}
fn u32be(v: usize, out: &mut Vec<u8>) -> Result<(), Error> {
    out.extend(
        u32::try_from(v)
            .map_err(|_| error("bitmap table offset overflow"))?
            .to_be_bytes(),
    );
    Ok(())
}
fn byte(v: i32) -> Result<u8, Error> {
    u8::try_from(v).map_err(|_| error("embedded bitmap metric exceeds unsigned byte range"))
}
fn signed(v: i32) -> Result<u8, Error> {
    Ok(
        i8::try_from(v).map_err(|_| error("embedded bitmap bearing exceeds signed byte range"))?
            as u8,
    )
}
fn metrics(g: &BitmapGlyph, big: bool) -> Result<Vec<u8>, Error> {
    let b = &g.bitmap;
    let top = i32::from(b.origin_y) + i32::from(b.height);
    let mut result = vec![
        byte(i32::from(b.height))?,
        byte(i32::from(b.width))?,
        signed(i32::from(b.origin_x))?,
        signed(top)?,
        byte(i32::from(g.advance))?,
    ];
    if big {
        let (x, y) = g
            .vertical_origin
            .ok_or_else(|| error("vertical origin is required with embedded vertical advance"))?;
        result.extend([
            signed(i32::from(b.origin_x) - i32::from(x))?,
            signed(top - i32::from(y))?,
            byte(i32::from(g.vertical_advance.ok_or_else(|| {
                error("vertical advance is required with embedded vertical origin")
            })?))?,
        ]);
    }
    Ok(result)
}
fn bdt(
    strikes: &[&BitmapStrike],
    order: &[GlyphName],
    color: bool,
) -> Result<(Vec<u8>, Vec<u8>), Error> {
    if strikes.len() > 256 || order.len() > 65535 {
        return Err(error("embedded bitmap count limit"));
    }
    let version = if color { 0x0003_0000usize } else { 0x0002_0000 };
    let mut location = Vec::new();
    u32be(version, &mut location)?;
    u32be(strikes.len(), &mut location)?;
    location.resize(8 + strikes.len() * 48, 0);
    let mut data = Vec::new();
    u32be(version, &mut data)?;
    for (index, strike) in strikes.iter().enumerate() {
        let ppem = byte(i32::from(strike.ppem))?;
        let mut records = Vec::new();
        for (gid, name) in order.iter().enumerate() {
            if !strike.glyphs.contains_key(name) {
                continue;
            }
            let glyph = checked(strike.flattened(name))?;
            let big = glyph.vertical_advance.is_some() || glyph.vertical_origin.is_some();
            let png = color && glyph.bitmap.width > 0 && glyph.bitmap.height > 0;
            let format = match (png, big) {
                (false, false) => 1,
                (false, true) => 6,
                (true, false) => 17,
                (true, true) => 18,
            };
            let mut image = metrics(&glyph, big)?;
            if png {
                let bytes = png_codec::encode(&glyph.bitmap)?;
                u32be(bytes.len(), &mut image)?;
                image.extend(bytes);
            } else {
                image.extend(&glyph.bitmap.rows);
            }
            let offset = data.len();
            data.extend(&image);
            records.push((gid as u16, format, offset, image.len()));
        }
        let start = location.len();
        let count = records.len();
        for (i, (gid, _, _, _)) in records.iter().enumerate() {
            u16be(*gid, &mut location);
            u16be(*gid, &mut location);
            u32be(count * 8 + i * 16, &mut location)?;
        }
        for (_, format, offset, length) in &records {
            u16be(1, &mut location);
            u16be(*format, &mut location);
            u32be(*offset, &mut location)?;
            u32be(0, &mut location)?;
            u32be(*length, &mut location)?;
        }
        let mut header = Vec::new();
        u32be(start, &mut header)?;
        u32be(location.len() - start, &mut header)?;
        u32be(count, &mut header)?;
        u32be(0, &mut header)?;
        let width = records
            .iter()
            .map(|(gid, _, _, _)| strike.glyphs[&order[*gid as usize]].bitmap.width)
            .max()
            .unwrap_or(0);
        header.extend([
            signed(i32::from(strike.ascent))?,
            signed(-i32::from(strike.descent))?,
            byte(i32::from(width))?,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ]);
        header.extend([0; 12]);
        u16be(records.first().map(|r| r.0).unwrap_or(0), &mut header);
        u16be(records.last().map(|r| r.0).unwrap_or(0), &mut header);
        header.extend([ppem, ppem, strike.depth, 1]);
        location[8 + index * 48..8 + (index + 1) * 48].copy_from_slice(&header);
    }
    Ok((location, data))
}
fn sbix(
    strikes: &[&BitmapStrike],
    order: &[GlyphName],
    face: &FontRef<'_>,
) -> Result<Vec<u8>, Error> {
    let upem = face
        .head()
        .map_err(|e| error(e.to_string()))?
        .units_per_em();
    let hmtx = face.hmtx().map_err(|e| error(e.to_string()))?;
    let mut result = Vec::new();
    u16be(1, &mut result);
    u16be(1, &mut result);
    u32be(strikes.len(), &mut result)?;
    result.resize(8 + strikes.len() * 4, 0);
    for (index, strike) in strikes.iter().enumerate() {
        let start = result.len();
        result[8 + index * 4..12 + index * 4].copy_from_slice(&(start as u32).to_be_bytes());
        let mut bytes = Vec::new();
        u16be(strike.ppem, &mut bytes);
        u16be(72, &mut bytes);
        bytes.resize(4 + (order.len() + 1) * 4, 0);
        for (gid, name) in order.iter().enumerate() {
            let offset = bytes.len() as u32;
            bytes[4 + gid * 4..8 + gid * 4].copy_from_slice(&offset.to_be_bytes());
            if !strike.glyphs.contains_key(name) {
                continue;
            }
            let glyph = checked(strike.flattened(name))?;
            let advance = hmtx
                .advance(read_fonts::types::GlyphId::new(gid as u32))
                .ok_or_else(|| error("missing sbix advance"))?;
            if glyph.vertical_advance.is_some()
                || glyph.vertical_origin.is_some()
                || (f64::from(advance) * f64::from(strike.ppem) / f64::from(upem)).round()
                    != f64::from(glyph.advance)
            {
                return Err(error("sbix advances must match scaled outline metrics; use CBDT for independent bitmap metrics"));
            }
            if glyph.bitmap.width == 0 || glyph.bitmap.height == 0 {
                continue;
            }
            bytes.extend(glyph.bitmap.origin_x.to_be_bytes());
            bytes.extend(glyph.bitmap.origin_y.to_be_bytes());
            bytes.extend(b"png ");
            bytes.extend(png_codec::encode(&glyph.bitmap)?);
        }
        let end = bytes.len() as u32;
        bytes[4 + order.len() * 4..8 + order.len() * 4].copy_from_slice(&end.to_be_bytes());
        result.extend(bytes);
    }
    Ok(result)
}
