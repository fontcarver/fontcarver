use std::collections::{BTreeSet, HashSet};

use fc_model::{Font, Glyph, GlyphName, KernSide, LayerId};

use crate::Error;

use super::apply::{glyph_mut, unique_names, validate_glyph_outlines};

pub(super) fn add(font: &mut Font, glyph: &Glyph) -> Result<(), Error> {
    validate_glyph_outlines(glyph)?;
    font.insert_glyph(glyph.clone()).map_err(Error::from)
}

pub(super) fn rename(font: &mut Font, from: &GlyphName, to: &GlyphName) -> Result<(), Error> {
    if from != to && font.glyph(from).is_some() && font.glyph(to).is_none() {
        let paired = font
            .masters
            .iter()
            .flat_map(|master| master.kerning_h.0.keys().chain(master.kerning_v.0.keys()))
            .any(|(left, right)| {
                matches!(left, KernSide::Glyph(name) if name == to)
                    || matches!(right, KernSide::Glyph(name) if name == to)
            });
        let grouped = font
            .kerning_groups
            .side1
            .values()
            .chain(font.kerning_groups.side2.values())
            .any(|members| members.contains(to));
        if paired || grouped {
            return Err(Error::InvalidCommand(format!(
                "glyph `{to}` already has unresolved kerning references; rename would change existing source records"
            )));
        }
    }
    font.rename_glyph(from, to).map_err(Error::from)?;
    follow_rename(font, from, to);
    Ok(())
}

/// Follows a rename into the references `Font::rename_glyph` does not rewrite.
///
/// The model layer already moves components, constructions, kerning pairs and
/// groups, bitmap references, CID, MATH, colour and SVG data. Metrics formulas,
/// feature source and the project's named glyph lists are stored as text or as
/// metadata, so they are rewritten here, inside the same undoable command.
///
/// History calls this from `Document::rename_references` with the two names
/// swapped, which is what makes undo and redo of a rename restore them.
pub(crate) fn follow_rename(font: &mut Font, from: &GlyphName, to: &GlyphName) {
    let from = from.as_ref();
    for glyph in font.glyphs.values_mut() {
        rename_metrics_keys(&mut glyph.metrics_keys, from, to);
        for layer in &mut glyph.layers {
            if let Some(keys) = layer.metrics_keys.as_mut() {
                rename_metrics_keys(keys, from, to);
            }
        }
    }
    for class in &mut font.features.classes {
        class.code = rename_in_fea(&class.code, from, to);
    }
    for block in font
        .features
        .prefixes
        .iter_mut()
        .chain(font.features.features.iter_mut())
    {
        block.code = rename_in_fea(&block.code, from, to);
    }
    rename_in_glyph_lists(&mut font.info.user_data, from, to);
}

fn rename_metrics_keys(keys: &mut fc_model::MetricsKeys, from: &str, to: &GlyphName) {
    for key in [
        &mut keys.left,
        &mut keys.right,
        &mut keys.width,
        &mut keys.vertical_origin,
        &mut keys.vertical_width,
    ] {
        if let Some(text) = key.as_mut() {
            if let Some(renamed) = rename_in_metrics_key(text, from, to) {
                *text = renamed;
            }
        }
    }
}

/// Rewrites the glyph reference of one metrics formula, keeping everything else.
///
/// A formula is `=`/`==`, an optional mirror `|`, the referenced glyph name, an
/// optional `+-*/` adjustment and an optional `@height`. Literal and automatic
/// formulas hold no glyph name and are left alone.
fn rename_in_metrics_key(key: &str, from: &str, to: &GlyphName) -> Option<String> {
    let trimmed = key.trim();
    let body = trimmed.strip_prefix('=')?;
    let (prefix, body) = match body.strip_prefix('=') {
        Some(body) => ("==", body),
        None => ("=", body),
    };
    let body = body.trim();
    if body.starts_with(['+', '-']) || body.parse::<f64>().is_ok() {
        return None;
    }
    let (body, height) = match body.rsplit_once('@') {
        Some((body, height)) if height.parse::<f64>().is_ok() => (body, Some(height)),
        _ => (body, None),
    };
    let (mirror, body) = match body.strip_prefix('|') {
        Some(body) => ("|", body),
        None => ("", body),
    };
    let (name, operation) = split_metrics_operation(body);
    if name != from {
        return None;
    }
    let height = height.map(|value| format!("@{value}")).unwrap_or_default();
    Some(format!("{prefix}{mirror}{to}{operation}{height}"))
}

/// Splits a formula body into its glyph name and its trailing adjustment.
fn split_metrics_operation(body: &str) -> (&str, &str) {
    for (index, character) in body.char_indices().rev() {
        if index > 0
            && matches!(character, '+' | '-' | '*' | '/')
            && body[index + 1..].parse::<f64>().is_ok()
        {
            return (&body[..index], &body[index..]);
        }
    }
    (body, "")
}

