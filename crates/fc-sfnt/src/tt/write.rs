use std::collections::BTreeMap;

use fc_model::TtMasterData;
use read_fonts::{tables::glyf::bytecode::Decoder, types::Tag, FontRef, TableProvider};
use write_fonts::FontBuilder;

use super::{basis::Source, invalid, span, variation};
use crate::Error;

const MAX_FONT_BYTES: usize = 256 * 1024 * 1024;

/// A glyph program explicitly bound to the compiled outline and metrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphProgram {
    /// Glyph index in the compiled sfnt, resolved from a model glyph name by the caller.
    pub glyph_id: u16,
    /// Versioned point binding obtained from [`super::point_basis`].
    pub fingerprint: String,
    /// Assembled TrueType instructions.
    pub bytes: Vec<u8>,
}

/// Replaces all instruction data in a compiled TrueType sfnt.
///
/// The supplied glyph list is authoritative: unlisted glyph programs are removed.
/// Outline byte encoding and point order are preserved. Bindings must match before
/// any output is returned. All interpreter maxima except `max_size_of_instructions`
/// must be supplied; the latter is computed from the actual glyph programs.
///
/// Existing variable-font tables are retained byte-for-byte. When `cvar` is
/// present, the supplied CVT must equal the original indexed CVT exactly. This
/// binary-only path does not generate variation deltas or make static bindings
/// usable for a newly compiled variable font.
///
/// This validates structure, bytecode framing and declared bounds, not the runtime
/// behavior of arbitrary TrueType programs. Invalid execution is a preview error.
pub fn with_instructions(
    bytes: &[u8],
    programs: &[GlyphProgram],
    globals: &TtMasterData,
) -> Result<Vec<u8>, Error> {
    let mut source = Source::new(bytes)?;
    let face = FontRef::new(bytes).map_err(invalid)?;
    let count = face.maxp().map_err(invalid)?.num_glyphs();
    if programs.len() > usize::from(count) {
        return Err(invalid("too many glyph programs"));
    }
    validate_program(&globals.fpgm)?;
    validate_program(&globals.prep)?;
    if globals.cvt.len() > 65536 {
        return Err(invalid("CVT exceeds the 16-bit index range"));
    }
    variation::check_cvt(&face, &globals.cvt)?;
    let mut maxp = table(&face, b"maxp")?;
    if maxp.len() != 32 || maxp[..4] != [0, 1, 0, 0] {
        return Err(invalid("TrueType instructions require maxp version 1.0"));
    }
    set_maxima(&mut maxp, globals)?;
    let mut selected = BTreeMap::new();
    let mut max_length = 0;
    for program in programs {
        validate_program(&program.bytes)?;
        if selected
            .insert(program.glyph_id, program.bytes.as_slice())
            .is_some()
        {
            return Err(invalid("duplicate glyph program"));
        }
        if source.fingerprint(program.glyph_id)? != program.fingerprint {
            return Err(invalid(format!(
                "stale instruction binding for glyph {}",
                program.glyph_id
            )));
        }
        max_length = max_length.max(program.bytes.len() as u16);
    }
    maxp[26..28].copy_from_slice(&max_length.to_be_bytes());

    let mut glyf = Vec::new();
    let mut loca = Vec::with_capacity((usize::from(count) + 1) * 4);
    for id in 0..count {
        loca.extend((glyf.len() as u32).to_be_bytes());
        let glyph = span::replace(
            source.raw_glyph(id)?,
            selected.get(&id).copied().unwrap_or_default(),
        )?;
        if glyf.len() + glyph.len() + 3 > MAX_FONT_BYTES {
            return Err(invalid("instruction output exceeds the font size limit"));
        }
        glyf.extend(glyph);
        glyf.resize(glyf.len().next_multiple_of(4), 0);
    }
    loca.extend((glyf.len() as u32).to_be_bytes());
    let mut head = table(&face, b"head")?;
    if head.len() < 54 {
        return Err(invalid("truncated head"));
    }
    head[50..52].copy_from_slice(&1i16.to_be_bytes());
    // Authored instructions may adjust advances. Other head flags are preserved.
    if max_length != 0 || !globals.prep.is_empty() || !globals.fpgm.is_empty() {
        let flags = u16::from_be_bytes([head[16], head[17]]) | 0x10;
        head[16..18].copy_from_slice(&flags.to_be_bytes());
    }
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        let tag = record.tag();
        // Device metric caches and signatures no longer describe the programs.
        if [
            b"glyf", b"loca", b"head", b"maxp", b"cvt ", b"fpgm", b"prep", b"DSIG", b"hdmx",
            b"LTSH", b"VDMX",
        ]
        .iter()
        .any(|value| tag == Tag::new(value))
        {
            continue;
        }
        builder.add_raw(
            tag,
            face.table_data(tag)
                .ok_or_else(|| invalid("missing input table"))?
                .as_bytes(),
        );
    }
    for (tag, data) in [
        (b"glyf", glyf),
        (b"loca", loca),
        (b"head", head),
        (b"maxp", maxp),
    ] {
        builder.add_raw(Tag::new(tag), data);
    }
    for (tag, data) in [
        (
            b"cvt ",
            globals
                .cvt
                .iter()
                .flat_map(|v| v.to_be_bytes())
                .collect::<Vec<_>>(),
        ),
        (b"fpgm", globals.fpgm.clone()),
        (b"prep", globals.prep.clone()),
    ] {
        if !data.is_empty() {
            builder.add_raw(Tag::new(tag), data);
        }
    }
    let result = builder.build();
    if result.len() > MAX_FONT_BYTES {
        return Err(invalid("font output size limit exceeded"));
    }
    Ok(result)
}

fn table(face: &FontRef<'_>, tag: &[u8; 4]) -> Result<Vec<u8>, Error> {
    face.table_data(Tag::new(tag))
        .map(|data| data.as_bytes().to_vec())
        .ok_or_else(|| invalid("missing TrueType output table"))
}

fn set_maxima(bytes: &mut [u8], globals: &TtMasterData) -> Result<(), Error> {
    let fields = [
        globals.max_zones,
        globals.max_twilight_points,
        globals.max_storage,
        globals.max_function_defs,
        globals.max_instruction_defs,
        globals.max_stack_elements,
    ];
    for (index, value) in fields.into_iter().enumerate() {
        let value = value.ok_or_else(|| invalid("all interpreter maxima must be provided"))?;
        if index == 0 && !matches!(value, 1 | 2) {
            return Err(invalid("maxZones must be 1 or 2"));
        }
        if index == 4 && value > 256 {
            return Err(invalid("instruction definitions exceed opcode capacity"));
        }
        let offset = 14 + index * 2;
        bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }
    Ok(())
}

fn validate_program(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > 65535 {
        return Err(invalid("TrueType program exceeds 65,535 bytes"));
    }
    let mut decoder = Decoder::new(bytes, 0);
    while decoder.pc < bytes.len() {
        if !matches!(decoder.decode(), Some(Ok(_))) {
            return Err(invalid("truncated TrueType push instruction"));
        }
    }
    Ok(())
}
