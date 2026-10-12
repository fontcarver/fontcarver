//! WOFF2 null transforms preserve glyph programs that normalization discards.
//! Implemented from https://www.w3.org/TR/WOFF2/#woff20Header and table directory.

use std::io::Cursor;

use read_fonts::{types::Tag, FontRef};

use crate::Error;

// WOFF2 section 4.1 assigns these normative table-directory indices.
const TAGS: [[u8; 4]; 63] = [
    *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"OS/2", *b"post", *b"cvt ",
    *b"fpgm", *b"glyf", *b"loca", *b"prep", *b"CFF ", *b"VORG", *b"EBDT", *b"EBLC", *b"gasp",
    *b"hdmx", *b"kern", *b"LTSH", *b"PCLT", *b"VDMX", *b"vhea", *b"vmtx", *b"BASE", *b"GDEF",
    *b"GPOS", *b"GSUB", *b"EBSC", *b"JSTF", *b"MATH", *b"CBDT", *b"CBLC", *b"COLR", *b"CPAL",
    *b"SVG ", *b"sbix", *b"acnt", *b"avar", *b"bdat", *b"bloc", *b"bsln", *b"cvar", *b"fdsc",
    *b"feat", *b"fmtx", *b"fvar", *b"gvar", *b"hsty", *b"just", *b"lcar", *b"mort", *b"morx",
    *b"opbd", *b"prop", *b"trak", *b"Zapf", *b"Silf", *b"Glat", *b"Gloc", *b"Feat", *b"Sill",
];

pub(super) fn encode(sfnt: &[u8], face: &FontRef<'_>) -> Result<Vec<u8>, Error> {
    const LIMIT: usize = 256 * 1024 * 1024;
    if sfnt.len() > LIMIT {
        return Err(Error::Limit("raw WOFF2 source size"));
    }
    let mut directory = Vec::new();
    let mut raw = Vec::new();
    let mut padded_size = 0;
    let mut count = 0u16;
    for record in face.table_directory().table_records() {
        let tag = record.tag();
        if tag == Tag::new(b"DSIG") {
            continue;
        }
        let data = face
            .table_data(tag)
            .ok_or_else(|| Error::parse("raw WOFF2 table", tag))?
            .as_bytes();
        if raw.len().saturating_add(data.len()) > LIMIT {
            return Err(Error::Limit("raw WOFF2 table bytes"));
        }
        let index = TAGS
            .iter()
            .position(|value| Tag::new(value) == tag)
            .unwrap_or(63);
        let null_transform = if tag == Tag::new(b"glyf") || tag == Tag::new(b"loca") {
            0xc0
        } else {
            0
        };
        directory.push(index as u8 | null_transform);
        if index == 63 {
            directory.extend(tag.to_be_bytes());
        }
        base128(&mut directory, data.len() as u32);
        raw.extend(data);
        padded_size += data.len().next_multiple_of(4);
        count = count
            .checked_add(1)
            .ok_or(Error::Limit("WOFF2 table count"))?;
    }
    let mut compressed = Vec::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: 8,
        lgwin: 22,
        ..Default::default()
    };
    brotli::BrotliCompress(&mut Cursor::new(raw), &mut compressed, &params)?;
    let total = (48 + directory.len() + compressed.len()).next_multiple_of(4);
    let total_sfnt = 12 + usize::from(count) * 16 + padded_size;
    let mut output = Vec::with_capacity(total);
    output.extend(b"wOF2");
    output.extend(&sfnt[..4]);
    output.extend(
        u32::try_from(total)
            .map_err(|_| Error::Limit("WOFF2 file size"))?
            .to_be_bytes(),
    );
    output.extend(count.to_be_bytes());
    output.extend(0u16.to_be_bytes());
    output.extend(
        u32::try_from(total_sfnt)
            .map_err(|_| Error::Limit("WOFF2 sfnt size"))?
            .to_be_bytes(),
    );
    output.extend((compressed.len() as u32).to_be_bytes());
    output.extend([0; 24]); // Version, metadata and private block fields.
    output.extend(directory);
    output.extend(compressed);
    output.resize(total, 0);
    Ok(output)
}

fn base128(output: &mut Vec<u8>, value: u32) {
    let highest = (0..5)
        .rev()
        .find(|index| value >> (index * 7) != 0)
        .unwrap_or(0);
    for index in (0..=highest).rev() {
        output.push(((value >> (index * 7)) & 0x7f) as u8 | if index == 0 { 0 } else { 0x80 });
    }
}
