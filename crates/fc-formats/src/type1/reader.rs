//! Bounded fontations Type 1 reader with an exact fixed-point outline sink.

use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

use fc_model::{Contour, Font, Glyph, Layer};
use kurbo::{Affine, BezPath};
use read_fonts::{
    ps::{
        cs::{CharstringContext, CommandSink},
        type1::Type1Font,
    },
    types::{Fixed, GlyphId},
};

use super::{source_error, MAX_FILE_BYTES, SOURCE_KEY};
use crate::Error;

pub(super) fn read(path: &Path) -> Result<Font, Error> {
    let file = File::open(path).map_err(|e| Error::io(path, e))?;
    let mut bytes = Vec::new();
    file.take((MAX_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::io(path, e))?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(source_error("input exceeds the 32 MiB limit"));
    }
    validate_framing(&bytes)?;
    let header = inspect_header(&bytes)?;
    let data = super::guards::check_horizontal_charstrings(&bytes)?;
    let source = Type1Font::new(&bytes).map_err(|e| source_error(e.to_string()))?;
    if !source.weight_vector().is_empty() {
        return Err(source_error(
            "Multiple Master Type 1 fonts require instancing before import",
        ));
    }
    if source.num_glyphs() > 65535 {
        return Err(source_error("more than 65535 glyphs"));
    }
    let (upem, transform) = normalized_matrix(header.font_matrix)?;
    let mut font = Font::default();
    font.info.units_per_em = upem;
    font.info.family_name = unescape_string(source.family_name().unwrap_or("Untitled"));
    font.info.style_name = unescape_string(source.weight().unwrap_or("Regular"));
    font.info.postscript_font_name = source.name().map(str::to_owned);
    font.info.postscript_full_name = source.full_name().map(unescape_string);
    font.info.copyright = header.notice;
    font.info.postscript_is_fixed_pitch =
        Some(header.fixed_pitch.unwrap_or(source.is_fixed_pitch()));
    font.info.postscript_underline_position = Some(
        header
            .underline_position
            .unwrap_or(f64::from(source.underline_position())),
    );
    font.info.postscript_underline_thickness = Some(
        header
            .underline_thickness
            .unwrap_or(f64::from(source.underline_thickness())),
    );
    font.masters[0].name = font.info.style_name.clone();
    let normalized_private = super::private::normalized(&data.private, transform);
    let unretained_private = normalized_private.is_none();
    font.masters[0].private_dict = normalized_private.unwrap_or_default();
    font.masters[0].metrics.italic_angle = header
        .italic_angle
        .unwrap_or(f64::from(source.italic_angle()));
    let master = font.default_master().id.clone();
    let encoding: Vec<_> = (0..=255u8)
        .map(|code| {
            source
                .encoding()
                .and_then(|encoding| encoding.glyph_name(code))
                .unwrap_or(".notdef")
                .to_owned()
        })
        .collect();
    let mut unicode = unicode_assignments(source.glyph_names(), &encoding);
    // Fontations' draw() intentionally truncates fractions for FreeType-compatible
    // rasterization. Evaluate into our own sink and apply its normalized matrix.
    let mut total_elements = 0usize;
    let mut hint_budget = 2_000_000usize;
    let mut unretained_glyph_hints = Vec::new();
    for (gid, name) in source.glyph_names() {
        if font.glyph(name).is_some() {
            return Err(source_error(format!("duplicate glyph name {name}")));
        }
        let mut sink = Sink::default();
        let width = source
            .evaluate_charstring(gid, &mut sink)
            .map_err(|e| source_error(format!("glyph {name}: {e}")))?;
        total_elements += sink.count;
        if sink.count > 100_000 || total_elements > 2_000_000 {
            return Err(source_error("outline complexity limit exceeded"));
        }
        let mut glyph = Glyph::new(name);
        glyph.unicodes = unicode.remove(&gid.to_u32()).unwrap_or_default();
        let mut layer = Layer::new_master(master.clone());
        layer.width = width
            .map(|w| fixed(w) * transform.as_coeffs()[0])
            .unwrap_or(0.0);
        if !layer.width.is_finite() || layer.width < 0.0 {
            return Err(source_error(format!("glyph {name}: invalid advance")));
        }
        layer.contours = Contour::from_bezpath(&(transform * sink.path));
        if let Some(program) = data.glyphs.get(name) {
            match super::hints::read(program, &source, transform, &mut hint_budget) {
                Ok(hints) => layer.hints = hints,
                Err(reason) => {
                    tracing::debug!(glyph = name, reason, "Type 1 hints not retained");
                    unretained_glyph_hints.push(name.to_owned());
                }
            }
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph)
            .map_err(|e| source_error(e.to_string()))?;
    }
    font.user_data = serde_json::json!({(SOURCE_KEY): {
        "encoding": encoding,
        "unretained_glyph_hints": unretained_glyph_hints,
        "unretained_private_hints": unretained_private,
        "computed_private_fields": data.computed_private_fields,
    }});
    if unretained_private {
        font.user_data[SOURCE_KEY]["original_private"] =
            serde_json::to_value(&data.private).map_err(|error| source_error(error.to_string()))?;
    }
    let bbox = source.bbox();
    let bbox = transform.transform_rect_bbox(kurbo::Rect::new(
        fixed(bbox.x_min),
        fixed(bbox.y_min),
        fixed(bbox.x_max),
        fixed(bbox.y_max),
    ));
    font.masters[0].metrics.ascender = bbox.y1;
    font.masters[0].metrics.descender = bbox.y0;
    Ok(font)
}

