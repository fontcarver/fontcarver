//! Bounded numeric expressions used by feature-source tokens.

use crate::{Font, Master, MasterId};

const MAX_EXPRESSION_BYTES: usize = 16_384;
const MAX_NESTING: usize = 16;
const MAX_NODES: usize = 256;

/// Expands one numeric token body to a rounded, signed 32-bit FEA integer.
pub(super) fn expand(font: &Font, master: &MasterId, expression: &str) -> Result<String, String> {
    if expression.len() > MAX_EXPRESSION_BYTES {
        return Err(format!(
            "numeric expression exceeds {MAX_EXPRESSION_BYTES} bytes"
        ));
    }
    let master = font
        .masters
        .iter()
        .find(|candidate| &candidate.id == master)
        .ok_or_else(|| format!("numeric expression references missing master '{master}'"))?;
    let mut parser = Parser {
        font,
        master,
        source: expression,
        offset: 0,
        nodes: 0,
    };
    let value = parser.sum(0)?;
    parser.whitespace();
    if parser.offset != expression.len() {
        return Err(format!(
            "unexpected numeric expression input at byte {}",
            parser.offset
        ));
    }
    let rounded = finite(value)?.round();
    if rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err("numeric expression result is outside the signed 32-bit range".to_owned());
    }
    Ok((rounded as i32).to_string())
}

struct Parser<'a> {
    font: &'a Font,
    master: &'a Master,
    source: &'a str,
    offset: usize,
    nodes: usize,
}

impl<'a> Parser<'a> {
    fn sum(&mut self, depth: usize) -> Result<f64, String> {
        let mut value = self.product(depth)?;
        loop {
            self.whitespace();
            let operator = match self.byte() {
                Some(operator @ (b'+' | b'-')) => operator,
                _ => return Ok(value),
            };
            self.offset += 1;
            self.node()?;
            let right = self.product(depth)?;
            value = finite(if operator == b'+' {
                value + right
            } else {
                value - right
            })?;
        }
    }

    fn product(&mut self, depth: usize) -> Result<f64, String> {
        let mut value = self.unary(depth)?;
        loop {
            self.whitespace();
            let operator = match self.byte() {
                Some(operator @ (b'*' | b'/')) => operator,
                _ => return Ok(value),
            };
            self.offset += 1;
            self.node()?;
            let right = self.unary(depth)?;
            if operator == b'/' && right == 0.0 {
                return Err("division by zero in numeric expression".to_owned());
            }
            value = finite(if operator == b'*' {
                value * right
            } else {
                value / right
            })?;
        }
    }

    fn unary(&mut self, depth: usize) -> Result<f64, String> {
        let mut negative = false;
        loop {
            self.whitespace();
            match self.byte() {
                Some(b'+') => {}
                Some(b'-') => negative = !negative,
                _ => break,
            }
            self.offset += 1;
            self.node()?;
        }
        let value = self.primary(depth)?;
        Ok(if negative { -value } else { value })
    }

    fn primary(&mut self, depth: usize) -> Result<f64, String> {
        self.whitespace();
        if self.byte() == Some(b'(') {
            if depth >= MAX_NESTING {
                return Err(format!(
                    "numeric expression exceeds {MAX_NESTING} nested parentheses"
                ));
            }
            self.offset += 1;
            let value = self.sum(depth + 1)?;
            self.whitespace();
            if self.byte() != Some(b')') {
                return Err("numeric expression is missing a closing parenthesis".to_owned());
            }
            self.offset += 1;
            return Ok(value);
        }
        self.node()?;
        let is_number = self.byte().is_some_and(|byte| byte.is_ascii_digit())
            || (self.byte() == Some(b'.')
                && self
                    .source
                    .as_bytes()
                    .get(self.offset + 1)
                    .is_some_and(u8::is_ascii_digit));
        if is_number {
            return self.number();
        }
        let name = self.identifier()?;
        self.whitespace();
        let value = if self.byte() == Some(b':') {
            self.offset += 1;
            self.whitespace();
            let property = self.identifier()?;
            self.glyph_metric(name, property)?
        } else {
            self.named_number(name)?
        };
        finite(value)
    }

