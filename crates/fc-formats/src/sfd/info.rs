//! Native font information and PostScript private dictionary fields.

use super::{error, integer, number, strings};
use crate::Error;
use fc_model::{Font, NamePlatform, NameRecord};
use std::collections::BTreeMap;

pub(super) fn apply_header(font: &mut Font, h: &BTreeMap<String, String>) -> Result<(), Error> {
    let get = |key: &str| h.get(key).map(String::as_str);
    if let Some(v) = get("FamilyName") {
        font.info.family_name = strings::text(v)?;
    }
    if let Some(v) = get("Weight") {
        font.info.style_name = strings::text(v)?;
        font.masters[0].name = font.info.style_name.clone();
    }
    for (key, target) in [
        ("FontName", &mut font.info.postscript_font_name),
        ("FullName", &mut font.info.postscript_full_name),
        ("Copyright", &mut font.info.copyright),
    ] {
        if let Some(v) = get(key) {
            *target = Some(strings::text(v)?);
        }
    }
    if let Some(v) = get("UComments").or_else(|| get("Comments")) {
        font.info.note = Some(strings::text(v)?);
    }
    let ascent = number(get("Ascent").unwrap_or("800"))?;
    let descent = number(get("Descent").unwrap_or("200"))?;
    let upem = ascent + descent;
    if ascent < 0.0 || descent < 0.0 || !(16.0..=16384.0).contains(&upem) || upem.fract() != 0.0 {
        return Err(error("invalid SFD em ascent/descent"));
    }
    font.info.units_per_em = upem as u16;
    let metrics = &mut font.masters[0].metrics;
    metrics.ascender = get("UFOAscent").map(number).transpose()?.unwrap_or(ascent);
    metrics.descender = get("UFODescent")
        .map(number)
        .transpose()?
        .unwrap_or(-descent);
    metrics.italic_angle = get("ItalicAngle").map(number).transpose()?.unwrap_or(0.0);
    metrics.cap_height = get("OS2CapHeight").map(number).transpose()?.unwrap_or(0.0);
    metrics.x_height = get("OS2XHeight").map(number).transpose()?.unwrap_or(0.0);
    for (key, offset, target) in [
        ("HheadAscent", "HheadAOffset", &mut metrics.hhea_ascender),
        ("HheadDescent", "HheadDOffset", &mut metrics.hhea_descender),
        (
            "OS2TypoAscent",
            "OS2TypoAOffset",
            &mut metrics.typo_ascender,
        ),
        (
            "OS2TypoDescent",
            "OS2TypoDOffset",
            &mut metrics.typo_descender,
        ),
        ("OS2WinAscent", "OS2WinAOffset", &mut metrics.win_ascent),
        ("OS2WinDescent", "OS2WinDOffset", &mut metrics.win_descent),
    ] {
        if let Some(v) = get(key) {
            let n = number(v)?;
            if get(offset) == Some("1") {
                if n != 0.0 {
                    return Err(error(format!(
                        "relative {key} offset cannot be represented"
                    )));
                }
            } else {
                *target = Some(n);
            }
        }
    }
    metrics.hhea_line_gap = get("LineGap").map(number).transpose()?;
    metrics.typo_line_gap = get("OS2TypoLinegap")
        .or_else(|| get("OS2TypoLineGap"))
        .map(number)
        .transpose()?;
    font.info.postscript_underline_position = get("UnderlinePosition").map(number).transpose()?;
    font.info.postscript_underline_thickness = get("UnderlineWidth").map(number).transpose()?;
    font.info.open_type_os2_weight_class = get("TTFWeight")
        .map(|v| v.parse().map_err(|_| error("invalid TTFWeight")))
        .transpose()?;
    font.info.open_type_os2_width_class = get("TTFWidth")
        .map(|v| v.parse().map_err(|_| error("invalid TTFWidth")))
        .transpose()?;
    font.info.open_type_os2_vendor_id = get("OS2Vendor").map(|v| v.trim_matches('\'').to_owned());
    if let Some(v) = get("Version") {
        if let Some((major, minor)) = v.split_once('.') {
            font.info.version_major = major.parse().map_err(|_| error("invalid font version"))?;
            font.info.version_minor = minor.parse().map_err(|_| error("invalid font version"))?;
        }
    }
    Ok(())
}

