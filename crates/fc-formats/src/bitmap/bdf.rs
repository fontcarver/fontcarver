//! Adobe BDF 2.1/2.2 with packed grayscale extension and Unicode encodings.

use super::{add_glyph, checked, error};
use crate::Error;
use fc_model::{BdfValue, Bitmap, BitmapGlyph, BitmapStrike, Font};
use std::{collections::BTreeMap, fmt::Write, path::Path};

const META: &str = "org.fontcarver.bdf";

/// Read a BDF font (plain or gzip wrapped).
pub fn read(path: &Path) -> Result<Font, Error> {
    decode(&super::read_bytes(path)?)
}

/// Write one selected strike as native BDF.
pub fn write(font: &Font, path: &Path, key: Option<(u16, u8)>) -> Result<(), Error> {
    super::write_bytes(path, &encode(font, super::select_strike(font, key)?)?)
}

fn ints(value: &str, count: usize) -> Result<Vec<i32>, Error> {
    let values = value
        .split_whitespace()
        .map(|s| s.parse::<i32>().map_err(|_| error("invalid BDF integer")))
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != count {
        return Err(error("wrong BDF field count"));
    }
    Ok(values)
}

fn short(value: i32) -> Result<i16, Error> {
    i16::try_from(value).map_err(|_| error("BDF metric exceeds signed 16-bit range"))
}
fn unsigned(value: i32) -> Result<u16, Error> {
    u16::try_from(value).map_err(|_| error("negative or oversized BDF dimension"))
}
fn pair(line: &str) -> (&str, &str) {
    line.split_once(char::is_whitespace)
        .map_or((line, ""), |(a, b)| (a, b.trim()))
}

fn property(value: &str) -> Result<BdfValue, Error> {
    if value.starts_with('"') {
        if !value.ends_with('"') || value.len() < 2 {
            return Err(error("unterminated BDF string"));
        }
        let inner = &value[1..value.len() - 1];
        let mut chars = inner.chars();
        let mut output = String::new();
        while let Some(c) = chars.next() {
            if c == '"' && chars.next() != Some('"') {
                return Err(error("invalid BDF quote escape"));
            }
            output.push(c);
        }
        Ok(BdfValue::Str(output))
    } else {
        Ok(BdfValue::Int(
            value.parse().map_err(|_| error("invalid BDF property"))?,
        ))
    }
}