fn fixed(value: Fixed) -> f64 {
    f64::from(value.to_bits()) / 65536.0
}

pub(super) fn unicode_assignments<'a>(
    glyphs: impl Iterator<Item = (GlyphId, &'a str)>,
    encoding: &[String],
) -> BTreeMap<u32, Vec<u32>> {
    let glyphs: Vec<_> = glyphs.collect();
    let names: BTreeMap<_, _> = glyphs.iter().map(|(gid, name)| (*name, *gid)).collect();
    let charmap = read_fonts::ps::charmap::Charmap::from_glyph_names(glyphs.iter().copied());
    let mut encoded = BTreeMap::<u32, Vec<u32>>::new();
    for (code, name) in encoding
        .iter()
        .enumerate()
        .filter(|(_, name)| *name != ".notdef")
    {
        if let Some(gid) = names.get(name.as_str()) {
            encoded.entry(gid.to_u32()).or_default().push(code as u32);
        }
    }
    let mut candidates = BTreeMap::<u32, (u8, u32)>::new();
    for (tagged, gid) in charmap.iter() {
        let code = tagged & 0x7fff_ffff;
        if char::from_u32(code).is_none() {
            continue;
        }
        let variant = tagged & 0x8000_0000 != 0;
        let slots = encoded.get(&gid.to_u32());
        // AGL exposes alternates as fallback candidates. Assigning their base
        // scalar to every glyph creates an invalid cmap. Native encoding picks
        // an alternate explicitly; unencoded dotted glyphs remain unassigned.
        if variant && slots.is_none() {
            continue;
        }
        let priority = match slots {
            Some(slots) if slots.contains(&code) => 0,
            Some(_) => 1,
            None => 2,
        };
        let candidate = (priority, gid.to_u32());
        candidates
            .entry(code)
            .and_modify(|old| *old = (*old).min(candidate))
            .or_insert(candidate);
    }
    let mut mappings = BTreeMap::<u32, Vec<u32>>::new();
    for (code, (_, gid)) in candidates {
        mappings.entry(gid).or_default().push(code);
    }
    mappings
}

fn normalized_matrix(matrix: Option<[f64; 6]>) -> Result<(u16, Affine), Error> {
    let matrix = matrix.unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);
    if matrix[1] != 0.0 {
        return Err(source_error(
            "FontMatrix with nonhorizontal advances is not supported",
        ));
    }
    if matrix[0] <= 0.0 || matrix[3] == 0.0 {
        return Err(source_error(
            "invalid or reflected FontMatrix advance scale",
        ));
    }
    let units = (1.0 / matrix[3].abs()).round();
    if !units.is_finite() || !(1.0..=65535.0).contains(&units) {
        return Err(source_error(
            "FontMatrix units per em must be between 1 and 65535",
        ));
    }
    let normalized = matrix.map(|value| {
        let scaled = value * units;
        // Decimal PostScript output may approximate a reciprocal UPEM. Remove
        // only floating serialization noise, retaining meaningful fractions.
        if (scaled - scaled.round()).abs() < 1e-9 {
            scaled.round()
        } else {
            scaled
        }
    });
    Ok((units as u16, Affine::new(normalized)))
}

// Type 1 is a declarative subset of PostScript. Inspect its clear-text dictionary
// without executing code. The locked reader spells isFixedPitch with an uppercase
// initial I, so preserve the correctly spelled native property here as well.
#[derive(Default)]
struct HeaderMetadata {
    fixed_pitch: Option<bool>,
    italic_angle: Option<f64>,
    underline_position: Option<f64>,
    underline_thickness: Option<f64>,
    font_matrix: Option<[f64; 6]>,
    notice: Option<String>,
}

