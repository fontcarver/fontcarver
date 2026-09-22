//! Bounded syntax preflight for constructs the locked Type 1 reader cannot retain.
//!
//! This does not execute PostScript or charstrings. In particular, all `sbw`
//! programs are rejected: evaluating its possibly computed vertical advance is
//! outside the supported horizontal-font scope.

use super::{reader::header_token, source_error};
use crate::Error;

pub(super) struct SourceData {
    pub private: fc_model::PsPrivate,
    pub glyphs: std::collections::BTreeMap<String, Vec<u8>>,
    pub computed_private_fields: std::collections::BTreeSet<String>,
}

pub(super) fn check_horizontal_charstrings(bytes: &[u8]) -> Result<SourceData, Error> {
    let encrypted = eexec_bytes(bytes)?;
    let private = decrypt(&encrypted, 55665);
    let mut input = private
        .get(4..)
        .ok_or_else(|| source_error("truncated eexec prefix"))?;
    let mut previous = None;
    let mut len_iv = 4i32;
    let mut in_charstrings = false;
    let mut in_glyphs = false;
    let mut procedures = 0usize;
    let mut values = fc_model::PsPrivate::default();
    let mut glyphs = std::collections::BTreeMap::new();
    let mut glyph_name = None;
    let mut computed_private_fields = std::collections::BTreeSet::new();
    while let Some(token) = header_token(&mut input)? {
        match token {
            b"{" => {
                procedures += 1;
                if procedures > 64 {
                    return Err(source_error("private procedure nesting limit exceeded"));
                }
            }
            b"}" => procedures = procedures.saturating_sub(1),
            _ if procedures > 0 => {}
            _ if in_glyphs && token.starts_with(b"/") => {
                glyph_name = Some(
                    std::str::from_utf8(&token[1..])
                        .map_err(|_| source_error("glyph name is not UTF-8"))?
                        .to_owned(),
                );
            }
            b"closefile" => {
                values
                    .validate()
                    .map_err(|error| source_error(error.to_string()))?;
                return Ok(SourceData {
                    private: values,
                    glyphs,
                    computed_private_fields,
                });
            }
            b"/lenIV" => {
                len_iv = header_token(&mut input)?
                    .and_then(integer)
                    .ok_or_else(|| source_error("invalid lenIV"))?;
                if len_iv < -1 {
                    return Err(source_error("invalid lenIV"));
                }
            }
            b"/Subrs" => in_charstrings = true,
            b"/CharStrings" => {
                in_charstrings = true;
                in_glyphs = true;
            }
            b"/Metrics" | b"/Metrics2" | b"/CDevProc" => {
                return Err(source_error(
                    "PostScript metric overrides are not supported",
                ))
            }
            b"RD" | b"-|" if in_charstrings => {
                let size = previous
                    .and_then(integer)
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| source_error("invalid charstring binary length"))?;
                if !input.first().is_some_and(u8::is_ascii_whitespace) {
                    return Err(source_error("missing binary charstring separator"));
                }
                input = &input[1..];
                let data = input
                    .get(..size)
                    .ok_or_else(|| source_error("truncated binary charstring"))?;
                input = &input[size..];
                let program = if len_iv == -1 {
                    data.to_vec()
                } else {
                    let decrypted = decrypt(data, 4330);
                    decrypted
                        .get(len_iv as usize..)
                        .ok_or_else(|| source_error("charstring is shorter than lenIV"))?
                        .to_vec()
                };
                check_program(&program)?;
                if in_glyphs {
                    let name = glyph_name
                        .take()
                        .ok_or_else(|| source_error("missing glyph name"))?;
                    if glyphs.len() >= 65535 || glyphs.insert(name, program).is_some() {
                        return Err(source_error("duplicate or excessive charstrings"));
                    }
                }
            }
            _ if !in_glyphs => {
                if let Some(name) = super::private::field(token, &mut input, &mut values)? {
                    computed_private_fields.insert(name);
                }
            }
            _ => {}
        }
        previous = Some(token);
    }
    // Decryption may include a clear-text trailer after its closefile instruction.
    // A missing instruction is malformed rather than a reason to skip the guard.
    Err(source_error(
        "Type 1 private dictionary is missing closefile",
    ))
}

fn integer(token: &[u8]) -> Option<i32> {
    std::str::from_utf8(token).ok()?.parse().ok()
}

fn check_program(program: &[u8]) -> Result<(), Error> {
    let tokens = tokens(program)?;
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token, ProgramToken::Integer(0))
            && matches!(tokens.get(index + 1), Some(ProgramToken::Operator(0x0c0c)))
        {
            return Err(source_error("charstring division by zero"));
        }
        if matches!(token, ProgramToken::Integer(value) if !(-32000..=32000).contains(value))
            && !safe_large_division(&tokens, index)
            && !index
                .checked_sub(1)
                .is_some_and(|start| safe_large_division(&tokens, start))
        {
            return Err(source_error("unsupported large-integer charstring arithmetic; use an adjacent large numerator / integer denominator div pair with a result below 32000"));
        }
    }
    Ok(())
}