/// Decode a bounded BDF buffer, rejecting truncated records and counts.
pub fn decode(bytes: &[u8]) -> Result<Font, Error> {
    if bytes.len() > super::MAX_INPUT as usize {
        return Err(error("BDF input limit"));
    }
    let source = std::str::from_utf8(bytes).map_err(|_| error("BDF is not UTF-8/ASCII"))?;
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|s| !s.starts_with("COMMENT") && !s.is_empty())
        .peekable();
    let first = lines.next().ok_or_else(|| error("empty BDF"))?;
    if !matches!(first, "STARTFONT 2.1" | "STARTFONT 2.2" | "STARTFONT 2.3") {
        return Err(error("unsupported BDF header"));
    }
    let mut strike = BitmapStrike {
        depth: 1,
        ..Default::default()
    };
    let mut globals = BTreeMap::<String, String>::new();
    let mut count = None;
    while let Some(line) = lines.next() {
        let (key, value) = pair(line);
        match key {
            "STARTPROPERTIES" => {
                let expected: usize = value
                    .parse()
                    .map_err(|_| error("invalid BDF property count"))?;
                if expected > 65535 {
                    return Err(error("too many BDF properties"));
                }
                for _ in 0..expected {
                    let (key, value) =
                        pair(lines.next().ok_or_else(|| error("truncated properties"))?);
                    if key == "ENDPROPERTIES"
                        || strike
                            .properties
                            .insert(key.into(), property(value)?)
                            .is_some()
                    {
                        return Err(error("duplicate or missing BDF property"));
                    }
                }
                if lines.next() != Some("ENDPROPERTIES") {
                    return Err(error("BDF property count mismatch"));
                }
            }
            "CHARS" => {
                count = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| error("invalid BDF glyph count"))?,
                );
                break;
            }
            "FONT" | "SIZE" | "FONTBOUNDINGBOX" | "METRICSSET" | "SWIDTH" | "DWIDTH"
            | "SWIDTH1" | "DWIDTH1" | "VVECTOR" | "CONTENTVERSION" => {
                globals.insert(key.into(), value.into());
            }
            _ => return Err(error(format!("unsupported BDF header field {key}"))),
        }
    }
    let count = count
        .filter(|n| *n <= 65535)
        .ok_or_else(|| error("missing or oversized BDF glyph count"))?;
    let size = globals
        .get("SIZE")
        .ok_or_else(|| error("missing BDF SIZE"))?;
    let size: Vec<f64> = size
        .split_whitespace()
        .map(|s| s.parse().map_err(|_| error("invalid BDF SIZE")))
        .collect::<Result<_, _>>()?;
    if !(3..=4).contains(&size.len()) || size.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err(error("invalid BDF SIZE"));
    }
    let ppem = (size[0] * size[2] / 72.0).round();
    if !(1.0..=4096.0).contains(&ppem) {
        return Err(error("unsupported BDF ppem"));
    }
    strike.ppem = ppem as u16;
    if size.len() == 4 {
        if ![1.0, 2.0, 4.0, 8.0].contains(&size[3]) {
            return Err(error("unsupported BDF depth"));
        }
        strike.depth = size[3] as u8;
    }
    let bbox = ints(
        globals
            .get("FONTBOUNDINGBOX")
            .ok_or_else(|| error("missing font bounding box"))?,
        4,
    )?;
    strike.ascent = short(bbox[1].saturating_add(bbox[3]).max(0))?;
    strike.descent = short(bbox[3].saturating_neg().max(0))?;
    for (name, target) in [
        ("FONT_ASCENT", &mut strike.ascent),
        ("FONT_DESCENT", &mut strike.descent),
    ] {
        if let Some(BdfValue::Int(value)) = strike.properties.get(name) {
            *target = i16::try_from(*value).map_err(|_| error("BDF ascent/descent overflow"))?;
        }
    }
    let family = match strike.properties.get("FAMILY_NAME") {
        Some(BdfValue::Str(s)) => s.as_str(),
        _ => globals.get("FONT").map_or("Bitmap", String::as_str),
    };
    let mut font = Font::new_empty(family, 1000);
    let unicode = match strike.properties.get("CHARSET_REGISTRY") {
        Some(BdfValue::Str(s)) => {
            s.eq_ignore_ascii_case("ISO10646")
                || (s.eq_ignore_ascii_case("ISO8859")
                    && matches!(strike.properties.get("CHARSET_ENCODING"),Some(BdfValue::Str(e)) if e=="1"))
        }
        _ => true,
    };
    let mut byte_count = 0usize;
    for _ in 0..count {
        let (key, name) = pair(lines.next().ok_or_else(|| error("missing STARTCHAR"))?);
        if key != "STARTCHAR" || name.is_empty() || name.chars().any(char::is_whitespace) {
            return Err(error("invalid STARTCHAR"));
        }
        let mut fields = globals.clone();
        loop {
            let line = lines.next().ok_or_else(|| error("truncated BDF glyph"))?;
            let (key, value) = pair(line);
            if key == "BITMAP" {
                break;
            }
            if !matches!(
                key,
                "ENCODING"
                    | "SWIDTH"
                    | "DWIDTH"
                    | "SWIDTH1"
                    | "DWIDTH1"
                    | "VVECTOR"
                    | "BBX"
                    | "ATTRIBUTES"
            ) {
                return Err(error(format!("unsupported BDF glyph field {key}")));
            }
            fields.insert(key.into(), value.into());
        }
        let bbox = ints(fields.get("BBX").ok_or_else(|| error("missing BBX"))?, 4)?;
        let mut bitmap = checked(Bitmap::new(
            unsigned(bbox[0])?,
            unsigned(bbox[1])?,
            strike.depth,
        ))?;
        bitmap.origin_x = short(bbox[2])?;
        bitmap.origin_y = short(bbox[3])?;
        byte_count += bitmap.rows.len();
        if byte_count > fc_model::MAX_FONT_BITMAP_BYTES {
            return Err(error("BDF bitmap memory limit"));
        }
        let stride = checked(bitmap.stride())?;
        for row in 0..if stride == 0 {
            0
        } else {
            usize::from(bitmap.height)
        } {
            let line = lines.next().ok_or_else(|| error("truncated BDF pixels"))?;
            if line.len() != stride * 2 || !line.is_ascii() {
                return Err(error("BDF bitmap row length mismatch"));
            }
            for col in 0..stride {
                bitmap.rows[row * stride + col] =
                    u8::from_str_radix(&line[col * 2..col * 2 + 2], 16)
                        .map_err(|_| error("invalid hex pixels"))?;
            }
        }
        if lines.next() != Some("ENDCHAR") {
            return Err(error("missing ENDCHAR"));
        }
        checked(bitmap.validate())?;
        let width = ints(
            fields
                .get("DWIDTH")
                .ok_or_else(|| error("missing DWIDTH"))?,
            2,
        )?;
        if width[1] != 0 {
            return Err(error("nonhorizontal DWIDTH is not supported"));
        }
        let vertical = if let Some(value) = fields.get("DWIDTH1") {
            let values = ints(value, 2)?;
            if values[0] != 0 {
                return Err(error("nonvertical DWIDTH1"));
            }
            Some(short(
                values[1]
                    .checked_neg()
                    .ok_or_else(|| error("vertical width overflow"))?,
            )?)
        } else {
            None
        };
        let encoding = fields
            .get("ENCODING")
            .ok_or_else(|| error("missing ENCODING"))?;
        let enc = encoding
            .split_whitespace()
            .map(|s| s.parse::<i32>().map_err(|_| error("invalid ENCODING")))
            .collect::<Result<Vec<_>, _>>()?;
        if enc.is_empty() || enc.len() > 2 || enc[0] < -1 || (enc.len() == 2 && enc[0] != -1) {
            return Err(error("invalid ENCODING"));
        }
        let code = *enc.last().ok_or_else(|| error("missing encoding"))?;
        let codes = if unicode && code >= 0 {
            if char::from_u32(code as u32).is_none() {
                return Err(error("invalid Unicode BDF encoding"));
            }
            vec![code as u32]
        } else {
            Vec::new()
        };
        let vertical_origin = fields
            .get("VVECTOR")
            .map(|v| {
                let xy = ints(v, 2)?;
                Ok::<_, Error>((short(xy[0])?, short(xy[1])?))
            })
            .transpose()?;
        add_glyph(
            &mut font,
            &mut strike,
            name,
            codes,
            BitmapGlyph {
                bitmap,
                advance: short(width[0])?,
                vertical_advance: vertical,
                vertical_origin,
                refs: Vec::new(),
            },
        )?;
        if let Some(glyph) = font.glyph_mut(name) {
            glyph.user_data = serde_json::json!({META:{"ENCODING": encoding,"VVECTOR": fields.get("VVECTOR"),"ATTRIBUTES": fields.get("ATTRIBUTES")}});
            if let Some(swidth) = fields.get("SWIDTH") {
                let widths = swidth
                    .split_whitespace()
                    .map(|s| s.parse::<f64>().map_err(|_| error("invalid SWIDTH")))
                    .collect::<Result<Vec<_>, _>>()?;
                if widths.len() != 2 || widths.iter().any(|v| !v.is_finite()) || widths[1] != 0.0 {
                    return Err(error("unsupported SWIDTH"));
                }
                if let Some(layer) = glyph.layers.first_mut() {
                    layer.width = widths[0];
                }
            }
        }
    }
    if lines.next() != Some("ENDFONT") || lines.next().is_some() {
        return Err(error("missing ENDFONT or extra glyphs/trailing data"));
    }
    font.user_data = serde_json::json!({META:globals});
    font.masters[0].metrics.ascender = f64::from(strike.ascent) * 1000.0 / f64::from(strike.ppem);
    font.masters[0].metrics.descender =
        -f64::from(strike.descent) * 1000.0 / f64::from(strike.ppem);
    font.bitmap_strikes.push(strike);
    checked(font.validate_bitmaps())?;
    Ok(font)
}