fn inspect_header(bytes: &[u8]) -> Result<HeaderMetadata, Error> {
    let mut input = bytes;
    if bytes.starts_with(&[0x80, 1]) {
        let size = u32::from_le_bytes(
            bytes[2..6]
                .try_into()
                .map_err(|_| source_error("invalid PFB header"))?,
        ) as usize;
        input = &bytes[6..6 + size];
    }
    let mut header = HeaderMetadata::default();
    let mut procedure_depth = 0usize;
    while let Some(token) = header_token(&mut input)? {
        if token == b"{" {
            procedure_depth += 1;
            if procedure_depth > 64 {
                return Err(source_error("PostScript procedure nesting limit exceeded"));
            }
            continue;
        }
        if token == b"}" {
            procedure_depth = procedure_depth.saturating_sub(1);
            continue;
        }
        if procedure_depth > 0 {
            continue;
        }
        match token {
            b"eexec" => break,
            b"/Metrics" | b"/Metrics2" | b"/CDevProc" => {
                return Err(source_error(
                    "PostScript Metrics, Metrics2 and CDevProc overrides are not supported",
                ))
            }
            b"/FontMatrix" => {
                let opening = header_token(&mut input)?;
                let closing = match opening {
                    Some(b"[") => b"]".as_slice(),
                    Some(b"{") => b"}".as_slice(),
                    _ => return Err(source_error("invalid FontMatrix array")),
                };
                let mut matrix = [0.0; 6];
                for coefficient in &mut matrix {
                    *coefficient = header_number(&mut input)?;
                }
                if header_token(&mut input)? != Some(closing) {
                    return Err(source_error("invalid FontMatrix array length"));
                }
                header.font_matrix = Some(matrix);
            }
            b"/Notice" | b"/Copyright" => {
                let value = header_token(&mut input)?
                    .ok_or_else(|| source_error("missing Notice string"))?;
                if value.starts_with(b"(") && value.ends_with(b")") {
                    header.notice = Some(unescape_bytes(&value[1..value.len() - 1]));
                } else {
                    return Err(source_error("unsupported computed Notice string"));
                }
            }
            b"/isFixedPitch" | b"/IsFixedPitch" => {
                header.fixed_pitch = match header_token(&mut input)? {
                    Some(b"true") => Some(true),
                    Some(b"false") => Some(false),
                    _ => return Err(source_error("unsupported computed isFixedPitch value")),
                };
            }
            b"/ItalicAngle" | b"/UnderlinePosition" | b"/UnderlineThickness" => {
                let value = header_token(&mut input)?
                    .and_then(|raw| std::str::from_utf8(raw).ok())
                    .and_then(|raw| raw.parse::<f64>().ok())
                    .filter(|v| v.is_finite() && v.abs() <= 32767.0)
                    .ok_or_else(|| source_error("unsupported or invalid Type 1 font metric"))?;
                match token {
                    b"/ItalicAngle" => header.italic_angle = Some(value),
                    b"/UnderlinePosition" => header.underline_position = Some(value),
                    _ if value >= 0.0 => header.underline_thickness = Some(value),
                    _ => return Err(source_error("negative underline thickness")),
                }
            }
            b"/PaintType" => {
                if header_token(&mut input)? != Some(b"0".as_slice()) {
                    return Err(source_error("stroked Type 1 PaintType is not supported"));
                }
            }
            b"/FontType" => {
                if header_token(&mut input)? != Some(b"1".as_slice()) {
                    return Err(source_error("only FontType 1 is supported"));
                }
            }
            _ => {}
        }
    }
    Ok(header)
}

pub(super) fn header_token<'a>(input: &mut &'a [u8]) -> Result<Option<&'a [u8]>, Error> {
    loop {
        while input.first().is_some_and(u8::is_ascii_whitespace) {
            *input = &input[1..];
        }
        let Some(&first) = input.first() else {
            return Ok(None);
        };
        if first == b'%' {
            let end = input
                .iter()
                .position(|b| *b == b'\n' || *b == b'\r')
                .unwrap_or(input.len());
            *input = &input[end..];
            continue;
        }
        if first == b'(' {
            let mut depth = 1usize;
            let mut end = 1;
            while end < input.len() && depth > 0 {
                match input[end] {
                    b'\\' => end += 1,
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                end += 1;
                if depth > 64 {
                    return Err(source_error("PostScript string nesting limit exceeded"));
                }
            }
            if depth != 0 || end > input.len() {
                return Err(source_error("unterminated PostScript string"));
            }
            let (token, rest) = input.split_at(end);
            *input = rest;
            return Ok(Some(token));
        }
        let length = if b"[]{}<>".contains(&first) {
            1
        } else {
            input
                .iter()
                .skip(1)
                .position(|b| b.is_ascii_whitespace() || b"()[]{}<>/%".contains(b))
                .map_or(input.len(), |pos| pos + 1)
        };
        let (token, rest) = input.split_at(length);
        *input = rest;
        return Ok(Some(token));
    }
}

