//! Standalone, single-font CFF1 interchange (Adobe Technical Note 5176).
//!
//! Bare CFF has no OpenType character map, layout tables or name records. The
//! importer retains CFF metadata and reports the absence of that surrounding
//! information; unambiguous production glyph names supply inferred Unicode.

use std::{io::Read, path::Path};

use fc_model::{Font, GlyphName, MasterId, SourceProvenance};
use read_fonts::{
    ps::{cff::CffFontRef, cs::CommandSink},
    tables::cff::Cff,
    types::{Fixed, GlyphId, Tag},
    FontRead,
};
use sha2::{Digest, Sha256};
use write_fonts::{
    tables::{head::Head, hhea::Hhea, maxp::Maxp},
    FontBuilder,
};

use crate::{Error, ReadOptions, ReadOutcome, Warning};

/// Maximum accepted standalone CFF input size.
pub const MAX_BYTES: usize = 128 * 1024 * 1024;

/// Reads a single standalone CFF1 font into the editable document model.
///
/// Use [`read_outcome`] when the caller displays import diagnostics. CFF2 and
/// multi-font CFF1 sets are rejected because neither can be represented here as
/// one standalone static font without choosing additional context.
pub fn read_bytes(bytes: &[u8]) -> Result<Font, Error> {
    read_outcome(bytes)?
        .fonts
        .into_iter()
        .next()
        .ok_or_else(|| Error::parse("CFF", "missing font"))
}

/// Reads a standalone CFF file with a bounded allocation and import diagnostics.
pub fn read_path(path: &Path) -> Result<ReadOutcome, Error> {
    let file = std::fs::File::open(path).map_err(|source| Error::Io {
        path: path.into(),
        source,
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| Error::Io {
            path: path.into(),
            source,
        })?;
    let mut outcome = read_outcome(&bytes)?;
    for font in &mut outcome.fonts {
        if let Some(source) = &mut font.extra.source {
            source.path = Some(path.to_string_lossy().into_owned());
        }
    }
    Ok(outcome)
}

/// Reads CFF1 with diagnostics about inferred metadata and hint limitations.
pub fn read_outcome(bytes: &[u8]) -> Result<ReadOutcome, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::parse("CFF", "input exceeds 128 MiB"));
    }
    if bytes.first() != Some(&1) {
        return Err(Error::parse("CFF", "expected standalone CFF1 data"));
    }
    let table = Cff::read(bytes.into()).map_err(|error| Error::parse("CFF", error))?;
    if table.names().count() != 1 || table.top_dicts().count() != 1 {
        return Err(Error::parse(
            "CFF",
            "expected exactly one font in the CFF font set",
        ));
    }
    let source = CffFontRef::new_cff(bytes, 0, None).map_err(|error| Error::parse("CFF", error))?;
    let count = u16::try_from(source.num_glyphs())
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| Error::parse("CFF", "expected 1..=65535 glyphs"))?;
    let upem = u16::try_from(source.upem())
        .ok()
        .filter(|upem| (16..=16384).contains(upem))
        .ok_or_else(|| {
            Error::parse("CFF", "font matrix implies units per em outside 16..=16384")
        })?;

    // CFF outlines/hints already have a checked model importer. Supply only the
    // enclosing metrics it requires, then discard these synthetic tables.
    let mut widths = Vec::with_capacity(usize::from(count));
    let mut hmtx = Vec::with_capacity(usize::from(count) * 4);
    for gid in 0..count {
        let gid = GlyphId::new(u32::from(gid));
        let subfont_index = source
            .subfont_index(gid)
            .ok_or_else(|| Error::parse("CFF", "missing glyph subfont"))?;
        let subfont = source
            .subfont(subfont_index, &[])
            .map_err(|error| Error::parse("CFF Private DICT", error))?;
        let width = source
            .evaluate_charstring(&subfont, gid, &[], &mut WidthSink)
            .map_err(|error| Error::parse("CFF CharString", error))?
            .unwrap_or(Fixed::ZERO)
            .to_f64();
        let transform = source.transform(&subfont, None);
        let width = width * transform.matrix.xx.to_f64() + transform.matrix.dx.to_f64();
        if !width.is_finite() || !(0.0..=65535.0).contains(&width) {
            return Err(Error::parse("CFF", "advance width outside 0..=65535"));
        }
        widths.push(width);
        hmtx.extend_from_slice(&(width.round() as u16).to_be_bytes());
        hmtx.extend_from_slice(&0i16.to_be_bytes());
    }
    let head = Head {
        units_per_em: upem,
        ..Head::default()
    };
    let hhea = Hhea {
        number_of_h_metrics: count,
        ascender: ((f64::from(upem) * 0.8).round() as i16).into(),
        descender: (-(f64::from(upem) * 0.2).round() as i16).into(),
        caret_slope_rise: 1,
        ..Hhea::default()
    };
    let mut builder = FontBuilder::new();
    builder
        .add_table(&head)
        .map_err(|error| Error::parse("head", error))?;
    builder
        .add_table(&hhea)
        .map_err(|error| Error::parse("hhea", error))?;
    builder
        .add_table(&Maxp::new(count))
        .map_err(|error| Error::parse("maxp", error))?;
    builder.add_raw(Tag::new(b"hmtx"), hmtx);
    builder.add_raw(Tag::new(b"CFF "), bytes);
    let mut outcome = crate::read_bytes(
        &builder.build(),
        &ReadOptions {
            keep_original_bytes: false,
            decode_bitmaps: false,
            ..ReadOptions::default()
        },
    )?;
    let font = outcome
        .fonts
        .first_mut()
        .ok_or_else(|| Error::parse("CFF", "missing decoded font"))?;
    if let Some(metadata) = source.metadata() {
        let name = metadata.name().unwrap_or("Untitled-Regular");
        let (family, style) = name.rsplit_once('-').unwrap_or((name, "Regular"));
        font.info.family_name = metadata.family_name().unwrap_or(family).into();
        font.info.style_name = metadata.weight().unwrap_or(style).into();
        font.info.postscript_font_name = Some(name.into());
        font.info.postscript_full_name = metadata.full_name().map(str::to_owned);
        font.info.postscript_is_fixed_pitch = Some(metadata.is_fixed_pitch());
        font.info.postscript_underline_position = Some(metadata.underline_position().to_f64());
        font.info.postscript_underline_thickness = Some(metadata.underline_thickness().to_f64());
        if let Some(master) = font.masters.first_mut() {
            master.name = font.info.style_name.clone();
            master.metrics.italic_angle = metadata.italic_angle().to_f64();
        }
    }
    for (glyph, width) in font.glyphs.values_mut().zip(widths) {
        if !source.is_cid() {
            glyph.unicodes = unicode_from_name(glyph.name.as_ref()).into_iter().collect();
        }
        if let Some(layer) = glyph.layers.first_mut() {
            layer.width = width;
        }
    }
    font.extra.raw_tables.clear();
    font.info.open_type_head_created = None;
    font.extra.source = Some(SourceProvenance {
        path: None,
        format: "cff".into(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        table_tags: vec![],
        glyph_order: font.glyph_order().cloned().collect(),
        original_bytes: None,
    });
    outcome.warnings.push(Warning {
        code: "bare-cff-metadata",
        message: "Standalone CFF has no Unicode cmap, OpenType layout or sfnt metadata. Unicode values were inferred only from unambiguous glyph names; vertical metrics use defaults.".into(),
    });
    Ok(outcome)
}

