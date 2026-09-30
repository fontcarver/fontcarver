//! Bounded, table-preserving OpenType collection packing and face extraction.
//!
//! Follows the Microsoft OpenType `otff` collection specification. Faces are
//! independent: glyphs and table contents are never merged or renumbered.

use std::collections::BTreeSet;

use read_fonts::{types::Tag, FontRef};
use write_fonts::FontBuilder;

use crate::{Container, Error};

/// Largest supported standalone face or aggregate collection, in bytes.
pub const MAX_BYTES: usize = 128 * 1024 * 1024;
/// Largest supported number of faces in one output collection.
pub const MAX_FACES: usize = 256;
pub(crate) const MAX_TABLES: usize = 4095;

/// Packs complete standalone sfnt fonts into a TTC version 1 container.
///
/// Both TrueType and CFF/CFF2 faces are supported. Table sharing is deliberately
/// omitted. WOFF and nested collections must be decoded/extracted first. Signed
/// DSIG data is rejected because repacking invalidates its signature.
pub fn encode(faces: &[&[u8]]) -> Result<Vec<u8>, Error> {
    if faces.is_empty() || faces.len() > MAX_FACES {
        return Err(Error::parse("TTC", "expected 1..=256 faces"));
    }
    let header_len = 12 + 4 * faces.len();
    let mut total = header_len;
    // Validate the aggregate before allocating or copying any output.
    for bytes in faces {
        let face = checked_face(bytes, 0, true)?;
        total = total
            .checked_add(12 + face.table_directory().table_records().len() * 16)
            .ok_or_else(|| Error::parse("TTC", "collection size overflow"))?;
        for record in face.table_directory().table_records() {
            total = total
                .checked_add((record.length() as usize + 3) & !3)
                .ok_or_else(|| Error::parse("TTC", "collection size overflow"))?;
        }
        if total > MAX_BYTES {
            return Err(Error::parse("TTC", "collection exceeds 128 MiB"));
        }
    }
    let mut output = vec![0; header_len];
    output[..4].copy_from_slice(b"ttcf");
    put_u32(&mut output, 4, 0x0001_0000);
    put_u32(&mut output, 8, faces.len() as u32);
    for (index, bytes) in faces.iter().enumerate() {
        // Rebuild the directory to normalize offsets, table checksums and padding.
        let mut standalone = extract_face(bytes, 0)?;
        let offset = output.len();
        put_u32(&mut output, 12 + 4 * index, offset as u32);
        let count = u16::from_be_bytes([standalone[4], standalone[5]]) as usize;
        for record in 0..count {
            let pos = 12 + 16 * record;
            let start = u32::from_be_bytes([
                standalone[pos + 8],
                standalone[pos + 9],
                standalone[pos + 10],
                standalone[pos + 11],
            ]) as usize;
            if &standalone[pos..pos + 4] == b"head" {
                // Collection checkSumAdjustment is unused; the table checksum
                // already corresponds to this zeroed field.
                standalone[start + 8..start + 12].fill(0);
            }
            put_u32(&mut standalone, pos + 8, (offset + start) as u32);
        }
        output.extend_from_slice(&standalone);
    }
    Ok(output)
}

/// Rebuilds one face as a standalone sfnt, preserving all complete table data.
///
/// The `head` checksum adjustment is recalculated. This operation accepts sfnt
/// or collection input and rejects malformed ranges and invalidated signatures.
pub fn extract_face(bytes: &[u8], index: u32) -> Result<Vec<u8>, Error> {
    let face = checked_face(bytes, index, false)?;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("sfnt", "table range exceeds input"))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }
    Ok(builder.build())
}

pub(crate) fn checked_face(
    bytes: &[u8],
    index: u32,
    standalone: bool,
) -> Result<FontRef<'_>, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::parse("sfnt", "input exceeds 128 MiB"));
    }
    match crate::sniff(bytes) {
        Some(Container::Sfnt) => {}
        Some(Container::Ttc) if !standalone => {
            let count = read_u32(bytes, 8)? as usize;
            if count == 0 || count > MAX_FACES {
                return Err(Error::parse("TTC", "expected 1..=256 faces"));
            }
            if read_u32(bytes, 4)? == 0x0002_0000 {
                let signature = 12 + 4 * count;
                if read_u32(bytes, signature)? != 0
                    || read_u32(bytes, signature + 4)? != 0
                    || read_u32(bytes, signature + 8)? != 0
                {
                    return Err(Error::parse(
                        "TTC",
                        "collection DSIG signature would be invalidated",
                    ));
                }
            }
        }
        _ => {
            return Err(Error::parse(
                "sfnt",
                "expected raw sfnt input or a selected collection face",
            ))
        }
    }
    let face = FontRef::from_index(bytes, index).map_err(|error| Error::parse("sfnt", error))?;
    let records = face.table_directory().table_records();
    if records.is_empty() || records.len() > MAX_TABLES {
        return Err(Error::parse("sfnt", "expected 1..=4095 tables"));
    }
    let mut tags = BTreeSet::new();
    let mut total = 12 + records.len() * 16;
    for record in records {
        if !tags.insert(record.tag()) {
            return Err(Error::parse("sfnt", "duplicate table tag"));
        }
        if record.tag() == Tag::new(b"DSIG") {
            return Err(Error::parse(
                "sfnt",
                "DSIG signature would be invalidated; remove it before repacking",
            ));
        }
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("sfnt", "table range exceeds input"))?;
        if record.tag() == Tag::new(b"head") && data.len() < 54 {
            return Err(Error::parse("head", "truncated header"));
        }
        total += (data.len() + 3) & !3;
        if total > MAX_BYTES {
            return Err(Error::parse("sfnt", "expanded tables exceed 128 MiB"));
        }
    }
    Ok(face)
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    let value = bytes
        .get(offset..offset + 4)
        .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
        .ok_or_else(|| Error::parse("TTC", "truncated collection header"))?;
    Ok(u32::from_be_bytes(value))
}
