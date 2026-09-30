//! Bounded feature dialect expansion while retaining original source positions.

use std::{collections::HashMap, fmt, ops::Range};

use crate::{Font, MasterId};

use super::{AssembledFea, FeaSourceMapping, FeatureSourceId, Features, FeaturesSource};

const MAX_BYTES: usize = 1_048_576;
const MAX_DEPTH: usize = 32;

/// A preprocessing failure located in the original editable source item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureTokenError {
    /// Persistent identity of the source item containing the error.
    pub source_id: FeatureSourceId,
    /// Half-open UTF-16 range in the original source item's code.
    pub range: Range<usize>,
    /// Detail describing unsupported or malformed syntax or an expansion limit.
    pub message: String,
}

impl fmt::Display for FeatureTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.source_id, self.message)
    }
}

impl std::error::Error for FeatureTokenError {}

struct Piece {
    range: Range<usize>,
    source: Option<Range<usize>>,
    utf16_start: usize,
}

#[derive(Default)]
struct Expansion {
    text: String,
    pieces: Vec<Piece>,
    source_cursor: usize,
    source_utf16: usize,
    predicate_budget: usize,
}

impl Expansion {
    fn push(&mut self, text: &str, code: &str, source: Option<Range<usize>>) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        if self.text.len().saturating_add(text.len()) > MAX_BYTES {
            return Err("Expanded feature source exceeds 1 MiB.".into());
        }
        let start = self.text.len();
        self.text.push_str(text);
        let utf16_start = source.as_ref().map_or(0, |r| {
            self.source_utf16 += code[self.source_cursor..r.start].encode_utf16().count();
            self.source_cursor = r.start;
            self.source_utf16
        });
        if let Some(last) = self.pieces.last_mut() {
            match (&mut last.source, &source) {
                (Some(previous), Some(current)) if previous.end == current.start => {
                    previous.end = current.end;
                    last.range.end = self.text.len();
                    return Ok(());
                }
                (None, None) => {
                    last.range.end = self.text.len();
                    return Ok(());
                }
                _ => {}
            }
        }
        if self.pieces.len() >= 100_000 {
            return Err("Feature source exceeds 100000 expansion regions.".into());
        }
        self.pieces.push(Piece {
            range: start..self.text.len(),
            source,
            utf16_start,
        });
        Ok(())
    }
}

impl Features {
    /// Expand bounded numeric/glyph tokens and select target-specific branches.
    ///
    /// `$name` and `${expression}` use font/master metrics; `$[predicate]` emits
    /// exported glyph names. Whole-line `#ifdef VARIABLE` / `#ifndef VARIABLE`,
    /// `#else` and `#endif` select the actual compilation target. Strings,
    /// comments and escaped text remain literal. This leaves stored source
    /// unchanged; use `assemble_fea` when preserving the editable dialect.
    /// Variable numeric tokens resolve against the supplied default master;
    /// they do not synthesize interpolating values.
    pub fn assemble_fea_for(
        &self,
        font: &Font,
        master: &MasterId,
        variable: bool,
    ) -> Result<AssembledFea, FeatureTokenError> {
        if self.source_of_truth != FeaturesSource::Fea {
            return Ok(AssembledFea::default());
        }
        let mut expanded = self.clone();
        let mut pieces = HashMap::new();
        let mut bytes = 0usize;
        let mut predicate_budget = 1_000_000usize;
        for (id, code) in expanded
            .prefixes
            .iter_mut()
            .filter(|b| !b.disabled)
            .map(|b| (&b.id, &mut b.code))
            .chain(expanded.classes.iter_mut().map(|b| (&b.id, &mut b.code)))
            .chain(
                expanded
                    .features
                    .iter_mut()
                    .filter(|b| !b.disabled)
                    .map(|b| (&b.id, &mut b.code)),
            )
        {
            let result = expand_code(font, master, variable, id, code, &mut predicate_budget)?;
            bytes = bytes.saturating_add(result.text.len());
            if bytes > MAX_BYTES {
                return Err(error(
                    id,
                    code,
                    0..code.len(),
                    "Expanded feature source exceeds 1 MiB.",
                ));
            }
            *code = result.text;
            pieces.insert(id.clone(), result.pieces);
        }
        let mut assembled = expanded.assemble_fea();
        let mut mappings = Vec::new();
        for mapping in assembled.mappings {
            let Some(parts) = pieces
                .get(&mapping.item)
                .filter(|_| mapping.source_range.is_some())
            else {
                mappings.push(mapping);
                continue;
            };
            for part in parts {
                mappings.push(FeaSourceMapping {
                    range: mapping.range.start + part.range.start
                        ..mapping.range.start + part.range.end,
                    item: mapping.item.clone(),
                    kind: mapping.kind,
                    source_range: part.source.clone(),
                    source_utf16_start: part.utf16_start,
                });
            }
        }
        assembled.mappings = mappings;
        Ok(assembled)
    }
}

fn error(
    id: &FeatureSourceId,
    code: &str,
    range: Range<usize>,
    message: impl Into<String>,
) -> FeatureTokenError {
    FeatureTokenError {
        source_id: id.clone(),
        range: code[..range.start].encode_utf16().count()..code[..range.end].encode_utf16().count(),
        message: message.into(),
    }
}

struct Conditional {
    parent: bool,
    condition: bool,
    saw_else: bool,
    start: usize,
}

