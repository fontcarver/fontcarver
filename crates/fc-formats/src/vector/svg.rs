//! SVG font parsing follows W3C SVG 1.1 Fonts, not the sfnt SVG color table.

use std::{collections::BTreeSet, path::Path};

use fc_model::{Contour, Font, Glyph, Layer, MasterId};
use roxmltree::{Document, Node, ParsingOptions};

use super::{
    atomic_write, glyph_path, read_text, source_error, valid_number, MAX_ELEMENTS, MAX_FILE_BYTES,
};
use crate::Error;

const FORMAT: &str = "SVG font";
const NS: &str = "http://www.w3.org/2000/svg";
const MAX_GLYPHS: usize = 65535;

fn error(reason: impl Into<String>) -> Error {
    source_error(FORMAT, reason)
}

pub(super) fn read(path: &Path) -> Result<Font, Error> {
    let text = read_text(path, FORMAT)?;
    if text.contains("<!DOCTYPE") {
        return Err(error("DTD declarations are unsupported"));
    }
    let doc = Document::parse_with_options(
        &text,
        ParsingOptions {
            nodes_limit: 200_000,
            ..ParsingOptions::default()
        },
    )
    .map_err(|e| error(e.to_string()))?;
    let root = doc.root_element();
    if !root.has_tag_name("svg") {
        return Err(error("expected SVG root"));
    }
    let mut fonts = Vec::new();
    for node in doc.descendants().filter(Node::is_element) {
        if node.tag_name().namespace().is_some_and(|ns| ns != NS) {
            return Err(error("foreign elements are unsupported"));
        }
        if node.ancestors().take(33).count() > 32 {
            return Err(error("XML nesting exceeds limit"));
        }
        match node.tag_name().name() {
            "svg" => attributes(node, &["version", "width", "height", "viewBox", "id"])?,
            "defs" => attributes(node, &["id"])?,
            "font" => {
                attributes(
                    node,
                    &["id", "horiz-adv-x", "horiz-origin-x", "horiz-origin-y"],
                )?;
                fonts.push(node);
            }
            "font-face" | "glyph" | "missing-glyph" => {}
            "title" | "desc" => {
                attributes(node, &[])?;
                if node.children().any(|n| n.is_element()) {
                    return Err(error("nested metadata is unsupported"));
                }
            }
            name => return Err(error(format!("unsupported element {name}"))),
        }
    }
    if fonts.len() != 1 {
        return Err(error("exactly one font is required"));
    }
    let source = fonts[0];
    if source.children().filter(Node::is_element).any(|n| {
        !matches!(
            n.tag_name().name(),
            "font-face" | "glyph" | "missing-glyph" | "title" | "desc"
        )
    }) {
        return Err(error("unsupported font child"));
    }
    for node in doc
        .descendants()
        .filter(Node::is_element)
        .filter(|n| matches!(n.tag_name().name(), "font-face" | "glyph" | "missing-glyph"))
    {
        if node.parent() != Some(source) {
            return Err(error("font data must be direct children of font"));
        }
    }
    for attr in ["horiz-origin-x", "horiz-origin-y"] {
        if number(source, attr, 0.0)? != 0.0 {
            return Err(error("nonzero font origins are unsupported"));
        }
    }
    let advance = number(source, "horiz-adv-x", 0.0)?;
    if advance < 0.0 {
        return Err(error("negative advance width"));
    }
    let faces: Vec<_> = source
        .children()
        .filter(|n| n.has_tag_name("font-face"))
        .collect();
    if faces.len() != 1 {
        return Err(error("exactly one font-face is required"));
    }
    let face = faces[0];
    attributes(
        face,
        &[
            "font-family",
            "units-per-em",
            "ascent",
            "descent",
            "cap-height",
            "x-height",
            "font-style",
            "font-weight",
            "id",
        ],
    )?;
    no_graphics(face)?;
    let upem = number(face, "units-per-em", 1000.0)?;
    if upem.fract() != 0.0 || !(1.0..=65535.0).contains(&upem) {
        return Err(error("units per em must be an integer from 1 to 65535"));
    }
    let mut font = Font::new_empty(
        face.attribute("font-family").unwrap_or("SVG Font"),
        upem as u16,
    );
    font.info.style_name = match face.attribute("font-style").unwrap_or("normal") {
        "normal" => "Regular",
        "italic" => "Italic",
        "oblique" => "Oblique",
        _ => return Err(error("unsupported font-style")),
    }
    .into();
    if let Some(weight) = face.attribute("font-weight") {
        let weight = match weight {
            "normal" => 400,
            "bold" => 700,
            _ => weight
                .parse()
                .map_err(|_| error("unsupported font-weight"))?,
        };
        if !(1..=1000).contains(&weight) {
            return Err(error("invalid font-weight"));
        }
        font.info.open_type_os2_weight_class = Some(weight);
    }
    let metrics = &mut font.masters[0].metrics;
    metrics.ascender = number(face, "ascent", upem * 0.8)?;
    metrics.descender = -number(face, "descent", upem * 0.2)?.abs();
    metrics.cap_height = number(face, "cap-height", 0.0)?;
    metrics.x_height = number(face, "x-height", 0.0)?;
    let master = font.default_master().id.clone();
    let mut count = 0usize;
    let mut elements = 0usize;
    let mut unicode_owners = BTreeSet::new();
    for node in source
        .children()
        .filter(|n| n.has_tag_name("glyph") || n.has_tag_name("missing-glyph"))
    {
        count += 1;
        if count > MAX_GLYPHS {
            return Err(error("glyph limit exceeded"));
        }
        attributes(node, &["glyph-name", "unicode", "horiz-adv-x", "d", "id"])?;
        no_graphics(node)?;
        let unicode = node
            .attribute("unicode")
            .map(|value| {
                let mut chars = value.chars();
                let first = chars
                    .next()
                    .ok_or_else(|| error("empty Unicode assignment"))?;
                if chars.next().is_some() {
                    return Err(error("ligature Unicode sequences are unsupported"));
                }
                if !unicode_owners.insert(first as u32) {
                    return Err(error("ambiguous Unicode mapping"));
                }
                Ok(first as u32)
            })
            .transpose()?;
        let missing = node.has_tag_name("missing-glyph");
        if missing && (unicode.is_some() || node.attribute("glyph-name").is_some()) {
            return Err(error(
                "missing-glyph cannot have a name or Unicode assignment",
            ));
        }
        let name = if missing {
            ".notdef".into()
        } else {
            node.attribute("glyph-name")
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    unicode.map_or_else(
                        || format!("glyph{count}"),
                        |code| {
                            if code <= 0xffff {
                                format!("uni{code:04X}")
                            } else {
                                format!("u{code:05X}")
                            }
                        },
                    )
                })
        };
        if name.is_empty()
            || name.contains(',')
            || name.len() > 1024
            || (!missing && name == ".notdef")
        {
            return Err(error("invalid or multiple glyph names"));
        }
        let mut layer = Layer::new_master(master.clone());
        layer.width = number(node, "horiz-adv-x", advance)?;
        if layer.width < 0.0 {
            return Err(error("negative advance width"));
        }
        let path = super::svg_path::parse(node.attribute("d").unwrap_or(""))?;
        elements = elements.saturating_add(path.elements().len());
        if elements > MAX_ELEMENTS {
            return Err(error("total path element limit exceeded"));
        }
        layer.contours = Contour::from_bezpath(&path);
        for contour in &mut layer.contours {
            contour.closed = true;
        }
        if let Some(existing) = font.glyph_mut(&name) {
            if unicode.is_none()
                || existing.layers[0].contours != layer.contours
                || existing.layers[0].width != layer.width
            {
                return Err(error("conflicting duplicate glyph definition"));
            }
            existing.unicodes.extend(unicode);
        } else {
            let mut glyph = Glyph::new(name);
            glyph.unicodes.extend(unicode);
            glyph.layers.push(layer);
            font.insert_glyph(glyph).map_err(|e| error(e.to_string()))?;
        }
    }
    Ok(font)
}

