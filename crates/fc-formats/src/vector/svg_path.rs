//! Strict lexical/arity guard before kurbo's best-effort SVG path decoder.

use kurbo::BezPath;

use super::{source_error, valid_number, validate_path, MAX_ELEMENTS};
use crate::Error;

const FORMAT: &str = "SVG font";

/// Parse only path operators with exact polynomial outline representations.
pub(super) fn parse(source: &str) -> Result<BezPath, Error> {
    if source.len() > 8 * 1024 * 1024 {
        return Err(source_error(FORMAT, "individual path exceeds 8 MiB limit"));
    }
    let mut input = source.as_bytes();
    let mut canonical = String::new();
    let mut previous = None;
    let mut groups = 0usize;
    loop {
        skip_space(&mut input);
        if input.is_empty() {
            break;
        }
        let command = if input[0].is_ascii_alphabetic() {
            let command = input[0];
            input = &input[1..];
            command
        } else {
            previous.ok_or_else(|| source_error(FORMAT, "path requires a command"))?
        };
        if canonical.is_empty() && !matches!(command, b'M' | b'm') {
            return Err(source_error(FORMAT, "path must start with moveto"));
        }
        let arity =
            match command {
                b'M' | b'm' | b'L' | b'l' => 2,
                b'H' | b'h' | b'V' | b'v' => 1,
                b'Q' | b'q' => 4,
                b'C' | b'c' => 6,
                b'Z' | b'z' => 0,
                _ => return Err(source_error(
                    FORMAT,
                    "unsupported SVG path command; supported: M L H V Q C Z and relative variants",
                )),
            };
        canonical.push(command as char);
        for position in 0..arity {
            skip_space(&mut input);
            if position > 0 && input.first() == Some(&b',') {
                input = &input[1..];
                skip_space(&mut input);
            }
            let value = number(&mut input)?;
            canonical.push_str(&format!("{value} "));
        }
        groups += 1;
        if groups > MAX_ELEMENTS || canonical.len() > 32 * 1024 * 1024 {
            return Err(source_error(FORMAT, "path element limit exceeded"));
        }
        previous = match command {
            b'M' => Some(b'L'),
            b'm' => Some(b'l'),
            b'Z' | b'z' => None,
            _ => Some(command),
        };
        skip_space(&mut input);
        if input.first() == Some(&b',') {
            if previous.is_none() {
                return Err(source_error(FORMAT, "unexpected path comma"));
            }
            input = &input[1..];
            skip_space(&mut input);
            if input.is_empty() || input[0].is_ascii_alphabetic() {
                return Err(source_error(FORMAT, "comma must separate coordinates"));
            }
        }
    }
    let path = BezPath::from_svg(&canonical).map_err(|e| source_error(FORMAT, e.to_string()))?;
    validate_path(&path, FORMAT)?;
    Ok(path)
}

fn skip_space(input: &mut &[u8]) {
    while input
        .first()
        .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
    {
        *input = &input[1..];
    }
}

fn number(input: &mut &[u8]) -> Result<f64, Error> {
    let source = *input;
    let mut index = usize::from(source.first().is_some_and(|b| matches!(b, b'+' | b'-')));
    let mut digits = 0usize;
    while source.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
        digits += 1;
    }
    if source.get(index) == Some(&b'.') {
        index += 1;
        while source.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return Err(source_error(FORMAT, "expected path coordinate"));
    }
    if source.get(index).is_some_and(|b| matches!(b, b'e' | b'E')) {
        index += 1;
        if source.get(index).is_some_and(|b| matches!(b, b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while source.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == start {
            return Err(source_error(FORMAT, "invalid path exponent"));
        }
    }
    let value = std::str::from_utf8(&source[..index])
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| valid_number(*v))
        .ok_or_else(|| source_error(FORMAT, "nonfinite or out-of-range coordinate"))?;
    *input = &source[index..];
    Ok(value)
}
