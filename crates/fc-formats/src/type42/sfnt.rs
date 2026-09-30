use std::collections::BTreeSet;

use read_fonts::{types::GlyphId, FontRef, TableProvider};

use super::error;
use crate::Error;

pub(super) const MAX_SFNT_BYTES: usize = 64 * 1024 * 1024;

pub(super) fn validate(bytes: &[u8]) -> Result<FontRef<'_>, Error> {
    if bytes.len() > MAX_SFNT_BYTES || !matches!(bytes.get(..4), Some(b"\0\x01\0\0" | b"true")) {
        return Err(error(
            "expected a standalone TrueType SFNT of at most 64 MiB",
        ));
    }
    let face = FontRef::new(bytes).map_err(|e| error(e.to_string()))?;
    let records = face.table_directory().table_records();
    if records.len() > 4095 {
        return Err(error("SFNT table count limit exceeded"));
    }
    let directory_end = 12 + records.len() * 16;
    let mut ranges = Vec::with_capacity(records.len());
    let mut tags = BTreeSet::new();
    for record in records {
        let start = record.offset() as usize;
        let end = start
            .checked_add(record.length() as usize)
            .ok_or_else(|| error("SFNT table range overflow"))?;
        if !tags.insert(record.tag())
            || start < directory_end
            || !start.is_multiple_of(4)
            || end > bytes.len()
        {
            return Err(error("invalid, duplicate or truncated SFNT table"));
        }
        if end > start {
            ranges.push(start..end);
        }
    }
    ranges.sort_by_key(|r| r.start);
    if ranges.windows(2).any(|p| p[0].end > p[1].start) {
        return Err(error("overlapping SFNT tables"));
    }
    let head = face.head().map_err(|e| error(e.to_string()))?;
    if head.magic_number() != 0x5f0f3cf5
        || !(16..=16384).contains(&head.units_per_em())
        || !matches!(head.index_to_loc_format(), 0 | 1)
    {
        return Err(error("invalid TrueType head table"));
    }
    let glyph_count = face.maxp().map_err(|e| error(e.to_string()))?.num_glyphs();
    if glyph_count == 0 {
        return Err(error("TrueType font has no glyphs"));
    }
    face.hhea().map_err(|e| error(e.to_string()))?;
    face.hmtx().map_err(|e| error(e.to_string()))?;
    let loca = face.loca(None).map_err(|e| error(e.to_string()))?;
    if loca.len() != usize::from(glyph_count) || !loca.all_offsets_are_ascending() {
        return Err(error("invalid TrueType glyph locations"));
    }
    let glyf = face.glyf().map_err(|e| error(e.to_string()))?;
    let glyf_len = records
        .iter()
        .find(|r| r.tag().to_be_bytes() == *b"glyf")
        .map(|r| r.length())
        .ok_or_else(|| error("missing glyf table"))?;
    for gid in 0..=glyph_count {
        let offset = loca
            .get_raw(usize::from(gid))
            .ok_or_else(|| error("missing glyph offset"))?;
        if offset > glyf_len || !offset.is_multiple_of(2) {
            return Err(error("invalid TrueType glyph boundary"));
        }
        if gid < glyph_count {
            loca.get_glyf(GlyphId::from(u32::from(gid)), &glyf)
                .map_err(|e| error(e.to_string()))?;
        }
    }
    Ok(face)
}

pub(super) fn chunk_ends(face: &FontRef<'_>, length: usize) -> Result<Vec<usize>, Error> {
    let records = face.table_directory().table_records();
    let mut boundaries: Vec<usize> = records.iter().map(|r| r.offset() as usize).collect();
    let glyf = records
        .iter()
        .find(|r| r.tag().to_be_bytes() == *b"glyf")
        .ok_or_else(|| error("missing glyf table"))?;
    let loca = face.loca(None).map_err(|e| error(e.to_string()))?;
    for gid in 0..=loca.len() {
        if let Some(offset) = loca.get_raw(gid) {
            boundaries.push(glyf.offset() as usize + offset as usize);
        }
    }
    boundaries.push(length);
    boundaries.sort_unstable();
    boundaries.dedup();
    let mut ends = Vec::new();
    let mut start = 0;
    while start < length {
        let end = boundaries
            .iter()
            .rev()
            .copied()
            .find(|end| *end > start && *end - start <= 65_534)
            .ok_or_else(|| {
                error("a table or glyph exceeds the compatible Type 42 string limit (65534 bytes)")
            })?;
        ends.push(end);
        start = end;
    }
    Ok(ends)
}