/// Feature-syntax words that are never glyph names in the position they appear.
const FEA_KEYWORDS: &[&str] = &[
    "feature",
    "table",
    "lookup",
    "lookupflag",
    "script",
    "language",
    "sub",
    "substitute",
    "by",
    "from",
    "ignore",
    "pos",
    "position",
    "enum",
    "enumerate",
    "except",
    "anchor",
    "anchorDef",
    "mark",
    "markClass",
    "subtable",
    "useExtension",
    "exclude_dflt",
    "include_dflt",
    "required",
    "RightToLeft",
    "IgnoreBaseGlyphs",
    "IgnoreLigatures",
    "IgnoreMarks",
    "UseMarkFilteringSet",
    "MarkAttachmentType",
    "valueRecordDef",
    "languagesystem",
    "cursive",
    "base",
    "ligature",
    "NULL",
    "contourpoint",
    "device",
    "include",
    "parameters",
    "nameid",
    "sizemenuname",
    "FontRevision",
    "GlyphClassDef",
    "Attach",
    "LigatureCaretByDev",
    "LigatureCaretByIndex",
    "LigatureCaretByPos",
    "MarkAttachClass",
    "HorizAxis",
    "VertAxis",
    "BaseTagList",
    "BaseScriptList",
    "MinMax",
    "winAscent",
    "winDescent",
    "TypoAscender",
    "TypoDescender",
    "TypoLineGap",
    "Panose",
    "Vendor",
    "UnicodeRange",
    "CodePageRange",
    "XHeight",
    "CapHeight",
    "WeightClass",
    "WidthClass",
    "FSType",
    "LowerOpSize",
    "UpperOpSize",
    "conditionset",
    "variation",
    "featureNames",
    "cvParameters",
    "Character",
    "ParamUILabelNameID",
];

/// Words whose following token names a feature, lookup, script or table, not a glyph.
///
/// `markClass` is deliberately absent: its next token is a glyph or a glyph
/// class, and the class form already starts with `@`, which the rewriter skips
/// on its own.
const FEA_NAMING_KEYWORDS: &[&str] = &[
    "feature",
    "lookup",
    "script",
    "language",
    "table",
    "nameid",
    "include",
    "conditionset",
    "variation",
];

/// Rewrites bare glyph-name tokens in feature source, leaving everything else.
///
/// Comments, quoted strings, class names after `@`, feature/lookup/script/table
/// names and the syntax keywords are skipped, so `sub f i by f_i;` follows a
/// rename of `f_i` while `feature liga` keeps its tag. A glyph whose name is
/// itself a feature-syntax keyword is not rewritten; that is a documented bound.
fn rename_in_fea(code: &str, from: &str, to: &GlyphName) -> String {
    if !code.contains(from) {
        return code.to_owned();
    }
    let mut out = String::with_capacity(code.len());
    let bytes = code.as_bytes();
    let mut index = 0;
    let mut previous_word: Option<&str> = None;
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b'#' => {
                let end = code[index..]
                    .find('\n')
                    .map_or(code.len(), |offset| index + offset);
                out.push_str(&code[index..end]);
                index = end;
            }
            b'"' => {
                let mut end = index + 1;
                while end < bytes.len() && bytes[end] != b'"' {
                    end += 1;
                }
                end = (end + 1).min(bytes.len());
                out.push_str(&code[index..end]);
                index = end;
            }
            b'@' => {
                out.push('@');
                index += 1;
                let end = word_end(bytes, index);
                out.push_str(&code[index..end]);
                previous_word = None;
                index = end;
            }
            byte if is_word_byte(byte) => {
                let end = word_end(bytes, index);
                let word = &code[index..end];
                let naming =
                    previous_word.is_some_and(|previous| FEA_NAMING_KEYWORDS.contains(&previous));
                if word == from && !naming && !FEA_KEYWORDS.contains(&word) {
                    out.push_str(to.as_ref());
                } else {
                    out.push_str(word);
                }
                previous_word = Some(word);
                index = end;
            }
            byte => {
                out.push(char::from(byte));
                if !byte.is_ascii_whitespace() {
                    previous_word = None;
                }
                index += 1;
            }
        }
    }
    out
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.')
}

fn word_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }
    end
}

/// Follows a rename into the project's persisted named glyph lists.
fn rename_in_glyph_lists(user_data: &mut serde_json::Value, from: &str, to: &GlyphName) {
    let Some(lists) = user_data
        .get_mut("com.fontcarver.inventory")
        .and_then(|inventory| inventory.get_mut("glyph_lists"))
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    for list in lists {
        let Some(names) = list
            .get_mut("glyphs")
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        for name in names {
            if name.as_str() == Some(from) {
                *name = serde_json::Value::String(to.to_string());
            }
        }
    }
}

