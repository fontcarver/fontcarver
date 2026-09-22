//! Native Type 1 replacement programs generated from validated point masks.

use super::{source_error, writer::number};
use crate::Error;
use fc_model::{Contour, PsHints};
use std::collections::BTreeMap;

pub(super) struct Plan {
    initial: Vec<u8>,
    replacements: BTreeMap<usize, Vec<u8>>,
}

impl Plan {
    pub(super) fn new(hints: &PsHints, contours: &[Contour]) -> Result<Self, Error> {
        hints
            .validate(contours)
            .map_err(|error| source_error(error.to_string()))?;
        // Validate the actual 16.16 output values as well as the editable model.
        // Tiny positive widths must not silently become zero-width output stems.
        let mut quantized = hints.clone();
        for stem in quantized.h_stems.iter_mut().chain(&mut quantized.v_stems) {
            stem.start = (stem.start * 65536.0).round() / 65536.0;
            stem.width = (stem.width * 65536.0).round() / 65536.0;
        }
        let hints = &quantized;
        let masks = fc_model::query::path_hint_masks(hints, contours)
            .map_err(|error| source_error(error.to_string()))?;
        if !hints.d_stems.is_empty()
            || hints.flex
            || hints
                .h_stems
                .iter()
                .chain(&hints.v_stems)
                .any(|s| !s.ranges.is_empty())
        {
            return Err(source_error("Type 1 output requires axis-aligned stems and explicit active masks; general counter control, flex, diagonal hints and active ranges are not yet supported"));
        }
        let counters = Counters::new(hints, &masks)?;
        let mut initial = None;
        let mut replacements = BTreeMap::new();
        for (index, entries) in masks {
            for (counter, mask) in entries {
                if counter {
                    continue;
                }
                let program = stems(hints, Some(mask), &counters)?;
                if index == 0 {
                    initial = Some(program);
                } else {
                    replacements.insert(index, program);
                }
            }
        }
        Ok(Self {
            initial: initial.unwrap_or(stems(hints, None, &counters)?),
            replacements,
        })
    }

    pub(super) fn initial(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.initial);
    }

    pub(super) fn at(
        &self,
        index: usize,
        out: &mut Vec<u8>,
        subrs: &mut Subrs,
    ) -> Result<(), Error> {
        if let Some(program) = self.replacements.get(&index) {
            let index = subrs.insert(program)?;
            number(out, index as f64)?;
            out.extend_from_slice(&[140, 142, 12, 16, 12, 17, 10]); // 1 3 callothersubr pop callsubr
        }
        Ok(())
    }
}

fn stems(hints: &PsHints, mask: Option<&[u8]>, counters: &Counters) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for (offset, stems, opcode) in [
        (0, &hints.h_stems, 1),
        (hints.h_stems.len(), &hints.v_stems, 3),
    ] {
        let counter = counters.0[usize::from(opcode == 3)].as_ref();
        if let Some(group) = counter {
            let enabled = group.iter().filter(|&&i| active(mask, offset + i)).count();
            if enabled == 0 {
                continue;
            }
            if enabled != 3 {
                return Err(source_error(
                    "Type 1 counter triple must be active as a complete group",
                ));
            }
            for &index in group {
                number(&mut out, stems[index].start)?;
                number(&mut out, stems[index].width)?;
            }
            out.extend_from_slice(&[12, if opcode == 1 { 2 } else { 1 }]);
            continue;
        }
        for (index, stem) in stems.iter().enumerate() {
            let index = offset + index;
            if mask.is_some_and(|m| m[index / 8] & (0x80 >> (index % 8)) == 0) {
                continue;
            }
            number(&mut out, stem.start)?;
            number(&mut out, stem.width)?;
            out.push(opcode);
        }
    }
    Ok(out)
}

fn active(mask: Option<&[u8]>, index: usize) -> bool {
    mask.is_none_or(|m| m[index / 8] & (0x80 >> (index % 8)) != 0)
}

#[derive(Default)]
struct Counters([Option<Vec<usize>>; 2]);

impl Counters {
    fn new(hints: &PsHints, masks: &fc_model::query::PathHintMasks<'_>) -> Result<Self, Error> {
        let mut result = Self::default();
        for (position, entries) in masks {
            for (_, mask) in entries.iter().filter(|(counter, _)| *counter) {
                if *position != 0 {
                    return Err(source_error(
                        "Type 1 counter groups must be declared before the first contour",
                    ));
                }
                let mut matched = false;
                for (axis, offset, stems) in [
                    (0, 0, &hints.h_stems),
                    (1, hints.h_stems.len(), &hints.v_stems),
                ] {
                    let mut indices: Vec<_> = (0..stems.len())
                        .filter(|i| active(Some(mask), offset + i))
                        .collect();
                    if indices.is_empty() {
                        continue;
                    }
                    if indices.len() != 3 || stems.len() != 3 {
                        return Err(source_error("Type 1 counters require exactly one triple per axis, without ordinary stems on that axis"));
                    }
                    indices.sort_by(|a, b| stems[*a].start.total_cmp(&stems[*b].start));
                    let group: Vec<_> = indices.iter().map(|i| &stems[*i]).collect();
                    if !super::hints::valid_triplet(&group) {
                        return Err(source_error(
                            "invalid Type 1 counter triple spacing or widths",
                        ));
                    }
                    if result.0[axis]
                        .as_ref()
                        .is_some_and(|previous| *previous != indices)
                    {
                        return Err(source_error("Type 1 counter triple changes across masks"));
                    }
                    result.0[axis] = Some(indices);
                    matched = true;
                }
                if !matched {
                    return Err(source_error("empty Type 1 counter mask"));
                }
            }
        }
        Ok(result)
    }
}

