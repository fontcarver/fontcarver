//! Bounded retention of whole-glyph Type 1 stems, without executing PostScript.

use fc_model::{PsHints, StemHint};
use kurbo::Affine;
use read_fonts::ps::cs::CharstringContext;

use super::{
    guards::{tokens, ProgramToken},
    source_error,
    writer::number,
};
use crate::Error;

pub(super) fn read(
    program: &[u8],
    context: &impl CharstringContext,
    transform: Affine,
    budget: &mut usize,
) -> Result<PsHints, &'static str> {
    let mut scan = Scan::default();
    scan.walk(program, context, 0, budget)?;
    let [xx, xy, yx, yy, dx, dy] = transform.as_coeffs();
    if !scan.values.is_empty() && (xx <= 0.0 || yy <= 0.0 || xy != 0.0 || yx != 0.0) {
        return Err("stem hint transform is not axis-aligned and positive");
    }
    for (stems, scale, offset) in [
        (&mut scan.values.h_stems, yy, dy),
        (&mut scan.values.v_stems, xx, dx),
    ] {
        for stem in stems {
            if stem.width < 0.0 {
                // The -21 ghost's real edge is start + width; its sentinel width
                // is a format marker and must not scale with the outline.
                let edge = if stem.width == -21.0 {
                    stem.start - 21.0
                } else {
                    stem.start
                };
                stem.start = edge * scale + offset + if stem.width == -21.0 { 21.0 } else { 0.0 };
            } else {
                stem.start = stem.start * scale + offset;
                stem.width *= scale;
            }
        }
    }
    scan.values
        .validate(&[])
        .map_err(|_| "invalid stem hints")?;
    Ok(scan.values)
}

#[derive(Default)]
struct Scan {
    values: PsHints,
    stack: Vec<f64>,
    sidebearing: Option<f64>,
    drawing: bool,
}

impl Scan {
    fn walk(
        &mut self,
        program: &[u8],
        context: &impl CharstringContext,
        depth: usize,
        budget: &mut usize,
    ) -> Result<bool, &'static str> {
        if depth > 16 || *budget == 0 {
            return Err("hint inspection complexity limit exceeded");
        }
        for token in tokens(program).map_err(|_| "invalid charstring")? {
            *budget = budget
                .checked_sub(1)
                .ok_or("hint inspection complexity limit exceeded")?;
            match token {
                ProgramToken::Integer(value) => {
                    if self.stack.len() >= 24 {
                        return Err("hint operand stack limit exceeded");
                    }
                    self.stack.push(f64::from(value));
                }
                ProgramToken::Operator(0x0c0c) => {
                    let denominator = self.stack.pop().ok_or("missing division operand")?;
                    let numerator = self.stack.pop().ok_or("missing division operand")?;
                    let value = numerator / denominator;
                    if !value.is_finite() {
                        return Err("invalid division");
                    }
                    self.stack.push(value);
                }
                ProgramToken::Operator(13) => {
                    if self.stack.len() != 2 || self.sidebearing.is_some() || self.drawing {
                        return Err("invalid sidebearing");
                    }
                    self.sidebearing = self.stack.first().copied();
                    self.stack.clear();
                }
                ProgramToken::Operator(opcode @ (1 | 3)) => {
                    if self.drawing || self.stack.len() != 2 {
                        return Err("late or unsupported stem hints");
                    }
                    let sidebearing = self.sidebearing.ok_or("stem precedes sidebearing")?;
                    if self.values.h_stems.len() + self.values.v_stems.len() >= 96 {
                        return Err("stem count limit exceeded");
                    }
                    let stem = StemHint {
                        start: self.stack[0] + if opcode == 3 { sidebearing } else { 0.0 },
                        width: self.stack[1],
                        ..Default::default()
                    };
                    if opcode == 1 {
                        self.values.h_stems.push(stem);
                    } else {
                        self.values.v_stems.push(stem);
                    }
                    self.stack.clear();
                }
                ProgramToken::Operator(10) => {
                    let index = self.stack.pop().ok_or("missing subroutine index")?;
                    if index.fract() != 0.0 || !(0.0..=65535.0).contains(&index) {
                        return Err("invalid subroutine index");
                    }
                    let subr = context
                        .subr(index as i32)
                        .map_err(|_| "missing subroutine")?;
                    if self.walk(subr, context, depth + 1, budget)? {
                        return Ok(true);
                    }
                }
                ProgramToken::Operator(11) => return Ok(false),
                ProgramToken::Operator(14) => return Ok(true),
                ProgramToken::Operator(4..=9 | 21 | 22 | 30 | 31) => {
                    self.drawing = true;
                    self.stack.clear();
                }
                // Counter stems, dot sections, seac, flex and hint replacement
                // require a path-bound representation. Do not retain a subset
                // as if it applied to the entire expanded outline.
                ProgramToken::Operator(_) => return Err("unsupported charstring hint mechanism"),
            }
        }
        Ok(false)
    }
}

pub(super) fn write(out: &mut Vec<u8>, hints: &PsHints) -> Result<(), Error> {
    hints
        .validate(&[])
        .map_err(|error| source_error(error.to_string()))?;
    if !hints.hint_masks.is_empty()
        || !hints.d_stems.is_empty()
        || hints.flex
        || hints
            .h_stems
            .iter()
            .chain(&hints.v_stems)
            .any(|stem| !stem.ranges.is_empty())
    {
        return Err(source_error("Type 1 output currently requires whole-glyph axis-aligned stems; hint replacement, counter masks, flex, diagonal hints and active ranges are not yet supported"));
    }
    for (stems, opcode) in [(&hints.h_stems, 1), (&hints.v_stems, 3)] {
        let mut intervals: Vec<_> = stems
            .iter()
            .map(|stem| {
                if stem.width == -20.0 {
                    (stem.start, stem.start)
                } else if stem.width == -21.0 {
                    (stem.start - 21.0, stem.start - 21.0)
                } else {
                    (stem.start, stem.start + stem.width)
                }
            })
            .collect();
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        if intervals.windows(2).any(|pair| pair[0].1 > pair[1].0) {
            return Err(source_error(
                "overlapping Type 1 stems require hint replacement",
            ));
        }
        for stem in stems {
            number(out, stem.start)?;
            number(out, stem.width)?;
            out.push(opcode);
        }
    }
    Ok(())
}