// The locked reader returns literal-string contents without interpreting the
// PostScript backslash escapes; decode those before presenting editable names.
fn unescape_string(value: &str) -> String {
    unescape_bytes(value.as_bytes())
}

fn unescape_bytes(value: &[u8]) -> String {
    let mut bytes = value.iter().copied().peekable();
    let mut out = Vec::new();
    while let Some(byte) = bytes.next() {
        if byte != b'\\' {
            out.push(byte);
            continue;
        }
        let Some(escaped) = bytes.next() else {
            break;
        };
        match escaped {
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'b' => out.push(8),
            b'f' => out.push(12),
            b'\n' => {}
            b'\r' => {
                if bytes.peek() == Some(&b'\n') {
                    bytes.next();
                }
            }
            b'0'..=b'7' => {
                let mut value = u16::from(escaped - b'0');
                for _ in 0..2 {
                    if let Some(next) = bytes.peek().copied().filter(|b| (b'0'..=b'7').contains(b))
                    {
                        bytes.next();
                        value = value * 8 + u16::from(next - b'0');
                    } else {
                        break;
                    }
                }
                out.push(value as u8);
            }
            _ => out.push(escaped),
        }
    }
    String::from_utf8(out)
        .unwrap_or_else(|error| error.into_bytes().into_iter().map(char::from).collect())
}

fn header_number(input: &mut &[u8]) -> Result<f64, Error> {
    header_token(input)?
        .and_then(|token| std::str::from_utf8(token).ok())
        .and_then(|text| text.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .ok_or_else(|| source_error("invalid PostScript numeric value"))
}

#[derive(Default)]
struct Sink {
    path: BezPath,
    count: usize,
}

impl Sink {
    fn push(&mut self, element: kurbo::PathEl) {
        self.count += 1;
        if self.count <= 100_000 {
            self.path.push(element);
        }
    }
}

impl CommandSink for Sink {
    fn move_to(&mut self, x: Fixed, y: Fixed) {
        self.push(kurbo::PathEl::MoveTo((fixed(x), fixed(y)).into()));
    }
    fn line_to(&mut self, x: Fixed, y: Fixed) {
        self.push(kurbo::PathEl::LineTo((fixed(x), fixed(y)).into()));
    }
    fn curve_to(&mut self, ax: Fixed, ay: Fixed, bx: Fixed, by: Fixed, x: Fixed, y: Fixed) {
        self.push(kurbo::PathEl::CurveTo(
            (fixed(ax), fixed(ay)).into(),
            (fixed(bx), fixed(by)).into(),
            (fixed(x), fixed(y)).into(),
        ));
    }
    fn close(&mut self) {
        self.push(kurbo::PathEl::ClosePath);
    }
}

fn validate_framing(bytes: &[u8]) -> Result<(), Error> {
    if bytes.first() != Some(&0x80) {
        if !bytes.windows(11).any(|w| w == b"cleartomark") {
            return Err(source_error("PFA is truncated or missing cleartomark"));
        }
        return Ok(());
    }
    let mut offset = 0usize;
    let mut saw_binary = false;
    let mut trailer = false;
    loop {
        let tag = bytes
            .get(offset..offset + 2)
            .ok_or_else(|| source_error("truncated PFB segment"))?;
        if tag == [0x80, 3] {
            if !saw_binary
                || !trailer
                || bytes
                    .get(offset + 2..)
                    .is_none_or(|tail| !tail.iter().all(u8::is_ascii_whitespace))
            {
                return Err(source_error("invalid PFB end marker"));
            }
            return Ok(());
        }
        if tag[0] != 0x80
            || !matches!(tag[1], 1 | 2)
            || (offset == 0 && tag[1] != 1)
            || (trailer && tag[1] == 2)
        {
            return Err(source_error("invalid PFB segment order"));
        }
        let length: [u8; 4] = bytes
            .get(offset + 2..offset + 6)
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| source_error("truncated PFB length"))?;
        let end = offset
            .checked_add(6)
            .and_then(|v| v.checked_add(u32::from_le_bytes(length) as usize))
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| source_error("PFB segment exceeds file"))?;
        if tag[1] == 2 {
            saw_binary = true;
        } else if saw_binary {
            trailer = true;
        }
        offset = end;
    }
}