pub(super) fn tokens(program: &[u8]) -> Result<Vec<ProgramToken>, Error> {
    if program.len() > 65535 {
        return Err(source_error(
            "charstring exceeds the PostScript string limit",
        ));
    }
    let mut index = 0usize;
    let mut tokens = Vec::new();
    while let Some(byte) = program.get(index).copied() {
        index += 1;
        let token = match byte {
            12 => {
                let escaped = program
                    .get(index)
                    .ok_or_else(|| source_error("truncated escaped charstring operator"))?;
                if *escaped == 7 {
                    return Err(source_error(
                        "Type 1 sbw advances are unsupported; use horizontal hsbw charstrings",
                    ));
                }
                index += 1;
                ProgramToken::Operator(0x0c00 | u16::from(*escaped))
            }
            28 => {
                let bytes: [u8; 2] = program
                    .get(index..index + 2)
                    .and_then(|data| data.try_into().ok())
                    .ok_or_else(|| source_error("truncated integer operand"))?;
                index += 2;
                ProgramToken::Integer(i32::from(i16::from_be_bytes(bytes)))
            }
            32..=246 => ProgramToken::Integer(i32::from(byte) - 139),
            247..=254 => {
                let next = program
                    .get(index)
                    .ok_or_else(|| source_error("truncated integer operand"))?;
                index += 1;
                let value = if byte < 251 {
                    (i32::from(byte) - 247) * 256 + i32::from(*next) + 108
                } else {
                    -(i32::from(byte) - 251) * 256 - i32::from(*next) - 108
                };
                ProgramToken::Integer(value)
            }
            255 => {
                let bytes: [u8; 4] = program
                    .get(index..index + 4)
                    .and_then(|data| data.try_into().ok())
                    .ok_or_else(|| source_error("truncated integer operand"))?;
                index += 4;
                ProgramToken::Integer(i32::from_be_bytes(bytes))
            }
            _ => ProgramToken::Operator(u16::from(byte)),
        };
        tokens.push(token);
        if index > program.len() {
            return Err(source_error("truncated charstring operand"));
        }
    }
    Ok(tokens)
}

#[derive(Clone, Copy)]
pub(super) enum ProgramToken {
    Integer(i32),
    Operator(u16),
}

fn safe_large_division(tokens: &[ProgramToken], start: usize) -> bool {
    let Some(
        [ProgramToken::Integer(numerator), ProgramToken::Integer(denominator), ProgramToken::Operator(0x0c0c)],
    ) = tokens.get(start..start + 3)
    else {
        return false;
    };
    !(-32000..=32000).contains(numerator)
        && *denominator != 0
        && (f64::from(*numerator) / f64::from(*denominator)).abs() < 32000.0
}

fn decrypt(bytes: &[u8], mut state: u16) -> Vec<u8> {
    bytes
        .iter()
        .map(|cipher| {
            let plain = *cipher ^ (state >> 8) as u8;
            state = state
                .wrapping_add(u16::from(*cipher))
                .wrapping_mul(52845)
                .wrapping_add(22719);
            plain
        })
        .collect()
}

fn eexec_bytes(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    if bytes.starts_with(&[0x80, 1]) {
        let mut data = bytes;
        let mut encrypted = Vec::new();
        while data.len() >= 6 && data[0] == 0x80 && matches!(data[1], 1 | 2) {
            let size = u32::from_le_bytes(
                data[2..6]
                    .try_into()
                    .map_err(|_| source_error("invalid PFB segment"))?,
            ) as usize;
            let payload = data
                .get(6..6 + size)
                .ok_or_else(|| source_error("truncated PFB segment"))?;
            if data[1] == 2 {
                encrypted.extend_from_slice(payload);
            }
            data = &data[6 + size..];
        }
        return Ok(encrypted);
    }
    let mut data = bytes;
    while let Some(token) = header_token(&mut data)? {
        if token == b"eexec" {
            break;
        }
    }
    while data.first().is_some_and(u8::is_ascii_whitespace) {
        data = &data[1..];
    }
    if data
        .get(..4)
        .is_some_and(|prefix| prefix.iter().all(u8::is_ascii_hexdigit))
    {
        let mut output = Vec::new();
        let mut high = None;
        for byte in data.iter().copied().filter(|b| !b.is_ascii_whitespace()) {
            let Some(digit) = char::from(byte).to_digit(16) else {
                break;
            };
            if let Some(first) = high.take() {
                output.push(first * 16 + digit as u8);
            } else {
                high = Some(digit as u8);
            }
        }
        Ok(output)
    } else {
        Ok(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::check_program;

    fn numbers(values: &[i32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| std::iter::once(255).chain(value.to_be_bytes()))
            .collect()
    }

    #[test]
    fn unsafe_large_number_flow_is_rejected_without_interpreting_programs() {
        let mut safe = numbers(&[32001, 2_097_217_536]);
        safe.extend_from_slice(&[12, 12]);
        assert!(check_program(&safe).is_ok());
        let mut divisor_only = numbers(&[2, 65536]);
        divisor_only.extend_from_slice(&[12, 12]);
        let mut computed_numerator = numbers(&[1, 2]);
        computed_numerator.extend_from_slice(&[12, 12]);
        computed_numerator.extend(numbers(&[65536]));
        computed_numerator.extend_from_slice(&[12, 12]);
        let mut crosses_subroutine = numbers(&[65536, 0]);
        crosses_subroutine.push(10);
        let mut zero_divisor = numbers(&[2, 0]);
        zero_divisor.extend_from_slice(&[12, 12]);
        for program in [divisor_only, computed_numerator, crosses_subroutine] {
            assert!(check_program(&program)
                .unwrap_err()
                .to_string()
                .contains("large-integer"));
        }
        assert!(check_program(&zero_divisor)
            .unwrap_err()
            .to_string()
            .contains("division by zero"));
    }
}
