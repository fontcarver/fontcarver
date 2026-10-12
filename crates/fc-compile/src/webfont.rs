//! WOFF 1 and WOFF 2 encoders.

use std::io::Write;

use flate2::{write::ZlibEncoder, Compression};
use read_fonts::{FontRef, TableProvider};

use crate::Error;

mod raw_woff2;

const WOFF_HEADER_LEN: usize = 44;
const WOFF_TABLE_RECORD_LEN: usize = 20;

#[derive(Debug)]
struct WoffTable {
    tag: [u8; 4],
    checksum: u32,
    original_len: u32,
    data: Vec<u8>,
}

/// Encodes one sfnt as WOFF 1 using independent zlib-compressed tables.
pub fn wrap_woff1(sfnt: &[u8]) -> Result<Vec<u8>, Error> {
    let flavor: [u8; 4] = sfnt
        .get(..4)
        .ok_or_else(|| Error::parse("sfnt header", "fewer than four bytes"))?
        .try_into()
        .map_err(|_| Error::parse("sfnt header", "invalid flavor"))?;
    let face = FontRef::new(sfnt).map_err(|error| Error::parse("sfnt for WOFF", error))?;
    let mut tables = Vec::new();
    for record in face.table_directory().table_records() {
        let original = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("sfnt table", record.tag()))?
            .as_bytes();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(original)?;
        let compressed = encoder.finish()?;
        let data = if compressed.len() < original.len() {
            compressed
        } else {
            original.to_vec()
        };
        tables.push(WoffTable {
            tag: record.tag().to_be_bytes(),
            checksum: record.checksum(),
            original_len: record.length(),
            data,
        });
    }
    tables.sort_unstable_by_key(|table| table.tag);

    let count = u16::try_from(tables.len()).map_err(|_| Error::Limit("WOFF table count"))?;
    let directory_len = WOFF_TABLE_RECORD_LEN
        .checked_mul(tables.len())
        .and_then(|length| length.checked_add(WOFF_HEADER_LEN))
        .ok_or(Error::Limit("WOFF directory"))?;
    let total_len = tables.iter().try_fold(directory_len, |length, table| {
        length
            .checked_add(padded_len(table.data.len()))
            .ok_or(Error::Limit("WOFF length"))
    })?;
    let total_sfnt_size = tables.iter().try_fold(
        12usize
            .checked_add(
                16usize
                    .checked_mul(tables.len())
                    .ok_or(Error::Limit("sfnt size"))?,
            )
            .ok_or(Error::Limit("sfnt size"))?,
        |length, table| {
            length
                .checked_add(padded_len(table.original_len as usize))
                .ok_or(Error::Limit("sfnt size"))
        },
    )?;
    let mut output = Vec::with_capacity(total_len);
    output.extend_from_slice(b"wOFF");
    output.extend_from_slice(&flavor);
    push_u32(
        &mut output,
        u32::try_from(total_len).map_err(|_| Error::Limit("WOFF length"))?,
    );
    push_u16(&mut output, count);
    push_u16(&mut output, 0);
    push_u32(
        &mut output,
        u32::try_from(total_sfnt_size).map_err(|_| Error::Limit("sfnt size"))?,
    );
    push_u16(&mut output, 0);
    push_u16(&mut output, 0);
    for _ in 0..5 {
        push_u32(&mut output, 0);
    }

    let mut offset = directory_len;
    for table in &tables {
        output.extend_from_slice(&table.tag);
        push_u32(
            &mut output,
            u32::try_from(offset).map_err(|_| Error::Limit("WOFF table offset"))?,
        );
        push_u32(
            &mut output,
            u32::try_from(table.data.len()).map_err(|_| Error::Limit("WOFF table length"))?,
        );
        push_u32(&mut output, table.original_len);
        push_u32(&mut output, table.checksum);
        offset = offset
            .checked_add(padded_len(table.data.len()))
            .ok_or(Error::Limit("WOFF table offset"))?;
    }
    for table in tables {
        output.extend_from_slice(&table.data);
        while output.len() % 4 != 0 {
            output.push(0);
        }
    }
    if output.len() != total_len {
        return Err(Error::write(
            "WOFF",
            "assembled length disagrees with header",
        ));
    }
    Ok(output)
}

/// Encodes one sfnt as WOFF 2 at Brotli quality 8.
///
/// `woofwoof` vendors Google's WOFF2 implementation and therefore requires a
/// C++11 compiler on every build and cross-compilation host.
/// Glyph transforms are disabled when zero-contour programs would be lost.
pub fn wrap_woff2(sfnt: &[u8]) -> Result<Vec<u8>, Error> {
    let face = FontRef::new(sfnt).map_err(|error| Error::parse("sfnt for WOFF2", error))?;
    if has_empty_glyph_programs(&face)? {
        return raw_woff2::encode(sfnt, &face);
    }
    woofwoof::compress(sfnt, Vec::<u8>::new(), 8, true).ok_or(Error::Woff2Compression)
}

fn has_empty_glyph_programs(face: &FontRef<'_>) -> Result<bool, Error> {
    let glyf = match face.glyf() {
        Ok(glyf) => glyf,
        Err(read_fonts::ReadError::TableIsMissing(_)) => return Ok(false),
        Err(error) => return Err(Error::parse("glyf for WOFF2", error)),
    };
    let head = face.head().map_err(|e| Error::parse("head for WOFF2", e))?;
    let loca = face
        .loca(head.index_to_loc_format() == 1)
        .map_err(|e| Error::parse("loca for WOFF2", e))?;
    let count = face
        .maxp()
        .map_err(|e| Error::parse("maxp for WOFF2", e))?
        .num_glyphs();
    for gid in 0..count {
        let start = loca
            .get_raw(usize::from(gid))
            .ok_or_else(|| Error::parse("loca", "missing offset"))? as usize;
        let end = loca
            .get_raw(usize::from(gid) + 1)
            .ok_or_else(|| Error::parse("loca", "missing end offset"))? as usize;
        let raw = glyf
            .offset_data()
            .as_bytes()
            .get(start..end)
            .ok_or_else(|| Error::parse("glyf", "invalid offset range"))?;
        if raw.len() >= 12 && raw[..2] == [0, 0] {
            let length = usize::from(u16::from_be_bytes([raw[10], raw[11]]));
            if 12 + length > raw.len() {
                return Err(Error::parse("glyf", "truncated empty glyph program"));
            }
            if length > 0 {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn padded_len(length: usize) -> usize {
    length.saturating_add(3) & !3
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}
