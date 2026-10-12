//! Bounded inspection of Type 1 stem sets without executing PostScript.

use fc_model::{Contour, HintMask, PsHints, StemHint};
use kurbo::Affine;
use read_fonts::ps::cs::CharstringContext;

use super::guards::{tokens, ProgramToken};

#[path = "hints_seac.rs"]
mod seac;

pub(super) fn read(
    program: &[u8],
    context: &impl CharstringContext,
    transform: Affine,
    contours: &[Contour],
    budget: &mut usize,
) -> Result<PsHints, &'static str> {
    let mut scan = Scan::default();
    scan.walk(program, context, 0, budget)?;
    if scan.result.is_some()
        || scan.replacement_call
        || scan.flex.is_some()
        || scan.flex_results > 0
        || scan.expect_setcurrentpoint
        || scan.dot_open.is_some()
        || (!scan.counter_calls.is_empty() && !scan.counter_done)
    {
        return Err("unconsumed OtherSubrs result");
    }
    if scan
        .seac_contours
        .is_some_and(|count| count != contours.len())
    {
        return Err("seac contours differ from evaluated outline");
    }
    scan.bind(contours)?;
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
                // Ghost widths are format markers. Only the real edge scales.
                let edge = stem.start + if stem.width == -21.0 { -21.0 } else { 0.0 };
                stem.start = edge * scale + offset + if stem.width == -21.0 { 21.0 } else { 0.0 };
            } else {
                stem.start = stem.start * scale + offset;
                stem.width *= scale;
            }
        }
    }
    fc_model::query::path_hint_masks(&scan.values, contours)
        .map_err(|_| "invalid or overlapping active stem hints")?;
    Ok(scan.values)
}

// Stable references survive the final horizontal-then-vertical bit ordering.
type Stem = (bool, usize);

struct Event {
    contour: usize,
    point: usize,
    closing: bool,
    active: Vec<Stem>,
}

#[derive(Default)]
struct Scan {
    values: PsHints,
    stack: Vec<f64>,
    sidebearing: Option<f64>,
    result: Option<f64>,
    replacement_call: bool,
    hint_only: bool,
    active: Vec<Stem>,
    axis_modes: [Option<bool>; 2],
    counters: Vec<Vec<Stem>>,
    general_counters: Vec<CounterGroup>,
    counter_calls: Vec<Vec<f64>>,
    counter_done: bool,
    events: Vec<Event>,
    contour: Option<usize>,
    point: usize,
    closed: bool,
    dirty: bool,
    in_seac: bool,
    seac_contours: Option<usize>,
    flex: Option<FlexScan>,
    flex_results: u8,
    expect_setcurrentpoint: bool,
    dot_open: Option<usize>,
}

