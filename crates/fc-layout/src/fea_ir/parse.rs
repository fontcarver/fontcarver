//! Build the typed IR by walking the `fea-rs` concrete syntax tree.
//!
//! The walk is deliberately strict: a node whose children do not match the
//! expected shape becomes [`Statement::Verbatim`] with its exact source text
//! rather than a guess. Only `Kind`, `Node` and `Token` are used, which are the
//! public parts of `fea-rs`'s tree.

use std::{path::Path, sync::Arc};

use fc_model::GlyphName;
use fea_rs::{
    parse::{SourceLoadError, SourceResolver},
    GlyphMap, Kind, Node, NodeOrToken,
};

use super::{
    AnchorPoint, ClassItem, ContextItem, FeaIr, GlyphSet, IgnorePattern, LanguagePolicy,
    LookupFlag, LookupType, MarkAttachment, PairValues, Rule, Statement, TypedRule, ValueRecord,
};

/// The virtual path the in-memory source is parsed from.
const ROOT: &str = "fontcarver-fea-ir";

/// Why feature source could not be parsed into the IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable parser messages, in source order.
    pub messages: Vec<String>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.messages.join("; "))
    }
}

impl std::error::Error for ParseError {}

/// Parse feature source into the typed IR.
///
/// `glyph_order` disambiguates hyphenated tokens and CIDs; an empty order still
/// parses, keeping ambiguous members as [`ClassItem::Unresolved`]. Parse errors
/// are returned rather than guessed around, because a guessed rule would change
/// the meaning of the user's source.
///
/// Unsupported *statements* are preserved as [`Statement::Verbatim`], but
/// source that does not parse at all is an error, and so is `include(...)`:
/// FontCarver's feature editor and compiler both read one self-contained
/// source and disable external includes, so the IR does the same rather than
/// pretending to understand a file it never read.
pub fn parse_fea(source: &str, glyph_order: &[GlyphName]) -> Result<FeaIr, ParseError> {
    let map = glyph_map(glyph_order);
    let text: Arc<str> = source.to_owned().into();
    let resolver = Resolver { text };
    let (tree, diagnostics) =
        fea_rs::parse::parse_root(ROOT.into(), map.as_ref(), Box::new(resolver)).map_err(
            |error| ParseError {
                messages: vec![error.to_string()],
            },
        )?;
    let errors: Vec<String> = diagnostics
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .map(|diagnostic| tree.format_diagnostic(diagnostic, false))
        .collect();
    if !errors.is_empty() {
        return Err(ParseError { messages: errors });
    }
    let mut statements = Vec::new();
    for child in tree.root().iter_children() {
        push_statement(child, &mut statements);
    }
    Ok(FeaIr { statements })
}

fn glyph_map(glyph_order: &[GlyphName]) -> Option<GlyphMap> {
    if glyph_order.is_empty() {
        return None;
    }
    let mut names: Vec<&str> = Vec::with_capacity(glyph_order.len() + 1);
    if !glyph_order.iter().any(|name| name.as_ref() == ".notdef") {
        names.push(".notdef");
    }
    names.extend(glyph_order.iter().map(|name| name.as_ref()));
    GlyphMap::new(names).ok()
}

struct Resolver {
    text: Arc<str>,
}

impl SourceResolver for Resolver {
    fn get_contents(&self, path: &Path) -> Result<Arc<str>, SourceLoadError> {
        if path == Path::new(ROOT) {
            Ok(Arc::clone(&self.text))
        } else {
            Err(SourceLoadError::new(
                path.to_path_buf(),
                "an include statement is not supported: the typed lookup IR reads \
                 one self-contained source, as the feature editor and the \
                 compiler do. Inline the included file first.",
            ))
        }
    }
}

/// Exact source text of a node or token.
fn text(item: &NodeOrToken) -> String {
    match item {
        NodeOrToken::Token(token) => token.text.to_string(),
        NodeOrToken::Node(node) => node_text(node),
    }
}

fn node_text(node: &Node) -> String {
    node.iter_tokens()
        .map(|token| token.text.as_str())
        .collect()
}

