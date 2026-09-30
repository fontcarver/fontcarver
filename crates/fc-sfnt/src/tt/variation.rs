//! Original binary-preservation checks for OpenType fvar, gvar and cvar.
//!
//! Variation deltas are retained, not regenerated or interpreted here. The
//! dependency digest is intentionally conservative: changing any variation table
//! invalidates all variable glyph bindings, including component dependencies.

use read_fonts::{types::Tag, FontRef};
use sha2::{Digest, Sha256};

use super::{invalid, span::word};
use crate::Error;

const TAGS: [&[u8; 4]; 7] = [
    b"fvar", b"avar", b"gvar", b"HVAR", b"VVAR", b"MVAR", b"cvar",
];

pub(super) fn identity(face: &FontRef<'_>, glyph_count: u16) -> Result<Option<[u8; 32]>, Error> {
    if face.table_data(Tag::new(b"VARC")).is_some() {
        return Err(invalid("VARC point bindings are not supported"));
    }
    let Some(fvar) = face.table_data(Tag::new(b"fvar")) else {
        if TAGS[1..]
            .iter()
            .any(|tag| face.table_data(Tag::new(tag)).is_some())
        {
            return Err(invalid("variation tables (including cvar) require fvar"));
        }
        return Ok(None);
    };
    let axes = validate_fvar(fvar.as_bytes())?;
    let gvar = face
        .table_data(Tag::new(b"gvar"))
        .ok_or_else(|| invalid("variable TrueType point bindings require gvar"))?;
    validate_gvar(gvar.as_bytes(), axes, glyph_count)?;
    if let Some(cvar) = face.table_data(Tag::new(b"cvar")) {
        let cvt = face
            .table_data(Tag::new(b"cvt "))
            .ok_or_else(|| invalid("cvar requires an existing CVT"))?;
        let cvt_len = cvt.as_bytes().len();
        if cvt_len == 0 || !cvt_len.is_multiple_of(2) || cvt_len / 2 > 65536 {
            return Err(invalid(
                "cvar requires a nonempty, bounded, even-length CVT",
            ));
        }
        validate_cvar(cvar.as_bytes(), axes, cvt_len / 2)?;
    }
    let mut hash = Sha256::new();
    hash.update(b"fontcarver-tt-variation-context-v1");
    for tag in TAGS {
        hash.update(tag);
        if let Some(data) = face.table_data(Tag::new(tag)) {
            hash.update([1]);
            hash.update((data.as_bytes().len() as u64).to_be_bytes());
            hash.update(data.as_bytes());
        } else {
            hash.update([0]);
        }
    }
    Ok(Some(hash.finalize().into()))
}

pub(super) fn check_cvt(face: &FontRef<'_>, cvt: &[i16]) -> Result<(), Error> {
    if face.table_data(Tag::new(b"cvar")).is_none() {
        return Ok(());
    }
    let original = face
        .table_data(Tag::new(b"cvt "))
        .ok_or_else(|| invalid("cvar requires an existing CVT"))?;
    let bytes = original.as_bytes();
    if bytes.len() != cvt.len() * 2
        || bytes
            .chunks_exact(2)
            .zip(cvt)
            .any(|(stored, value)| stored != value.to_be_bytes())
    {
        return Err(invalid("preserving cvar requires unchanged CVT values and order; variable CVT editing is not supported"));
    }
    Ok(())
}

fn validate_fvar(bytes: &[u8]) -> Result<u16, Error> {
    if word(bytes, 0)? != 1 || word(bytes, 2)? != 0 || word(bytes, 6)? != 2 {
        return Err(invalid("unsupported fvar header"));
    }
    let offset = usize::from(word(bytes, 4)?);
    let count = word(bytes, 8)?;
    let axis_size = usize::from(word(bytes, 10)?);
    let instances = usize::from(word(bytes, 12)?);
    let instance_size = usize::from(word(bytes, 14)?);
    let coordinates_size = usize::from(count) * 4;
    if offset < 16
        || count == 0
        || axis_size != 20
        || ![coordinates_size + 4, coordinates_size + 6].contains(&instance_size)
    {
        return Err(invalid("invalid fvar record sizes"));
    }
    let records = slice(bytes, offset, usize::from(count) * axis_size)?;
    let mut tags = std::collections::HashSet::new();
    for axis in records.chunks_exact(axis_size) {
        let values = [
            dword(axis, 4)? as i32,
            dword(axis, 8)? as i32,
            dword(axis, 12)? as i32,
        ];
        if !tags.insert(&axis[..4]) || values[0] > values[1] || values[1] > values[2] {
            return Err(invalid("invalid fvar axis ordering or range"));
        }
    }
    slice(bytes, offset + records.len(), instances * instance_size)?;
    Ok(count)
}

