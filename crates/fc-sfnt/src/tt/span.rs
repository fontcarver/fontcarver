use std::ops::Range;

use super::invalid;
use crate::Error;

/// The byte range includes the length word when present. Component flag offsets
/// let the writer normalize WE_HAVE_INSTRUCTIONS onto the final component.
pub(super) struct ProgramSpan {
    pub range: Range<usize>,
    pub component_flags: Vec<usize>,
    data_end: usize,
}

pub(super) fn locate(bytes: &[u8]) -> Result<ProgramSpan, Error> {
    if bytes.is_empty() {
        return Ok(ProgramSpan {
            range: 0..0,
            component_flags: Vec::new(),
            data_end: 0,
        });
    }
    if bytes.len() < 10 {
        return Err(invalid("truncated glyph header"));
    }
    let contours = word(bytes, 0)? as i16;
    if contours >= 0 {
        let start = 10 + contours as usize * 2;
        // A zero-contour header alone is valid, without an instruction length.
        let end = if contours == 0 && bytes.len() == 10 {
            10
        } else {
            program_end(bytes, start)?
        };
        return Ok(ProgramSpan {
            range: start..end,
            component_flags: Vec::new(),
            data_end: coordinates_end(bytes, contours as usize, end)?,
        });
    }
    let mut cursor = 10;
    let mut component_flags = Vec::new();
    let mut has_program = false;
    loop {
        let flags = word(bytes, cursor)?;
        let transforms = flags & (0x08 | 0x40 | 0x80);
        if transforms.count_ones() > 1 {
            return Err(invalid("contradictory component transform flags"));
        }
        if flags & 0x1800 == 0x1800 {
            return Err(invalid("contradictory component offset flags"));
        }
        if component_flags.is_empty() && flags & 2 == 0 {
            return Err(invalid("first component must use an xy offset"));
        }
        if component_flags.len() == 65535 {
            return Err(invalid("component count exceeds maxp capacity"));
        }
        component_flags.push(cursor);
        has_program |= flags & 0x100 != 0;
        cursor += 4 + if flags & 1 != 0 { 4 } else { 2 };
        cursor += match transforms {
            0x08 => 2,
            0x40 => 4,
            0x80 => 8,
            _ => 0,
        };
        if cursor > bytes.len() {
            return Err(invalid("truncated component record"));
        }
        if flags & 0x20 == 0 {
            break;
        }
    }
    let end = if has_program {
        program_end(bytes, cursor)?
    } else {
        cursor
    };
    Ok(ProgramSpan {
        range: cursor..end,
        component_flags,
        data_end: end,
    })
}

fn coordinates_end(bytes: &[u8], contours: usize, mut cursor: usize) -> Result<usize, Error> {
    let mut points = 0;
    for contour in 0..contours {
        let end = usize::from(word(bytes, 10 + contour * 2)?) + 1;
        if end <= points || end > 65535 {
            return Err(invalid("invalid contour endpoint"));
        }
        points = end;
    }
    let mut coordinate_bytes = 0;
    while points > 0 {
        let flags = *bytes
            .get(cursor)
            .ok_or_else(|| invalid("truncated point flags"))?;
        cursor += 1;
        let repeats = if flags & 8 != 0 {
            let count = usize::from(
                *bytes
                    .get(cursor)
                    .ok_or_else(|| invalid("truncated flag repeat"))?,
            ) + 1;
            cursor += 1;
            count
        } else {
            1
        };
        points = points
            .checked_sub(repeats)
            .ok_or_else(|| invalid("flag repeat exceeds point count"))?;
        for (short, same) in [(2, 0x10), (4, 0x20)] {
            let size = if flags & short != 0 {
                1
            } else if flags & same != 0 {
                0
            } else {
                2
            };
            coordinate_bytes += repeats * size;
        }
    }
    let end = cursor + coordinate_bytes;
    if end > bytes.len() {
        return Err(invalid("truncated coordinate stream"));
    }
    Ok(end)
}

fn program_end(bytes: &[u8], start: usize) -> Result<usize, Error> {
    let end = start + 2 + usize::from(word(bytes, start)?);
    if end > bytes.len() {
        return Err(invalid("truncated glyph instruction stream"));
    }
    Ok(end)
}

pub(super) fn word(bytes: &[u8], offset: usize) -> Result<u16, Error> {
    bytes
        .get(offset..offset + 2)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_be_bytes)
        .ok_or_else(|| invalid("truncated glyph data"))
}

pub(super) fn replace(bytes: &[u8], program: &[u8]) -> Result<Vec<u8>, Error> {
    let length =
        u16::try_from(program.len()).map_err(|_| invalid("glyph program exceeds 65,535 bytes"))?;
    if bytes.is_empty() {
        if program.is_empty() {
            return Ok(Vec::new());
        }
        let mut result = vec![0; 10];
        result.extend(length.to_be_bytes());
        result.extend(program);
        return Ok(result);
    }
    let span = locate(bytes)?;
    let mut result = bytes[..span.range.start].to_vec();
    let composite = !span.component_flags.is_empty();
    let last = span.component_flags.last().copied();
    for offset in span.component_flags {
        let mut flags = word(bytes, offset)? & !0x100;
        if Some(offset) == last && !program.is_empty() {
            flags |= 0x100;
        }
        result[offset..offset + 2].copy_from_slice(&flags.to_be_bytes());
    }
    if !composite || !program.is_empty() {
        result.extend(length.to_be_bytes());
        result.extend(program);
    }
    // loca padding belongs to the container, not to the outline encoding. The
    // caller adds fresh alignment after changing the instruction byte count.
    result.extend_from_slice(&bytes[span.range.end..span.data_end]);
    Ok(result)
}