fn attributes(node: Node<'_, '_>, allowed: &[&str]) -> Result<(), Error> {
    for attribute in node.attributes() {
        if attribute.namespace().is_some() || !allowed.contains(&attribute.name()) {
            return Err(error(format!(
                "unsupported {} attribute {}",
                node.tag_name().name(),
                attribute.name()
            )));
        }
    }
    Ok(())
}

fn no_graphics(node: Node<'_, '_>) -> Result<(), Error> {
    if node
        .children()
        .any(|n| n.is_element() || n.is_text() && !n.text().unwrap_or("").trim().is_empty())
    {
        return Err(error(
            "child graphics or text are unsupported; use glyph d outlines",
        ));
    }
    Ok(())
}

fn number(node: Node<'_, '_>, name: &str, default: f64) -> Result<f64, Error> {
    let value = node
        .attribute(name)
        .map(|s| s.trim().parse::<f64>())
        .transpose()
        .map_err(|_| error(format!("invalid {name}")))?
        .unwrap_or(default);
    if !valid_number(value) {
        return Err(error(format!("nonfinite or out-of-range {name}")));
    }
    Ok(value)
}

pub(super) fn write(font: &Font, master: &MasterId, path: &Path) -> Result<(), Error> {
    if font.has_unapplied_feature_draft() {
        return Err(Error::UnappliedFeatureDraft);
    }
    let source = font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.to_string()))?;
    if font.info.units_per_em == 0 {
        return Err(error("units per em must be positive"));
    }
    let metrics = &source.metrics;
    if [
        metrics.ascender,
        metrics.descender,
        metrics.cap_height,
        metrics.x_height,
    ]
    .iter()
    .any(|v| !valid_number(*v))
    {
        return Err(error("invalid font metrics"));
    }
    if metrics.ascender < 0.0 || metrics.descender > 0.0 {
        return Err(error(
            "SVG requires nonnegative ascent and nonpositive model descent",
        ));
    }
    let family = escape(&font.info.family_name)?;
    let style_name = font.info.style_name.to_ascii_lowercase();
    let style = if style_name.contains("oblique") {
        "oblique"
    } else if style_name.contains("italic") || metrics.italic_angle != 0.0 {
        "italic"
    } else {
        "normal"
    };
    let weight = font.info.open_type_os2_weight_class.unwrap_or(400);
    if !(1..=1000).contains(&weight) {
        return Err(error("invalid font weight"));
    }
    let mut output = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"{NS}\" version=\"1.1\"><defs><font id=\"FontCarver\" horiz-adv-x=\"0\">\n<font-face font-family=\"{family}\" units-per-em=\"{}\" ascent=\"{}\" descent=\"{}\" cap-height=\"{}\" x-height=\"{}\" font-style=\"{style}\" font-weight=\"{weight}\"/>\n", font.info.units_per_em, metrics.ascender, -metrics.descender, metrics.cap_height, metrics.x_height);
    let mut elements = 0usize;
    let mut count = 0usize;
    let mut unicodes = BTreeSet::new();
    for glyph in font.glyphs.values().filter(|g| g.export) {
        count = count.saturating_add(glyph.unicodes.len().max(1));
        if count > MAX_GLYPHS {
            return Err(error("output glyph limit exceeded"));
        }
        let geometry = glyph_path(font, master, &glyph.name, FORMAT)?;
        elements = elements.saturating_add(geometry.elements().len());
        if elements > MAX_ELEMENTS {
            return Err(error("total path element limit exceeded"));
        }
        let layer = glyph
            .master_layer(master)
            .ok_or_else(|| error("missing master layer"))?;
        let name = escape(glyph.name.as_ref())?;
        if name.is_empty() || glyph.name.as_ref().contains(',') || glyph.name.as_ref().len() > 1024
        {
            return Err(error("invalid glyph name"));
        }
        if !glyph.variation_selectors.is_empty() {
            return Err(error("variation-selector mappings are unsupported"));
        }
        let d = geometry.to_svg();
        if glyph.name.as_ref() == ".notdef" {
            if !glyph.unicodes.is_empty() {
                return Err(error("missing glyph cannot have Unicode mappings"));
            }
            output.push_str(&format!(
                "<missing-glyph horiz-adv-x=\"{}\" d=\"{d}\"/>\n",
                layer.width
            ));
        } else {
            for code in if glyph.unicodes.is_empty() {
                vec![None]
            } else {
                glyph.unicodes.iter().map(|code| Some(*code)).collect()
            } {
                let unicode = if let Some(code) = code {
                    let scalar = char::from_u32(code)
                        .filter(|c| xml_char(*c))
                        .ok_or_else(|| error("Unicode mapping is not an XML character"))?;
                    if !unicodes.insert(code) {
                        return Err(error("ambiguous or repeated Unicode mapping"));
                    }
                    format!(" unicode=\"&#x{:X};\"", scalar as u32)
                } else {
                    String::new()
                };
                if output
                    .len()
                    .saturating_add(name.len())
                    .saturating_add(d.len())
                    .saturating_add(256)
                    > MAX_FILE_BYTES
                {
                    return Err(error("output byte limit exceeded"));
                }
                output.push_str(&format!(
                    "<glyph glyph-name=\"{name}\"{unicode} horiz-adv-x=\"{}\" d=\"{d}\"/>\n",
                    layer.width
                ));
            }
        }
        if count > MAX_GLYPHS || output.len() > MAX_FILE_BYTES {
            return Err(error("output glyph or byte limit exceeded"));
        }
    }
    output.push_str("</font></defs></svg>\n");
    atomic_write(path, output.as_bytes())
}

fn xml_char(value: char) -> bool {
    matches!(value as u32, 9 | 10 | 13 | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
}

fn escape(value: &str) -> Result<String, Error> {
    let mut result = String::new();
    for ch in value.chars() {
        if !xml_char(ch) {
            return Err(error("metadata contains an invalid XML character"));
        }
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\n' => result.push_str("&#10;"),
            '\r' => result.push_str("&#13;"),
            '\t' => result.push_str("&#9;"),
            ch => result.push(ch),
        }
    }
    Ok(result)
}