/// Accumulates the seven points of one in-progress flex sequence.
struct FlexScan {
    contour: usize,
    point: usize,
    collected: u8,
    moved: bool,
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
        // Charge every decoded byte, including unreachable tails. Repeated
        // calls to a large subroutine ending with an early return stay bounded.
        *budget = budget
            .checked_sub(program.len())
            .ok_or("hint inspection complexity limit exceeded")?;
        for token in tokens(program).map_err(|_| "invalid charstring")? {
            if self.result.is_some() && !matches!(token, ProgramToken::Operator(0x0c11))
                || self.replacement_call && !matches!(token, ProgramToken::Operator(10))
                || self.flex_results > 0 && !matches!(token, ProgramToken::Operator(0x0c11))
                || self.expect_setcurrentpoint && !matches!(token, ProgramToken::Operator(0x0c21))
            {
                return Err("nonstandard hint replacement result sequence");
            }
            if self.hint_only
                && matches!(token, ProgramToken::Operator(op)
                if !matches!(op, 1 | 3 | 10 | 11 | 0x0c01 | 0x0c02 | 0x0c0c))
            {
                return Err("hint replacement subroutine contains non-hint operators");
            }
            match token {
                ProgramToken::Integer(value) => self.push(f64::from(value))?,
                ProgramToken::Operator(0x0c0c) => {
                    let denominator = self.pop()?;
                    let value = self.pop()? / denominator;
                    if !value.is_finite() {
                        return Err("invalid division");
                    }
                    self.push(value)?;
                }
                ProgramToken::Operator(13) => {
                    self.operands(2)?;
                    if self.sidebearing.is_some() || self.contour.is_some() {
                        return Err("invalid sidebearing");
                    }
                    self.sidebearing = self.stack.first().copied();
                    self.stack.clear();
                }
                ProgramToken::Operator(opcode @ (1 | 3)) => {
                    self.operands(2)?;
                    self.axis_mode(opcode == 1, false)?;
                    self.stem(opcode == 1, self.stack[0], self.stack[1])?;
                    self.stack.clear();
                }
                ProgramToken::Operator(opcode @ (0x0c01 | 0x0c02)) => {
                    self.operands(6)?;
                    let horizontal = opcode == 0x0c02;
                    self.axis_mode(horizontal, true)?;
                    let mut group = Vec::new();
                    for i in [0, 2, 4] {
                        group.push(self.stem(horizontal, self.stack[i], self.stack[i + 1])?);
                    }
                    let stems = if horizontal {
                        &self.values.h_stems
                    } else {
                        &self.values.v_stems
                    };
                    let triple: Vec<_> = group.iter().map(|(_, i)| &stems[*i]).collect();
                    if !valid_triplet(&triple) {
                        return Err("invalid Type 1 counter triple");
                    }
                    if let Some(previous) = self.counters.iter().find(|g| g[0].0 == horizontal) {
                        if *previous != group {
                            return Err("counter triple changes across hint replacement");
                        }
                    } else {
                        self.counters.push(group);
                    }
                    self.stack.clear();
                }
                ProgramToken::Operator(0x0c10) => {
                    let procedure = self.pop()?;
                    let count = self.pop()?;
                    match (procedure, count) {
                        // The standard hint replacement procedure returns its one
                        // argument, which the following pop transfers back here.
                        (3.0, 1.0) => {
                            if self.result.is_some() {
                                return Err("unsupported OtherSubrs hint mechanism");
                            }
                            let index = self.pop()?;
                            if index.fract() != 0.0 || !(4.0..=65535.0).contains(&index) {
                                return Err("invalid hint replacement subroutine");
                            }
                            self.result = Some(index);
                            self.active.clear();
                            self.dirty = true;
                        }
                        (1.0, 0.0) => self.begin_flex()?,
                        (2.0, 0.0) => self.collect_flex()?,
                        (0.0, 3.0) => self.end_flex()?,
                        (12.0, _) | (13.0, _) => self.counter_call(procedure, count)?,
                        _ => return Err("unsupported OtherSubrs hint mechanism"),
                    }
                }
                ProgramToken::Operator(0x0c11) => {
                    if self.flex_results > 0 {
                        // OtherSubrs 0 returns the end coordinate for the
                        // setcurrentpoint that follows; the values are already known.
                        self.flex_results -= 1;
                        self.push(0.0)?;
                        self.expect_setcurrentpoint = self.flex_results == 0;
                        continue;
                    }
                    let value = self.result.take().ok_or("missing OtherSubrs result")?;
                    self.push(value)?;
                    self.replacement_call = true;
                }
                ProgramToken::Operator(0x0c21) => {
                    if !self.expect_setcurrentpoint {
                        return Err("setcurrentpoint outside a flex sequence");
                    }
                    self.operands(2)?;
                    self.expect_setcurrentpoint = false;
                    self.stack.clear();
                }
                ProgramToken::Operator(0x0c00) => {
                    self.operands(0)?;
                    self.dot_section()?;
                }
                ProgramToken::Operator(10) => {
                    let index = self.pop()?;
                    if index.fract() != 0.0 || !(0.0..=65535.0).contains(&index) {
                        return Err("invalid subroutine index");
                    }
                    let subr = context
                        .subr(index as i32)
                        .map_err(|_| "missing subroutine")?;
                    let previous = self.hint_only;
                    self.hint_only |= self.replacement_call;
                    self.replacement_call = false;
                    let ended = self.walk(subr, context, depth + 1, budget)?;
                    self.hint_only = previous;
                    if ended {
                        return Ok(true);
                    }
                }
                ProgramToken::Operator(11) => return Ok(false),
                ProgramToken::Operator(14) => {
                    self.operands(0)?;
                    if self.flex.is_some() || self.dot_open.is_some() {
                        return Err("unterminated flex or dot section");
                    }
                    return Ok(true);
                }
                ProgramToken::Operator(0x0c06) => {
                    self.seac(context, depth, budget)?;
                    return Ok(true);
                }
                ProgramToken::Operator(21) if self.flex.is_some() => {
                    // Inside flex an rmoveto only supplies the next flex point.
                    self.operands(2)?;
                    let Some(flex) = self.flex.as_mut() else {
                        return Err("flex state lost");
                    };
                    if flex.moved {
                        return Err("flex point is not followed by OtherSubrs 2");
                    }
                    flex.moved = true;
                    self.stack.clear();
                }
                ProgramToken::Operator(opcode @ (4..=9 | 21 | 22 | 30 | 31)) => {
                    if self.flex.is_some() {
                        return Err("drawing operator inside a flex sequence");
                    }
                    self.draw(opcode)?;
                    self.stack.clear();
                }
                // Any remaining mechanism has no representation here. Never
                // misapply a partial set of hints.
                ProgramToken::Operator(_) => return Err("unsupported charstring hint mechanism"),
            }
        }
        Ok(false)
    }

    fn operands(&self, count: usize) -> Result<(), &'static str> {
        if self.stack.len() == count {
            Ok(())
        } else {
            Err("invalid Type 1 operand count")
        }
    }

    fn pop(&mut self) -> Result<f64, &'static str> {
        self.stack.pop().ok_or("missing charstring operand")
    }

    fn push(&mut self, value: f64) -> Result<(), &'static str> {
        if self.stack.len() >= 24 {
            return Err("hint operand stack limit exceeded");
        }
        self.stack.push(value);
        Ok(())
    }

    fn axis_mode(&mut self, horizontal: bool, counter: bool) -> Result<(), &'static str> {
        let mode = &mut self.axis_modes[usize::from(horizontal)];
        if mode.is_some_and(|value| value != counter) {
            return Err("Type 1 stem3 cannot mix with ordinary stems on the same axis");
        }
        *mode = Some(counter);
        Ok(())
    }

    fn stem(&mut self, horizontal: bool, start: f64, width: f64) -> Result<Stem, &'static str> {
        let sidebearing = self.sidebearing.ok_or("stem precedes sidebearing")?;
        let start = start + if horizontal { 0.0 } else { sidebearing };
        let count = self.values.h_stems.len() + self.values.v_stems.len();
        let stems = if horizontal {
            &mut self.values.h_stems
        } else {
            &mut self.values.v_stems
        };
        let index = if let Some(index) = stems
            .iter()
            .position(|s| s.start == start && s.width == width)
        {
            index
        } else {
            if count >= 96 {
                return Err("stem count limit exceeded");
            }
            stems.push(StemHint {
                start,
                width,
                ..Default::default()
            });
            stems.len() - 1
        };
        let id = (horizontal, index);
        if !self.active.contains(&id) {
            self.active.push(id);
            self.active.sort_unstable();
            self.dirty = true;
        }
        Ok(id)
    }

    fn draw(&mut self, opcode: u16) -> Result<(), &'static str> {
        let count = match opcode {
            4 | 6 | 7 | 22 => 1,
            5 | 21 => 2,
            8 => 6,
            30 | 31 => 4,
            _ => 0,
        };
        self.operands(count)?;
        let closing = opcode == 9;
        match opcode {
            4 | 21 | 22 => {
                self.contour = Some(self.contour.map_or(0, |i| i + 1));
                self.point = 0;
                self.closed = false;
            }
            9 => {
                if self.contour.is_none() || self.closed {
                    return Err("close outside contour");
                }
                self.closed = true;
            }
            _ => {
                if self.contour.is_none() || self.closed {
                    return Err("drawing outside contour");
                }
                self.point += if matches!(opcode, 8 | 30 | 31) { 3 } else { 1 };
            }
        }
        self.record(closing)
    }

    fn record(&mut self, closing: bool) -> Result<(), &'static str> {
        if self.events.is_empty() || self.dirty {
            if self.events.len() >= 65535 {
                return Err("hint mask count limit exceeded");
            }
            self.events.push(Event {
                contour: self.contour.ok_or("missing contour")?,
                point: if closing { 0 } else { self.point },
                closing,
                active: self.active.clone(),
            });
            self.dirty = false;
        }
        Ok(())
    }

    /// Type 1 section 8.3: `Subrs` 1 begins a flex at the current point.
    fn begin_flex(&mut self) -> Result<(), &'static str> {
        self.operands(0)?;
        if self.flex.is_some() || self.closed {
            return Err("nested or closed flex sequence");
        }
        let contour = self.contour.ok_or("flex outside a contour")?;
        self.flex = Some(FlexScan {
            contour,
            point: self.point,
            collected: 0,
            moved: false,
        });
        Ok(())
    }

    /// `Subrs` 2 accepts one of the seven flex coordinate pairs.
    fn collect_flex(&mut self) -> Result<(), &'static str> {
        self.operands(0)?;
        let flex = self.flex.as_mut().ok_or("flex point outside a flex")?;
        if !flex.moved || flex.collected == 7 {
            return Err("unexpected flex point");
        }
        flex.moved = false;
        flex.collected += 1;
        Ok(())
    }

    /// `Subrs` 0 finishes a flex, leaving two values for `setcurrentpoint`.
    fn end_flex(&mut self) -> Result<(), &'static str> {
        self.operands(3)?;
        let flex = self.flex.take().ok_or("flex end outside a flex")?;
        if flex.collected != 7 || flex.moved {
            return Err("a flex needs exactly seven coordinate pairs");
        }
        let depth = self.stack[0];
        if !depth.is_finite() || !(0.0..=1000.0).contains(&depth) {
            return Err("invalid flex height control parameter");
        }
        self.stack.clear();
        if self.values.flex_hints.len() >= 64 {
            return Err("flex hint count limit exceeded");
        }
        self.values.flex_hints.push(fc_model::FlexHint {
            contour: flex.contour,
            point: flex.point,
            depth,
        });
        // The two curves add six stored points; the reference point is not one.
        for _ in 0..2 {
            self.point += 3;
            self.record(false)?;
        }
        self.flex_results = 2;
        Ok(())
    }

    /// Type 1 section 8.2: `dotsection` brackets one closed dot subpath.
    fn dot_section(&mut self) -> Result<(), &'static str> {
        let contour = self.contour.ok_or("dot section outside a contour")?;
        match self.dot_open {
            None => {
                if self.closed || self.point != 0 {
                    return Err("a dot section must open right after its rmoveto");
                }
                self.dot_open = Some(contour);
            }
            Some(open) => {
                if open != contour || !self.closed {
                    return Err("a dot section must close right after its closepath");
                }
                if self.values.dot_sections.len() >= 64 {
                    return Err("dot section count limit exceeded");
                }
                self.values.dot_sections.push(contour);
                self.dot_open = None;
            }
        }
        Ok(())
    }

    /// Accumulates one general counter-control call (Technical Note #5015 2.2).
    fn counter_call(&mut self, procedure: f64, count: f64) -> Result<(), &'static str> {
        if self.counter_done
            || self.sidebearing.is_none()
            || self.contour.is_some()
            || !self.values.h_stems.is_empty()
            || !self.values.v_stems.is_empty()
        {
            return Err("counter control must immediately follow hsbw");
        }
        if count.fract() != 0.0 || !(0.0..=22.0).contains(&count) {
            return Err("invalid counter-control argument count");
        }
        if self.stack.len() != count as usize || self.counter_calls.len() >= 10 {
            return Err("invalid counter-control call");
        }
        self.counter_calls.push(std::mem::take(&mut self.stack));
        if procedure == 13.0 {
            self.counter_done = true;
            // The interpreter replays the calls in reverse order, so the final
            // call carries the start of the data and the first call its tail.
            let data: Vec<f64> = self
                .counter_calls
                .iter()
                .rev()
                .flat_map(|call| call.iter().copied())
                .collect();
            self.general_counters = parse_counter_data(&data)?;
        }
        Ok(())
    }

    fn bind(&mut self, contours: &[Contour]) -> Result<(), &'static str> {
        let count = self.values.h_stems.len() + self.values.v_stems.len();
        if count == 0 {
            if !self.general_counters.is_empty() {
                return Err("counter-control data without any declared stem hint");
            }
            return Ok(());
        }
        // An unchanged set needs no active mask and also works for empty glyphs.
        let unchanged = self.events.iter().all(|e| e.active.len() == count);
        let mut previous = None;
        for event in self.events.iter().filter(|_| !unchanged) {
            let contour = contours
                .get(event.contour)
                .ok_or("hint contour differs from evaluated outline")?;
            let closing = event.closing || event.point == contour.points.len();
            let point = if closing { 0 } else { event.point };
            if point >= contour.points.len() {
                return Err("hint point differs from evaluated outline");
            }
            let mut mask = vec![0; count.div_ceil(8)];
            for &(horizontal, index) in &event.active {
                let index = index
                    + if horizontal {
                        0
                    } else {
                        self.values.h_stems.len()
                    };
                mask[index / 8] |= 0x80 >> (index % 8);
            }
            if previous.as_ref() == Some(&mask) {
                continue;
            }
            previous = Some(mask.clone());
            self.values.hint_masks.push(HintMask {
                counter: false,
                closing,
                contour: event.contour,
                point,
                mask,
            });
        }
        for group in &self.counters {
            let mut mask = vec![0; count.div_ceil(8)];
            for &(horizontal, index) in group {
                let index = index
                    + if horizontal {
                        0
                    } else {
                        self.values.h_stems.len()
                    };
                mask[index / 8] |= 0x80 >> (index % 8);
            }
            self.values.hint_masks.push(HintMask {
                counter: true,
                mask,
                ..Default::default()
            });
        }
        let mut general = Vec::new();
        for (horizontal, group) in &self.general_counters {
            let (stems, offset, sidebearing) = if *horizontal {
                (&self.values.h_stems, 0, 0.0)
            } else {
                (
                    &self.values.v_stems,
                    self.values.h_stems.len(),
                    self.sidebearing.unwrap_or_default(),
                )
            };
            let mut mask = vec![0; count.div_ceil(8)];
            for (start, width) in group {
                let start = start + sidebearing;
                let index = stems
                    .iter()
                    .position(|stem| {
                        (stem.start - start).abs() <= 1.0 / 65536.0
                            && (stem.width - width).abs() <= 1.0 / 65536.0
                    })
                    .ok_or("counter-control stem is not declared as a stem hint")?;
                let index = offset + index;
                mask[index / 8] |= 0x80 >> (index % 8);
            }
            general.push(HintMask {
                counter: true,
                mask,
                ..Default::default()
            });
        }
        self.values.hint_masks.extend(general);
        Ok(())
    }
}