/// Children with whitespace removed, keeping comments so nothing is lost.
fn children(node: &Node) -> Vec<NodeOrToken> {
    node.iter_children()
        .filter(|child| child.kind() != Kind::Whitespace)
        .cloned()
        .collect()
}

/// Children with whitespace and comments removed, for shape matching.
fn parts(node: &Node) -> Vec<NodeOrToken> {
    node.iter_children()
        .filter(|child| !matches!(child.kind(), Kind::Whitespace | Kind::Comment))
        .cloned()
        .collect()
}

fn push_statement(item: &NodeOrToken, out: &mut Vec<Statement>) {
    match item.kind() {
        Kind::Whitespace => {}
        Kind::Comment => out.push(Statement::Comment(text(item))),
        _ => match item {
            NodeOrToken::Node(node) => out.push(statement(node)),
            // Structural tokens belong to their parent node; anything else at
            // statement position is unexpected and kept as written.
            NodeOrToken::Token(token) => {
                if !matches!(
                    token.kind,
                    Kind::Semi
                        | Kind::LBrace
                        | Kind::RBrace
                        | Kind::Tag
                        | Kind::Label
                        | Kind::FeatureKw
                        | Kind::LookupKw
                        | Kind::UseExtensionKw
                        | Kind::Eof
                ) {
                    out.push(Statement::Verbatim(token.text.to_string()));
                }
            }
        },
    }
}

fn statement(node: &Node) -> Statement {
    let verbatim = || Statement::Verbatim(node_text(node));
    match node.kind() {
        Kind::LanguageSystemNode => language_system(node).unwrap_or_else(verbatim),
        Kind::GlyphClassDefNode => glyph_class_def(node).unwrap_or_else(verbatim),
        Kind::MarkClassNode => mark_class_def(node).unwrap_or_else(verbatim),
        Kind::LookupFlagNode => lookup_flag(node).unwrap_or_else(verbatim),
        Kind::ScriptNode => script(node).unwrap_or_else(verbatim),
        Kind::LanguageNode => language(node).unwrap_or_else(verbatim),
        Kind::SubtableNode => Statement::Subtable,
        Kind::LookupRefNode => lookup_call(node).unwrap_or_else(verbatim),
        Kind::AaltFeatureNode => feature_call(node).unwrap_or_else(verbatim),
        Kind::FeatureNode => feature(node).unwrap_or_else(verbatim),
        Kind::LookupBlockNode => lookup_block(node).unwrap_or_else(verbatim),
        _ => rule(node)
            .map(Statement::Rule)
            .unwrap_or_else(|| Statement::Verbatim(node_text(node))),
    }
}

fn tag_at(items: &[NodeOrToken], index: usize) -> Option<String> {
    match items.get(index)? {
        NodeOrToken::Token(token) if token.kind == Kind::Tag => Some(token.text.to_string()),
        _ => None,
    }
}

fn language_system(node: &Node) -> Option<Statement> {
    let items = parts(node);
    if items.len() != 4 || items[0].kind() != Kind::LanguagesystemKw {
        return None;
    }
    Some(Statement::LanguageSystem {
        script: tag_at(&items, 1)?,
        language: tag_at(&items, 2)?,
    })
}

fn class_name(item: &NodeOrToken) -> Option<String> {
    match item {
        NodeOrToken::Token(token) if token.kind == Kind::NamedGlyphClass => {
            Some(token.text.trim_start_matches('@').to_string())
        }
        _ => None,
    }
}

fn number(item: &NodeOrToken) -> Option<i32> {
    match item {
        NodeOrToken::Token(token) if token.kind == Kind::Number => token.text.parse().ok(),
        _ => None,
    }
}

fn glyph_class_def(node: &Node) -> Option<Statement> {
    let items = parts(node);
    if items.len() != 4 || items[2].kind() != Kind::GlyphClass {
        return None;
    }
    let name = class_name(&items[0])?;
    let literal = items[2].as_node()?;
    Some(Statement::GlyphClassDef {
        name,
        members: class_items(literal)?,
    })
}

