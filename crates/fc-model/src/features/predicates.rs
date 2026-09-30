//! Bounded, data-only glyph predicates for feature source expansion.

use crate::{Font, Glyph};

const MAX_INPUT_BYTES: usize = 16 * 1024;
const MAX_DEPTH: usize = 16;
const MAX_NODES: usize = 128;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_GLYPHS: usize = 100_000;

/// Expands a validated predicate to escaped, exported glyph names in font order.
///
/// String properties are `name`, `category`, `subCategory`, `script`, and
/// `productionName`. They support `==`, `!=`, `BEGINSWITH`, `ENDSWITH`, and
/// `CONTAINS` with quoted, case-sensitive values. Missing metadata is the empty
/// string. `export` supports equality and inequality with `true` or `false`.
/// Operators and booleans are case insensitive. `AND` binds tighter than `OR`;
/// parentheses can override precedence. Quoted strings allow escaped matching
/// quotes and backslashes. Input, parentheses, and expression sizes are bounded;
/// at most 100,000 glyphs are scanned, producing at most 1 MiB of feature text.
pub(super) fn expand(font: &Font, predicate: &str) -> Result<String, String> {
    if predicate.len() > MAX_INPUT_BYTES {
        return Err("glyph predicate exceeds the 16384-byte limit".into());
    }
    let mut parser = Parser {
        input: predicate,
        offset: 0,
        nodes: 0,
    };
    let expression = parser.expression(0)?;
    parser.whitespace();
    if parser.offset != predicate.len() {
        return Err(parser.error("unexpected trailing predicate input"));
    }

    if font.glyphs.len() > MAX_GLYPHS {
        return Err("glyph predicate exceeds the 100000-glyph evaluation limit".into());
    }
    let mut output = String::new();
    for name in font.glyph_order() {
        let Some(glyph) = font.glyph(name) else {
            continue;
        };
        if glyph.export && expression.matches(name, glyph) {
            let separator_bytes = usize::from(!output.is_empty());
            let expanded_len = output
                .len()
                .saturating_add(separator_bytes + 1)
                .saturating_add(name.len());
            if expanded_len > MAX_OUTPUT_BYTES {
                return Err("glyph predicate output exceeds the 1 MiB limit".into());
            }
            // Numeric names become CIDs after a backslash. Restrict output to
            // unambiguous glyph identifiers, excluding every FEA delimiter.
            let first = name.as_bytes().first().copied();
            if !first.is_some_and(|byte| byte.is_ascii_alphabetic() || b"_.".contains(&byte))
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.".contains(&byte))
            {
                return Err(format!(
                    "glyph name {name:?} cannot be safely emitted in feature source"
                ));
            }
            if separator_bytes != 0 {
                output.push(' ');
            }
            output.push('\\');
            output.push_str(name);
        }
    }
    Ok(output)
}

enum Expression {
    String(Property, Comparison, String),
    Export(bool),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
}

impl Expression {
    fn matches(&self, name: &str, glyph: &Glyph) -> bool {
        match self {
            Self::String(property, comparison, value) => {
                let actual = match property {
                    Property::Name => name,
                    Property::Category => glyph.category.as_deref().unwrap_or_default(),
                    Property::SubCategory => glyph.sub_category.as_deref().unwrap_or_default(),
                    Property::Script => glyph.script.as_deref().unwrap_or_default(),
                    Property::ProductionName => {
                        glyph.production_name.as_deref().unwrap_or_default()
                    }
                };
                match comparison {
                    Comparison::Equal => actual == value,
                    Comparison::NotEqual => actual != value,
                    Comparison::BeginsWith => actual.starts_with(value),
                    Comparison::EndsWith => actual.ends_with(value),
                    Comparison::Contains => actual.contains(value),
                }
            }
            Self::Export(value) => glyph.export == *value,
            Self::And(left, right) => left.matches(name, glyph) && right.matches(name, glyph),
            Self::Or(left, right) => left.matches(name, glyph) || right.matches(name, glyph),
        }
    }
}

enum Property {
    Name,
    Category,
    SubCategory,
    Script,
    ProductionName,
}

enum Comparison {
    Equal,
    NotEqual,
    BeginsWith,
    EndsWith,
    Contains,
}

struct Parser<'a> {
    input: &'a str,
    offset: usize,
    nodes: usize,
}