/// One counter-control group: whether it is horizontal, and its absolute stems.
type CounterGroup = (bool, Vec<(f64, f64)>);

/// Parses general counter-control data into absolute stems per group.
///
/// Technical Note #5015 section 2.4: each group lists pairs of
/// (distance from the previous stem's far edge, width) in ascending order, and
/// the final pair gives the distance to the last stem's far edge with a negated
/// width. Horizontal groups come first, each list preceded by its count.
fn parse_counter_data(data: &[f64]) -> Result<Vec<CounterGroup>, &'static str> {
    let mut values = data.iter().copied();
    let mut result = Vec::new();
    for horizontal in [true, false] {
        let count = values.next().ok_or("truncated counter-control data")?;
        if count.fract() != 0.0 || !(0.0..=32.0).contains(&count) {
            return Err("invalid counter-control group count");
        }
        for _ in 0..count as usize {
            let mut stems: Vec<(f64, f64)> = Vec::new();
            let mut previous = 0.0;
            loop {
                let delta = values.next().ok_or("truncated counter-control group")?;
                let width = values.next().ok_or("truncated counter-control group")?;
                if !delta.is_finite() || !width.is_finite() || width == 0.0 {
                    return Err("invalid counter-control stem");
                }
                if stems.len() >= 48 {
                    return Err("counter-control group limit exceeded");
                }
                if width < 0.0 {
                    stems.push((previous + delta + width, -width));
                    break;
                }
                stems.push((previous + delta, width));
                previous += delta + width;
            }
            if stems.len() < 2 {
                return Err("a counter-control group needs at least two stems");
            }
            if stems
                .windows(2)
                .any(|pair| pair[0].0 + pair[0].1 > pair[1].0)
            {
                return Err("counter-control stems in one group overlap");
            }
            result.push((horizontal, stems));
        }
    }
    if values.next().is_some() {
        return Err("excess counter-control data");
    }
    Ok(result)
}

