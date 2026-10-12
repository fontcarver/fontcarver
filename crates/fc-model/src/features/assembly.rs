//! Pure source assembly and source-item mappings; no parsing or file access.

use std::ops::Range;

use super::{FeatureSourceId, Features, FeaturesSource};

/// The source-tree category of an assembled item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureSourceKind {
    /// Verbatim top-level source.
    Prefix,
    /// A named glyph class.
    Class,
    /// A complete feature block or its body.
    Feature,
}

/// One copied or generated region in an assembled feature file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeaSourceMapping {
    /// UTF-8 byte range in the assembled text.
    pub range: Range<usize>,
    /// Stable source item identity.
    pub item: FeatureSourceId,
    /// Source-tree category; never an index or a feature tag.
    pub kind: FeatureSourceKind,
    /// Byte range in the item's `code`, or none for synthesized syntax.
    ///
    /// Copied text is verbatim. Generated class names, feature tags, delimiters
    /// and separators belong to the item but have no position in its code.
    pub source_range: Option<Range<usize>>,
    /// UTF-16 offset of the copied region in the original source item.
    pub source_utf16_start: usize,
}

/// A position resolved back to a source item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeaSourceLocation {
    /// Stable source item identity.
    pub item: FeatureSourceId,
    /// Source-tree category.
    pub kind: FeatureSourceKind,
    /// UTF-8 byte offset in `code`, absent for generated syntax.
    pub byte_offset: Option<usize>,
    /// UTF-16 code-unit offset in `code`, absent for generated syntax.
    pub utf16_offset: Option<usize>,
}

/// Effective FEA and ordered mappings for its source and generated syntax.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssembledFea {
    /// Text supplied unchanged to compilation and source export.
    pub text: String,
    /// Ordered, non-overlapping mappings; empty code may have an empty range.
    pub mappings: Vec<FeaSourceMapping>,
}

impl AssembledFea {
    /// Locate a UTF-8 character boundary, including end of file.
    ///
    /// At an interior boundary the following region owns the position; at EOF
    /// the last region owns it. Invalid/out-of-bounds byte offsets return none.
    /// This maps a position, not a span crossing several source items: callers
    /// use `mappings` to split such spans instead of attributing them to one item.
    pub fn locate(&self, offset: usize) -> Option<FeaSourceLocation> {
        if !self.text.is_char_boundary(offset) {
            return None;
        }
        let mapping = if offset == self.text.len() {
            self.mappings.last()
        } else {
            self.mappings
                .iter()
                .find(|mapping| mapping.range.contains(&offset))
        }?;
        let byte_offset = mapping
            .source_range
            .as_ref()
            .map(|source| source.start + offset - mapping.range.start);
        let utf16_offset = mapping.source_range.as_ref().map(|_| {
            mapping.source_utf16_start
                + self.text[mapping.range.start..offset]
                    .encode_utf16()
                    .count()
        });
        Some(FeaSourceLocation {
            item: mapping.item.clone(),
            kind: mapping.kind,
            byte_offset,
            utf16_offset,
        })
    }

    fn push(&mut self, id: &FeatureSourceId, kind: FeatureSourceKind, text: &str, copied: bool) {
        let start = self.text.len();
        self.text.push_str(text);
        self.mappings.push(FeaSourceMapping {
            range: start..self.text.len(),
            item: id.clone(),
            kind,
            source_range: copied.then_some(0..text.len()),
            source_utf16_start: 0,
        });
    }

    fn separate(&mut self, id: &FeatureSourceId, kind: FeatureSourceKind) {
        if !self.text.is_empty() && !self.text.ends_with(['\r', '\n']) {
            self.push(id, kind, "\n", false);
        }
    }

    fn end_comment(&mut self, id: &FeatureSourceId, kind: FeatureSourceKind) {
        if self
            .text
            .rsplit(['\r', '\n'])
            .next()
            .is_some_and(|line| line.contains('#'))
        {
            self.push(id, kind, "\n", false);
        }
    }
}

impl Features {
    /// Assemble effective source without rewriting user text or loading includes.
    ///
    /// Imported tables emit no FEA. Active prefixes, classes and features retain
    /// their stored order. Syntax recognition only decides whether wrappers are
    /// needed; callers still validate the result with the feature compiler.
    pub fn assemble_fea(&self) -> AssembledFea {
        let mut output = AssembledFea::default();
        if self.source_of_truth != FeaturesSource::Fea {
            return output;
        }
        for prefix in self.prefixes.iter().filter(|block| !block.disabled) {
            let kind = FeatureSourceKind::Prefix;
            output.separate(&prefix.id, kind);
            output.push(&prefix.id, kind, &prefix.code, true);
        }
        for class in &self.classes {
            let kind = FeatureSourceKind::Class;
            output.separate(&class.id, kind);
            let opening = format!("@{} = ", class.name.trim_start_matches('@'));
            output.push(&class.id, kind, &opening, false);
            let bracketed = is_bracketed(&class.code);
            if !bracketed {
                output.push(&class.id, kind, "[", false);
            }
            output.push(&class.id, kind, &class.code, true);
            output.end_comment(&class.id, kind);
            output.push(&class.id, kind, if bracketed { ";" } else { "];" }, false);
        }
        for feature in self.features.iter().filter(|block| !block.disabled) {
            let kind = FeatureSourceKind::Feature;
            output.separate(&feature.id, kind);
            if is_feature_block(&feature.code) {
                output.push(&feature.id, kind, &feature.code, true);
            } else {
                let opening = format!("feature {} {{\n", feature.tag);
                output.push(&feature.id, kind, &opening, false);
                output.push(&feature.id, kind, &feature.code, true);
                output.separate(&feature.id, kind);
                output.push(&feature.id, kind, &format!("}} {};", feature.tag), false);
            }
        }
        output
    }
}

fn trivia(mut source: &str) -> &str {
    loop {
        source = source.trim_start_matches(|c: char| c.is_ascii_whitespace());
        if source.starts_with('#') {
            source = source.find(['\r', '\n']).map_or("", |end| &source[end..]);
        } else {
            return source;
        }
    }
}

fn token(source: &str) -> (&str, &str) {
    let source = trivia(source);
    if source.starts_with(['{', '}', '[', ']', ';']) {
        return (&source[..1], &source[1..]);
    }
    let end = source
        .find(|c: char| c.is_ascii_whitespace() || "#{}[];".contains(c))
        .unwrap_or(source.len());
    (&source[..end], &source[end..])
}

fn is_feature_block(source: &str) -> bool {
    let (keyword, rest) = token(source);
    if keyword != "feature" {
        return false;
    }
    let (tag, rest) = token(rest);
    if tag.is_empty() || matches!(tag, "{" | "}" | ";") {
        return false;
    }
    let (next, rest) = token(rest);
    next == "{" || (next == "useExtension" && token(rest).0 == "{")
}

fn is_bracketed(source: &str) -> bool {
    let (first, mut rest) = token(source);
    if first != "[" {
        return false;
    }
    let mut depth = 1usize;
    loop {
        let (next, remaining) = token(rest);
        match next {
            "" => return false,
            "[" => depth += 1,
            "]" => {
                depth -= 1;
                if depth == 0 {
                    return trivia(remaining).is_empty();
                }
            }
            _ => {}
        }
        rest = remaining;
    }
}
