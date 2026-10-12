//! Independently implemented from Adobe Technical Note 5004, AFM 4.1.

use std::collections::BTreeSet;

use super::{err, GlyphMetrics, KerningPair, Ligature, MetricsFont};
use crate::Error;

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_RECORDS: usize = 65_536;

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Header,
    Characters(usize),
    AfterCharacters,
    KernData,
    Pairs(usize),
    AfterPairs,
    AfterKern,
    End,
}

/// Parse AFM 2.0–4.1 horizontal metrics, rejecting unsupported content.
///
/// Input is limited to 32 MiB and 65,536 characters or kerning pairs. Character
/// codes, names and horizontal widths are required; boxes are optional. Unknown
/// properties, vertical components, composites and track kerning are errors.
/// Comments are retained without positional significance.
pub fn read_afm(text: &str) -> Result<MetricsFont, Error> {
    if text.len() > MAX_BYTES || text.contains('\0') || !text.is_ascii() {
        return Err(err("input exceeds 32 MiB, contains NUL or is not ASCII"));
    }
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let first = lines.next().ok_or_else(|| err("empty metrics file"))?;
    let (key, version) = split(first);
    if key != "StartFontMetrics" || !matches!(version, "2.0" | "3.0" | "4.0" | "4.1") {
        return Err(err("expected StartFontMetrics 2.0, 3.0, 4.0 or 4.1"));
    }
    let mut metrics = MetricsFont::default();
    let mut section = Section::Header;
    let mut named = false;
    let mut seen_chars = false;
    for line in lines {
        let (key, value) = split(line);
        if section == Section::End {
            return Err(err("content after EndFontMetrics"));
        }
        if key == "Comment" {
            metrics.comments.push(value.into());
            continue;
        }
        match section {
            Section::Characters(expected) => {
                if key == "EndCharMetrics" {
                    no_value(value)?;
                    if metrics.glyphs.len() != expected {
                        return Err(err("StartCharMetrics count does not match records"));
                    }
                    section = Section::AfterCharacters;
                } else {
                    if metrics.glyphs.len() >= expected {
                        return Err(err("too many character records"));
                    }
                    metrics.glyphs.push(character(line)?);
                }
            }
            Section::Pairs(expected) => {
                if key == "EndKernPairs" {
                    no_value(value)?;
                    if metrics.kerning.len() != expected {
                        return Err(err("StartKernPairs count does not match records"));
                    }
                    section = Section::AfterPairs;
                } else {
                    if metrics.kerning.len() >= expected {
                        return Err(err("too many kerning records"));
                    }
                    metrics.kerning.push(pair(key, value)?);
                }
            }
            Section::Header if key == "FontName" => {
                if named {
                    return Err(err("duplicate FontName"));
                }
                named = true;
                metrics.font_name = value.into();
            }
            Section::Header if key == "StartCharMetrics" => {
                section = Section::Characters(count(value)?);
                seen_chars = true;
            }
            Section::AfterCharacters if key == "StartKernData" => {
                no_value(value)?;
                section = Section::KernData;
            }
            Section::KernData if matches!(key, "StartKernPairs" | "StartKernPairs0") => {
                section = Section::Pairs(count(value)?);
            }
            Section::KernData | Section::AfterPairs if key == "EndKernData" => {
                no_value(value)?;
                section = Section::AfterKern;
            }
            Section::Header | Section::AfterCharacters | Section::AfterKern
                if key == "EndFontMetrics" =>
            {
                no_value(value)?;
                section = Section::End;
            }
            Section::Header => {
                property(key, value)?;
                if metrics
                    .properties
                    .insert(key.into(), value.into())
                    .is_some()
                {
                    return Err(err(format!("duplicate global property {key}")));
                }
            }
            _ => return Err(err(format!("unexpected or unsupported AFM record {key}"))),
        }
    }
    if section != Section::End || !named || !seen_chars {
        return Err(err("missing FontName, character section or EndFontMetrics"));
    }
    validate(&metrics)?;
    Ok(metrics)
}

fn split(line: &str) -> (&str, &str) {
    line.split_once(char::is_whitespace)
        .map_or((line, ""), |(key, value)| (key, value.trim()))
}

fn no_value(value: &str) -> Result<(), Error> {
    if value.is_empty() {
        Ok(())
    } else {
        Err(err("unexpected value after section delimiter"))
    }
}

fn count(value: &str) -> Result<usize, Error> {
    value
        .parse::<usize>()
        .ok()
        .filter(|count| *count <= MAX_RECORDS)
        .ok_or_else(|| err("invalid record count; maximum is 65536"))
}