fn validate_gvar(bytes: &[u8], axes: u16, glyphs: u16) -> Result<(), Error> {
    if word(bytes, 0)? != 1
        || word(bytes, 2)? != 0
        || word(bytes, 4)? != axes
        || word(bytes, 12)? != glyphs
        || word(bytes, 14)? & !1 != 0
    {
        return Err(invalid("invalid gvar version, axes or glyph count"));
    }
    let long = word(bytes, 14)? != 0;
    let width = if long { 4 } else { 2 };
    let header_end = 20 + (usize::from(glyphs) + 1) * width;
    slice(bytes, 0, header_end)?;
    let shared_count = usize::from(word(bytes, 6)?);
    if shared_count > 4095 {
        return Err(invalid("too many shared gvar tuples"));
    }
    let shared_offset = dword(bytes, 8)? as usize;
    let shared_len = shared_count * usize::from(axes) * 2;
    let base = dword(bytes, 16)? as usize;
    if base < header_end || (shared_count != 0 && shared_offset < header_end) {
        return Err(invalid("gvar data overlaps its header"));
    }
    slice(bytes, shared_offset, shared_len)?;
    slice(bytes, base, 0)?;
    let mut previous = 0;
    for index in 0..=usize::from(glyphs) {
        let offset = if long {
            dword(bytes, 20 + index * width)? as usize
        } else {
            usize::from(word(bytes, 20 + index * width)?) * 2
        };
        if offset < previous {
            return Err(invalid("gvar offsets are not increasing"));
        }
        slice(bytes, base, offset)?;
        previous = offset;
    }
    Ok(())
}

fn validate_cvar(bytes: &[u8], axes: u16, cvt_len: usize) -> Result<(), Error> {
    let packed_count = word(bytes, 4)?;
    let count = usize::from(packed_count & 0x0fff);
    let data_offset = usize::from(word(bytes, 6)?);
    if word(bytes, 0)? != 1
        || word(bytes, 2)? != 0
        || count == 0
        || packed_count & 0x7000 != 0
        || data_offset < 8
    {
        return Err(invalid("invalid cvar header"));
    }
    let headers = slice(bytes, 0, data_offset)?;
    let mut header_offset = 8;
    let mut tuples = Vec::with_capacity(count);
    for _ in 0..count {
        let size = usize::from(word(headers, header_offset)?);
        let flags = word(headers, header_offset + 2)?;
        if flags & 0x8000 == 0 || flags & 0x1000 != 0 {
            return Err(invalid("cvar requires embedded peak tuples"));
        }
        let tuple_count = if flags & 0x4000 != 0 { 3 } else { 1 };
        let length = 4 + usize::from(axes) * 2 * tuple_count;
        slice(headers, header_offset, length)?;
        header_offset += length;
        tuples.push((size, flags));
    }
    let mut data = Cursor {
        bytes: &bytes[data_offset..],
        at: 0,
    };
    let shared_count = if packed_count & 0x8000 != 0 {
        data.points(cvt_len)?
    } else {
        cvt_len
    };
    for (size, flags) in tuples {
        let mut tuple = Cursor {
            bytes: data.take(size)?,
            at: 0,
        };
        let points = if flags & 0x2000 != 0 {
            tuple.points(cvt_len)?
        } else {
            shared_count
        };
        tuple.deltas(points)?;
        if tuple.at != tuple.bytes.len() {
            return Err(invalid("cvar tuple has excess delta data"));
        }
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let result = slice(self.bytes, self.at, len)?;
        self.at += len;
        Ok(result)
    }

    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn points(&mut self, cvt_len: usize) -> Result<usize, Error> {
        let first = self.byte()?;
        let count = if first & 0x80 != 0 {
            (usize::from(first & 0x7f) << 8) | usize::from(self.byte()?)
        } else {
            usize::from(first)
        };
        if count == 0 {
            return Ok(cvt_len);
        }
        if count > cvt_len {
            return Err(invalid("cvar point count exceeds CVT length"));
        }
        let mut processed = 0;
        let mut index = 0usize;
        while processed < count {
            let control = self.byte()?;
            let run = usize::from(control & 0x7f) + 1;
            if processed + run > count {
                return Err(invalid("cvar point run exceeds count"));
            }
            for _ in 0..run {
                let delta = if control & 0x80 != 0 {
                    usize::from(word(self.take(2)?, 0)?)
                } else {
                    usize::from(self.byte()?)
                };
                index += delta;
                if index >= cvt_len || (processed != 0 && delta == 0) {
                    return Err(invalid("cvar point index outside CVT or not increasing"));
                }
                processed += 1;
            }
        }
        Ok(count)
    }

    fn deltas(&mut self, count: usize) -> Result<(), Error> {
        let mut processed = 0;
        while processed < count {
            let control = self.byte()?;
            let run = usize::from(control & 0x3f) + 1;
            if processed + run > count {
                return Err(invalid("cvar delta run exceeds point count"));
            }
            let width = match control & 0xc0 {
                0 => 1,
                0x40 => 2,
                0x80 => 0,
                _ => 4,
            };
            self.take(run * width)?;
            processed += run;
        }
        Ok(())
    }
}

fn slice(bytes: &[u8], start: usize, length: usize) -> Result<&[u8], Error> {
    start
        .checked_add(length)
        .and_then(|end| bytes.get(start..end))
        .ok_or_else(|| invalid("truncated variation data"))
}

fn dword(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    Ok((u32::from(word(bytes, offset)?) << 16) | u32::from(word(bytes, offset + 2)?))
}