/// Members of a `[ ... ]` literal, preserving ranges and class references.
fn class_items(literal: &Node) -> Option<Vec<ClassItem>> {
    let mut members = Vec::new();
    let mut escaped = false;
    for child in parts(literal) {
        match child.kind() {
            Kind::LSquare | Kind::RSquare => {}
            Kind::Backslash => {
                escaped = true;
                continue;
            }
            Kind::GlyphName => members.push(ClassItem::Glyph(text(&child))),
            Kind::Cid => members.push(ClassItem::Cid(text(&child).parse().ok()?)),
            Kind::NamedGlyphClass => members.push(ClassItem::Named(class_name(&child)?)),
            Kind::GlyphNameOrRange => members.push(ClassItem::Unresolved(text(&child))),
            Kind::GlyphRange => members.push(range_item(child.as_node()?)?),
            _ => return None,
        }
        escaped = false;
    }
    if escaped {
        return None;
    }
    Some(members)
}

fn range_item(node: &Node) -> Option<ClassItem> {
    let ends: Vec<NodeOrToken> = parts(node)
        .into_iter()
        .filter(|child| !matches!(child.kind(), Kind::Hyphen | Kind::Backslash))
        .collect();
    let [start, end] = ends.as_slice() else {
        return None;
    };
    match (start.kind(), end.kind()) {
        (Kind::GlyphName, Kind::GlyphName) => Some(ClassItem::Range(text(start), text(end))),
        (Kind::Cid, Kind::Cid) => Some(ClassItem::CidRange(
            text(start).parse().ok()?,
            text(end).parse().ok()?,
        )),
        _ => None,
    }
}

/// A glyph operand starting at `index`, and the index after it.
fn glyph_set(items: &[NodeOrToken], index: usize) -> Option<(GlyphSet, usize)> {
    let item = items.get(index)?;
    match item.kind() {
        Kind::Backslash => {
            let (set, next) = glyph_set(items, index + 1)?;
            Some((set, next))
        }
        Kind::GlyphName => Some((GlyphSet::Glyph(text(item)), index + 1)),
        Kind::Cid => Some((GlyphSet::Cid(text(item).parse().ok()?), index + 1)),
        Kind::NamedGlyphClass => Some((GlyphSet::Named(class_name(item)?), index + 1)),
        Kind::NullKw => Some((GlyphSet::Null, index + 1)),
        Kind::GlyphClass => Some((GlyphSet::Literal(class_items(item.as_node()?)?), index + 1)),
        _ => None,
    }
}

/// Every glyph operand in a flat sequence of children.
fn glyph_sequence(items: &[NodeOrToken]) -> Option<Vec<GlyphSet>> {
    let mut sets = Vec::new();
    let mut index = 0;
    while index < items.len() {
        let (set, next) = glyph_set(items, index)?;
        sets.push(set);
        index = next;
    }
    Some(sets)
}

fn anchor(node: &Node) -> AnchorPoint {
    let items = parts(node);
    let numbers: Vec<i32> = items.iter().filter_map(number).collect();
    let has_contour = items.iter().any(|item| item.kind() == Kind::ContourpointKw);
    match (items.len(), numbers.as_slice(), has_contour) {
        (4, _, _) if items[2].kind() == Kind::NullKw => AnchorPoint::Null,
        (5, [x, y], false) => AnchorPoint::At { x: *x, y: *y },
        (7, [x, y, point], true) if *point >= 0 => AnchorPoint::Contour {
            x: *x,
            y: *y,
            point: *point as u32,
        },
        _ => AnchorPoint::Other(node_text(node)),
    }
}

fn value_record(node: &Node) -> ValueRecord {
    let items = parts(node);
    let numbers: Vec<i32> = items.iter().filter_map(number).collect();
    match (items.len(), numbers.as_slice()) {
        (1, [advance]) => ValueRecord::Advance(*advance),
        (3, _) if items[1].kind() == Kind::NullKw => ValueRecord::Null,
        (6, [x_placement, y_placement, x_advance, y_advance]) => ValueRecord::Placement {
            x_placement: *x_placement,
            y_placement: *y_placement,
            x_advance: *x_advance,
            y_advance: *y_advance,
        },
        _ => ValueRecord::Other(node_text(node)),
    }
}