fn number(value: &str) -> Result<f64, Error> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| err(format!("invalid finite number {value:?}")))
}

fn numbers<const N: usize>(value: &str) -> Result<[f64; N], Error> {
    let mut values = value.split_whitespace();
    let mut result = [0.0; N];
    for output in &mut result {
        *output = number(values.next().ok_or_else(|| err("missing numeric value"))?)?;
    }
    if values.next().is_some() {
        return Err(err("too many numeric values"));
    }
    Ok(result)
}

fn horizontal(value: &str) -> Result<f64, Error> {
    let [x, y] = numbers(value)?;
    if y != 0.0 {
        return Err(err("vertical advance components are unsupported"));
    }
    Ok(x)
}

fn character(line: &str) -> Result<GlyphMetrics, Error> {
    let mut result = GlyphMetrics::default();
    let mut seen = BTreeSet::new();
    for field in line.split(';').map(str::trim).filter(|v| !v.is_empty()) {
        let (key, value) = split(field);
        let canonical = match key {
            "CH" => "C",
            "W0X" | "W" | "W0" => "WX",
            _ => key,
        };
        if canonical != "L" && !seen.insert(canonical) {
            return Err(err(format!("duplicate character property {key}")));
        }
        match key {
            "C" => {
                let code = value
                    .parse::<i32>()
                    .map_err(|_| err("invalid character code"))?;
                result.code = if code == -1 {
                    None
                } else {
                    Some(u16::try_from(code).map_err(|_| err("character code exceeds 0..65535"))?)
                };
            }
            "CH" => {
                let hex = value
                    .strip_prefix('<')
                    .and_then(|s| s.strip_suffix('>'))
                    .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit()))
                    .ok_or_else(|| err("invalid hexadecimal character code"))?;
                result.code =
                    Some(u16::from_str_radix(hex, 16).map_err(|_| err("hex code exceeds 65535"))?);
            }
            "WX" | "W0X" => result.width = number(value)?,
            "W" | "W0" => result.width = horizontal(value)?,
            "N" => result.name = value.into(),
            "B" => result.bbox = Some(numbers(value)?),
            "L" => {
                let names = value.split_whitespace().collect::<Vec<_>>();
                if names.len() != 2 {
                    return Err(err("ligature requires successor and replacement names"));
                }
                result.ligatures.push(Ligature {
                    successor: names[0].into(),
                    replacement: names[1].into(),
                });
            }
            _ => return Err(err(format!("unsupported character property {key}"))),
        }
    }
    if !["C", "WX", "N"].iter().all(|key| seen.contains(key)) {
        return Err(err("character requires code, horizontal width and name"));
    }
    Ok(result)
}

fn pair(key: &str, value: &str) -> Result<KerningPair, Error> {
    let fields = value.split_whitespace().collect::<Vec<_>>();
    let value = match (key, fields.len()) {
        ("KPX", 3) => number(fields[2])?,
        ("KP", 4) if number(fields[3])? == 0.0 => number(fields[2])?,
        _ => {
            return Err(err(
                "only horizontal named KPX or zero-y KP pairs are supported",
            ))
        }
    };
    Ok(KerningPair {
        left: fields[0].into(),
        right: fields[1].into(),
        value,
    })
}

fn text_value(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || !value.is_ascii()
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(err("property values must be nonempty single-line text"));
    }
    Ok(())
}

fn name(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || !value.is_ascii()
        || value
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control() || b"()<>[]{}/%;".contains(&b))
    {
        return Err(err(format!("invalid PostScript name {value:?}")));
    }
    Ok(())
}

fn property(key: &str, value: &str) -> Result<(), Error> {
    text_value(value)?;
    match key {
        "FullName" | "FamilyName" | "Weight" | "Version" | "Notice" | "EncodingScheme"
        | "CharacterSet" => {}
        "ItalicAngle" | "UnderlinePosition" | "UnderlineThickness" | "CapHeight" | "XHeight"
        | "Ascender" | "Descender" | "StdHW" | "StdVW" => {
            number(value)?;
        }
        "FontBBox" => {
            bbox(&numbers(value)?)?;
        }
        "IsFixedPitch" if matches!(value, "true" | "false") => {}
        "IsBaseFont" if value == "true" => {}
        "IsCIDFont" if value == "false" => {}
        "MetricsSets" if value == "0" => {}
        "Characters" => {
            count(value)?;
        }
        "CharWidth" => {
            horizontal(value)?;
        }
        _ => return Err(err(format!("unsupported AFM property {key} {value}"))),
    }
    Ok(())
}

