//! Native encrypted Type 1 charstring and PFA/PFB serialization.

use std::collections::{BTreeMap, BTreeSet};

use fc_model::{Font, MasterId, PsHints};
use kurbo::{BezPath, PathEl, Point, Rect, Shape};

use super::{source_error, MAX_FILE_BYTES};
use crate::Error;

pub(super) fn encode(font: &Font, master: &MasterId, binary: bool) -> Result<Vec<u8>, Error> {
    if font.has_unapplied_feature_draft() {
        return Err(Error::UnappliedFeatureDraft);
    }
    let master_data = font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.to_string()))?;
    if font.info.units_per_em == 0 {
        return Err(source_error("units per em must be positive"));
    }
    if font.cid.is_some() {
        return Err(source_error("CID-keyed Type 1 output is not supported"));
    }
    let angle = master_data.metrics.italic_angle;
    let underline = font.info.postscript_underline_position.unwrap_or(-100.0);
    let thickness = font.info.postscript_underline_thickness.unwrap_or(50.0);
    for value in [angle, underline, thickness] {
        if !value.is_finite() || value.abs() > 32767.0 {
            return Err(source_error("nonfinite or out-of-range font metrics"));
        }
    }
    if thickness < 0.0 {
        return Err(source_error("negative underline thickness"));
    }
    let fallback = format!("{}-{}", font.info.family_name, font.info.style_name).replace(' ', "");
    let ps_name = font
        .info
        .postscript_font_name
        .as_deref()
        .unwrap_or(&fallback);
    validate_name(ps_name)?;
    if ps_name.len() > 63 {
        return Err(source_error("PostScript font name exceeds 63 bytes"));
    }

    // Validate and budget reachable outlines before using the display-oriented
    // model resolver, which deliberately tolerates malformed point sequences.
    let mut costs = BTreeMap::new();
    let mut total_points = 0usize;
    for glyph in font.glyphs.values().filter(|g| g.export) {
        total_points = total_points.saturating_add(outline_cost(
            font,
            &glyph.name,
            master,
            &mut costs,
            &mut BTreeSet::new(),
        )?);
        if total_points > 2_000_000 {
            return Err(source_error("font outline complexity limit exceeded"));
        }
    }
    let mut glyphs = Vec::new();
    let mut bbox: Option<Rect> = None;
    let mut total_size = 0usize;
    let mut add = |name: &str, width: f64, path: BezPath, hints: &PsHints| -> Result<(), Error> {
        validate_name(name)?;
        if !width.is_finite() || !(0.0..=32000.0).contains(&width) {
            return Err(source_error(format!(
                "glyph {name}: advance must be between 0 and 32000"
            )));
        }
        let path = quantized_path(&path)?;
        let data = charstring(&path, width, hints)
            .map_err(|e| source_error(format!("glyph {name}: {e}")))?;
        if !path.elements().is_empty() {
            let bounds = path.bounding_box();
            bbox = Some(bbox.map_or(bounds, |b| b.union(bounds)));
        }
        total_size += data.len() + name.len() + 40;
        if total_size > MAX_FILE_BYTES / 2 {
            return Err(source_error("serialized font exceeds size limit"));
        }
        glyphs.push((name.to_owned(), data));
        Ok(())
    };
    if font.glyph(".notdef").is_none_or(|g| !g.export) {
        add(".notdef", 0.0, BezPath::new(), &PsHints::default())?;
    }
    for (name, glyph) in &font.glyphs {
        if !glyph.export {
            continue;
        }
        if name != &glyph.name {
            return Err(source_error("glyph map key differs from glyph name"));
        }
        let layer = glyph.master_layer(master).ok_or_else(|| {
            source_error(format!("glyph {name} has no layer for the selected master"))
        })?;
        let path = fc_model::query::flatten_layer(font, name, master)
            .map_err(|e| source_error(e.to_string()))?;
        add(name, layer.width, path, &layer.hints)?;
    }
    if glyphs.len() > 65535 {
        return Err(source_error("more than 65535 exported glyphs"));
    }
    glyphs.sort_by_key(|(name, _)| name != ".notdef");
    let bounds = bbox.unwrap_or(Rect::ZERO);
    let full_name = font
        .info
        .postscript_full_name
        .clone()
        .unwrap_or_else(|| format!("{} {}", font.info.family_name, font.info.style_name));
    let scale = 1.0 / f64::from(font.info.units_per_em);
    let mut clear = format!(
        "%!PS-AdobeFont-1.0: {ps_name} 1.0\n%%Creator: FontCarver\n11 dict begin\n/FontInfo 9 dict dup begin\n/version (1.0) def\n/FullName {} def\n/FamilyName {} def\n/Weight {} def\n/ItalicAngle {angle} def\n/isFixedPitch {} def\n/UnderlinePosition {underline} def\n/UnderlineThickness {thickness} def\n/Notice {} def\nend readonly def\n/FontName /{ps_name} def\n/PaintType 0 def\n/FontType 1 def\n/FontMatrix [{scale:.15} 0 0 {scale:.15} 0 0] readonly def\n/FontBBox [{} {} {} {}] readonly def\n/Encoding 256 array\n0 1 255 {{1 index exch /.notdef put}} for\n",
        ps_string(&full_name)?, ps_string(&font.info.family_name)?, ps_string(&font.info.style_name)?,
        font.info.postscript_is_fixed_pitch.unwrap_or(false), ps_string(font.info.copyright.as_deref().unwrap_or(""))?,
        bounds.x0.floor(), bounds.y0.floor(), bounds.x1.ceil(), bounds.y1.ceil(),
    );
    let encoding = super::encoding_names(font);
    for (code, name) in encoding.iter().enumerate().filter(|(_, n)| *n != ".notdef") {
        clear.push_str(&format!("dup {code} /{name} put\n"));
    }
    clear.push_str("readonly def\ncurrentdict end\ncurrentfile eexec\n");
    let mut private = b"FC01dup /Private 24 dict dup begin\n/RD {string currentfile exch readstring pop} executeonly def\n/ND {noaccess def} executeonly def\n/NP {noaccess put} executeonly def\n/lenIV 4 def\n/MinFeature {16 16} def\n/password 5839 def\n/Subrs 0 array ND\n".to_vec();
    private.extend_from_slice(super::private::write(&master_data.private_dict)?.as_bytes());
    private.extend_from_slice(
        format!("2 index /CharStrings {} dict dup begin\n", glyphs.len()).as_bytes(),
    );
    for (name, data) in glyphs {
        private.extend_from_slice(format!("/{name} {} RD ", data.len()).as_bytes());
        private.extend_from_slice(&data);
        private.extend_from_slice(b" ND\n");
    }
    private.extend_from_slice(b"end\nend\nreadonly put\nnoaccess put\ndup /FontName get exch definefont pop\nmark currentfile closefile\n");
    let encrypted = encrypt(&private, 55665);
    let trailer = format!(
        "\n{}cleartomark\n",
        format!("{}\n", "0".repeat(64)).repeat(8)
    );
    let mut output = Vec::new();
    if binary {
        segment(&mut output, 1, clear.as_bytes())?;
        segment(&mut output, 2, &encrypted)?;
        segment(&mut output, 1, trailer.as_bytes())?;
        output.extend_from_slice(&[0x80, 3]);
    } else {
        const HEX: &[u8] = b"0123456789abcdef";
        output.extend_from_slice(clear.as_bytes());
        for chunk in encrypted.chunks(32) {
            for byte in chunk {
                output.push(HEX[(byte >> 4) as usize]);
                output.push(HEX[(byte & 15) as usize]);
            }
            output.push(b'\n');
        }
        output.extend_from_slice(trailer.as_bytes());
    }
    if output.len() > MAX_FILE_BYTES {
        return Err(source_error("serialized font exceeds 32 MiB"));
    }
    Ok(output)
}

