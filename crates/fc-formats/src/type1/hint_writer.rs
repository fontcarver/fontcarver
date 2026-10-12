//! Native Type 1 replacement programs generated from validated point masks.

use super::{source_error, writer::number};
use crate::Error;
use fc_model::{query::PathFlex, Contour, FlexGeometry, PsHints};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Plan {
    counters: Vec<u8>,
    initial: Vec<u8>,
    replacements: BTreeMap<usize, Vec<u8>>,
    flex: BTreeMap<usize, (PathFlex, FlexGeometry)>,
    dot_sections: BTreeSet<usize>,
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
            return Err(source_error("Type 1 output requires axis-aligned stems and explicit active masks; an unresolved imported flex marker, diagonal hints and active ranges are not supported"));
        }
        let placements = fc_model::query::path_flex(hints, contours)
            .map_err(|error| source_error(error.to_string()))?;
        let mut flex = BTreeMap::new();
        for hint in &hints.flex_hints {
            let geometry = hints
                .flex_geometry(contours, hint)
                .map_err(|error| source_error(error.to_string()))?;
            if geometry.reference().is_none() {
                return Err(source_error("Type 1 flex requires the two outer endpoints of the curve pair to share an x or y coordinate (Adobe Type 1 section 8.3)"));
            }
            let element = fc_model::query::element_indices(&contours[hint.contour])
                .map_err(|error| source_error(error.to_string()))?;
            let count = contours[hint.contour].points.len();
            let first = element
                .get(&((hint.point + 3) % count))
                .copied()
                .unwrap_or_default();
            let offset = contours[..hint.contour]
                .iter()
                .map(|contour| {
                    contour
                        .try_to_bezpath()
                        .map(|path| path.elements().len())
                        .unwrap_or_default()
                })
                .sum::<usize>();
            let placement = placements
                .get(&(offset + first))
                .copied()
                .ok_or_else(|| source_error("flex hint lost its curve pair"))?;
            flex.insert(offset + first, (placement, geometry));
        }
        let counter_plan = CounterPlan::new(hints, &masks)?;
        let mut initial = None;
        let mut replacements = BTreeMap::new();
        for (index, entries) in &masks {
            for (counter, mask) in entries {
                if *counter {
                    continue;
                }
                let program = stems(hints, Some(mask), &counter_plan)?;
                if *index == 0 {
                    initial = Some(program);
                } else {
                    replacements.insert(*index, program);
                }
            }
        }
        Ok(Self {
            counters: counter_plan.program().to_vec(),
            initial: match initial {
                Some(program) => program,
                None => stems(hints, None, &counter_plan)?,
            },
            replacements,
            flex,
            dot_sections: fc_model::query::path_dot_sections(hints, contours)
                .map_err(|error| source_error(error.to_string()))?,
        })
    }

    pub(super) fn initial(&self, out: &mut Vec<u8>) {
        // Technical Note #5015 section 2.2: the counter-control calls must
        // immediately follow hsbw, before any stem declaration.
        out.extend_from_slice(&self.counters);
        out.extend_from_slice(&self.initial);
    }

    /// The flex pair starting at this path element, if any.
    pub(super) fn flex(&self, index: usize) -> Option<(PathFlex, FlexGeometry)> {
        self.flex.get(&index).copied()
    }

    /// Whether a `dotsection` command belongs immediately after this element.
    pub(super) fn dot_section(&self, index: usize) -> bool {
        self.dot_sections.contains(&index)
    }

    /// Whether this glyph needs the four standard `Subrs` entries reserved.
    pub(super) fn needs_standard_subrs(&self) -> bool {
        !self.flex.is_empty() || !self.replacements.is_empty()
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

fn stems(hints: &PsHints, mask: Option<&[u8]>, counters: &CounterPlan) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for (offset, stems, opcode) in [
        (0, &hints.h_stems, 1),
        (hints.h_stems.len(), &hints.v_stems, 3),
    ] {
        let counter = counters.triple(opcode == 3);
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

/// How one glyph's counter-control masks are expressed in Type 1 output.
///
/// `hstem3`/`vstem3` is preferred because every PostScript interpreter
/// understands it, but it only covers one exact triple per axis with no ordinary
/// stems on that axis. Anything else is written as general counter control
/// through `OtherSubrs` 12 and 13 (Adobe Technical Note #5015 sections 2.2-2.6).
enum CounterPlan {
    None,
    Triples([Option<Vec<usize>>; 2]),
    General(Vec<u8>),
}

impl CounterPlan {
    fn new(hints: &PsHints, masks: &fc_model::query::PathHintMasks<'_>) -> Result<Self, Error> {
        if !masks.values().flatten().any(|(counter, _)| *counter) {
            return Ok(Self::None);
        }
        match triples(hints, masks) {
            Ok(triples) => Ok(Self::Triples(triples)),
            // The triple form cannot express this glyph; fall back to the general
            // mechanism rather than refusing to write the counter data.
            Err(_) => Ok(Self::General(general_counters(hints, masks)?)),
        }
    }

    fn triple(&self, vertical: bool) -> Option<&Vec<usize>> {
        match self {
            Self::Triples(groups) => groups[usize::from(vertical)].as_ref(),
            _ => None,
        }
    }

    fn program(&self) -> &[u8] {
        match self {
            Self::General(bytes) => bytes,
            _ => &[],
        }
    }
}

/// The representable `hstem3`/`vstem3` interpretation of the counter masks.
fn triples(
    hints: &PsHints,
    masks: &fc_model::query::PathHintMasks<'_>,
) -> Result<[Option<Vec<usize>>; 2], Error> {
    let mut result: [Option<Vec<usize>>; 2] = Default::default();
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
                if result[axis]
                    .as_ref()
                    .is_some_and(|previous| *previous != indices)
                {
                    return Err(source_error("Type 1 counter triple changes across masks"));
                }
                result[axis] = Some(indices);
                matched = true;
            }
            if !matched {
                return Err(source_error("empty Type 1 counter mask"));
            }
        }
    }
    Ok(result)
}