/// Type 1 section 6.4: positive widths, equal outer widths and equal center spacing.
pub(super) fn valid_triplet(stems: &[&StemHint]) -> bool {
    stems.len() == 3
        && stems.iter().all(|s| s.width > 0.0)
        && stems
            .windows(2)
            .all(|pair| pair[0].start + pair[0].width <= pair[1].start)
        && (stems[0].width - stems[2].width).abs() <= 1.0 / 65536.0
        && ((stems[0].start + stems[0].width / 2.0) - 2.0 * (stems[1].start + stems[1].width / 2.0)
            + (stems[2].start + stems[2].width / 2.0))
            .abs()
            <= 1.0 / 65536.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use read_fonts::ps::{cs::CharstringKind, error::Error};

    struct Context(Vec<u8>);
    impl CharstringContext for Context {
        fn kind(&self) -> CharstringKind {
            CharstringKind::Type1
        }
        fn subr(&self, _: i32) -> Result<&[u8], Error> {
            Ok(&self.0)
        }
        fn global_subr(&self, _: i32) -> Result<&[u8], Error> {
            Err(Error::MissingSubroutines)
        }
        fn seac_components(&self, _: i32, _: i32) -> Result<[&[u8]; 2], Error> {
            Err(Error::MissingSubroutines)
        }
    }

    #[test]
    fn bounded_inspection_covers_recursion_stack_stems_and_unreachable_tails() {
        let mut budget = 1000;
        assert!(Scan::default()
            .walk(&[139, 10], &Context(vec![139, 10]), 0, &mut budget)
            .unwrap_err()
            .contains("complexity"));
        let mut budget = 1000;
        assert!(Scan::default()
            .walk(&[139; 25], &Context(vec![]), 0, &mut budget)
            .unwrap_err()
            .contains("stack"));
        let mut subr = vec![139; 65535];
        subr[0] = 11;
        let mut budget = 1000;
        assert!(Scan::default()
            .walk(&[139, 10], &Context(subr), 0, &mut budget)
            .unwrap_err()
            .contains("complexity"));
        let mut program = vec![139, 139, 13];
        for i in 0..97 {
            program.extend([139 + i, 140, 3]);
        }
        let mut budget = 1000;
        assert!(Scan::default()
            .walk(&program, &Context(vec![]), 0, &mut budget)
            .unwrap_err()
            .contains("stem count"));
    }
}
