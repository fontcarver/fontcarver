//! Render the typed IR back to canonical feature syntax.
//!
//! Every glyph name that can be escaped is escaped, so a name that collides
//! with a keyword keeps its meaning. Constructs the IR kept verbatim are
//! written back byte for byte.

use super::{
    AnchorPoint, ClassItem, ContextItem, FeaIr, GlyphSet, IgnorePattern, LanguagePolicy,
    LookupFlag, MarkAttachment, PairValues, Rule, Statement, TypedRule, ValueRecord,
};

/// Render a parsed feature file.
pub fn write_fea(ir: &FeaIr) -> String {
    let mut out = String::new();
    statements(&ir.statements, 0, &mut out);
    out
}

fn statements(items: &[Statement], depth: usize, out: &mut String) {
    for item in items {
        statement(item, depth, out);
    }
}

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn statement(item: &Statement, depth: usize, out: &mut String) {
    indent(depth, out);
    match item {
        Statement::LanguageSystem { script, language } => {
            out.push_str(&format!("languagesystem {script} {language};\n"));
        }
        Statement::GlyphClassDef { name, members } => {
            out.push_str(&format!("@{name} = [{}];\n", class_items(members)));
        }
        Statement::MarkClassDef {
            glyphs,
            anchor,
            class,
        } => {
            out.push_str(&format!(
                "markClass {} {} @{class};\n",
                glyph_set(glyphs),
                anchor_text(anchor)
            ));
        }
        Statement::LookupFlag(flag) => out.push_str(&format!("{};\n", lookup_flag(flag))),
        Statement::Script(tag) => out.push_str(&format!("script {tag};\n")),
        Statement::Language { tag, policy } => {
            out.push_str(&format!("language {tag}{};\n", language_policy(policy)));
        }
        Statement::Subtable => out.push_str("subtable;\n"),
        Statement::LookupCall(name) => out.push_str(&format!("lookup {name};\n")),
        Statement::FeatureCall(tag) => out.push_str(&format!("feature {tag};\n")),
        Statement::Feature {
            tag,
            statements: body,
        } => {
            out.push_str(&format!("feature {tag} {{\n"));
            statements(body, depth + 1, out);
            indent(depth, out);
            out.push_str(&format!("}} {tag};\n"));
        }
        Statement::Lookup {
            name,
            use_extension,
            statements: body,
        } => {
            let extension = if *use_extension { " useExtension" } else { "" };
            out.push_str(&format!("lookup {name}{extension} {{\n"));
            statements(body, depth + 1, out);
            indent(depth, out);
            out.push_str(&format!("}} {name};\n"));
        }
        Statement::Rule(rule) => out.push_str(&format!("{};\n", typed_rule(rule))),
        Statement::Comment(text) => {
            out.push_str(text);
            out.push('\n');
        }
        Statement::Verbatim(text) => {
            out.push_str(text);
            out.push('\n');
        }
    }
}

fn language_policy(policy: &LanguagePolicy) -> String {
    // The feature file grammar puts the default-language policy before
    // `required`; reversing them is a syntax error.
    let mut out = String::new();
    if policy.exclude_dflt {
        out.push_str(" exclude_dflt");
    }
    if policy.include_dflt {
        out.push_str(" include_dflt");
    }
    if policy.required {
        out.push_str(" required");
    }
    out
}

fn lookup_flag(flag: &LookupFlag) -> String {
    match flag {
        LookupFlag::Value(value) => format!("lookupflag {value}"),
        LookupFlag::Named {
            right_to_left,
            ignore_base_glyphs,
            ignore_ligatures,
            ignore_marks,
            mark_attachment_type,
            use_mark_filtering_set,
        } => {
            let mut parts = Vec::new();
            if *right_to_left {
                parts.push("RightToLeft".to_string());
            }
            if *ignore_base_glyphs {
                parts.push("IgnoreBaseGlyphs".to_string());
            }
            if *ignore_ligatures {
                parts.push("IgnoreLigatures".to_string());
            }
            if *ignore_marks {
                parts.push("IgnoreMarks".to_string());
            }
            if let Some(class) = mark_attachment_type {
                parts.push(format!("MarkAttachmentType @{class}"));
            }
            if let Some(class) = use_mark_filtering_set {
                parts.push(format!("UseMarkFilteringSet @{class}"));
            }
            if parts.is_empty() {
                // An all-clear named flag has no spelling; `0` means the same.
                return "lookupflag 0".to_string();
            }
            format!("lookupflag {}", parts.join(" "))
        }
    }
}

