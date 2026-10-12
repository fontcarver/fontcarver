//! Bounded plist parsing and typed source-field helpers.

use super::error;
use crate::Error;
use glyphs_reader::Plist;

pub(super) fn parse(text: &str) -> Result<Plist, Error> {
    if text.len() as u64 > super::MAX_BYTES {
        return Err(error("source exceeds the 64 MiB limit"));
    }
    // The upstream plist parser is recursive and ignores trailing input. Check
    // balanced nesting and EOF ourselves before invoking it on untrusted data.
    let mut stack = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut data = false;
    let mut done = false;
    let mut items = 0usize;
    for ch in text.chars() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        if data {
            if ch == '>' {
                data = false;
            }
            continue;
        }
        if done {
            if !ch.is_ascii_whitespace() {
                return Err(error("trailing data after root dictionary"));
            }
            continue;
        }
        if matches!(ch, '{' | '(' | '=' | ',' | ';') {
            items += 1;
            if items > 2_000_000 {
                return Err(error(
                    "source exceeds the two-million-item complexity limit",
                ));
            }
        }
        match ch {
            '"' => quoted = true,
            '<' => data = true,
            '{' | '(' => {
                stack.push(ch);
                if stack.len() > 64 {
                    return Err(error("plist nesting exceeds 64 levels"));
                }
            }
            '}' | ')' => {
                if stack.pop() != Some(if ch == '}' { '{' } else { '(' }) {
                    return Err(error("unbalanced plist delimiters"));
                }
                if stack.is_empty() {
                    done = true;
                }
            }
            _ => {}
        }
    }
    if quoted || data || !stack.is_empty() || !done {
        return Err(error("incomplete Glyphs property list"));
    }
    let raw = Plist::parse(text).map_err(|err| error(format!("invalid property list: {err}")))?;
    if raw.as_dict().is_none() {
        return Err(error("root must be a dictionary"));
    }
    validate(&raw, 0)?;
    Ok(raw)
}

fn validate(raw: &Plist, depth: usize) -> Result<(), Error> {
    if depth > 64 {
        return Err(error("plist nesting exceeds 64 levels"));
    }
    match raw {
        Plist::Float(value) if !value.into_inner().is_finite() => {
            return Err(error("nonfinite number"))
        }
        Plist::Array(values) => {
            if values.len() > 1_000_000 {
                return Err(error("plist array exceeds one million entries"));
            }
            for value in values {
                validate(value, depth + 1)?;
            }
        }
        Plist::Dictionary(values) => {
            for value in values.values() {
                validate(value, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn dict() -> Plist {
    Plist::Dictionary(Default::default())
}
pub(super) fn serialize(value: &Plist) -> String {
    fn emit(value: &Plist, out: &mut String) {
        match value {
            Plist::Dictionary(values) => {
                out.push_str("{\n");
                for (key, value) in values {
                    out.push_str(&Plist::String(key.to_string()).to_string());
                    out.push_str(" = ");
                    emit(value, out);
                    out.push_str(";\n");
                }
                out.push('}');
            }
            Plist::Array(values) => {
                let compact = values
                    .iter()
                    .all(|v| !matches!(v, Plist::Dictionary(_) | Plist::Array(_)));
                out.push('(');
                if !compact {
                    out.push('\n');
                }
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                        if !compact {
                            out.push('\n');
                        }
                    }
                    emit(value, out);
                }
                if !compact {
                    out.push('\n');
                }
                out.push(')');
            }
            other => out.push_str(&other.to_string()),
        }
    }
    let mut out = String::new();
    emit(value, &mut out);
    out
}
pub(super) fn set(raw: &mut Plist, key: &str, value: Plist) {
    if let Plist::Dictionary(values) = raw {
        values.insert(key.into(), value);
    }
}
pub(super) fn remove(raw: &mut Plist, keys: &[&str]) {
    if let Plist::Dictionary(values) = raw {
        for key in keys {
            values.remove(*key);
        }
    }
}
pub(super) fn string(value: impl Into<String>) -> Plist {
    Plist::String(value.into())
}
pub(super) fn number(value: f64) -> Result<Plist, Error> {
    if !value.is_finite() {
        return Err(error("nonfinite numeric value"));
    }
    Ok(Plist::Float(value.into()))
}
pub(super) fn numbers(values: impl IntoIterator<Item = f64>) -> Result<Plist, Error> {
    Ok(Plist::Array(
        values.into_iter().map(number).collect::<Result<_, _>>()?,
    ))
}
pub(super) fn integer(value: impl Into<i64>) -> Plist {
    Plist::Integer(value.into())
}
pub(super) fn boolean(value: bool) -> Plist {
    integer(i64::from(value))
}
pub(super) fn text<'a>(raw: &'a Plist, key: &str) -> Option<&'a str> {
    raw.get(key).and_then(Plist::as_str)
}
pub(super) fn required<'a>(raw: &'a Plist, key: &str) -> Result<&'a str, Error> {
    text(raw, key)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| error(format!("missing or invalid {key}")))
}
pub(super) fn array<'a>(raw: &'a Plist, key: &str) -> Result<&'a [Plist], Error> {
    match raw.get(key) {
        None => Ok(&[]),
        Some(value) => value
            .as_array()
            .ok_or_else(|| error(format!("{key} must be an array"))),
    }
}
pub(super) fn numeric(raw: &Plist, key: &str, default: f64) -> Result<f64, Error> {
    match raw.get(key) {
        None => Ok(default),
        Some(value) => {
            as_number(value).map_err(|_| error(format!("{key} must be a finite number")))
        }
    }
}
pub(super) fn optional_number(raw: &Plist, key: &str) -> Result<Option<f64>, Error> {
    raw.get(key).map(as_number).transpose()
}
pub(super) fn as_number(raw: &Plist) -> Result<f64, Error> {
    raw.as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| error("expected finite number"))
}
pub(super) fn flag(raw: &Plist, key: &str, default: bool) -> Result<bool, Error> {
    Ok(numeric(raw, key, if default { 1. } else { 0. })? != 0.)
}
pub(super) fn tuple<const N: usize>(raw: &Plist) -> Result<[f64; N], Error> {
    let values = match raw {
        Plist::Array(values) => values
            .iter()
            .map(as_number)
            .collect::<Result<Vec<_>, _>>()?,
        Plist::String(value) => value
            .trim_matches(['{', '}'])
            .split(',')
            .map(|part| {
                part.trim()
                    .parse::<f64>()
                    .map_err(|_| error("invalid numeric tuple"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(error("invalid coordinate tuple")),
    };
    if values.iter().any(|v| !v.is_finite()) {
        return Err(error("nonfinite coordinate tuple"));
    }
    values
        .try_into()
        .map_err(|_| error(format!("coordinate tuple needs {N} values")))
}
pub(super) fn find<'a>(raw: &'a Plist, key: &str, field: &str, value: &str) -> Option<&'a Plist> {
    raw.get(key)?
        .as_array()?
        .iter()
        .find(|item| text(item, field) == Some(value))
}