fn mark_class_def(node: &Node) -> Option<Statement> {
    let items = parts(node);
    if items.first()?.kind() != Kind::MarkClassKw {
        return None;
    }
    let (glyphs, next) = glyph_set(&items, 1)?;
    let anchor_node = items.get(next)?.as_node()?;
    if anchor_node.kind() != Kind::AnchorNode {
        return None;
    }
    let class = class_name(items.get(next + 1)?)?;
    if items.len() != next + 3 {
        return None;
    }
    Some(Statement::MarkClassDef {
        glyphs,
        anchor: anchor(anchor_node),
        class,
    })
}

fn lookup_flag(node: &Node) -> Option<Statement> {
    let items = parts(node);
    if items.first()?.kind() != Kind::LookupflagKw {
        return None;
    }
    if let (3, Some(value)) = (items.len(), number(&items[1])) {
        return u16::try_from(value)
            .ok()
            .map(|value| Statement::LookupFlag(LookupFlag::Value(value)));
    }
    let mut flag = LookupFlag::Named {
        right_to_left: false,
        ignore_base_glyphs: false,
        ignore_ligatures: false,
        ignore_marks: false,
        mark_attachment_type: None,
        use_mark_filtering_set: None,
    };
    let LookupFlag::Named {
        right_to_left,
        ignore_base_glyphs,
        ignore_ligatures,
        ignore_marks,
        mark_attachment_type,
        use_mark_filtering_set,
    } = &mut flag
    else {
        return None;
    };
    let mut index = 1;
    while index < items.len() {
        match items[index].kind() {
            Kind::Semi => {}
            Kind::RightToLeftKw => *right_to_left = true,
            Kind::IgnoreBaseGlyphsKw => *ignore_base_glyphs = true,
            Kind::IgnoreLigaturesKw => *ignore_ligatures = true,
            Kind::IgnoreMarksKw => *ignore_marks = true,
            Kind::MarkAttachmentTypeKw => {
                index += 1;
                *mark_attachment_type = Some(class_name(items.get(index)?)?);
            }
            Kind::UseMarkFilteringSetKw => {
                index += 1;
                *use_mark_filtering_set = Some(class_name(items.get(index)?)?);
            }
            _ => return None,
        }
        index += 1;
    }
    Some(Statement::LookupFlag(flag))
}

fn script(node: &Node) -> Option<Statement> {
    let items = parts(node);
    if items.len() != 3 || items[0].kind() != Kind::ScriptKw {
        return None;
    }
    Some(Statement::Script(tag_at(&items, 1)?))
}

fn language(node: &Node) -> Option<Statement> {
    let items = parts(node);
    if items.first()?.kind() != Kind::LanguageKw {
        return None;
    }
    let tag = tag_at(&items, 1)?;
    let mut policy = LanguagePolicy::default();
    for item in &items[2..] {
        match item.kind() {
            Kind::Semi => {}
            Kind::ExcludeDfltKw => policy.exclude_dflt = true,
            Kind::IncludeDfltKw => policy.include_dflt = true,
            Kind::RequiredKw => policy.required = true,
            _ => return None,
        }
    }
    Some(Statement::Language { tag, policy })
}

fn lookup_call(node: &Node) -> Option<Statement> {
    let items = parts(node);
    if items.first()?.kind() != Kind::LookupKw {
        return None;
    }
    let name = items.get(1)?;
    if !matches!(name.kind(), Kind::Ident | Kind::Label) {
        return None;
    }
    Some(Statement::LookupCall(text(name)))
}

fn feature_call(node: &Node) -> Option<Statement> {
    let items = parts(node);
    if items.first()?.kind() != Kind::FeatureKw {
        return None;
    }
    Some(Statement::FeatureCall(tag_at(&items, 1)?))
}

fn feature(node: &Node) -> Option<Statement> {
    let items = children(node);
    let tag = items.iter().find_map(|item| match item {
        NodeOrToken::Token(token) if token.kind == Kind::Tag => Some(token.text.to_string()),
        _ => None,
    })?;
    let mut statements = Vec::new();
    let mut inside = false;
    for item in &items {
        match item.kind() {
            Kind::LBrace => inside = true,
            Kind::RBrace => inside = false,
            _ if inside => push_statement(item, &mut statements),
            _ => {}
        }
    }
    Some(Statement::Feature { tag, statements })
}