pub(super) fn remove(font: &mut Font, names: &[GlyphName]) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for name in names {
        if seen.insert(name) && font.glyph(name).is_none() {
            return Err(Error::MissingGlyph(name.clone()));
        }
    }
    // Resolve dependants before removing any referenced image, preserving their pixels.
    for strike in &mut font.bitmap_strikes {
        let affected: Vec<_> = strike
            .glyphs
            .iter()
            .filter(|(name, glyph)| {
                !seen.contains(name) && glyph.refs.iter().any(|r| seen.contains(&r.base))
            })
            .map(|(name, _)| name.clone())
            .collect();
        let resolved = affected
            .iter()
            .map(|name| Ok((name.clone(), strike.flattened(name)?)))
            .collect::<Result<Vec<_>, fc_model::Error>>()?;
        for (name, glyph) in resolved {
            strike.glyphs.insert(name, glyph);
        }
        for name in names {
            strike.glyphs.remove(name);
        }
    }
    if let Some(cid) = &mut font.cid {
        cid.cid_map.retain(|_, name| !seen.contains(name));
    }
    if let Some(math) = &mut font.math {
        math.retain_glyphs(|name| !seen.contains(name));
    }
    if let Some(color) = &mut font.color {
        color.retain_glyphs(|name| !seen.contains(name));
    }
    if let Some(svg) = &mut font.svg {
        svg.retain_glyphs(|name| !seen.contains(name));
    }
    for name in names {
        font.remove_glyph(name);
        for members in font
            .kerning_groups
            .side1
            .values_mut()
            .chain(font.kerning_groups.side2.values_mut())
        {
            members.remove(name);
        }
        for master in &mut font.masters {
            for pairs in [&mut master.kerning_h, &mut master.kerning_v] {
                pairs.0.retain(|(left, right), _| {
                    !matches!(left, fc_model::KernSide::Glyph(glyph) if glyph == name)
                        && !matches!(right, fc_model::KernSide::Glyph(glyph) if glyph == name)
                });
            }
        }
    }
    Ok(())
}

pub(super) fn set_meta(
    font: &mut Font,
    name: &GlyphName,
    export: &Option<bool>,
    color_label: &Option<Option<u8>>,
    tags: &Option<BTreeSet<String>>,
    note: &Option<String>,
) -> Result<(), Error> {
    let glyph = glyph_mut(font, name)?;
    if let Some(value) = export {
        glyph.export = *value;
    }
    if let Some(value) = color_label {
        glyph.color_label = *value;
    }
    if let Some(value) = tags {
        glyph.tags.clone_from(value);
    }
    if let Some(value) = note {
        glyph.note.clone_from(value);
    }
    Ok(())
}

pub(super) fn duplicate(
    font: &mut Font,
    source: &GlyphName,
    name: &GlyphName,
) -> Result<(), Error> {
    let mut glyph = font
        .glyph(source)
        .cloned()
        .ok_or_else(|| Error::MissingGlyph(source.clone()))?;
    glyph.name = name.clone();
    glyph.unicodes.clear();
    glyph.variation_selectors.clear();
    for layer in &mut glyph.layers {
        layer.id = LayerId::new();
    }
    font.insert_glyph(glyph).map_err(Error::from)?;
    for strike in &mut font.bitmap_strikes {
        if let Some(bitmap) = strike.glyphs.get(source).cloned() {
            strike.glyphs.insert(name.clone(), bitmap);
        }
    }
    if let Some(color) = &mut font.color {
        if let Some(graph) = &mut color.graph {
            graph.duplicate_glyph(source, name);
        }
        if let Some(layers) = color.glyphs.get(source).cloned() {
            color.glyphs.insert(name.clone(), layers);
        }
    }
    if let Some(svg) = &mut font.svg {
        svg.duplicate_glyph(source, name);
    }
    // The copy has the same shape, so its placement data applies unchanged.
    if let Some(math) = &mut font.math {
        if let Some(info) = math.glyphs.get(source).cloned() {
            math.glyphs.insert(name.clone(), info);
        }
    }
    Ok(())
}

pub(super) fn set_order(font: &mut Font, order: &[GlyphName]) -> Result<(), Error> {
    if order.len() != font.glyphs.len()
        || !unique_names(order)
        || order.iter().any(|name| font.glyph(name).is_none())
    {
        return Err(Error::InvalidCommand(
            "glyph order must contain every glyph exactly once".to_owned(),
        ));
    }
    for (target, name) in order.iter().enumerate() {
        let current = font
            .glyphs
            .get_index_of(name)
            .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
        font.glyphs.move_index(current, target);
    }
    Ok(())
}