fn expand_code(
    font: &Font,
    master: &MasterId,
    variable: bool,
    id: &FeatureSourceId,
    code: &str,
    predicate_budget: &mut usize,
) -> Result<Expansion, FeatureTokenError> {
    if code.len() > MAX_BYTES {
        return Err(error(
            id,
            code,
            0..code.len(),
            "Feature source exceeds 1 MiB.",
        ));
    }
    let mut output = Expansion {
        predicate_budget: *predicate_budget,
        ..Default::default()
    };
    let mut stack: Vec<Conditional> = Vec::new();
    let mut active = true;
    let mut quoted = false;
    let mut start = 0usize;
    for line in code.split_inclusive(['\r', '\n']) {
        let end = start + line.len();
        let trimmed = line.trim();
        let words: Vec<_> = trimmed.split_whitespace().collect();
        let directive = !quoted
            && matches!(
                words.first().copied(),
                Some("#ifdef" | "#ifndef" | "#else" | "#endif" | "#elif")
            );
        if directive {
            let fail = |message| error(id, code, start..end, message);
            match words.first().copied() {
                Some("#ifdef" | "#ifndef") => {
                    if words.len() != 2 || words[1] != "VARIABLE" {
                        return Err(fail("Only VARIABLE conditionals are supported."));
                    }
                    if stack.len() >= MAX_DEPTH {
                        return Err(fail("Feature conditional nesting exceeds 32."));
                    }
                    let condition = variable == (words[0] == "#ifdef");
                    stack.push(Conditional {
                        parent: active,
                        condition,
                        saw_else: false,
                        start,
                    });
                    active = active && condition;
                }
                Some("#else") => {
                    if words.len() != 1 {
                        return Err(fail("Unexpected text after #else."));
                    }
                    let state = stack
                        .last_mut()
                        .ok_or_else(|| fail("#else has no matching conditional."))?;
                    if state.saw_else {
                        return Err(fail("A conditional may contain only one #else."));
                    }
                    state.saw_else = true;
                    active = state.parent && !state.condition;
                }
                Some("#endif") => {
                    if words.len() != 1 {
                        return Err(fail("Unexpected text after #endif."));
                    }
                    active = stack
                        .pop()
                        .ok_or_else(|| fail("#endif has no matching conditional."))?
                        .parent;
                }
                _ => {
                    return Err(fail(
                        "#elif is not supported; use nested VARIABLE conditionals.",
                    ))
                }
            }
        } else if active {
            expand_line(font, master, id, code, start..end, &mut quoted, &mut output)?;
        }
        if directive || !active {
            let newline = line.trim_end_matches(['\r', '\n']).len();
            output
                .push(&line[newline..], code, None)
                .map_err(|m| error(id, code, start..end, m))?;
        }
        start = end;
    }
    if let Some(state) = stack.last() {
        return Err(error(
            id,
            code,
            state.start..code.len(),
            "Missing #endif for feature conditional.",
        ));
    }
    *predicate_budget = output.predicate_budget;
    Ok(output)
}

fn expand_line(
    font: &Font,
    master: &MasterId,
    id: &FeatureSourceId,
    code: &str,
    range: Range<usize>,
    quoted: &mut bool,
    output: &mut Expansion,
) -> Result<(), FeatureTokenError> {
    let mut pos = range.start;
    let mut copied = pos;
    while pos < range.end {
        let c = code[pos..].chars().next().unwrap_or_default();
        if c == '\\' {
            pos += c.len_utf8();
            if pos < range.end {
                pos += code[pos..].chars().next().unwrap_or_default().len_utf8();
            }
            continue;
        }
        if c == '"' {
            *quoted = !*quoted;
        }
        if c == '#' && !*quoted {
            break;
        }
        if c != '$' || *quoted {
            pos += c.len_utf8();
            continue;
        }
        output
            .push(&code[copied..pos], code, Some(copied..pos))
            .map_err(|m| error(id, code, copied..pos, m))?;
        let token_start = pos;
        pos += 1;
        let (body, end, predicate) = match code.as_bytes().get(pos) {
            Some(b'{' | b'[') => {
                let predicate = code.as_bytes()[pos] == b'[';
                let closing = if predicate { ']' } else { '}' };
                let body_start = pos + 1;
                let end = token_end(&code[body_start..range.end], closing)
                    .map(|n| body_start + n)
                    .ok_or_else(|| {
                        error(
                            id,
                            code,
                            token_start..range.end,
                            "Unclosed feature token (tokens must fit on one line).",
                        )
                    })?;
                (&code[body_start..end], end + 1, predicate)
            }
            _ => {
                let count = code[pos..range.end]
                    .bytes()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == b'_')
                    .count();
                (&code[pos..pos + count], pos + count, false)
            }
        };
        let replacement = if predicate {
            output.predicate_budget = output
                .predicate_budget
                .checked_sub(font.glyphs.len())
                .ok_or_else(|| {
                    error(
                        id,
                        code,
                        token_start..end,
                        "Feature predicates exceed 1000000 glyph evaluations.",
                    )
                })?;
            super::predicates::expand(font, body)
        } else {
            super::numbers::expand(font, master, body)
        }
        .map_err(|m| error(id, code, token_start..end, m))?;
        output
            .push(&replacement, code, None)
            .map_err(|m| error(id, code, token_start..end, m))?;
        pos = end;
        copied = end;
    }
    output
        .push(&code[copied..range.end], code, Some(copied..range.end))
        .map_err(|m| error(id, code, copied..range.end, m))
}

fn token_end(text: &str, closing: char) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (pos, c) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if matches!(c, '\'' | '"') {
            quote = Some(c);
        } else if c == closing {
            return Some(pos);
        }
    }
    None
}