fn safe(value: &str) -> Result<&str, Error> {
    if value.contains(['\n', '\r', '\0']) {
        Err(error("BDF string contains a line break or NUL"))
    } else {
        Ok(value)
    }
}

/// Serialize one validated strike; outlines and source-only data are not in BDF.
pub fn encode(font: &Font, strike: &BitmapStrike) -> Result<Vec<u8>, Error> {
    checked(strike.validate())?;
    if strike.depth == 32 {
        return Err(error("BDF cannot represent RGBA; select a coverage strike"));
    }
    let mut properties = strike.properties.clone();
    properties.insert(
        "FONT_ASCENT".into(),
        BdfValue::Int(i64::from(strike.ascent)),
    );
    properties.insert(
        "FONT_DESCENT".into(),
        BdfValue::Int(i64::from(strike.descent)),
    );
    properties.insert("PIXEL_SIZE".into(), BdfValue::Int(i64::from(strike.ppem)));
    properties
        .entry("FAMILY_NAME".into())
        .or_insert_with(|| BdfValue::Str(font.info.family_name.clone()));
    properties
        .entry("CHARSET_REGISTRY".into())
        .or_insert_with(|| BdfValue::Str("ISO10646".into()));
    properties
        .entry("CHARSET_ENCODING".into())
        .or_insert_with(|| BdfValue::Str("1".into()));
    let unicode = matches!(properties.get("CHARSET_REGISTRY"),Some(BdfValue::Str(s)) if s.eq_ignore_ascii_case("ISO10646") || (s.eq_ignore_ascii_case("ISO8859") && matches!(properties.get("CHARSET_ENCODING"),Some(BdfValue::Str(e)) if e=="1")));
    let mut entries = Vec::new();
    for name in strike.glyphs.keys() {
        let glyph = font
            .glyph(name)
            .ok_or_else(|| error("bitmap has no owner"))?;
        if glyph.export {
            entries.push((glyph, checked(strike.flattened(name))?));
        }
    }
    let left = entries
        .iter()
        .map(|(_, g)| i32::from(g.bitmap.origin_x))
        .min()
        .unwrap_or(0);
    let bottom = entries
        .iter()
        .map(|(_, g)| i32::from(g.bitmap.origin_y))
        .min()
        .unwrap_or(0);
    let right = entries
        .iter()
        .map(|(_, g)| i32::from(g.bitmap.origin_x) + i32::from(g.bitmap.width))
        .max()
        .unwrap_or(0);
    let top = entries
        .iter()
        .map(|(_, g)| i32::from(g.bitmap.origin_y) + i32::from(g.bitmap.height))
        .max()
        .unwrap_or(0);
    let name = font
        .user_data
        .get(META)
        .and_then(|m| m.get("FONT"))
        .and_then(|s| s.as_str())
        .unwrap_or(&font.info.family_name);
    let vertical = entries.iter().any(|(_, g)| g.vertical_advance.is_some());
    if vertical && entries.iter().any(|(_, g)| g.vertical_advance.is_none()) {
        return Err(error(
            "BDF vertical metrics must be defined for every exported glyph",
        ));
    }
    let mut output = format!(
        "STARTFONT {}\nFONT {}\nSIZE {} 72 72{}\nFONTBOUNDINGBOX {} {} {} {}\n",
        if vertical { "2.2" } else { "2.1" },
        safe(name)?,
        strike.ppem,
        if strike.depth == 1 {
            String::new()
        } else {
            format!(" {}", strike.depth)
        },
        right - left,
        top - bottom,
        left,
        bottom
    );
    if vertical {
        output.push_str("METRICSSET 2\n");
    }
    writeln!(output, "STARTPROPERTIES {}", properties.len())
        .map_err(|_| error("BDF formatting"))?;
    for (key, value) in properties {
        if key.is_empty() || key.chars().any(char::is_whitespace) {
            return Err(error("invalid BDF property name"));
        }
        let value = match value {
            BdfValue::Int(i) => i.to_string(),
            BdfValue::Str(s) => format!("\"{}\"", safe(&s)?.replace('"', "\"\"")),
        };
        writeln!(output, "{} {}", safe(&key)?, value).map_err(|_| error("BDF formatting"))?;
    }
    writeln!(output, "ENDPROPERTIES\nCHARS {}", entries.len())
        .map_err(|_| error("BDF formatting"))?;
    for (glyph, bitmap) in entries {
        if glyph.name.chars().any(char::is_whitespace) {
            return Err(error("BDF glyph name contains whitespace"));
        }
        if glyph.unicodes.len() > 1 {
            return Err(error("BDF has one encoding per glyph; multiple Unicode assignments require a source format"));
        }
        let encoding = if unicode {
            glyph
                .unicodes
                .first()
                .map_or_else(|| "-1".into(), u32::to_string)
        } else {
            glyph
                .user_data
                .get(META)
                .and_then(|m| m.get("ENCODING"))
                .and_then(|s| s.as_str())
                .unwrap_or("-1")
                .to_owned()
        };
        let swidth = glyph.layers.first().map_or(
            f64::from(bitmap.advance) * 1000.0 / f64::from(strike.ppem),
            |l| l.width * 1000.0 / f64::from(font.info.units_per_em),
        );
        if !swidth.is_finite() {
            return Err(error("nonfinite BDF scalable width"));
        }
        writeln!(
            output,
            "STARTCHAR {}\nENCODING {}\nSWIDTH {} 0\nDWIDTH {} 0",
            safe(&glyph.name)?,
            safe(&encoding)?,
            swidth,
            bitmap.advance
        )
        .map_err(|_| error("BDF formatting"))?;
        if let Some(advance) = bitmap.vertical_advance {
            writeln!(
                output,
                "SWIDTH1 0 {}\nDWIDTH1 0 {}",
                -f64::from(advance) * 1000.0 / f64::from(strike.ppem),
                -i32::from(advance)
            )
            .map_err(|_| error("BDF formatting"))?;
            if let Some((x, y)) = bitmap.vertical_origin {
                writeln!(output, "VVECTOR {x} {y}").map_err(|_| error("BDF formatting"))?;
            }
        }
        if let Some(attributes) = glyph
            .user_data
            .get(META)
            .and_then(|m| m.get("ATTRIBUTES"))
            .and_then(serde_json::Value::as_str)
        {
            if !attributes.is_empty() && attributes.chars().all(|c| c.is_ascii_hexdigit()) {
                writeln!(output, "ATTRIBUTES {attributes}").map_err(|_| error("BDF formatting"))?;
            }
        }
        let b = &bitmap.bitmap;
        writeln!(
            output,
            "BBX {} {} {} {}\nBITMAP",
            b.width, b.height, b.origin_x, b.origin_y
        )
        .map_err(|_| error("BDF formatting"))?;
        let stride = checked(b.stride())?;
        for row in 0..usize::from(b.height) {
            for byte in &b.rows[row * stride..(row + 1) * stride] {
                write!(output, "{byte:02X}").map_err(|_| error("BDF formatting"))?;
            }
            output.push('\n');
        }
        output.push_str("ENDCHAR\n");
    }
    output.push_str("ENDFONT\n");
    Ok(output.into_bytes())
}