    fn number(&mut self) -> Result<f64, String> {
        let start = self.offset;
        self.digits();
        if self.byte() == Some(b'.') {
            self.offset += 1;
            self.digits();
        }
        if matches!(self.byte(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.byte(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            self.digits();
        }
        let literal = &self.source[start..self.offset];
        finite(
            literal
                .parse::<f64>()
                .map_err(|_| format!("invalid numeric literal '{literal}'"))?,
        )
    }

    fn identifier(&mut self) -> Result<&'a str, String> {
        let start = self.offset;
        if !self
            .byte()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'.'))
        {
            return Err(format!(
                "expected a number or metric name at byte {}",
                self.offset
            ));
        }
        while self
            .byte()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
        {
            self.offset += 1;
        }
        Ok(&self.source[start..self.offset])
    }

    fn named_number(&self, name: &str) -> Result<f64, String> {
        let metrics = &self.master.metrics;
        let builtin = match name {
            "unitsPerEm" => Some(f64::from(self.font.info.units_per_em)),
            "ascender" => Some(metrics.ascender),
            "descender" => Some(metrics.descender),
            "xHeight" => Some(metrics.x_height),
            "capHeight" => Some(metrics.cap_height),
            "italicAngle" => Some(metrics.italic_angle),
            _ => None,
        };
        if let Some(value) = builtin {
            return Ok(value);
        }
        for data in [&self.master.user_data, &self.font.user_data] {
            let Some(numbers) = data.get("featureNumbers").filter(|value| !value.is_null()) else {
                continue;
            };
            let numbers = numbers
                .as_object()
                .ok_or_else(|| "featureNumbers must be an object of numeric values".to_owned())?;
            if let Some(value) = numbers.get(name) {
                return value
                    .as_f64()
                    .ok_or_else(|| format!("feature number '{name}' must have a numeric value"));
            }
        }
        Err(format!("unknown numeric metric or feature number '{name}'"))
    }

    fn glyph_metric(&self, name: &str, property: &str) -> Result<f64, String> {
        let glyph = self
            .font
            .glyphs
            .get(name)
            .ok_or_else(|| format!("numeric expression references missing glyph '{name}'"))?;
        let layer = glyph.master_layer(&self.master.id).ok_or_else(|| {
            format!(
                "glyph '{name}' has no layer for master '{}'",
                self.master.id
            )
        })?;
        match property {
            "width" => Some(layer.width),
            "verticalWidth" => layer.vertical_width,
            "verticalOrigin" => layer.vertical_origin,
            _ => return Err(format!("unsupported glyph metric '{property}'")),
        }
        .ok_or_else(|| format!("glyph '{name}' has no '{property}' metric"))
    }

    fn node(&mut self) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(format!(
                "numeric expression exceeds {MAX_NODES} values and operators"
            ));
        }
        Ok(())
    }

    fn byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.offset).copied()
    }

    fn digits(&mut self) {
        while self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
            self.offset += 1;
        }
    }

    fn whitespace(&mut self) {
        while self.byte().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.offset += 1;
        }
    }
}