impl<'a> Parser<'a> {
    fn expression(&mut self, depth: usize) -> Result<Expression, String> {
        let mut left = self.conjunction(depth)?;
        while self.keyword("OR") {
            let right = self.conjunction(depth)?;
            self.node()?;
            left = Expression::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn conjunction(&mut self, depth: usize) -> Result<Expression, String> {
        let mut left = self.primary(depth)?;
        while self.keyword("AND") {
            let right = self.primary(depth)?;
            self.node()?;
            left = Expression::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn primary(&mut self, depth: usize) -> Result<Expression, String> {
        if self.consume("(") {
            if depth >= MAX_DEPTH {
                return Err(self.error("glyph predicate exceeds the 16-level nesting limit"));
            }
            let expression = self.expression(depth + 1)?;
            if !self.consume(")") {
                return Err(self.error("expected closing predicate parenthesis"));
            }
            return Ok(expression);
        }
        self.node()?;
        let property = self.word()?;
        let property = match property {
            "name" => Some(Property::Name),
            "category" => Some(Property::Category),
            "subCategory" => Some(Property::SubCategory),
            "script" => Some(Property::Script),
            "productionName" => Some(Property::ProductionName),
            "export" => None,
            _ => return Err(self.error(&format!("unsupported glyph property {property:?}"))),
        };
        let comparison = if self.consume("==") {
            Comparison::Equal
        } else if self.consume("!=") {
            Comparison::NotEqual
        } else if self.keyword("BEGINSWITH") {
            Comparison::BeginsWith
        } else if self.keyword("ENDSWITH") {
            Comparison::EndsWith
        } else if self.keyword("CONTAINS") {
            Comparison::Contains
        } else {
            return Err(self.error("expected ==, !=, BEGINSWITH, ENDSWITH, or CONTAINS"));
        };
        if let Some(property) = property {
            Ok(Expression::String(property, comparison, self.quoted()?))
        } else {
            let value = self.word()?;
            let value = if value.eq_ignore_ascii_case("true") {
                true
            } else if value.eq_ignore_ascii_case("false") {
                false
            } else {
                return Err(self.error("export requires a true or false value"));
            };
            match comparison {
                Comparison::Equal => Ok(Expression::Export(value)),
                Comparison::NotEqual => Ok(Expression::Export(!value)),
                _ => Err(self.error("export supports only == and !=")),
            }
        }
    }

    fn quoted(&mut self) -> Result<String, String> {
        self.whitespace();
        let Some(quote @ ('\'' | '"')) = self.next_char() else {
            return Err(self.error("expected a quoted predicate string"));
        };
        let mut result = String::new();
        while let Some(character) = self.next_char() {
            if character == quote {
                return Ok(result);
            }
            if character == '\\' {
                let Some(escaped) = self.next_char() else {
                    return Err(self.error("unterminated predicate string escape"));
                };
                if escaped != quote && escaped != '\\' {
                    return Err(self.error("only matching quotes and backslashes can be escaped"));
                }
                result.push(escaped);
            } else if character.is_control() {
                return Err(self.error("control characters are not allowed in predicate strings"));
            } else {
                result.push(character);
            }
        }
        Err(self.error("unterminated predicate string"))
    }

    fn word(&mut self) -> Result<&'a str, String> {
        self.whitespace();
        let start = self.offset;
        while self
            .input
            .as_bytes()
            .get(self.offset)
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        {
            self.offset += 1;
        }
        if self.offset == start {
            Err(self.error("expected a predicate identifier"))
        } else {
            Ok(&self.input[start..self.offset])
        }
    }

    fn keyword(&mut self, keyword: &str) -> bool {
        self.whitespace();
        let remaining = &self.input[self.offset..];
        if !remaining
            .get(..keyword.len())
            .is_some_and(|word| word.eq_ignore_ascii_case(keyword))
        {
            return false;
        }
        if remaining
            .as_bytes()
            .get(keyword.len())
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            return false;
        }
        self.offset += keyword.len();
        true
    }

    fn consume(&mut self, token: &str) -> bool {
        self.whitespace();
        if self.input[self.offset..].starts_with(token) {
            self.offset += token.len();
            true
        } else {
            false
        }
    }

    fn whitespace(&mut self) {
        while let Some(character) = self.input[self.offset..].chars().next() {
            if !character.is_whitespace() {
                break;
            }
            self.offset += character.len_utf8();
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let character = self.input[self.offset..].chars().next()?;
        self.offset += character.len_utf8();
        Some(character)
    }

    fn node(&mut self) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            Err(self.error("glyph predicate exceeds the 128-node expression limit"))
        } else {
            Ok(())
        }
    }

    fn error(&self, message: &str) -> String {
        format!("{message} at byte {}", self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::expand;
    use crate::{Font, Glyph};

    fn font() -> Font {
        let mut font = Font::default();
        for name in ["z.alt", "A", "B", "sub", "hidden"] {
            let mut glyph = Glyph::new(name);
            glyph.category = Some("Letter".into());
            glyph.sub_category = Some("Uppercase".into());
            glyph.script = Some("latin".into());
            glyph.production_name = Some(format!("production.{name}"));
            glyph.export = name != "hidden";
            font.glyphs.insert(name.into(), glyph);
        }
        font
    }

    #[test]
    fn expansion_preserves_order_and_escapes_keywords() {
        assert_eq!(
            expand(&font(), "category == 'Letter'").unwrap(),
            r"\z.alt \A \B \sub"
        );
        assert_eq!(expand(&font(), "export == false").unwrap(), "");
        assert_eq!(
            expand(&font(), "export != false").unwrap(),
            r"\z.alt \A \B \sub"
        );
    }

    #[test]
    fn string_properties_and_operators_are_evaluated() {
        for predicate in [
            "name BEGINSWITH 'z'",
            "name ENDSWITH '.alt'",
            "productionName CONTAINS 'z.alt'",
            "subCategory == 'Uppercase' AND name == 'z.alt'",
            "script != 'arabic' AND name == 'z.alt'",
        ] {
            assert_eq!(expand(&font(), predicate).unwrap(), r"\z.alt");
        }
        assert_eq!(expand(&font(), "name beginswith 'Z'").unwrap(), "");
    }

    #[test]
    fn conjunction_precedence_and_parentheses_work() {
        assert_eq!(
            expand(&font(), "name == 'A' OR name == 'B' AND name == 'sub'").unwrap(),
            r"\A"
        );
        assert_eq!(
            expand(&font(), "(name == 'A' OR name == 'B') AND name != 'A'").unwrap(),
            r"\B"
        );
    }

    #[test]
    fn missing_metadata_and_quoted_escapes_are_explicit() {
        let mut font = Font::default();
        let mut glyph = Glyph::new("A");
        glyph.production_name = Some("a'b\\c".into());
        font.glyphs.insert("A".into(), glyph);
        assert_eq!(expand(&font, "script == ''").unwrap(), r"\A");
        assert_eq!(
            expand(&font, r"productionName == 'a\'b\\c'").unwrap(),
            r"\A"
        );
        assert_eq!(expand(&font, "name == \"A\"").unwrap(), r"\A");
    }

    #[test]
    fn malformed_predicates_are_rejected_even_without_glyphs() {
        for predicate in [
            "",
            "unknown == 'A'",
            "unicode == '0041'",
            "name MATCHES '.*'",
            "name == A",
            "name == true",
            "export == 'true'",
            "export CONTAINS true",
            "export == 1",
            "name == 'A",
            "name == 'A' OR unknown == 'B'",
            "name == 'A' OR",
            "(name == 'A'",
            "name == 'A')",
            "name == 'A' ; sub A by B;",
            "name == 'A'ORname == 'B'",
            "name === 'A'",
            "name == 'A' AND ()",
            "name == 'A' OR OR name == 'B'",
            "export == true_false",
            "name == 'A' # ignored?",
            "name == 'A'\0",
            "name == 'A' AND 名字 == 'A'",
            r"name == 'A\n'",
        ] {
            assert!(expand(&Font::default(), predicate).is_err(), "{predicate}");
        }
    }

    #[test]
    fn nesting_and_expression_limits_are_bounded() {
        let allowed = format!("{}name == 'A'{}", "(".repeat(16), ")".repeat(16));
        assert_eq!(expand(&font(), &allowed).unwrap(), r"\A");
        assert!(expand(&font(), &format!("({allowed})")).is_err());
        let many = vec!["name == 'A'"; 65].join(" OR ");
        assert!(expand(&Font::default(), &many).is_err());
        assert!(expand(&Font::default(), &" ".repeat(16 * 1024 + 1)).is_err());
    }

    #[test]
    fn unsafe_selected_names_fail_instead_of_injecting_feature_source() {
        for name in ["", "12", "A B", "A]; sub B by C; [", "A#", "A\\B", "A/B"] {
            let mut font = Font::default();
            font.glyphs.insert(name.into(), Glyph::new(name));
            assert!(expand(&font, "export == true").is_err(), "{name}");
        }
        let mut font = Font::default();
        font.glyphs.insert("A B".into(), Glyph::new("A B"));
        assert_eq!(expand(&font, "name == 'A'").unwrap(), "");
    }

    #[test]
    fn output_budget_includes_escapes_and_separators() {
        let mut font = Font::default();
        let name = "A".repeat(super::MAX_OUTPUT_BYTES - 1);
        font.glyphs.insert(name.clone().into(), Glyph::new(name));
        assert_eq!(
            expand(&font, "export == true").unwrap().len(),
            super::MAX_OUTPUT_BYTES
        );
        font.glyphs.insert("B".into(), Glyph::new("B"));
        assert!(expand(&font, "export == true")
            .unwrap_err()
            .contains("output"));

        let mut font = Font::default();
        let name = "A".repeat(super::MAX_OUTPUT_BYTES);
        font.glyphs.insert(name.clone().into(), Glyph::new(name));
        assert!(expand(&font, "export == true")
            .unwrap_err()
            .contains("output"));
    }

    #[test]
    fn glyph_budget_bounds_scanning_even_when_no_glyphs_are_exported() {
        let mut font = Font::default();
        for index in 0..super::MAX_GLYPHS {
            let name = format!("g{index}");
            let mut glyph = Glyph::new(name.clone());
            glyph.export = false;
            font.glyphs.insert(name.into(), glyph);
        }
        assert_eq!(expand(&font, "export == true").unwrap(), "");
        font.glyphs.insert("extra".into(), Glyph::new("extra"));
        assert!(expand(&font, "export == true")
            .unwrap_err()
            .contains("100000-glyph"));
        assert!(expand(&font, "unknown == 'A'")
            .unwrap_err()
            .contains("unsupported glyph property"));
    }
}
