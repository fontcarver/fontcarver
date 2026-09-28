//! Bounded OpenType SVG table codec (version 0, plain UTF-8 or gzip documents).
use crate::Error;
use fc_model::{
    svg::{MAX_SVG_DOCUMENT_BYTES, MAX_SVG_TABLE_BYTES},
    GlyphName, SvgDocument, SvgTable,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;

fn error(message: impl std::fmt::Display) -> Error {
    Error::parse("SVG ", message)
}
fn bytes(data: &[u8], offset: usize, len: usize) -> Result<&[u8], Error> {
    data.get(
        offset
            ..offset
                .checked_add(len)
                .ok_or_else(|| error("offset overflow"))?,
    )
    .ok_or_else(|| error("truncated table"))
}
fn u16_at(data: &[u8], offset: usize) -> Result<u16, Error> {
    let b = bytes(data, offset, 2)?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}
fn u32_at(data: &[u8], offset: usize) -> Result<u32, Error> {
    let b = bytes(data, offset, 4)?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Decode SVG records, deduplicating shared ranges before bounded decompression.
pub fn read_table(data: &[u8], order: &[GlyphName]) -> Result<SvgTable, Error> {
    if u16_at(data, 0)? != 0 || u32_at(data, 6)? != 0 {
        return Err(error("unsupported version or nonzero reserved field"));
    }
    let offset = u32_at(data, 2)? as usize;
    if offset < 10 || data.len() > MAX_SVG_TABLE_BYTES {
        return Err(error("invalid table size or list offset"));
    }
    let list = bytes(data, offset, data.len().saturating_sub(offset))?;
    let count = usize::from(u16_at(list, 0)?);
    let header = 2 + count * 12;
    bytes(list, 0, header)?;
    let mut table = SvgTable::default();
    let mut shared = HashMap::<(usize, usize), usize>::new();
    let mut previous = None;
    let mut total = 0usize;
    for index in 0..count {
        let record = 2 + index * 12;
        let start = usize::from(u16_at(list, record)?);
        let end = usize::from(u16_at(list, record + 2)?);
        if start > end || end >= order.len() || previous.is_some_and(|last| start <= last) {
            return Err(error("invalid, overlapping or unsorted glyph range"));
        }
        previous = Some(end);
        let offset = u32_at(list, record + 4)? as usize;
        let len = u32_at(list, record + 8)? as usize;
        if offset < header || len == 0 {
            return Err(error("document overlaps index or is empty"));
        }
        let document_index = if let Some(index) = shared.get(&(offset, len)) {
            *index
        } else {
            let raw = bytes(list, offset, len)?;
            let decoded = if raw.starts_with(&[0x1f, 0x8b]) {
                let mut decoded = Vec::new();
                flate2::read::MultiGzDecoder::new(raw)
                    .take(MAX_SVG_DOCUMENT_BYTES as u64 + 1)
                    .read_to_end(&mut decoded)
                    .map_err(error)?;
                decoded
            } else {
                raw.to_vec()
            };
            total = total.saturating_add(decoded.len());
            if decoded.len() > MAX_SVG_DOCUMENT_BYTES || total > MAX_SVG_TABLE_BYTES {
                return Err(error("uncompressed SVG budget exceeded"));
            }
            let source = String::from_utf8(decoded).map_err(error)?;
            let index = table.documents.len();
            table.documents.push(SvgDocument {
                source,
                glyphs: BTreeMap::new(),
            });
            shared.insert((offset, len), index);
            index
        };
        for (gid, name) in order.iter().enumerate().take(end + 1).skip(start) {
            table.documents[document_index]
                .glyphs
                .insert(name.clone(), format!("glyph{gid}"));
        }
    }
    table.validate(order).map_err(error)?;
    Ok(table)
}

/// Encode SVG for actual exported glyph IDs, rewriting IDs and local references.
/// Non-exported bindings are omitted by the caller. Output expansion is bounded.
pub fn write_table(table: &SvgTable, order: &[GlyphName]) -> Result<Vec<u8>, Error> {
    table.validate(order).map_err(error)?;
    let gids: HashMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(i, name)| (name, i as u16))
        .collect();
    let mut records = Vec::new();
    let mut payloads = Vec::new();
    let mut total = 0usize;
    for document in &table.documents {
        let ids = fc_model::svg::validate_xml(&document.source).map_err(error)?;
        // Most documents have distinct targets: encode them once and share the
        // bytes across discontinuous ranges. Duplicate bindings get a separate
        // document because SVG IDs must remain unique.
        let mut groups: Vec<BTreeMap<&str, u16>> = Vec::new();
        let mut counts = HashMap::<&str, usize>::new();
        for (name, target) in &document.glyphs {
            let gid = gids[name];
            let index = counts.entry(target).or_default();
            if *index == groups.len() {
                groups.push(BTreeMap::new());
            }
            groups[*index].insert(target, gid);
            *index += 1;
        }
        for group in groups {
            let mut mapping: BTreeMap<_, _> = ids
                .iter()
                .enumerate()
                .map(|(i, id)| (id.clone(), format!("fcsvg{i}")))
                .collect();
            for (target, gid) in &group {
                mapping.insert((*target).to_owned(), format!("glyph{gid}"));
            }
            let source = fc_model::svg::rewrite_ids(&document.source, &mapping).map_err(error)?;
            total = total.saturating_add(source.len());
            if total > MAX_SVG_TABLE_BYTES {
                return Err(error("exported SVG budget exceeded"));
            }
            let index = payloads.len();
            payloads.push(source);
            for gid in group.values() {
                records.push((*gid, index));
            }
        }
    }
    records.sort_unstable();
    let header = 2 + records.len() * 12;
    let mut out = Vec::with_capacity(10 + header + total);
    out.extend_from_slice(&[0, 0, 0, 0, 0, 10, 0, 0, 0, 0]);
    out.extend_from_slice(&(records.len() as u16).to_be_bytes());
    let mut offsets = Vec::new();
    let mut offset = header;
    for source in &payloads {
        offsets.push(offset);
        offset += source.len();
    }
    if offset + 10 > MAX_SVG_TABLE_BYTES {
        return Err(error("exported SVG table budget exceeded"));
    }
    let mut seen = HashSet::new();
    for (gid, index) in records {
        if !seen.insert(gid) {
            return Err(error("duplicate exported SVG glyph"));
        }
        out.extend_from_slice(&gid.to_be_bytes());
        out.extend_from_slice(&gid.to_be_bytes());
        out.extend_from_slice(&(offsets[index] as u32).to_be_bytes());
        out.extend_from_slice(&(payloads[index].len() as u32).to_be_bytes());
    }
    for source in payloads {
        out.extend_from_slice(source.as_bytes());
    }
    Ok(out)
}
