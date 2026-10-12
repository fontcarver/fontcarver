//! Bounded, side-effect-free smart-filter text grammar.
//!
//! Expressions use `and`, `or`, `not`, parentheses and predicates such as
//! `script:han and not export and paths>0`. Text literals use JSON quoting.
//! `:`, `=`, `==`, `!=`, `~`, `=~`, `>`, `>=`, `<`, `<=` are comparisons.

use crate::{ApiError, SmartFilterNode as Node, SmartFilterOperator as Op};

const MAX_BYTES: usize = 16_384;
const MAX_TOKENS: usize = 2_048;
const MAX_DEPTH: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Word(String),
    Quoted(String),
    Operator(Op),
    Left,
    Right,
}

/// Parse and validate all predicates, with bounded input and nesting.
pub(crate) fn parse(expression: &str) -> Result<Node, ApiError> {
    if expression.len() > MAX_BYTES {
        return Err(invalid("expression exceeds 16384 bytes"));
    }
    let tokens = tokenize(expression)?;
    if tokens.is_empty() {
        return Err(invalid("expression is empty"));
    }
    let mut parser = Parser { tokens, cursor: 0 };
    let node = parser.any(0)?;
    if parser.cursor != parser.tokens.len() {
        return Err(invalid("expected 'and', 'or', or end of expression"));
    }
    super::smart_filter_eval::CompiledSmartFilter::new(&node)?;
    if format(&node).len() > MAX_BYTES {
        return Err(invalid("canonical expression exceeds 16384 bytes"));
    }
    Ok(node)
}

/// Produce unambiguous DSL text for a validated tree.
pub(crate) fn format(node: &Node) -> String {
    match node {
        Node::All { children } => group(children, " and "),
        Node::Any { children } => group(children, " or "),
        Node::None { children } => format!("not {}", group(children, " or ")),
        Node::Predicate {
            field,
            operator,
            value,
        } => {
            let operator = match operator {
                Op::Eq => ":",
                Op::Ne => "!=",
                Op::Contains => "~",
                Op::Regex => "=~",
                Op::Gt => ">",
                Op::Gte => ">=",
                Op::Lt => "<",
                Op::Lte => "<=",
            };
            // Serializing a Rust string has no fallible data-dependent path.
            let quoted = serde_json::Value::String(value.clone()).to_string();
            format!("{field}{operator}{quoted}")
        }
    }
}

fn group(children: &[Node], separator: &str) -> String {
    format!(
        "({})",
        children
            .iter()
            .map(format)
            .collect::<Vec<_>>()
            .join(separator)
    )
}

fn invalid(message: impl std::fmt::Display) -> ApiError {
    ApiError::invalid_params(format!("invalid smart filter: {message}"))
}