fn lookup_block(node: &Node) -> Option<Statement> {
    let items = children(node);
    let name = items.iter().find_map(|item| match item {
        NodeOrToken::Token(token) if matches!(token.kind, Kind::Label | Kind::Ident) => {
            Some(token.text.to_string())
        }
        _ => None,
    })?;
    let use_extension = items.iter().any(|item| item.kind() == Kind::UseExtensionKw);
    let mut statements = Vec::new();
    let mut inside = false;
    for item in &items {
        match item.kind() {
            Kind::LBrace => inside = true,
            Kind::RBrace => inside = false,
            _ if inside => push_statement(item, &mut statements),
            _ => {}
        }
    }
    Some(Statement::Lookup {
        name,
        use_extension,
        statements,
    })
}

fn rule(node: &Node) -> Option<TypedRule> {
    let items = parts(node);
    let typed = |number: u8, table: fn(u8) -> LookupType, rule: Rule| {
        Some(TypedRule {
            lookup: table(number),
            rule,
        })
    };
    match node.kind() {
        Kind::GsubType1 => {
            let (target, next) = glyph_set(&items, 1)?;
            if items.get(next)?.kind() != Kind::ByKw {
                return None;
            }
            let (replacement, next) = glyph_set(&items, next + 1)?;
            (items.len() == next + 1).then_some(())?;
            typed(
                1,
                LookupType::gsub,
                Rule::SingleSub {
                    target,
                    replacement,
                },
            )
        }
        Kind::GsubType2 => {
            let (target, next) = glyph_set(&items, 1)?;
            if items.get(next)?.kind() != Kind::ByKw {
                return None;
            }
            let replacement = glyph_sequence(&items[next + 1..items.len() - 1])?;
            typed(
                2,
                LookupType::gsub,
                Rule::MultipleSub {
                    target,
                    replacement,
                },
            )
        }
        Kind::GsubType3 => {
            let (target, next) = glyph_set(&items, 1)?;
            if items.get(next)?.kind() != Kind::FromKw {
                return None;
            }
            let (alternates, next) = glyph_set(&items, next + 1)?;
            (items.len() == next + 1).then_some(())?;
            typed(
                3,
                LookupType::gsub,
                Rule::AlternateSub { target, alternates },
            )
        }
        Kind::GsubType4 => {
            let by = items.iter().position(|item| item.kind() == Kind::ByKw)?;
            let target = glyph_sequence(&items[1..by])?;
            let (replacement, next) = glyph_set(&items, by + 1)?;
            (items.len() == next + 1).then_some(())?;
            typed(
                4,
                LookupType::gsub,
                Rule::LigatureSub {
                    target,
                    replacement,
                },
            )
        }
        Kind::GsubType5 | Kind::GsubType6 => {
            let number = if node.kind() == Kind::GsubType5 { 5 } else { 6 };
            let (backtrack, input, lookahead, replacement) = chain(&items)?;
            typed(
                number,
                LookupType::gsub,
                Rule::ChainSub {
                    backtrack,
                    input,
                    lookahead,
                    replacement,
                },
            )
        }
        Kind::GsubType8 => {
            let (backtrack, input, lookahead, replacement) = chain(&items)?;
            typed(
                8,
                LookupType::gsub,
                Rule::ReverseSub {
                    backtrack,
                    input,
                    lookahead,
                    replacement,
                },
            )
        }
        Kind::GsubIgnore => typed(6, LookupType::gsub, Rule::IgnoreSub(ignore(&items)?)),
        Kind::GposType1 => {
            let (target, next) = glyph_set(&items, 1)?;
            let record = items.get(next)?.as_node()?;
            if record.kind() != Kind::ValueRecordNode || items.len() != next + 2 {
                return None;
            }
            typed(
                1,
                LookupType::gpos,
                Rule::SinglePos {
                    target,
                    value: value_record(record),
                },
            )
        }
        Kind::GposType2 => {
            let enumerated = items.first()?.kind() == Kind::EnumKw;
            let start = usize::from(enumerated) + 1;
            let (first, next) = glyph_set(&items, start)?;
            let first_record = items
                .get(next)
                .and_then(NodeOrToken::as_node)
                .filter(|node| node.kind() == Kind::ValueRecordNode);
            let after_first = next + usize::from(first_record.is_some());
            let (second, next) = glyph_set(&items, after_first)?;
            let second_record = items
                .get(next)
                .and_then(NodeOrToken::as_node)
                .filter(|node| node.kind() == Kind::ValueRecordNode);
            let after_second = next + usize::from(second_record.is_some());
            if items.len() != after_second + 1 {
                return None;
            }
            let values = match (first_record, second_record) {
                (Some(first_record), Some(second_record)) => {
                    PairValues::Both(value_record(first_record), value_record(second_record))
                }
                (Some(first_record), None) => PairValues::AfterFirst(value_record(first_record)),
                (None, Some(second_record)) => PairValues::AfterPair(value_record(second_record)),
                (None, None) => return None,
            };
            typed(
                2,
                LookupType::gpos,
                Rule::PairPos {
                    enumerated,
                    first,
                    second,
                    values,
                },
            )
        }
        Kind::GposType3 => {
            if items.get(1)?.kind() != Kind::CursiveKw {
                return None;
            }
            let (target, next) = glyph_set(&items, 2)?;
            let entry = items.get(next)?.as_node()?;
            let exit = items.get(next + 1)?.as_node()?;
            if entry.kind() != Kind::AnchorNode
                || exit.kind() != Kind::AnchorNode
                || items.len() != next + 3
            {
                return None;
            }
            typed(
                3,
                LookupType::gpos,
                Rule::CursivePos {
                    target,
                    entry: anchor(entry),
                    exit: anchor(exit),
                },
            )
        }
        Kind::GposType4 | Kind::GposType6 => {
            let (number, keyword) = if node.kind() == Kind::GposType4 {
                (4, Kind::BaseKw)
            } else {
                (6, Kind::MarkKw)
            };
            if items.get(1)?.kind() != keyword {
                return None;
            }
            let (target, next) = glyph_set(&items, 2)?;
            let mut marks = Vec::new();
            for item in &items[next..items.len() - 1] {
                marks.push(mark_attachment(item.as_node()?)?);
            }
            (!marks.is_empty()).then_some(())?;
            let rule = if number == 4 {
                Rule::MarkToBase { target, marks }
            } else {
                Rule::MarkToMark { target, marks }
            };
            typed(number, LookupType::gpos, rule)
        }
        Kind::GposType5 => {
            if items.get(1)?.kind() != Kind::LigatureKw {
                return None;
            }
            let (target, next) = glyph_set(&items, 2)?;
            let mut components = Vec::new();
            for item in &items[next..items.len() - 1] {
                let component = item.as_node()?;
                if component.kind() != Kind::LigatureComponentNode {
                    return None;
                }
                let mut marks = Vec::new();
                for entry in parts(component) {
                    match entry.kind() {
                        Kind::Ident => {}
                        Kind::AnchorMarkNode => marks.push(mark_attachment(entry.as_node()?)?),
                        _ => return None,
                    }
                }
                components.push(marks);
            }
            (!components.is_empty()).then_some(())?;
            typed(
                5,
                LookupType::gpos,
                Rule::MarkToLigature { target, components },
            )
        }
        Kind::GposType7 | Kind::GposType8 => {
            let number = if node.kind() == Kind::GposType7 { 7 } else { 8 };
            let (backtrack, input, lookahead, replacement) = chain(&items)?;
            replacement.is_none().then_some(())?;
            typed(
                number,
                LookupType::gpos,
                Rule::ChainPos {
                    backtrack,
                    input,
                    lookahead,
                },
            )
        }
        Kind::GposIgnore => typed(8, LookupType::gpos, Rule::IgnorePos(ignore(&items)?)),
        _ => None,
    }
}

