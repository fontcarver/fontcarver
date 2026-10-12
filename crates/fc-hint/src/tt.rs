//! Lossless textual TrueType assembly using fontations' opcode definitions.
//!
//! One instruction per line, decimal push operands, and `#` or `;` comments.
//! Mnemonics and flag values are those returned by `Opcode::name`, for example
//! `SVTCA[y]`, `PUSHB[1] 0 7` and `MDRP[rG]`. Push flags are encoded counts
//! minus one. This validates syntax, not the runtime behavior of a program.

use std::{collections::HashMap, sync::LazyLock};

use read_fonts::tables::glyf::bytecode::{Decoder, Opcode};

use crate::Error;

/// Maximum byte length for an authored program or glyph instruction stream.
pub const MAX_PROGRAM_BYTES: usize = 65_535;
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;

static OPCODES: LazyLock<HashMap<&'static str, u8>> = LazyLock::new(|| {
    (0..=255)
        .map(|byte| (Opcode::from_byte(byte).name(), byte))
        .collect()
});

/// A decoded instruction with byte offset and canonical, reassemblable source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    /// Zero-based offset in the original byte stream.
    pub offset: usize,
    /// Number of bytes occupied by this instruction and its inline operands.
    pub length: usize,
    /// Canonical instruction and decimal operands.
    pub source: String,
}

/// Decodes all instructions, returning an error immediately on a truncated push.
pub fn decode(bytes: &[u8]) -> Result<Vec<Instruction>, Error> {
    if bytes.len() > MAX_PROGRAM_BYTES {
        return Err(Error::Invalid(
            "TrueType program exceeds 65,535 bytes".into(),
        ));
    }
    let mut decoder = Decoder::new(bytes, 0);
    let mut output = Vec::new();
    while decoder.pc < bytes.len() {
        let offset = decoder.pc;
        let instruction = decoder
            .decode()
            .and_then(Result::ok)
            .ok_or_else(|| Error::Program {
                location: offset,
                message: "truncated push at byte offset".into(),
            })?;
        output.push(Instruction {
            offset,
            length: decoder.pc - offset,
            source: instruction.to_string(),
        });
    }
    Ok(output)
}

/// Produces canonical assembly that reassembles to the exact original bytes.
pub fn disassemble(bytes: &[u8]) -> Result<String, Error> {
    Ok(decode(bytes)?
        .into_iter()
        .map(|instruction| instruction.source)
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Validates authored global data and complete interpreter maxima before storage.
/// This checks bytecode framing and resource declarations, not runtime semantics.
pub fn validate_globals(data: &fc_model::TtMasterData) -> Result<(), Error> {
    decode(&data.fpgm)?;
    decode(&data.prep)?;
    if data.cvt.len() > 65536
        || !matches!(data.max_zones, Some(1 | 2))
        || data.max_stack_elements.is_none()
        || data.max_storage.is_none()
        || data.max_function_defs.is_none()
        || data.max_instruction_defs.is_none_or(|count| count > 256)
        || data.max_twilight_points.is_none()
    {
        return Err(Error::Invalid(
            "invalid or incomplete TrueType interpreter maxima/CVT".into(),
        ));
    }
    Ok(())
}

/// Assembles strict, line-oriented source with bounded allocation and exact pushes.
pub fn assemble(source: &str) -> Result<Vec<u8>, Error> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(Error::Invalid("TrueType assembly exceeds 2 MiB".into()));
    }
    let mut bytes = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let mut words = line
            .split(['#', ';'])
            .next()
            .unwrap_or("")
            .split_whitespace();
        let Some(name) = words.next() else { continue };
        let error = |message: &str| Error::Program {
            location: line_index + 1,
            message: format!("source line: {message}"),
        };
        let byte = *OPCODES
            .get(name)
            .ok_or_else(|| error("unknown opcode; use canonical disassembly mnemonics"))?;
        let operands: Vec<_> = words.take(257).collect();
        if operands.len() > 255 {
            return Err(error("at most 255 inline push operands are allowed"));
        }
        bytes.push(byte);
        let expected = match byte {
            0x40 | 0x41 => {
                bytes.push(operands.len() as u8);
                operands.len()
            }
            0xb0..=0xbf => usize::from(byte & 7) + 1,
            _ => 0,
        };
        if operands.len() != expected {
            return Err(error("inline operand count does not match the opcode"));
        }
        for operand in operands {
            let value = operand
                .parse::<i32>()
                .map_err(|_| error("expected a decimal integer operand"))?;
            if byte == 0x41 || (0xb8..=0xbf).contains(&byte) {
                let value = i16::try_from(value)
                    .map_err(|_| error("word operand must be in -32768..=32767"))?;
                bytes.extend(value.to_be_bytes());
            } else {
                bytes.push(
                    u8::try_from(value).map_err(|_| error("byte operand must be in 0..=255"))?,
                );
            }
        }
        if bytes.len() > MAX_PROGRAM_BYTES {
            return Err(error("TrueType program exceeds 65,535 bytes"));
        }
    }
    Ok(bytes)
}