/// Escape a glyph name unless a backslash would turn it into a CID.
fn glyph_name(name: &str) -> String {
    let escapable = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || matches!(first, '_' | '.'));
    if escapable {
        format!("\\{name}")
    } else {
        name.to_string()
    }
}

fn class_items(members: &[ClassItem]) -> String {
    members
        .iter()
        .map(|member| match member {
            ClassItem::Glyph(name) => glyph_name(name),
            ClassItem::Range(start, end) => format!("{} - {}", glyph_name(start), glyph_name(end)),
            ClassItem::Cid(cid) => format!("\\{cid}"),
            ClassItem::CidRange(start, end) => format!("\\{start} - \\{end}"),
            ClassItem::Named(name) => format!("@{name}"),
            ClassItem::Unresolved(text) => text.clone(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn glyph_set(set: &GlyphSet) -> String {
    match set {
        GlyphSet::Glyph(name) => glyph_name(name),
        GlyphSet::Cid(cid) => format!("\\{cid}"),
        GlyphSet::Named(name) => format!("@{name}"),
        GlyphSet::Literal(members) => format!("[{}]", class_items(members)),
        GlyphSet::Null => "NULL".to_string(),
    }
}

fn sequence(sets: &[GlyphSet]) -> String {
    sets.iter().map(glyph_set).collect::<Vec<_>>().join(" ")
}

fn value_record(record: &ValueRecord) -> String {
    match record {
        ValueRecord::Advance(advance) => advance.to_string(),
        ValueRecord::Placement {
            x_placement,
            y_placement,
            x_advance,
            y_advance,
        } => format!("<{x_placement} {y_placement} {x_advance} {y_advance}>"),
        ValueRecord::Null => "<NULL>".to_string(),
        ValueRecord::Other(text) => text.clone(),
    }
}

fn anchor_text(anchor: &AnchorPoint) -> String {
    match anchor {
        AnchorPoint::At { x, y } => format!("<anchor {x} {y}>"),
        AnchorPoint::Contour { x, y, point } => format!("<anchor {x} {y} contourpoint {point}>"),
        AnchorPoint::Null => "<anchor NULL>".to_string(),
        AnchorPoint::Other(text) => text.clone(),
    }
}

fn mark_attachment(mark: &MarkAttachment) -> String {
    match &mark.class {
        Some(class) => format!("{} mark @{class}", anchor_text(&mark.anchor)),
        None => anchor_text(&mark.anchor),
    }
}

fn context_item(item: &ContextItem) -> String {
    let mut out = format!("{}'", glyph_set(&item.glyphs));
    if let Some(value) = &item.value {
        out.push(' ');
        out.push_str(&value_record(value));
    }
    for lookup in &item.lookups {
        out.push_str(&format!(" lookup {lookup}"));
    }
    out
}

fn context(backtrack: &[GlyphSet], input: &[ContextItem], lookahead: &[GlyphSet]) -> String {
    let mut parts = Vec::new();
    if !backtrack.is_empty() {
        parts.push(sequence(backtrack));
    }
    parts.push(input.iter().map(context_item).collect::<Vec<_>>().join(" "));
    if !lookahead.is_empty() {
        parts.push(sequence(lookahead));
    }
    parts.join(" ")
}

fn ignore_pattern(pattern: &IgnorePattern) -> String {
    let mut parts = Vec::new();
    if !pattern.backtrack.is_empty() {
        parts.push(sequence(&pattern.backtrack));
    }
    parts.push(
        pattern
            .input
            .iter()
            .map(|set| format!("{}'", glyph_set(set)))
            .collect::<Vec<_>>()
            .join(" "),
    );
    if !pattern.lookahead.is_empty() {
        parts.push(sequence(&pattern.lookahead));
    }
    parts.join(" ")
}

fn ignore_patterns(patterns: &[IgnorePattern]) -> String {
    patterns
        .iter()
        .map(ignore_pattern)
        .collect::<Vec<_>>()
        .join(", ")
}

fn typed_rule(rule: &TypedRule) -> String {
    match &rule.rule {
        Rule::SingleSub {
            target,
            replacement,
        } => format!("sub {} by {}", glyph_set(target), glyph_set(replacement)),
        Rule::MultipleSub {
            target,
            replacement,
        } => format!("sub {} by {}", glyph_set(target), sequence(replacement)),
        Rule::AlternateSub { target, alternates } => {
            format!("sub {} from {}", glyph_set(target), glyph_set(alternates))
        }
        Rule::LigatureSub {
            target,
            replacement,
        } => format!("sub {} by {}", sequence(target), glyph_set(replacement)),
        Rule::ChainSub {
            backtrack,
            input,
            lookahead,
            replacement,
        } => {
            let mut out = format!("sub {}", context(backtrack, input, lookahead));
            if let Some(replacement) = replacement {
                out.push_str(&format!(" by {}", sequence(replacement)));
            }
            out
        }
        Rule::ReverseSub {
            backtrack,
            input,
            lookahead,
            replacement,
        } => {
            let mut out = format!("rsub {}", context(backtrack, input, lookahead));
            if let Some(replacement) = replacement {
                out.push_str(&format!(" by {}", sequence(replacement)));
            }
            out
        }
        Rule::IgnoreSub(patterns) => format!("ignore sub {}", ignore_patterns(patterns)),
        Rule::SinglePos { target, value } => {
            format!("pos {} {}", glyph_set(target), value_record(value))
        }
        Rule::PairPos {
            enumerated,
            first,
            second,
            values,
        } => {
            let prefix = if *enumerated { "enum pos" } else { "pos" };
            match values {
                PairValues::AfterPair(value) => format!(
                    "{prefix} {} {} {}",
                    glyph_set(first),
                    glyph_set(second),
                    value_record(value)
                ),
                PairValues::AfterFirst(value) => format!(
                    "{prefix} {} {} {}",
                    glyph_set(first),
                    value_record(value),
                    glyph_set(second)
                ),
                PairValues::Both(first_value, second_value) => format!(
                    "{prefix} {} {} {} {}",
                    glyph_set(first),
                    value_record(first_value),
                    glyph_set(second),
                    value_record(second_value)
                ),
            }
        }
        Rule::CursivePos {
            target,
            entry,
            exit,
        } => format!(
            "pos cursive {} {} {}",
            glyph_set(target),
            anchor_text(entry),
            anchor_text(exit)
        ),
        Rule::MarkToBase { target, marks } => format!(
            "pos base {} {}",
            glyph_set(target),
            marks
                .iter()
                .map(mark_attachment)
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Rule::MarkToMark { target, marks } => format!(
            "pos mark {} {}",
            glyph_set(target),
            marks
                .iter()
                .map(mark_attachment)
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Rule::MarkToLigature { target, components } => {
            let mut out = format!("pos ligature {}", glyph_set(target));
            for (index, marks) in components.iter().enumerate() {
                if index > 0 {
                    out.push_str(" ligComponent");
                }
                for mark in marks {
                    out.push(' ');
                    out.push_str(&mark_attachment(mark));
                }
            }
            out
        }
        Rule::ChainPos {
            backtrack,
            input,
            lookahead,
        } => format!("pos {}", context(backtrack, input, lookahead)),
        Rule::IgnorePos(patterns) => format!("ignore pos {}", ignore_patterns(patterns)),
    }
}