fn mark_attachment(node: &Node) -> Option<MarkAttachment> {
    if node.kind() != Kind::AnchorMarkNode {
        return None;
    }
    let items = parts(node);
    let anchor_node = items.first()?.as_node()?;
    if anchor_node.kind() != Kind::AnchorNode {
        return None;
    }
    let class = match items.len() {
        1 => None,
        3 if items[1].kind() == Kind::MarkKw => Some(class_name(&items[2])?),
        _ => return None,
    };
    Some(MarkAttachment {
        anchor: anchor(anchor_node),
        class,
    })
}

type Chain = (
    Vec<GlyphSet>,
    Vec<ContextItem>,
    Vec<GlyphSet>,
    Option<Vec<GlyphSet>>,
);

fn chain(items: &[NodeOrToken]) -> Option<Chain> {
    let mut backtrack = None;
    let mut input = None;
    let mut lookahead = None;
    let mut replacement = None;
    for item in items {
        match item.kind() {
            // A leading escape can sit outside the sequence node it belongs
            // to; the writer re-escapes every name it emits.
            Kind::SubKw
            | Kind::PosKw
            | Kind::RsubKw
            | Kind::Semi
            | Kind::IgnoreKw
            | Kind::Backslash => {}
            Kind::BacktrackSequence => backtrack = Some(glyph_sequence(&parts(item.as_node()?))?),
            Kind::LookaheadSequence => lookahead = Some(glyph_sequence(&parts(item.as_node()?))?),
            Kind::ContextSequence => input = Some(context_sequence(item.as_node()?)?),
            Kind::InlineSubNode => {
                let node = item.as_node()?;
                let inline = parts(node);
                if inline.first()?.kind() != Kind::ByKw {
                    return None;
                }
                replacement = Some(glyph_sequence(&inline[1..])?);
            }
            _ => return None,
        }
    }
    let input = input?;
    (!input.is_empty()).then_some(())?;
    Some((
        backtrack.unwrap_or_default(),
        input,
        lookahead.unwrap_or_default(),
        replacement,
    ))
}