fn outline_cost(
    font: &Font,
    name: &str,
    master: &MasterId,
    costs: &mut BTreeMap<String, usize>,
    active: &mut BTreeSet<String>,
) -> Result<usize, Error> {
    if let Some(cost) = costs.get(name) {
        return Ok(*cost);
    }
    if active.len() >= 64 || !active.insert(name.into()) {
        return Err(source_error(format!(
            "component cycle or nesting limit at {name}"
        )));
    }
    let glyph = font
        .glyph(name)
        .ok_or_else(|| source_error(format!("missing component {name}")))?;
    let layer = glyph
        .master_layer(master)
        .ok_or_else(|| source_error(format!("glyph {name} has no selected master layer")))?;
    let mut cost = 1usize;
    for contour in &layer.contours {
        if !contour.closed {
            return Err(source_error(format!("glyph {name} has an open contour")));
        }
        if contour
            .points
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
        {
            return Err(source_error(format!("glyph {name} has nonfinite geometry")));
        }
        contour
            .try_to_bezpath()
            .map_err(|e| source_error(format!("glyph {name}: {e}")))?;
        cost = cost.saturating_add(contour.points.len().saturating_add(2));
    }
    for component in &layer.components {
        if component
            .transform
            .as_coeffs()
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err(source_error(format!(
                "glyph {name} has a nonfinite component transform"
            )));
        }
        if component.smart.is_some() || component.point_match.is_some() {
            return Err(source_error(format!("glyph {name}: smart and point-matched components require explicit resolution before Type 1 export")));
        }
        cost = cost.saturating_add(outline_cost(font, &component.base, master, costs, active)?);
        if cost > 100_000 {
            return Err(source_error(format!(
                "glyph {name}: expanded outline exceeds complexity limit"
            )));
        }
    }
    if cost > 100_000 {
        return Err(source_error(format!(
            "glyph {name}: outline exceeds complexity limit"
        )));
    }
    active.remove(name);
    costs.insert(name.into(), cost);
    Ok(cost)
}