/// Writes a selected static master as standalone CFF1 with editable PS hints.
///
/// Only CFF-representable data is emitted. Surrounding OpenType Unicode, layout,
/// color and name tables cannot be stored in this format. Components are
/// expanded by the existing CFF writer and a missing `.notdef` is inserted.
pub fn write_bytes(font: &Font, master: &MasterId) -> Result<Vec<u8>, crate::write::WriteError> {
    if font
        .glyphs
        .values()
        .flat_map(|glyph| &glyph.layers)
        .any(|layer| !layer.effects.is_empty() || layer.has_smart_corners())
    {
        return Err(crate::write::WriteError(
            "bake live corners and layer effects before standalone CFF export".into(),
        ));
    }
    let mut order = vec![GlyphName::from(".notdef")];
    order.extend(
        font.glyph_order()
            .filter(|name| name.as_ref() != ".notdef")
            .cloned(),
    );
    let output = crate::write::write_cff(font, master, &order, false)?;
    if output.bytes.len() > MAX_BYTES {
        return Err(crate::write::WriteError(
            "standalone CFF exceeds 128 MiB".into(),
        ));
    }
    Ok(output.bytes)
}

fn unicode_from_name(name: &str) -> Option<u32> {
    let value = if name.len() == 1 && name.as_bytes()[0].is_ascii_alphabetic() {
        u32::from(name.as_bytes()[0])
    } else if name == "space" {
        0x20
    } else if let Some(digits) = name
        .strip_prefix("uni")
        .filter(|digits| digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        u32::from_str_radix(digits, 16).ok()?
    } else if let Some(digits) = name.strip_prefix('u').filter(|digits| {
        (4..=6).contains(&digits.len()) && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        u32::from_str_radix(digits, 16).ok()?
    } else {
        return None;
    };
    char::from_u32(value).map(u32::from)
}

struct WidthSink;

impl CommandSink for WidthSink {
    fn move_to(&mut self, _: Fixed, _: Fixed) {}
    fn line_to(&mut self, _: Fixed, _: Fixed) {}
    fn curve_to(&mut self, _: Fixed, _: Fixed, _: Fixed, _: Fixed, _: Fixed, _: Fixed) {}
    fn close(&mut self) {}
}