fn tokenize(source: &str) -> Result<Vec<Token>, ApiError> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let remaining = &source[cursor..];
        let Some(ch) = remaining.chars().next() else {
            break;
        };
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
            continue;
        }
        if tokens.len() == MAX_TOKENS {
            return Err(invalid("expression exceeds 2048 tokens"));
        }
        let (token, consumed) = match ch {
            '(' => (Token::Left, 1),
            ')' => (Token::Right, 1),
            '"' => {
                let mut escaped = false;
                let mut end = None;
                for (offset, next) in remaining.char_indices().skip(1) {
                    if escaped {
                        escaped = false;
                    } else if next == '\\' {
                        escaped = true;
                    } else if next == '"' {
                        end = Some(offset + 1);
                        break;
                    }
                }
                let end = end.ok_or_else(|| invalid("unterminated quoted string"))?;
                let value = serde_json::from_str(&remaining[..end])
                    .map_err(|_| invalid("quoted values must use valid JSON escapes"))?;
                (Token::Quoted(value), end)
            }
            ':' => (Token::Operator(Op::Eq), 1),
            '~' => (Token::Operator(Op::Contains), 1),
            '=' if remaining.starts_with("=~") => (Token::Operator(Op::Regex), 2),
            '=' => (
                Token::Operator(Op::Eq),
                if remaining.starts_with("==") { 2 } else { 1 },
            ),
            '!' if remaining.starts_with("!=") => (Token::Operator(Op::Ne), 2),
            '>' => (
                Token::Operator(if remaining.starts_with(">=") {
                    Op::Gte
                } else {
                    Op::Gt
                }),
                if remaining.starts_with(">=") { 2 } else { 1 },
            ),
            '<' => (
                Token::Operator(if remaining.starts_with("<=") {
                    Op::Lte
                } else {
                    Op::Lt
                }),
                if remaining.starts_with("<=") { 2 } else { 1 },
            ),
            '!' | '\'' => return Err(invalid(format!("unexpected character at byte {cursor}"))),
            _ => {
                let end = remaining
                    .char_indices()
                    .find_map(|(offset, c)| {
                        (c.is_whitespace() || "()<>!=~:\"'".contains(c)).then_some(offset)
                    })
                    .unwrap_or(remaining.len());
                (Token::Word(remaining[..end].to_owned()), end)
            }
        };
        cursor += consumed;
        tokens.push(token);
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn keyword(&mut self, expected: &str) -> bool {
        if matches!(self.tokens.get(self.cursor), Some(Token::Word(word)) if word.eq_ignore_ascii_case(expected))
        {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn any(&mut self, depth: usize) -> Result<Node, ApiError> {
        let first = self.all(depth)?;
        let mut children = vec![first];
        while self.keyword("or") {
            children.push(self.all(depth)?);
        }
        Ok(if children.len() == 1 {
            children.remove(0)
        } else {
            Node::Any { children }
        })
    }

    fn all(&mut self, depth: usize) -> Result<Node, ApiError> {
        let first = self.unary(depth)?;
        let mut children = vec![first];
        while self.keyword("and") {
            children.push(self.unary(depth)?);
        }
        Ok(if children.len() == 1 {
            children.remove(0)
        } else {
            Node::All { children }
        })
    }

    fn unary(&mut self, depth: usize) -> Result<Node, ApiError> {
        if depth >= MAX_DEPTH {
            return Err(invalid("nesting exceeds 32 levels"));
        }
        if self.keyword("not") {
            let child = self.unary(depth + 1)?;
            return Ok(match child {
                Node::Any { children } => Node::None { children },
                child => Node::None {
                    children: vec![child],
                },
            });
        }
        if self.tokens.get(self.cursor) == Some(&Token::Left) {
            self.cursor += 1;
            let child = self.any(depth + 1)?;
            if self.tokens.get(self.cursor) != Some(&Token::Right) {
                return Err(invalid("expected closing parenthesis"));
            }
            self.cursor += 1;
            return Ok(child);
        }
        self.predicate()
    }

    fn predicate(&mut self) -> Result<Node, ApiError> {
        let Some(Token::Word(field)) = self.tokens.get(self.cursor) else {
            return Err(invalid("expected a predicate name"));
        };
        let field = field.to_ascii_lowercase().replace('-', "_");
        let field = super::smart_filter_eval::canonical_field(&field)
            .ok_or_else(|| invalid(format!("unknown predicate `{field}`")))?
            .to_owned();
        self.cursor += 1;
        let (operator, value) =
            if let Some(Token::Operator(operator)) = self.tokens.get(self.cursor).cloned() {
                self.cursor += 1;
                let value = match self.tokens.get(self.cursor) {
                    Some(Token::Word(value) | Token::Quoted(value)) => value.clone(),
                    _ => return Err(invalid("expected a predicate value")),
                };
                self.cursor += 1;
                (operator, value)
            } else {
                (Op::Eq, "true".into())
            };
        super::smart_filter_eval::validate_predicate(&field, operator, &value)?;
        Ok(Node::Predicate {
            field,
            operator,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_parser_boolean_precedence_and_canonical_roundtrip() {
        let node = parse("script:han and not export or paths>=2").unwrap();
        let Node::Any { children } = &node else {
            panic!("or must have lower precedence")
        };
        assert!(matches!(children[0], Node::All { .. }));
        assert_eq!(parse(&format(&node)).unwrap(), node);
        let none = parse("not (name:A or name:B)").unwrap();
        assert!(matches!(&none, Node::None { children } if children.len() == 2));
        assert_eq!(parse(&format(&none)).unwrap(), none);
    }

    #[test]
    fn smart_parser_preserves_quoted_text_and_regex() {
        let node = parse(r#"note~"字形 \"review\"" and name=~"^A\\.sc$""#).unwrap();
        assert_eq!(parse(&format(&node)).unwrap(), node);
        assert_eq!(parse("export").unwrap(), parse("export == true").unwrap());
        assert_eq!(parse("glyph_name:A").unwrap(), parse("name:A").unwrap());
        assert_eq!(parse("path-count>0").unwrap(), parse("paths>0").unwrap());
    }

    #[test]
    fn smart_parser_rejects_malformed_or_unbounded_input() {
        for input in [
            "",
            "()",
            "name:",
            "name:A and",
            "(export",
            "export)",
            "unknown:true",
            "paths~0",
            "name=~\"[\"",
            "note:\"oops",
            "note:'x'",
            "name:x name:y",
        ] {
            assert!(parse(input).is_err(), "accepted {input}");
        }
        assert!(parse(&format!("{}export{}", "(".repeat(33), ")".repeat(33))).is_err());
        assert!(parse(&"not ".repeat(5000)).is_err());
    }

    #[test]
    fn smart_parser_every_accepted_nesting_roundtrips() {
        for depth in 0..33 {
            let text = format!("{}export", "not ".repeat(depth));
            if let Ok(node) = parse(&text) {
                assert_eq!(parse(&format(&node)).unwrap(), node);
            }
        }
        let text = format!(
            "name:{} or name:{} or name:{} or name:{}",
            "a".repeat(4090),
            "b".repeat(4090),
            "c".repeat(4090),
            "d".repeat(4090)
        );
        assert!(parse(&text).is_err());
    }
}