fn bbox(value: &[f64; 4]) -> Result<(), Error> {
    if value.iter().any(|n| !n.is_finite()) || value[0] > value[2] || value[1] > value[3] {
        return Err(err("bounding box is nonfinite or inverted"));
    }
    Ok(())
}

pub(crate) fn validate(metrics: &MetricsFont) -> Result<(), Error> {
    name(&metrics.font_name)?;
    if metrics.glyphs.len() > MAX_RECORDS || metrics.kerning.len() > MAX_RECORDS {
        return Err(err("too many metric records"));
    }
    for (key, value) in &metrics.properties {
        property(key, value)?;
    }
    for comment in &metrics.comments {
        if !comment.is_ascii() || comment.trim() != comment || comment.chars().any(char::is_control)
        {
            return Err(err("invalid comment text"));
        }
    }
    let mut names = BTreeSet::new();
    let mut codes = BTreeSet::new();
    for glyph in &metrics.glyphs {
        name(glyph.name.as_ref())?;
        if !glyph.width.is_finite()
            || !names.insert(&glyph.name)
            || glyph.code.is_some_and(|code| !codes.insert(code))
        {
            return Err(err(
                "nonfinite width, duplicate glyph name or duplicate character code",
            ));
        }
        if let Some(bounds) = glyph.bbox {
            bbox(&bounds)?;
        }
        let mut ligatures = BTreeSet::new();
        for ligature in &glyph.ligatures {
            name(ligature.successor.as_ref())?;
            name(ligature.replacement.as_ref())?;
            if !ligatures.insert(&ligature.successor) {
                return Err(err("duplicate ligature successor"));
            }
        }
    }
    let common_width = metrics
        .properties
        .get("CharWidth")
        .map(|value| horizontal(value))
        .transpose()?;
    if let Some(width) = common_width {
        if metrics
            .properties
            .get("IsFixedPitch")
            .is_some_and(|value| value == "false")
            || metrics.glyphs.iter().any(|glyph| glyph.width != width)
        {
            return Err(err("CharWidth conflicts with pitch or character advances"));
        }
    }
    if metrics
        .properties
        .get("IsFixedPitch")
        .is_some_and(|value| value == "true")
        && metrics
            .glyphs
            .windows(2)
            .any(|pair| pair[0].width != pair[1].width)
    {
        return Err(err("fixed-pitch font has varying character advances"));
    }
    let mut pairs = BTreeSet::new();
    for pair in &metrics.kerning {
        name(pair.left.as_ref())?;
        name(pair.right.as_ref())?;
        if !pair.value.is_finite() || !pairs.insert((&pair.left, &pair.right)) {
            return Err(err("nonfinite adjustment or duplicate kerning pair"));
        }
    }
    Ok(())
}

/// Serialize the supported horizontal AFM subset after validating every record.
pub fn write_afm(metrics: &MetricsFont) -> Result<String, Error> {
    validate(metrics)?;
    let mut result = format!("StartFontMetrics 4.1\nFontName {}\n", metrics.font_name);
    for comment in &metrics.comments {
        result.push_str(&format!("Comment {comment}\n"));
    }
    for (key, value) in &metrics.properties {
        result.push_str(&format!("{key} {value}\n"));
    }
    result.push_str(&format!("StartCharMetrics {}\n", metrics.glyphs.len()));
    for glyph in &metrics.glyphs {
        result.push_str(&format!(
            "C {} ; WX {} ; N {} ;",
            glyph.code.map_or(-1, i32::from),
            glyph.width,
            glyph.name
        ));
        if let Some([xmin, ymin, xmax, ymax]) = glyph.bbox {
            result.push_str(&format!(" B {xmin} {ymin} {xmax} {ymax} ;"));
        }
        for ligature in &glyph.ligatures {
            result.push_str(&format!(
                " L {} {} ;",
                ligature.successor, ligature.replacement
            ));
        }
        result.push('\n');
    }
    result.push_str("EndCharMetrics\n");
    if !metrics.kerning.is_empty() {
        result.push_str(&format!(
            "StartKernData\nStartKernPairs {}\n",
            metrics.kerning.len()
        ));
        for pair in &metrics.kerning {
            result.push_str(&format!(
                "KPX {} {} {}\n",
                pair.left, pair.right, pair.value
            ));
        }
        result.push_str("EndKernPairs\nEndKernData\n");
    }
    result.push_str("EndFontMetrics\n");
    if result.len() > MAX_BYTES {
        return Err(err("serialized AFM exceeds 32 MiB"));
    }
    Ok(result)
}
