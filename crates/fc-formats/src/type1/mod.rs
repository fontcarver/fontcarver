//! Static PostScript Type 1 PFA/PFB import and export.
//!
//! The writer implements Adobe's Type 1 format (T1_SPEC.pdf), without adapting
//! FontForge code. Output uses native charstrings and a 256-slot encoding;
//! OpenType layout, editable component relationships and extra layers are omitted.

mod guards;
mod hint_writer;
mod hints;
mod private;
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

/// Writes a selected master as a static PFA (`false`) or PFB (`true`).
///
/// Retains private parameters, horizontal/vertical stems, point-bound replacement
/// masks and representable stem3 counter groups. Unsupported mechanisms produce
/// an error before replacing output.
///
/// Validation and complete serialization precede the atomic destination replace.
/// Call [`loss_warnings`] before exporting to disclose Type 1's format limitations.
pub fn write_type1(font: &Font, master: &MasterId, path: &Path, binary: bool) -> Result<(), Error> {
    crate::check_layer_effects(font)?;
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
pub fn loss_warnings(font: &Font, master: &MasterId) -> Vec<Type1Warning> {
    let mut warnings = vec![
        Type1Warning { code: "type1-layout-loss", message: "Type 1 contains only static outlines, widths and an 8-bit encoding; OpenType layout, kerning, anchors, extra masters and source layers are not retained.".into() },
    ];
    if font
        .master(master)
        .is_some_and(|master| master.tt != fc_model::TtMasterData::default())
        || font
            .glyphs
            .values()
            .filter(|glyph| glyph.export)
            .any(|glyph| {
                glyph.master_layer(master).is_some_and(|layer| {
                    layer
                        .tt_instructions
                        .as_ref()
                        .is_some_and(|program| !program.is_empty())
                })
            })
    {
        warnings.push(Type1Warning { code: "type1-hinting-loss", message: "Type 1 output cannot retain TrueType instructions. Supported PostScript stems and private values are retained.".into() });
    }
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

/// Reports Type 1 hint metadata that could not be retained during source import.
///
/// Diagnostics are stored in font user data, so saving and reopening an FCF does
/// not erase the import warning or the affected glyph names.
pub fn read_warnings(font: &Font) -> Vec<Type1Warning> {
    let Some(source) = font.user_data.get(SOURCE_KEY) else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    if source
        .get("computed_private_fields")
        .and_then(|value| value.as_array())
        .is_some_and(|values| !values.is_empty())
    {
        warnings.push(Type1Warning {
            code: "type1-import-private-computed",
            message: "Some Type 1 private hint fields use runtime PostScript and were not imported. Their field names remain in source metadata; literal parameters are retained.".into(),
        });
    }
    if let Some(glyphs) = source
        .get("unretained_glyph_hints")
        .and_then(|value| value.as_array())
        .filter(|values| !values.is_empty())
    {
        warnings.push(Type1Warning {
            code: "type1-import-hints-partial",
            message: format!("Type 1 hint metadata could not be fully retained for {} glyphs. Those glyphs have editable outlines without imported stems; private parameters are retained where representable.", glyphs.len()),
        });
    }
    if source
        .get("unretained_private_hints")
        .is_some_and(|value| value.as_bool() == Some(true))
    {
        warnings.push(Type1Warning {
            code: "type1-import-private-transform",
            message: "Type 1 private hint parameters could not be represented after normalizing FontMatrix; the original values are retained in source metadata.".into(),
        });
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