fn context_sequence(node: &Node) -> Option<Vec<ContextItem>> {
    let mut items = Vec::new();
    for child in parts(node) {
        let marked = child.as_node()?;
        if marked.kind() != Kind::ContextGlyphNode {
            return None;
        }
        let children = parts(marked);
        let (glyphs, mut index) = glyph_set(&children, 0)?;
        if children.get(index)?.kind() != Kind::SingleQuote {
            return None;
        }
        index += 1;
        let mut lookups = Vec::new();
        let mut value = None;
        while index < children.len() {
            match children[index].kind() {
                // The escape of the following lookahead glyph is lexed inside
                // this node; the glyph itself belongs to the lookahead sequence.
                Kind::Backslash => {}
                Kind::LookupRefNode => {
                    let reference = children[index].as_node()?;
                    let name = parts(reference).into_iter().nth(1)?;
                    if !matches!(name.kind(), Kind::Ident | Kind::Label) {
                        return None;
                    }
                    lookups.push(text(&name));
                }
                Kind::ValueRecordNode => value = Some(value_record(children[index].as_node()?)),
                _ => return None,
            }
            index += 1;
        }
        items.push(ContextItem {
            glyphs,
            lookups,
            value,
        });
    }
    Some(items)
}

fn ignore(items: &[NodeOrToken]) -> Option<Vec<IgnorePattern>> {
    let mut patterns = Vec::new();
    for item in items {
        match item.kind() {
            Kind::IgnoreKw
            | Kind::SubKw
            | Kind::PosKw
            | Kind::Comma
            | Kind::Semi
            | Kind::Backslash => {}
            Kind::IgnoreRuleStatementNode => {
                let (backtrack, input, lookahead, replacement) = chain(&parts(item.as_node()?))?;
                replacement.is_none().then_some(())?;
                if input
                    .iter()
                    .any(|item| !item.lookups.is_empty() || item.value.is_some())
                {
                    return None;
                }
                patterns.push(IgnorePattern {
                    backtrack,
                    input: input.into_iter().map(|item| item.glyphs).collect(),
                    lookahead,
                });
            }
            _ => return None,
        }
    }
    (!patterns.is_empty()).then_some(())?;
    Some(patterns)
}
