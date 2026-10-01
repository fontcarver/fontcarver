//! Bounded Type 2 charstring scan that recovers flex hints.
//!
//! The outline dependency expands `flex`, `hflex`, `hflex1` and `flex1` into two
//! ordinary curves before any sink callback runs, so a flex hint would otherwise
//! disappear on import without a trace. This scan walks the same charstring
//! independently, tracking only the current point and the stem count, and reports
//! the seven coordinates of every flex it finds so the importer can bind the hint
//! to the evaluated outline instead of dropping it.
//!
//! It validates nothing beyond what it needs; malformed input ends the scan with
//! an error, and the caller discloses that flex detection was incomplete rather
//! than claiming the glyph had none.

use kurbo::Point;
use read_fonts::{ps::cff::index::Index, types::Fixed};

/// One recovered flex hint: the seven points it draws and its height parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Flex {
    /// Current point before the flex, then the two curves' six points.
    pub(super) points: [Point; 7],
    /// Flex height control parameter in hundredths of a device pixel.
    pub(super) depth: f64,
}

/// What one charstring scan found.
#[derive(Debug, Default)]
pub(super) struct Scanned {
    /// Every flex hint the charstring draws, in charstring order.
    pub(super) flexes: Vec<Flex>,
    /// Whether the charstring builds an accented composition through `seac`.
    pub(super) seac: bool,
    /// Whether the charstring uses the obsolete Type 2 `dotsection` operator.
    pub(super) dotsection: bool,
}

/// Why a charstring could not be scanned for flex.
pub(super) type Error = &'static str;

/// The subroutine indexes one charstring can call.
pub(super) struct Subroutines<'a> {
    /// Local subroutines from the glyph's Private DICT, when it declares any.
    pub(super) local: Option<Index<'a>>,
    /// Global subroutines from the CFF Top DICT.
    pub(super) global: &'a Index<'a>,
}

/// Scans one Type 2 charstring, returning every flex hint it draws.
pub(super) fn scan(
    charstring: &[u8],
    subroutines: &Subroutines<'_>,
    budget: &mut usize,
) -> Result<Scanned, Error> {
    let mut state = Scan {
        subroutines,
        stack: Vec::new(),
        current: Point::ZERO,
        stems: 0,
        width_pending: true,
        found: Scanned::default(),
    };
    state.walk(charstring, 0, budget)?;
    Ok(state.found)
}

struct Scan<'a> {
    subroutines: &'a Subroutines<'a>,
    stack: Vec<f64>,
    current: Point,
    stems: usize,
    width_pending: bool,
    found: Scanned,
}

