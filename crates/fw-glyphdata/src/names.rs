//! Conversion between Unicode values and standard glyph names.

use crate::aglfn;

/// Resolves an AGLFN or production glyph name to a Unicode scalar value.
///
/// A suffix beginning at the first `.` is ignored. Ligature names containing
/// `_` are rejected because this API can return only one scalar value.
pub fn unicode_for_name(name: &str) -> Option<u32> {
    let base_name = name.split('.').next().unwrap_or_default();
    if base_name.is_empty() || base_name.contains('_') {
        return None;
    }

    parse_production_name(base_name).or_else(|| aglfn::codepoint_for_name(base_name))
}

/// Returns the AGLFN name for a Unicode value, or a canonical production name.
pub fn name_for_unicode(cp: u32) -> String {
    if let Some(name) = aglfn::name_for_codepoint(cp) {
        return name.to_owned();
    }

    if cp <= 0xFFFF {
        format!("uni{cp:04X}")
    } else {
        format!("u{cp:05X}")
    }
}

fn parse_production_name(name: &str) -> Option<u32> {
    if let Some(digits) = name.strip_prefix("uni") {
        if digits.len() == 4 {
            return parse_scalar(digits);
        }
    }

    if let Some(digits) = name.strip_prefix('u') {
        if (4..=6).contains(&digits.len()) {
            return parse_scalar(digits);
        }
    }

    None
}

fn parse_scalar(digits: &str) -> Option<u32> {
    let cp = u32::from_str_radix(digits, 16).ok()?;
    char::from_u32(cp).map(|_| cp)
}