fn validate_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > 127
        || name
            .bytes()
            .any(|b| !(33..=126).contains(&b) || b"()<>[]{}/%".contains(&b))
    {
        return Err(source_error(format!("invalid PostScript name {name:?}")));
    }
    Ok(())
}

fn ps_string(value: &str) -> Result<String, Error> {
    if value.len() > 65535 {
        return Err(source_error("PostScript string exceeds 65535 bytes"));
    }
    let mut out = String::from("(");
    for ch in value.chars() {
        match ch {
            '\\' | '(' | ')' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out.push(')');
    Ok(out)
}

fn segment(out: &mut Vec<u8>, kind: u8, data: &[u8]) -> Result<(), Error> {
    let len = u32::try_from(data.len()).map_err(|_| source_error("PFB segment too large"))?;
    out.extend_from_slice(&[0x80, kind]);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(data);
    Ok(())
}

fn encrypt(bytes: &[u8], mut state: u16) -> Vec<u8> {
    bytes
        .iter()
        .map(|byte| {
            let cipher = *byte ^ (state >> 8) as u8;
            state = state
                .wrapping_add(u16::from(cipher))
                .wrapping_mul(52845)
                .wrapping_add(22719);
            cipher
        })
        .collect()
}

fn charstring(path: &BezPath, width: f64, hints: &PsHints) -> Result<Vec<u8>, Error> {
    let mut out = vec![0, 0, 0, 0];
    number(&mut out, 0.0)?;
    number(&mut out, width)?;
    out.push(13); // hsbw
    super::hints::write(&mut out, hints)?;
    let mut current = Point::ZERO;
    let mut source_current = Point::ZERO;
    let mut open = false;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => {
                if open {
                    return Err(source_error("open contour"));
                }
                source_current = point;
                let point = quantized(point)?;
                vector(&mut out, point - current)?;
                out.push(21);
                current = point;
                open = true;
            }
            PathEl::LineTo(point) => {
                if !open {
                    return Err(source_error("line outside contour"));
                }
                source_current = point;
                let point = quantized(point)?;
                vector(&mut out, point - current)?;
                out.push(5);
                current = point;
            }
            PathEl::QuadTo(control, point) => {
                let a = source_current + (control - source_current) * (2.0 / 3.0);
                let b = point + (control - point) * (2.0 / 3.0);
                cubic(&mut out, &mut current, a, b, point, open)?;
                source_current = point;
            }
            PathEl::CurveTo(a, b, point) => {
                cubic(&mut out, &mut current, a, b, point, open)?;
                source_current = point;
            }
            PathEl::ClosePath => {
                if !open {
                    return Err(source_error("close outside contour"));
                }
                out.push(9);
                open = false;
                // Type 1 closepath leaves the current point at the previous endpoint.
            }
        }
    }
    if open {
        return Err(source_error("open contour"));
    }
    out.push(14);
    if out.len() > 65535 {
        return Err(source_error(
            "charstring exceeds the PostScript 65,535-byte string limit",
        ));
    }
    Ok(encrypt(&out, 4330))
}

