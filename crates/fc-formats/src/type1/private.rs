//! Declarative Type 1 private values; no PostScript procedures are executed.

use fc_model::PsPrivate;

use super::{reader::header_token, source_error};
use crate::Error;

pub(super) fn field(
    name: &[u8],
    input: &mut &[u8],
    target: &mut PsPrivate,
) -> Result<Option<String>, Error> {
    match name {
        b"/BlueValues" => target.blue_values = array(input, 14)?,
        b"/OtherBlues" => target.other_blues = array(input, 10)?,
        b"/FamilyBlues" => target.family_blues = array(input, 14)?,
        b"/FamilyOtherBlues" => target.family_other_blues = array(input, 10)?,
        b"/StemSnapH" => target.stem_snap_h = array(input, 12)?,
        b"/StemSnapV" => target.stem_snap_v = array(input, 12)?,
        b"/StdHW" => target.std_hw = singleton(input)?,
        b"/StdVW" => target.std_vw = singleton(input)?,
        b"/BlueScale" => target.blue_scale = Some(number(input)?),
        b"/BlueShift" => target.blue_shift = Some(number(input)?),
        b"/BlueFuzz" => target.blue_fuzz = Some(number(input)?),
        b"/ForceBold" => {
            target.force_bold = Some(match header_token(input)? {
                Some(b"true") => true,
                Some(b"false") => false,
                _ => return Err(source_error("ForceBold must be a literal boolean")),
            })
        }
        b"/LanguageGroup" => {
            target.language_group = Some(match number(input)? {
                0.0 => 0,
                1.0 => 1,
                _ => return Err(source_error("LanguageGroup must be 0 or 1")),
            });
        }
        _ => return Ok(None),
    }
    for _ in 0..4 {
        let before = *input;
        match header_token(input)? {
            Some(b"readonly" | b"noaccess" | b"executeonly") => {}
            Some(b"def" | b"ND" | b"|-") => return Ok(None),
            _ => {
                *input = before;
                // Some real fonts trim StemSnap arrays through an interpreter-
                // dependent compatibility procedure. The literal prefix is not
                // the resulting value. Clear only this field and disclose it.
                clear(name, target);
                return Ok(Some(String::from_utf8_lossy(&name[1..]).into_owned()));
            }
        }
    }
    Err(source_error("private hint access modifiers exceed limit"))
}

fn clear(name: &[u8], target: &mut PsPrivate) {
    match name {
        b"/BlueValues" => target.blue_values.clear(),
        b"/OtherBlues" => target.other_blues.clear(),
        b"/FamilyBlues" => target.family_blues.clear(),
        b"/FamilyOtherBlues" => target.family_other_blues.clear(),
        b"/StemSnapH" => target.stem_snap_h.clear(),
        b"/StemSnapV" => target.stem_snap_v.clear(),
        b"/StdHW" => target.std_hw = None,
        b"/StdVW" => target.std_vw = None,
        b"/BlueScale" => target.blue_scale = None,
        b"/BlueShift" => target.blue_shift = None,
        b"/BlueFuzz" => target.blue_fuzz = None,
        b"/ForceBold" => target.force_bold = None,
        b"/LanguageGroup" => target.language_group = None,
        _ => {}
    }
}

fn singleton(input: &mut &[u8]) -> Result<Option<f64>, Error> {
    let values = array(input, 1)?;
    if values.len() != 1 {
        return Err(source_error(
            "standard stem width requires a one-element array",
        ));
    }
    Ok(values.first().copied())
}

fn number(input: &mut &[u8]) -> Result<f64, Error> {
    let token = header_token(input)?.ok_or_else(|| source_error("missing private value"))?;
    numeric_token(token)
}

fn numeric_token(token: &[u8]) -> Result<f64, Error> {
    std::str::from_utf8(token)
        .ok()
        .and_then(|text| text.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .ok_or_else(|| source_error("private values must be finite literal numbers"))
}

fn array(input: &mut &[u8], limit: usize) -> Result<Vec<f64>, Error> {
    let close: &[u8] = match header_token(input)? {
        Some(b"[") => b"]",
        Some(b"{") => b"}",
        _ => {
            return Err(source_error(
                "private array requires literal numeric values",
            ))
        }
    };
    let mut result = Vec::new();
    loop {
        let token =
            header_token(input)?.ok_or_else(|| source_error("unterminated private array"))?;
        if token == close {
            return Ok(result);
        }
        if result.len() == limit {
            return Err(source_error("private array exceeds format limit"));
        }
        result.push(numeric_token(token)?);
    }
}

pub(super) fn write(values: &PsPrivate) -> Result<String, Error> {
    values
        .validate()
        .map_err(|error| source_error(error.to_string()))?;
    let mut result = String::new();
    for (name, array) in [
        ("BlueValues", &values.blue_values),
        ("OtherBlues", &values.other_blues),
        ("FamilyBlues", &values.family_blues),
        ("FamilyOtherBlues", &values.family_other_blues),
        ("StemSnapH", &values.stem_snap_h),
        ("StemSnapV", &values.stem_snap_v),
    ] {
        if !array.is_empty() || name == "BlueValues" {
            let text = array
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            result.push_str(&format!("/{name} [{text}] def\n"));
        }
    }
    for (name, value) in [("StdHW", values.std_hw), ("StdVW", values.std_vw)] {
        if let Some(value) = value {
            result.push_str(&format!("/{name} [{value}] def\n"));
        }
    }
    for (name, value) in [
        ("BlueScale", values.blue_scale),
        ("BlueShift", values.blue_shift),
        ("BlueFuzz", values.blue_fuzz),
        ("LanguageGroup", values.language_group.map(f64::from)),
    ] {
        if let Some(value) = value {
            result.push_str(&format!("/{name} {value} def\n"));
        }
    }
    if let Some(value) = values.force_bold {
        result.push_str(&format!("/ForceBold {value} def\n"));
    }
    Ok(result)
}

pub(super) fn normalized(values: &PsPrivate, transform: kurbo::Affine) -> Option<PsPrivate> {
    let [xx, _, _, yy, _, dy] = transform.as_coeffs();
    if values == &PsPrivate::default() {
        return Some(values.clone());
    }
    // Reflection exchanges top and bottom zones. Keep the original dictionary
    // as source metadata rather than inventing a new baseline zone.
    if xx <= 0.0 || yy <= 0.0 {
        return None;
    }
    let mut result = values.clone();
    for zones in [
        &mut result.blue_values,
        &mut result.other_blues,
        &mut result.family_blues,
        &mut result.family_other_blues,
    ] {
        for value in zones {
            *value = *value * yy + dy;
        }
    }
    for value in &mut result.stem_snap_h {
        *value *= yy;
    }
    for value in &mut result.stem_snap_v {
        *value *= xx;
    }
    result.std_hw = result.std_hw.map(|value| value * yy);
    result.std_vw = result.std_vw.map(|value| value * xx);
    result.blue_scale = result.blue_scale.map(|value| value / yy);
    result.blue_shift = result.blue_shift.map(|value| value * yy);
    result.blue_fuzz = result.blue_fuzz.map(|value| value * yy);
    result.validate().ok()?;
    Some(result)
}