/// Encodes general counter control for every counter mask, in declaration order.
///
/// The data is `#H HG1..HGn #V VG1..VGm`, each group a run of
/// (distance-from-the-previous-stem, width) pairs whose final pair carries the
/// distance to the stem's far edge and a negated width. The Type 1 operand stack
/// holds at most 22 arguments per call, so the data is split from its end: later
/// chunks go through `OtherSubrs` 12 first and the leading chunk through
/// `OtherSubrs` 13, which is the order the interpreter's pseudo stack replays.
fn general_counters(
    hints: &PsHints,
    masks: &fc_model::query::PathHintMasks<'_>,
) -> Result<Vec<u8>, Error> {
    let mut groups: [Vec<Vec<usize>>; 2] = Default::default();
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
                let indices: Vec<_> = (0..stems.len())
                    .filter(|i| active(Some(mask), offset + i))
                    .collect();
                if indices.is_empty() {
                    continue;
                }
                if indices.len() < 2 {
                    return Err(source_error(
                        "a counter-control group needs at least two stems to delimit a counter",
                    ));
                }
                groups[axis].push(indices);
                matched = true;
            }
            if !matched {
                return Err(source_error("empty Type 1 counter mask"));
            }
        }
    }
    let mut values: Vec<f64> = Vec::new();
    for (axis, stems) in [(0, &hints.h_stems), (1, &hints.v_stems)] {
        values.push(groups[axis].len() as f64);
        for group in &groups[axis] {
            let mut sorted: Vec<_> = group.iter().map(|index| &stems[*index]).collect();
            sorted.sort_by(|a, b| a.start.total_cmp(&b.start));
            let mut previous = 0.0;
            for (position, stem) in sorted.iter().enumerate() {
                if stem.width <= 0.0 {
                    return Err(source_error(
                        "counter control cannot describe a ghost or zero-width stem",
                    ));
                }
                if stem.start < previous {
                    return Err(source_error(
                        "counter-control stems in one group must not overlap",
                    ));
                }
                if position + 1 == sorted.len() {
                    values.push(stem.start + stem.width - previous);
                    values.push(-stem.width);
                } else {
                    values.push(stem.start - previous);
                    values.push(stem.width);
                    previous = stem.start + stem.width;
                }
            }
        }
    }
    if values.len() > 220 {
        return Err(source_error(
            "counter-control data exceeds the supported ten calls",
        ));
    }
    let head = match values.len() % 22 {
        0 => 22.min(values.len()),
        remainder => remainder,
    };
    let mut out = Vec::new();
    let mut emit = |chunk: &[f64], othersubr: f64| -> Result<(), Error> {
        for value in chunk {
            number(&mut out, *value)?;
        }
        number(&mut out, chunk.len() as f64)?;
        number(&mut out, othersubr)?;
        out.extend_from_slice(&[12, 16]); // callothersubr
        Ok(())
    };
    let tail: Vec<&[f64]> = values[head..].chunks(22).collect();
    for chunk in tail.into_iter().rev() {
        emit(chunk, 12.0)?;
    }
    emit(&values[..head], 13.0)?;
    Ok(out)
}

#[derive(Default)]
pub(super) struct Subrs {
    programs: Vec<Vec<u8>>,
    indices: BTreeMap<Vec<u8>, usize>,
    size: usize,
    reserved: bool,
}

impl Subrs {
    /// Reserves the four standard entries even when no replacement program needs
    /// one, which flex requires because it calls `Subrs` 0, 1 and 2 directly.
    pub(super) fn reserve(&mut self) {
        self.reserved = true;
    }

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
        if self.programs.is_empty() && !self.reserved {
            out.extend_from_slice(b"/Subrs 0 array ND\n");
            return;
        }
        out.extend_from_slice(OTHER_SUBRS.as_bytes());
        out.extend_from_slice(format!("/Subrs {} array\n", self.programs.len() + 4).as_bytes());
        // The four standard slots: flex start/collect/finish and the hint
        // replacement fallback. Old interpreters can return slot 3 to ignore
        // new hints (Adobe Type 1 section 8.4).
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

// Entries 0-2 (flex) and 12-13 (counter control) are empty or stack-clearing
// procedures. Section 8.3 of the Type 1 specification allows an interpreter to
// provide "a built-in function equivalent to this procedure" for entries 0-3,
// and every interpreter does: FreeType, Ghostscript and Adobe's own rasterisers
// implement flex and hint replacement internally and never evaluate these
// procedures. Real fonts rely on that too - URW's Nimbus Sans and Computer
// Modern delegate entry 0 to the interpreter-internal `FlxProc`, which is not
// available outside Adobe's interpreter - so empty procedures are the safe
// choice: on an interpreter that does evaluate them nothing undefined is
// executed. Entry 3 below is original code implementing section 8.1 semantics:
// select the interpreter's hint-reset operator when available, otherwise return
// the reserved empty subroutine. Local names avoid altering the font dictionary.
// Entry 13 clears the counter-control arguments that entry 12 accumulated, as
// Technical Note #5015 section 2.2 requires for interpreters that ignore them.
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
} executeonly {} {} {} {} {} {} {} {} {}
{ 2 { cvi { { pop 0 lt { exit } if } loop } repeat } repeat } ] noaccess def
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
