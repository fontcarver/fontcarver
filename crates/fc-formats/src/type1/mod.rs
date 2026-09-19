//! Static PostScript Type 1 PFA/PFB import and export.
//!
//! The writer implements Adobe's Type 1 format (T1_SPEC.pdf), without adapting
//! FontForge code. Output uses unhinted native charstrings and a 256-slot encoding;
//! OpenType layout, editable component relationships and extra layers are omitted.

mod guards;
mod reader;
mod writer;

use std::{collections::BTreeSet, io::Write, path::Path};

use fc_model::{Font, MasterId};

use crate::Error;

const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
const SOURCE_KEY: &str = "org.fontcarver.type1";

/// A format limitation to disclose before Type 1 export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type1Warning {
    /// Stable localization key identifying the omitted data.
    pub code: &'static str,
    /// Human-readable fallback explanation.
    pub message: String,
}

/// Reads a static horizontal PFA or PFB font, retaining fractional cubic outlines.
///
/// Imported character encoding is retained in font user data. Unicode assignments
/// are inferred from Adobe glyph names, because Type 1 has no Unicode cmap.
/// Fonts using `sbw`, nonhorizontal FontMatrix advances, Multiple Master data or
/// PostScript metric override procedures are explicitly unsupported.
/// Large charstring integers must form locally verifiable adjacent division
/// operands; uncertain computed or cross-subroutine arithmetic is rejected.
pub fn read_type1(path: &Path) -> Result<Font, Error> {
    reader::read(path)
}

/// Writes a selected master as an unhinted static PFA (`false`) or PFB (`true`).
///
/// Validation and complete serialization precede the atomic destination replace.
/// Call [`loss_warnings`] before exporting to disclose Type 1's format limitations.
pub fn write_type1(font: &Font, master: &MasterId, path: &Path, binary: bool) -> Result<(), Error> {
    let bytes = writer::encode(font, master, binary)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| Error::io(parent, e))?;
    temporary
        .write_all(&bytes)
        .map_err(|e| Error::io(path, e))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| Error::io(path, e))?;
    temporary
        .persist(path)
        .map_err(|e| Error::io(path, e.error))?;
    Ok(())
}

/// Returns human-readable losses to disclose before writing Type 1.
///
/// Messages describe format behavior, not validation errors. Glyph names are
/// preserved; Unicode assignments inconsistent with those names cannot survive
/// a native Type 1 roundtrip. Imported 8-bit encodings are retained where possible.
pub fn loss_warnings(font: &Font, _master: &MasterId) -> Vec<Type1Warning> {
    let mut warnings = vec![
        Type1Warning { code: "type1-layout-loss", message: "Type 1 contains only static outlines, widths and an 8-bit encoding; OpenType layout, kerning, anchors, extra masters and source layers are not retained.".into() },
        Type1Warning { code: "type1-hinting-loss", message: "Type 1 output is unhinted; imported PostScript hints and TrueType instructions are not retained.".into() },
    ];
    let glyphs: Vec<_> = font.glyphs.values().filter(|glyph| glyph.export).collect();
    let inferred = reader::unicode_assignments(
        glyphs.iter().enumerate().map(|(index, glyph)| {
            (
                read_fonts::types::GlyphId::new(index as u32),
                glyph.name.as_ref(),
            )
        }),
        &encoding_names(font),
    );
    if glyphs.iter().enumerate().any(|(index, glyph)| {
        glyph.unicodes.iter().any(|code| {
            !inferred
                .get(&(index as u32))
                .is_some_and(|codes| codes.contains(code))
        })
    }) {
        warnings.push(Type1Warning { code: "type1-unicode-loss", message: "Type 1 has no Unicode cmap; some Unicode assignments cannot be recovered from the preserved glyph names.".into() });
    }
    warnings
}

pub(super) fn encoding_names(font: &Font) -> Vec<String> {
    let mut names: BTreeSet<&str> = font
        .glyphs
        .values()
        .filter(|glyph| glyph.export)
        .map(|glyph| glyph.name.as_ref())
        .collect();
    names.insert(".notdef");
    let mut encoding = vec![".notdef".to_owned(); 256];
    if let Some(original) = font
        .user_data
        .get(SOURCE_KEY)
        .and_then(|value| value.get("encoding"))
        .and_then(|value| value.as_array())
    {
        for (code, value) in original.iter().take(256).enumerate() {
            if let Some(name) = value.as_str().filter(|name| names.contains(name)) {
                encoding[code] = name.to_owned();
            }
        }
    } else {
        for glyph in font.glyphs.values().filter(|glyph| glyph.export) {
            for code in glyph.unicodes.iter().copied().filter(|code| *code < 256) {
                encoding[code as usize] = glyph.name.to_string();
            }
        }
    }
    encoding
}

fn source_error(reason: impl Into<String>) -> Error {
    Error::Source {
        format: "Type 1",
        reason: reason.into(),
    }
}
