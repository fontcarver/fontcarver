//! CFF INDEX, DICT and Type 2 operand encoding (Adobe specifications 5176/5177).

use super::WriteError;

pub(super) fn index(objects: &[Vec<u8>]) -> Result<Vec<u8>, WriteError> {
    let count =
        u16::try_from(objects.len()).map_err(|_| WriteError("too many INDEX entries".into()))?;
    let mut bytes = count.to_be_bytes().to_vec();
    if count == 0 {
        return Ok(bytes);
    }
    let mut end = 1u32;
    for object in objects {
        end = end
            .checked_add(
                u32::try_from(object.len())
                    .map_err(|_| WriteError("INDEX item exceeds 4 GiB".into()))?,
            )
            .ok_or_else(|| WriteError("INDEX exceeds 4 GiB".into()))?;
    }
    let off_size = if end <= 0xff {
        1
    } else if end <= 0xffff {
        2
    } else if end <= 0xffffff {
        3
    } else {
        4
    };
    bytes.push(off_size as u8);
    let mut offset = 1u32;
    bytes.extend_from_slice(&offset.to_be_bytes()[4 - off_size..]);
    for object in objects {
        offset += object.len() as u32;
        bytes.extend_from_slice(&offset.to_be_bytes()[4 - off_size..]);
    }
    for object in objects {
        bytes.extend_from_slice(object);
    }
    Ok(bytes)
}

pub(super) fn integer(bytes: &mut Vec<u8>, value: i32) {
    match value {
        -107..=107 => bytes.push((value + 139) as u8),
        108..=1131 => {
            let value = value - 108;
            bytes.extend([(value / 256 + 247) as u8, (value % 256) as u8]);
        }
        -1131..=-108 => {
            let value = -value - 108;
            bytes.extend([(value / 256 + 251) as u8, (value % 256) as u8]);
        }
        -32768..=32767 => {
            bytes.push(28);
            bytes.extend_from_slice(&(value as i16).to_be_bytes());
        }
        _ => {
            bytes.push(29);
            bytes.extend_from_slice(&value.to_be_bytes());
        }
    }
}

pub(super) fn offset(bytes: &mut Vec<u8>, value: usize) -> Result<(), WriteError> {
    // Fixed-width offsets make the Top DICT length independent of table positions.
    let value = i32::try_from(value).map_err(|_| WriteError("CFF offset exceeds 2 GiB".into()))?;
    bytes.push(29);
    bytes.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

pub(super) fn real(bytes: &mut Vec<u8>, value: f64) -> Result<(), WriteError> {
    if !value.is_finite() {
        return Err(WriteError("non-finite dictionary number".into()));
    }
    let text = format!("{value:.14}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    let text = if text.is_empty() || text == "-" {
        "0"
    } else {
        text
    };
    let mut nibbles: Vec<u8> = text
        .bytes()
        .map(|byte| match byte {
            b'.' => 10,
            b'-' => 14,
            _ => byte - b'0',
        })
        .collect();
    nibbles.push(15);
    if !nibbles.len().is_multiple_of(2) {
        nibbles.push(15);
    }
    bytes.push(30);
    bytes.extend(nibbles.chunks_exact(2).map(|pair| (pair[0] << 4) | pair[1]));
    Ok(())
}

pub(super) fn number(bytes: &mut Vec<u8>, value: f64) -> Result<(), WriteError> {
    if !value.is_finite() || !(-32768.0..=32767.99998474121).contains(&value) {
        return Err(WriteError(format!(
            "charstring operand {value} is outside the supported coordinate range"
        )));
    }
    if value.fract() == 0.0 {
        integer(bytes, value as i32);
    } else {
        bytes.push(255);
        bytes.extend_from_slice(&((value * 65536.0).round() as i32).to_be_bytes());
    }
    Ok(())
}
