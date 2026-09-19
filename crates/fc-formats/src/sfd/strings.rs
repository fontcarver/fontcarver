//! SFD quoted strings use UTF-7, including supplementary Unicode scalars.

use super::error;
use crate::Error;
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};

pub(super) fn encode(text: &str) -> String {
    let mut result = String::new();
    let mut shifted = Vec::new();
    for c in text.chars() {
        if c.is_ascii() && !c.is_ascii_control() && c != '"' && c != '\\' && c != '+' {
            flush(&mut result, &mut shifted);
            result.push(c);
        } else if c == '+' {
            flush(&mut result, &mut shifted);
            result.push_str("+-");
        } else {
            let mut buffer = [0u16; 2];
            for unit in c.encode_utf16(&mut buffer) {
                shifted.extend_from_slice(&unit.to_be_bytes());
            }
        }
    }
    flush(&mut result, &mut shifted);
    result
}

fn flush(result: &mut String, shifted: &mut Vec<u8>) {
    if shifted.is_empty() {
        return;
    }
    result.push('+');
    result.push_str(&STANDARD_NO_PAD.encode(&*shifted));
    result.push('-');
    shifted.clear();
}

pub(super) fn quote(text: &str) -> String {
    format!("\"{}\"", encode(text))
}

pub(super) fn decode(text: &str) -> Result<String, Error> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('+') {
        out.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        if let Some(next) = rest.strip_prefix('-') {
            out.push('+');
            rest = next;
            continue;
        }
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '/' && c != '+')
            .unwrap_or(rest.len());
        let bytes = STANDARD_NO_PAD
            .decode(&rest[..end])
            .map_err(|_| error("invalid UTF-7 shift sequence"))?;
        if bytes.len() % 2 != 0 {
            return Err(error("odd byte count in UTF-7 sequence"));
        }
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect();
        // Historical SFD writers sometimes append a UTF-16 NUL to a shift run.
        let decoded =
            String::from_utf16(&units).map_err(|_| error("invalid UTF-16 in UTF-7 string"))?;
        out.push_str(decoded.trim_end_matches('\0'));
        rest = rest[end..].strip_prefix('-').unwrap_or(&rest[end..]);
    }
    out.push_str(rest);
    if out.contains('\0') {
        return Err(error("embedded NUL in SFD string"));
    }
    Ok(out)
}

pub(super) fn tokens(text: &str) -> Result<Vec<String>, Error> {
    let mut result = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '"' {
            let mut value = String::new();
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '"' {
                    closed = true;
                    break;
                }
                if c == '\\' {
                    value.push(
                        chars
                            .next()
                            .ok_or_else(|| error("unterminated string escape"))?,
                    );
                } else {
                    value.push(c);
                }
            }
            if !closed {
                return Err(error("unterminated quoted string"));
            }
            result.push(decode(&value)?);
        } else if "{}[]()<>".contains(c) {
            result.push(c.to_string());
        } else if c == '\'' {
            let mut tag = String::new();
            let mut closed = false;
            for c in chars.by_ref() {
                if c == '\'' {
                    closed = true;
                    break;
                }
                tag.push(c);
            }
            if !closed {
                return Err(error("unterminated tag"));
            }
            result.push(tag);
        } else {
            let mut value = c.to_string();
            while chars
                .peek()
                .is_some_and(|c| !c.is_whitespace() && !"{}[]()<>\"".contains(*c))
            {
                if let Some(c) = chars.next() {
                    value.push(c);
                }
            }
            result.push(value);
        }
        if result.len() > super::MAX_ITEMS {
            return Err(error("too many tokens"));
        }
    }
    Ok(result)
}

pub(super) fn text(value: &str) -> Result<String, Error> {
    if value.starts_with('"') {
        let tokens = tokens(value)?;
        if tokens.len() != 1 {
            return Err(error("expected one quoted string"));
        }
        Ok(tokens[0].clone())
    } else {
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_and_sfd_legacy_nul() {
        let value = "字体 + Français 😀 \\\"\n";
        assert_eq!(decode(&encode(value)).unwrap(), value);
        assert_eq!(decode("+AAoACgAA-After").unwrap(), "\n\nAfter");
        assert!(decode("+2AA-").is_err());
    }
}