#[derive(Default)]
pub(super) struct Subrs {
    programs: Vec<Vec<u8>>,
    indices: BTreeMap<Vec<u8>, usize>,
    size: usize,
}

impl Subrs {
    fn insert(&mut self, program: &[u8]) -> Result<usize, Error> {
        if let Some(index) = self.indices.get(program) {
            return Ok(*index);
        }
        let index = self.programs.len() + 4;
        self.size += program.len() + 40;
        if index > 32000 || self.size > super::MAX_FILE_BYTES / 4 {
            return Err(source_error(
                "Type 1 replacement subroutine budget exceeded",
            ));
        }
        self.indices.insert(program.to_vec(), index);
        let mut body = program.to_vec();
        body.push(11);
        self.programs.push(body);
        Ok(index)
    }

    pub(super) fn write(&self, out: &mut Vec<u8>) {
        if self.programs.is_empty() {
            out.extend_from_slice(b"/Subrs 0 array ND\n");
            return;
        }
        out.extend_from_slice(OTHER_SUBRS.as_bytes());
        out.extend_from_slice(format!("/Subrs {} array\n", self.programs.len() + 4).as_bytes());
        // Reserve the four standard flex/fallback slots even though this writer
        // emits no flex. Old interpreters can return slot 3 to ignore new hints.
        let reserved: [&[u8]; 4] = [
            &[142, 139, 12, 16, 12, 17, 12, 17, 12, 33, 11],
            &[139, 140, 12, 16, 11],
            &[139, 141, 12, 16, 11],
            &[11],
        ];
        for (index, program) in reserved
            .into_iter()
            .chain(self.programs.iter().map(Vec::as_slice))
            .enumerate()
        {
            let mut plain = vec![0; 4];
            plain.extend_from_slice(program);
            let encrypted = super::writer::encrypt(&plain, 4330);
            out.extend_from_slice(format!("dup {index} {} RD ", encrypted.len()).as_bytes());
            out.extend_from_slice(&encrypted);
            out.extend_from_slice(b" NP\n");
        }
        out.extend_from_slice(b"ND\n");
    }
}

// Original PostScript implementation of Type 1 section 8.1 semantics. Select
// the interpreter's hint-reset operator when available; otherwise return the
// reserved empty subroutine. Local names avoid altering the font dictionary.
// Flex callbacks are deliberately empty because this writer emits no flex.
const OTHER_SUBRS: &str = r#"/OtherSubrs [ {} {} {} {
  4 dict begin
  /fcIndex exch def
  /fcReset {pop 3} def
  systemdict /internaldict known {
    /fcInternal 1183615869 systemdict /internaldict get exec def
    [/strtlck /startlock] {
      fcInternal exch 2 copy known
      {get /fcReset exch def} {pop pop} ifelse
    } forall
  } if
  fcIndex /fcReset load end exec
} executeonly ] noaccess def
"#;

#[cfg(test)]
mod tests {
    use super::OTHER_SUBRS;
    use std::process::Command;

    #[test]
    fn original_postscript_callback_handles_fallback_and_both_lock_names() {
        if !Command::new("gs")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            eprintln!("skipping OtherSubrs runtime check: Ghostscript unavailable");
            return;
        }
        // Isolate this procedure from the interpreter's real internal dictionary.
        // The callback must preserve its index for either lock operator, prefer
        // startlock when both exist, and return empty slot 3 for old interpreters.
        // Leave the test array readable so the callback can be invoked directly.
        let callbacks = OTHER_SUBRS.replace("] noaccess def", "] def");
        for (dictionary, expected) in [
            ("<< >>", 3),
            ("<< /internaldict { pop << >> } >>", 3),
            (
                "<< /internaldict { pop << /strtlck { 100 add } >> } >>",
                107,
            ),
            (
                "<< /internaldict { pop << /startlock { 200 add } >> } >>",
                207,
            ),
            (
                "<< /internaldict { pop << /strtlck { 100 add } /startlock { 200 add } >> } >>",
                207,
            ),
        ] {
            let program = format!("10 dict begin /systemdict {dictionary} def\n{callbacks}\n7 OtherSubrs 3 get exec {expected} ne {{ 1 .quit }} if count 0 ne {{ 2 .quit }} if end");
            let output = Command::new("gs")
                .args([
                    "-q",
                    "-dBATCH",
                    "-dNOPAUSE",
                    "-sDEVICE=nullpage",
                    "-c",
                    &program,
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{dictionary}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