fn quantized_path(path: &BezPath) -> Result<BezPath, Error> {
    let mut result = BezPath::new();
    let mut current = Point::ZERO;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => {
                result.move_to(quantized(point)?);
                current = point;
            }
            PathEl::LineTo(point) => {
                result.line_to(quantized(point)?);
                current = point;
            }
            PathEl::QuadTo(control, point) => {
                let a = current + (control - current) * (2.0 / 3.0);
                let b = point + (control - point) * (2.0 / 3.0);
                result.curve_to(quantized(a)?, quantized(b)?, quantized(point)?);
                current = point;
            }
            PathEl::CurveTo(a, b, point) => {
                result.curve_to(quantized(a)?, quantized(b)?, quantized(point)?);
                current = point;
            }
            PathEl::ClosePath => result.close_path(),
        }
    }
    Ok(result)
}

fn cubic(
    out: &mut Vec<u8>,
    current: &mut Point,
    a: Point,
    b: Point,
    end: Point,
    open: bool,
) -> Result<(), Error> {
    if !open {
        return Err(source_error("curve outside contour"));
    }
    let a = quantized(a)?;
    let b = quantized(b)?;
    let end = quantized(end)?;
    vector(out, a - *current)?;
    vector(out, b - a)?;
    vector(out, end - b)?;
    out.push(8);
    *current = end;
    Ok(())
}

fn quantized(point: Point) -> Result<Point, Error> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x.abs() > 16000.0
        || point.y.abs() > 16000.0
    {
        return Err(source_error(
            "coordinates must be finite and within -16000..16000",
        ));
    }
    Ok(Point::new(
        (point.x * 65536.0).round() / 65536.0,
        (point.y * 65536.0).round() / 65536.0,
    ))
}

fn vector(out: &mut Vec<u8>, delta: kurbo::Vec2) -> Result<(), Error> {
    number(out, delta.x)?;
    number(out, delta.y)
}

pub(super) fn number(out: &mut Vec<u8>, value: f64) -> Result<(), Error> {
    if !value.is_finite() || value.abs() > 32000.0 {
        return Err(source_error("charstring operand outside -32000..32000"));
    }
    if value.fract() == 0.0 {
        integer(out, value as i32);
    } else {
        let mut numerator = (value * 65536.0).round() as i32;
        if numerator % 65536 == 0 {
            integer(out, numerator / 65536);
            return Ok(());
        }
        let mut denominator = 65536;
        // Type 1 interpreters switch to large-integer division based on the
        // numerator. A small numerator with denominator 65536 overflows the
        // denominator's fixed-point conversion; rescale the exact ratio.
        if numerator.abs() <= 32000 {
            let factor = 32000 / numerator.abs() + 1;
            numerator *= factor;
            denominator *= factor;
        }
        integer(out, numerator);
        integer(out, denominator);
        out.extend_from_slice(&[12, 12]); // div
    }
    Ok(())
}

fn integer(out: &mut Vec<u8>, value: i32) {
    if (-107..=107).contains(&value) {
        out.push((value + 139) as u8);
    } else if (108..=1131).contains(&value) {
        let n = value - 108;
        out.extend_from_slice(&[(247 + n / 256) as u8, (n % 256) as u8]);
    } else if (-1131..=-108).contains(&value) {
        let n = -value - 108;
        out.extend_from_slice(&[(251 + n / 256) as u8, (n % 256) as u8]);
    } else {
        out.push(255);
        out.extend_from_slice(&value.to_be_bytes());
    }
}