fn finite(value: f64) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("numeric expression contains a non-finite value".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::expand;
    use crate::{Font, Glyph, Layer, Master, MasterId};
    use serde_json::json;

    fn fixture() -> Font {
        let mut font = Font::default();
        font.info.units_per_em = 1000;
        let master = &mut font.masters[0];
        master.metrics.ascender = 800.0;
        master.metrics.descender = -200.0;
        master.metrics.x_height = 500.0;
        master.metrics.cap_height = 700.0;
        master.metrics.italic_angle = -12.0;
        let mut layer = Layer::new_master(master.id.clone());
        layer.width = 601.0;
        layer.vertical_width = Some(1000.0);
        layer.vertical_origin = Some(880.0);
        let mut glyph = Glyph::new("A.sc");
        glyph.layers.push(layer);
        font.glyphs.insert(glyph.name.clone(), glyph);
        font
    }

    fn evaluate(font: &Font, source: &str) -> Result<String, String> {
        expand(font, &font.masters[0].id, source)
    }

    #[test]
    fn arithmetic_respects_precedence_and_rounds_once() {
        let font = fixture();
        for (source, expected) in [
            ("1 + 2 * 3", "7"),
            ("(1 + 2) * 3", "9"),
            ("20 / 2 / 5", "2"),
            ("10 - 3 - 2", "5"),
            ("--3 * -(2 + 1)", "-9"),
            ("+.25 + 1.25", "2"),
            ("-1.5", "-2"),
            ("1e2 + 2E+1 - 1e-1", "120"),
            ("-0.1", "0"),
            ("2147483647.4", "2147483647"),
            ("-2147483648.4", "-2147483648"),
        ] {
            assert_eq!(evaluate(&font, source).unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn reads_builtins_and_selected_master_glyph_metrics() {
        let mut font = fixture();
        assert_eq!(evaluate(&font, "unitsPerEm + descender").unwrap(), "800");
        assert_eq!(evaluate(&font, "ascender - capHeight").unwrap(), "100");
        assert_eq!(evaluate(&font, "xHeight + italicAngle").unwrap(), "488");
        assert_eq!(evaluate(&font, "A.sc:width / 2").unwrap(), "301");
        assert_eq!(
            evaluate(&font, "A.sc:verticalWidth - A.sc:verticalOrigin").unwrap(),
            "120"
        );
        let mut other = Master::default();
        other.metrics.ascender = 900.0;
        let mut layer = Layer::new_master(other.id.clone());
        layer.width = 750.0;
        font.glyphs.get_mut("A.sc").unwrap().layers.push(layer);
        let other_id = other.id.clone();
        font.masters.push(other);
        assert_eq!(
            expand(&font, &other_id, "ascender - A.sc:width").unwrap(),
            "150"
        );
    }

    #[test]
    fn named_numbers_use_master_override_without_changing_source() {
        let mut font = fixture();
        font.user_data = json!({"featureNumbers": {"spacing": 12, "offset": 5}});
        font.masters[0].user_data = json!({"featureNumbers": {"spacing": 20}});
        let before = font.clone();
        assert_eq!(evaluate(&font, "spacing + offset").unwrap(), "25");
        assert_eq!(font, before);
        font.masters[0].user_data["featureNumbers"]["spacing"] = json!("20");
        assert!(evaluate(&font, "spacing").unwrap_err().contains("numeric"));
    }

    #[test]
    fn missing_references_are_errors() {
        let mut font = fixture();
        for source in ["missing", "B:width", "A.sc:lsb", "A.sc:"] {
            assert!(evaluate(&font, source).is_err(), "{source}");
        }
        assert!(expand(&font, &MasterId::new(), "1").is_err());
        font.glyphs.get_mut("A.sc").unwrap().layers[0].vertical_width = None;
        assert!(evaluate(&font, "A.sc:verticalWidth").is_err());
        font.glyphs.get_mut("A.sc").unwrap().layers.clear();
        assert!(evaluate(&font, "A.sc:width").is_err());
    }

    #[test]
    fn malformed_nonfinite_and_out_of_range_values_are_errors() {
        let mut font = fixture();
        for source in [
            "",
            "1 +",
            "1 2",
            "1e+",
            "(1",
            "1)",
            "1 ** 2",
            "1 % 2",
            "1 / 0",
            "1 / -0",
            "1e309",
            "1e308 * 10",
            "2147483647.5",
            "-2147483648.5",
            "1;",
        ] {
            assert!(evaluate(&font, source).is_err(), "{source}");
        }
        font.masters[0].metrics.ascender = f64::NAN;
        assert!(evaluate(&font, "ascender").is_err());
        font.glyphs.get_mut("A.sc").unwrap().layers[0].width = f64::INFINITY;
        assert!(evaluate(&font, "A.sc:width").is_err());
    }

    #[test]
    fn bounds_parentheses_and_work_without_unary_recursion() {
        let font = fixture();
        assert_eq!(
            evaluate(&font, &format!("{}1{}", "(".repeat(16), ")".repeat(16))).unwrap(),
            "1"
        );
        assert!(evaluate(&font, &format!("{}1{}", "(".repeat(17), ")".repeat(17))).is_err());
        assert!(evaluate(&font, &format!("{}1", "-".repeat(256))).is_err());
        assert!(evaluate(&font, &vec!["1"; 129].join("+")).is_err());
        assert!(evaluate(&font, &" ".repeat(16_385)).is_err());
    }
}