pub(super) fn read_names(font: &mut Font, value: &str) -> Result<(), Error> {
    let t = strings::tokens(value)?;
    let Some(language) = t.first() else {
        return Err(error("missing LangName language"));
    };
    let language = integer(language)?;
    if !(0..=65535).contains(&language) || t.len() > 32768 {
        return Err(error("invalid language/name count"));
    }
    for (id, text) in t[1..]
        .iter()
        .enumerate()
        .filter(|(_, text)| !text.is_empty())
    {
        font.info.names.push(NameRecord {
            name_id: id as u16,
            language: format!("x-sfd-{language}"),
            text: text.clone(),
            platform: NamePlatform::Windows,
        });
        if language != 1033 {
            continue;
        }
        match id {
            1 => font.info.family_name = text.clone(),
            2 => font.info.style_name = text.clone(),
            0 => font.info.copyright = Some(text.clone()),
            4 => font.info.postscript_full_name = Some(text.clone()),
            5 => font.info.open_type_name_version = Some(text.clone()),
            6 => font.info.postscript_font_name = Some(text.clone()),
            7 => font.info.trademark = Some(text.clone()),
            8 => font.info.open_type_name_manufacturer = Some(text.clone()),
            9 => font.info.open_type_name_designer = Some(text.clone()),
            10 => font.info.open_type_name_description = Some(text.clone()),
            11 => font.info.open_type_name_manufacturer_url = Some(text.clone()),
            12 => font.info.open_type_name_designer_url = Some(text.clone()),
            13 => font.info.open_type_name_license = Some(text.clone()),
            14 => font.info.open_type_name_license_url = Some(text.clone()),
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn write_names(font: &Font, out: &mut String) -> Result<(), Error> {
    let mut names = BTreeMap::<u16, BTreeMap<u16, String>>::new();
    for record in &font.info.names {
        let language = match record.language.as_str() {
            "en" | "en-US" => 1033,
            "zh-CN" | "zh-Hans" => 2052,
            "fr" | "fr-FR" => 1036,
            value => value
                .strip_prefix("x-sfd-")
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| {
                    error(format!(
                        "SFD needs a Windows language ID for name language {value:?}"
                    ))
                })?,
        };
        if record.name_id > 32767 {
            return Err(error("name ID exceeds SFD limit"));
        }
        names
            .entry(language)
            .or_default()
            .insert(record.name_id, record.text.clone());
    }
    let english = names.entry(1033).or_default();
    // Omit names mirrored by the ASCII header, so native header edits remain authoritative.
    for id in [0, 1, 2, 4, 6] {
        english.remove(&id);
    }
    for (id, value) in [
        (5, &font.info.open_type_name_version),
        (7, &font.info.trademark),
        (8, &font.info.open_type_name_manufacturer),
        (9, &font.info.open_type_name_designer),
        (10, &font.info.open_type_name_description),
        (11, &font.info.open_type_name_manufacturer_url),
        (12, &font.info.open_type_name_designer_url),
        (13, &font.info.open_type_name_license),
        (14, &font.info.open_type_name_license_url),
    ] {
        if let Some(value) = value {
            english.insert(id, value.clone());
        }
    }
    for (language, names) in names {
        let Some(last) = names.keys().last().copied() else {
            continue;
        };
        out.push_str(&format!("LangName: {language}"));
        for id in 0..=last {
            out.push(' ');
            out.push_str(&strings::quote(
                names.get(&id).map(String::as_str).unwrap_or(""),
            ));
        }
        out.push('\n');
    }
    Ok(())
}

pub(super) fn private_entry(font: &mut Font, line: &str) -> Result<(), Error> {
    let mut parts = line.splitn(3, ' ');
    let key = parts.next().ok_or_else(|| error("missing private key"))?;
    let length = integer(
        parts
            .next()
            .ok_or_else(|| error("missing private string length"))?,
    )?;
    let value = parts.next().ok_or_else(|| error("missing private value"))?;
    if length < 0 || length as usize != value.len() {
        return Err(error("private dictionary string length mismatch"));
    }
    let p = &mut font.masters[0].private_dict;
    let array = |v: &str| -> Result<Vec<f64>, Error> {
        let v = v
            .strip_prefix('[')
            .and_then(|v| v.strip_suffix(']'))
            .ok_or_else(|| error("invalid private array"))?;
        v.split_whitespace().map(number).collect()
    };
    match key {
        "BlueValues" => p.blue_values = array(value)?,
        "OtherBlues" => p.other_blues = array(value)?,
        "FamilyBlues" => p.family_blues = array(value)?,
        "FamilyOtherBlues" => p.family_other_blues = array(value)?,
        "StemSnapH" => p.stem_snap_h = array(value)?,
        "StemSnapV" => p.stem_snap_v = array(value)?,
        "BlueScale" => p.blue_scale = Some(number(value)?),
        "BlueShift" => p.blue_shift = Some(number(value)?),
        "BlueFuzz" => p.blue_fuzz = Some(number(value)?),
        "StdHW" => p.std_hw = Some(number(value.trim_matches(['[', ']']))?),
        "StdVW" => p.std_vw = Some(number(value.trim_matches(['[', ']']))?),
        "ForceBold" => {
            p.force_bold = Some(match value {
                "true" => true,
                "false" => false,
                _ => return Err(error("invalid ForceBold")),
            })
        }
        "LanguageGroup" => {
            p.language_group = Some(value.parse().map_err(|_| error("invalid LanguageGroup"))?)
        }
        _ => {
            p.other
                .insert(key.into(), serde_json::Value::String(value.into()));
        }
    }
    Ok(())
}