impl Scan<'_> {
    fn walk(&mut self, code: &[u8], depth: usize, budget: &mut usize) -> Result<bool, Error> {
        if depth > 10 {
            return Err("charstring subroutine nesting limit exceeded");
        }
        *budget = budget
            .checked_sub(code.len())
            .ok_or("charstring flex scan work limit exceeded")?;
        let mut at = 0usize;
        while at < code.len() {
            let byte = code[at];
            at += 1;
            match byte {
                32..=246 => self.push(f64::from(i32::from(byte) - 139))?,
                247..=250 => {
                    let low = *code.get(at).ok_or("truncated operand")?;
                    at += 1;
                    self.push(f64::from(
                        (i32::from(byte) - 247) * 256 + i32::from(low) + 108,
                    ))?;
                }
                251..=254 => {
                    let low = *code.get(at).ok_or("truncated operand")?;
                    at += 1;
                    self.push(f64::from(
                        -(i32::from(byte) - 251) * 256 - i32::from(low) - 108,
                    ))?;
                }
                28 => {
                    let bytes = code.get(at..at + 2).ok_or("truncated operand")?;
                    at += 2;
                    self.push(f64::from(i16::from_be_bytes([bytes[0], bytes[1]])))?;
                }
                255 => {
                    let bytes = code.get(at..at + 4).ok_or("truncated operand")?;
                    at += 4;
                    let raw = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    self.push(Fixed::from_bits(raw).to_f64())?;
                }
                19 | 20 => {
                    // An implicit vstem may precede the first mask.
                    self.count_stems();
                    at += self.stems.div_ceil(8).max(1);
                    if at > code.len() {
                        return Err("truncated hint mask");
                    }
                }
                1 | 3 | 18 | 23 => self.count_stems(),
                21 => {
                    self.trim(2)?;
                    self.move_by(self.stack[0], self.stack[1]);
                    self.stack.clear();
                }
                22 => {
                    self.trim(1)?;
                    self.move_by(self.stack[0], 0.0);
                    self.stack.clear();
                }
                4 => {
                    self.trim(1)?;
                    self.move_by(0.0, self.stack[0]);
                    self.stack.clear();
                }
                5 => self.lines()?,
                6 | 7 => self.alternating_lines(byte == 6)?,
                8 => self.curves(0)?,
                24 => {
                    // rcurveline: curves then one line.
                    let count = self.stack.len();
                    if count < 8 || !(count - 2).is_multiple_of(6) {
                        return Err("invalid rcurveline operand count");
                    }
                    self.curves(2)?;
                    let (dx, dy) = (self.stack[count - 2], self.stack[count - 1]);
                    self.current = Point::new(self.current.x + dx, self.current.y + dy);
                    self.stack.clear();
                }
                25 => {
                    // rlinecurve: lines then one curve.
                    let count = self.stack.len();
                    if count < 8 || !(count - 6).is_multiple_of(2) {
                        return Err("invalid rlinecurve operand count");
                    }
                    for pair in self.stack[..count - 6].chunks_exact(2) {
                        self.current =
                            Point::new(self.current.x + pair[0], self.current.y + pair[1]);
                    }
                    let tail: Vec<f64> = self.stack[count - 6..].to_vec();
                    self.emit_curve(&tail);
                    self.stack.clear();
                }
                26 | 27 => self.axis_curves(byte == 27)?,
                30 | 31 => self.alternating_curves(byte == 31)?,
                10 | 29 => {
                    let index = self.stack.pop().ok_or("missing subroutine index")?;
                    if index.fract() != 0.0 || !(-32768.0..=32767.0).contains(&index) {
                        return Err("invalid subroutine index");
                    }
                    let subroutine = if byte == 10 {
                        let local = self
                            .subroutines
                            .local
                            .as_ref()
                            .ok_or("charstring calls a subroutine the Private DICT omits")?;
                        local.get((index as i32 + local.subr_bias()) as usize)
                    } else {
                        let global = self.subroutines.global;
                        global.get((index as i32 + global.subr_bias()) as usize)
                    }
                    .map_err(|_| "missing charstring subroutine")?;
                    if self.walk(subroutine, depth + 1, budget)? {
                        return Ok(true);
                    }
                }
                11 => return Ok(false),
                14 => {
                    // Four or five trailing operands select a seac composition.
                    self.found.seac = self.stack.len() >= 4;
                    return Ok(true);
                }
                12 => {
                    let second = *code.get(at).ok_or("truncated escaped operator")?;
                    at += 1;
                    match second {
                        0 => {
                            // Obsolete in Type 2; recorded so the caller can
                            // disclose that it was not retained.
                            self.found.dotsection = true;
                            self.stack.clear();
                        }
                        34 => self.flex_h()?,
                        35 => self.flex_full()?,
                        36 => self.flex_h1()?,
                        37 => self.flex_1()?,
                        _ => return Err("unsupported escaped charstring operator"),
                    }
                }
                _ => return Err("unsupported charstring operator"),
            }
        }
        Ok(false)
    }

    fn push(&mut self, value: f64) -> Result<(), Error> {
        if self.stack.len() >= 48 || !value.is_finite() {
            return Err("charstring operand stack limit exceeded");
        }
        self.stack.push(value);
        Ok(())
    }

    /// Drops the optional leading width so the remaining operands are the deltas.
    fn trim(&mut self, expected: usize) -> Result<(), Error> {
        if self.width_pending && self.stack.len() == expected + 1 {
            self.stack.remove(0);
        }
        self.width_pending = false;
        if self.stack.len() != expected {
            return Err("invalid moveto operand count");
        }
        Ok(())
    }

    fn count_stems(&mut self) {
        if self.width_pending && !self.stack.len().is_multiple_of(2) {
            self.stack.remove(0);
        }
        self.width_pending = false;
        self.stems += self.stack.len() / 2;
        self.stack.clear();
    }

    fn move_by(&mut self, dx: f64, dy: f64) {
        self.current = Point::new(self.current.x + dx, self.current.y + dy);
    }

    fn lines(&mut self) -> Result<(), Error> {
        self.width_pending = false;
        if self.stack.is_empty() || !self.stack.len().is_multiple_of(2) {
            return Err("invalid rlineto operand count");
        }
        for pair in self.stack.chunks_exact(2) {
            self.current = Point::new(self.current.x + pair[0], self.current.y + pair[1]);
        }
        self.stack.clear();
        Ok(())
    }

    fn alternating_lines(&mut self, mut horizontal: bool) -> Result<(), Error> {
        self.width_pending = false;
        if self.stack.is_empty() {
            return Err("invalid alternating lineto operand count");
        }
        let values: Vec<f64> = std::mem::take(&mut self.stack);
        for value in values {
            self.current = if horizontal {
                Point::new(self.current.x + value, self.current.y)
            } else {
                Point::new(self.current.x, self.current.y + value)
            };
            horizontal = !horizontal;
        }
        Ok(())
    }

    /// Consumes groups of six operands as curves, leaving `tail` operands unused.
    fn curves(&mut self, tail: usize) -> Result<(), Error> {
        self.width_pending = false;
        let available = self
            .stack
            .len()
            .checked_sub(tail)
            .ok_or("invalid curve operand count")?;
        if available == 0 || !available.is_multiple_of(6) {
            return Err("invalid curve operand count");
        }
        let values: Vec<f64> = self.stack[..available].to_vec();
        for group in values.chunks_exact(6) {
            self.emit_curve(group);
        }
        if tail == 0 {
            self.stack.clear();
        }
        Ok(())
    }

    fn emit_curve(&mut self, deltas: &[f64]) {
        let mut point = self.current;
        for pair in deltas.chunks_exact(2) {
            point = Point::new(point.x + pair[0], point.y + pair[1]);
        }
        self.current = point;
    }

    /// `vvcurveto` and `hhcurveto`: an optional cross delta then groups of four.
    fn axis_curves(&mut self, horizontal: bool) -> Result<(), Error> {
        self.width_pending = false;
        let mut values: Vec<f64> = std::mem::take(&mut self.stack);
        let mut cross = 0.0;
        if values.len() % 4 == 1 {
            cross = values.remove(0);
        } else if !values.len().is_multiple_of(4) || values.is_empty() {
            return Err("invalid axis curve operand count");
        }
        for group in values.chunks_exact(4) {
            let deltas = if horizontal {
                [group[0], cross, group[1], group[2], group[3], 0.0]
            } else {
                [cross, group[0], group[1], group[2], 0.0, group[3]]
            };
            self.emit_curve(&deltas);
            cross = 0.0;
        }
        Ok(())
    }

    /// `hvcurveto` and `vhcurveto`: alternating tangents with an optional last delta.
    fn alternating_curves(&mut self, mut horizontal: bool) -> Result<(), Error> {
        self.width_pending = false;
        let values: Vec<f64> = std::mem::take(&mut self.stack);
        if values.len() < 4 {
            return Err("invalid alternating curve operand count");
        }
        let remainder = values.len() % 4;
        if remainder != 0 && remainder != 1 {
            return Err("invalid alternating curve operand count");
        }
        let groups = values.len() / 4;
        for index in 0..groups {
            let group = &values[index * 4..index * 4 + 4];
            let last = index + 1 == groups;
            let extra = if last && remainder == 1 {
                values[values.len() - 1]
            } else {
                0.0
            };
            let deltas = if horizontal {
                [group[0], 0.0, group[1], group[2], extra, group[3]]
            } else {
                [0.0, group[0], group[1], group[2], group[3], extra]
            };
            self.emit_curve(&deltas);
            horizontal = !horizontal;
        }
        Ok(())
    }

    fn record_flex(&mut self, deltas: [f64; 12], depth: f64) -> Result<(), Error> {
        if self.found.flexes.len() >= 64 {
            return Err("flex hint count limit exceeded");
        }
        let mut points = [self.current; 7];
        let mut point = self.current;
        for (index, pair) in deltas.chunks_exact(2).enumerate() {
            point = Point::new(point.x + pair[0], point.y + pair[1]);
            points[index + 1] = point;
        }
        self.current = point;
        self.found.flexes.push(Flex { points, depth });
        self.stack.clear();
        Ok(())
    }

    fn flex_full(&mut self) -> Result<(), Error> {
        self.width_pending = false;
        if self.stack.len() != 13 {
            return Err("invalid flex operand count");
        }
        let mut deltas = [0.0; 12];
        deltas.copy_from_slice(&self.stack[..12]);
        let depth = self.stack[12];
        self.record_flex(deltas, depth)
    }

    fn flex_1(&mut self) -> Result<(), Error> {
        self.width_pending = false;
        if self.stack.len() != 11 {
            return Err("invalid flex1 operand count");
        }
        let (dx, dy) = self.stack[..10]
            .chunks_exact(2)
            .fold((0.0, 0.0), |sum, pair| (sum.0 + pair[0], sum.1 + pair[1]));
        let last = self.stack[10];
        let (dx6, dy6) = if dx.abs() > dy.abs() {
            (last, -dy)
        } else {
            (-dx, last)
        };
        let mut deltas = [0.0; 12];
        deltas[..10].copy_from_slice(&self.stack[..10]);
        deltas[10] = dx6;
        deltas[11] = dy6;
        self.record_flex(deltas, 50.0)
    }

    fn flex_h(&mut self) -> Result<(), Error> {
        self.width_pending = false;
        if self.stack.len() != 7 {
            return Err("invalid hflex operand count");
        }
        let v = &self.stack;
        let deltas = [
            v[0], 0.0, v[1], v[2], v[3], 0.0, v[4], 0.0, v[5], -v[2], v[6], 0.0,
        ];
        self.record_flex(deltas, 50.0)
    }

    fn flex_h1(&mut self) -> Result<(), Error> {
        self.width_pending = false;
        if self.stack.len() != 9 {
            return Err("invalid hflex1 operand count");
        }
        let v = &self.stack;
        let sum = v[1] + v[3] + v[7];
        let deltas = [
            v[0], v[1], v[2], v[3], v[4], 0.0, v[5], 0.0, v[6], v[7], v[8], -sum,
        ];
        self.record_flex(deltas, 50.0)
    }
}
