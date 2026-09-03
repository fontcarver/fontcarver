//! Valid-table fallbacks for sfnts near the 65,535-glyph OpenType limit.

use std::collections::BTreeMap;

use fw_model::Font;
use read_fonts::FontRef;
use write_fonts::FontBuilder;

use crate::Error;

const LARGE_GLYPH_COUNT: usize = 65_000;

pub(crate) fn requires_workaround(font: &Font) -> bool {
    font.glyphs.values().filter(|glyph| glyph.export).count() > LARGE_GLYPH_COUNT
}

pub(crate) fn patch_tables(compiled: &[u8], font: &Font) -> Result<Vec<u8>, Error> {
    let face =
        FontRef::new(compiled).map_err(|error| Error::parse("large compiled sfnt", error))?;
    let cmap = cmap_format_12(font)?;
    let post = post_format_3(&face)?;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("large compiled table", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }
    builder.add_raw(read_fonts::types::Tag::new(b"cmap"), cmap);
    builder.add_raw(read_fonts::types::Tag::new(b"post"), post);
    Ok(builder.build())
}

fn cmap_format_12(font: &Font) -> Result<Vec<u8>, Error> {
    let mut mappings = BTreeMap::new();
    for (gid, glyph) in font
        .glyphs
        .values()
        .filter(|glyph| glyph.export)
        .enumerate()
    {
        let gid = u32::try_from(gid).map_err(|_| Error::Limit("glyph ID"))?;
        for codepoint in &glyph.unicodes {
            if char::from_u32(*codepoint).is_some() {
                mappings.entry(*codepoint).or_insert(gid);
            }
        }
    }
    let mut groups: Vec<(u32, u32, u32)> = Vec::new();
    for (codepoint, gid) in mappings {
        match groups.last_mut() {
            Some((start, end, start_gid))
                if end.checked_add(1) == Some(codepoint)
                    && start_gid.checked_add(codepoint - *start) == Some(gid) =>
            {
                *end = codepoint;
            }
            _ => groups.push((codepoint, codepoint, gid)),
        }
    }
    let subtable_len = 16usize
        .checked_add(
            groups
                .len()
                .checked_mul(12)
                .ok_or(Error::Limit("cmap groups"))?,
        )
        .ok_or(Error::Limit("cmap length"))?;
    let mut bytes = Vec::with_capacity(20 + subtable_len);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 2);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 4);
    push_u32(&mut bytes, 20);
    push_u16(&mut bytes, 3);
    push_u16(&mut bytes, 10);
    push_u32(&mut bytes, 20);
    push_u16(&mut bytes, 12);
    push_u16(&mut bytes, 0);
    push_u32(
        &mut bytes,
        u32::try_from(subtable_len).map_err(|_| Error::Limit("cmap length"))?,
    );
    push_u32(&mut bytes, 0);
    push_u32(
        &mut bytes,
        u32::try_from(groups.len()).map_err(|_| Error::Limit("cmap groups"))?,
    );
    for (start, end, gid) in groups {
        push_u32(&mut bytes, start);
        push_u32(&mut bytes, end);
        push_u32(&mut bytes, gid);
    }
    Ok(bytes)
}

fn post_format_3(face: &FontRef<'_>) -> Result<Vec<u8>, Error> {
    let source = face
        .table_data(read_fonts::types::Tag::new(b"post"))
        .ok_or_else(|| Error::parse("post", "table is missing"))?;
    let header = source
        .as_bytes()
        .get(..32)
        .ok_or_else(|| Error::parse("post", "table is shorter than its header"))?;
    let mut post = header.to_vec();
    post[..4].copy_from_slice(&0x0003_0000_u32.to_be_bytes());
    Ok(post)
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}
