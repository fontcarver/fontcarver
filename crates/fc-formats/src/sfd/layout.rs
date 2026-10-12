//! The native, bounded GSUB/GPOS subset expressible without contextual machinery.

use super::{error, integer, number, strings, MAX_ITEMS};
use crate::Error;
use fc_model::{FeatureBlock, Font};
use std::collections::BTreeMap;

type FeatureBindings = Vec<(String, Vec<(String, Vec<String>)>)>;

#[derive(Clone, Debug)]
pub(super) struct Lookup {
    pub kind: i64,
    pub flags: i64,
    pub name: String,
    pub subtables: Vec<String>,
    pub features: FeatureBindings,
}

impl Lookup {
    pub fn parse(value: &str) -> Result<Self, Error> {
        let tokens = strings::tokens(value)?;
        let mut t = Cursor::new(tokens);
        let kind = integer(&t.next()?)?;
        if !matches!(kind, 1..=4 | 257..=262) {
            return Err(error(format!("unsupported lookup type {kind}")));
        }
        let flags = integer(&t.next()?)?;
        if flags & !15 != 0 {
            return Err(error(
                "mark-filtering/attachment lookup flags are unsupported",
            ));
        }
        integer(&t.next()?)?;
        let name = t.next()?;
        t.eat("{")?;
        let mut subtables = Vec::new();
        while t.peek() != Some("}") {
            subtables.push(t.next()?);
            if t.peek() == Some("[") {
                t.next()?;
                // These three values are editor autokerning defaults.
                while t.peek() != Some("]") {
                    t.next()?;
                }
                t.next()?;
            }
            if t.peek() == Some("(") {
                return Err(error(
                    "lookup subtable suffix/vertical settings are unsupported",
                ));
            }
        }
        t.eat("}")?;
        t.eat("[")?;
        let mut features = Vec::new();
        while t.peek() != Some("]") {
            let feature = t.next()?;
            tag(&feature)?;
            t.eat("(")?;
            let mut scripts = Vec::new();
            while t.peek() != Some(")") {
                let script = t.next()?;
                tag(&script)?;
                t.eat("<")?;
                let mut languages = Vec::new();
                while t.peek() != Some(">") {
                    let language = t.next()?;
                    tag(&language)?;
                    languages.push(language);
                }
                t.eat(">")?;
                scripts.push((script, languages));
            }
            t.eat(")")?;
            features.push((feature, scripts));
        }
        t.eat("]")?;
        if t.peek().is_some() || subtables.is_empty() {
            return Err(error("malformed lookup declaration"));
        }
        Ok(Self {
            kind,
            flags,
            name,
            subtables,
            features,
        })
    }

    pub fn native(&self) -> String {
        let mut out = format!(
            "Lookup: {} {} 0 {} {{",
            self.kind,
            self.flags,
            strings::quote(&self.name)
        );
        for table in &self.subtables {
            out.push_str(&format!("{} ", strings::quote(table)));
        }
        out.push_str("} [");
        for (feature, scripts) in &self.features {
            out.push_str(&format!("'{feature}' ("));
            for (script, languages) in scripts {
                out.push_str(&format!("'{script}' <"));
                for language in languages {
                    out.push_str(&format!("'{language}' "));
                }
                out.push_str("> ");
            }
            out.push_str(") ");
        }
        out.push_str("]\n");
        out
    }

    pub fn simple_kern(&self) -> bool {
        self.kind == 258
            && self.flags == 0
            && (self.features.is_empty()
                || self.features
                    == vec![("kern".into(), vec![("DFLT".into(), vec!["dflt".into()])])])
    }
}

#[derive(Default)]
pub(super) struct Output {
    pub lookups: Vec<Lookup>,
    pub glyph_lines: BTreeMap<String, Vec<String>>,
}

pub(super) fn native_rule(
    key: &str,
    glyph: &str,
    value: &str,
) -> Result<(String, String, i64), Error> {
    let t = strings::tokens(value)?;
    let Some(table) = t.first() else {
        return Err(error("missing layout subtable name"));
    };
    let rest = &t[1..];
    for name in rest
        .iter()
        .filter(|name| !name.contains('=') && !matches!(name.as_str(), "{" | "}"))
    {
        glyph_token(name)?;
    }
    let glyph = fea_glyph(glyph);
    let glyphs = || {
        rest.iter()
            .map(|name| fea_glyph(name))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let (rule, kind) = match key {
        "Substitution2" if rest.len() == 1 => {
            (format!("sub {glyph} by {};", fea_glyph(&rest[0])), 1)
        }
        "MultipleSubs2" if !rest.is_empty() => (format!("sub {glyph} by {};", glyphs()), 2),
        "AlternateSubs2" if !rest.is_empty() => (format!("sub {glyph} from [{}];", glyphs()), 3),
        "Ligature2" if rest.len() >= 2 => (format!("sub {} by {glyph};", glyphs()), 4),
        "Position2" => (format!("pos {glyph} {};", value_record(rest)?), 257),
        "PairPos2" if rest.len() == 9 => (
            format!(
                "pos {glyph} {} {} {};",
                value_record(&rest[1..5])?,
                fea_glyph(&rest[0]),
                value_record(&rest[5..])?
            ),
            258,
        ),
        _ => return Err(error(format!("malformed or unsupported {key}"))),
    };
    Ok((table.clone(), rule, kind))
}

pub(super) fn fea_glyph(name: &str) -> String {
    format!("\\{name}")
}

fn value_record(words: &[String]) -> Result<String, Error> {
    if words.len() != 4 {
        return Err(error("device tables in positioning are unsupported"));
    }
    let mut values = Vec::new();
    for (word, prefix) in words.iter().zip(["dx=", "dy=", "dh=", "dv="]) {
        values.push(number(
            word.strip_prefix(prefix)
                .ok_or_else(|| error("invalid positioning record"))?,
        )?);
    }
    Ok(format!(
        "<{} {} {} {}>",
        values[0], values[1], values[2], values[3]
    ))
}

pub(super) fn install(
    font: &mut Font,
    lookups: &[Lookup],
    rules: &BTreeMap<String, Vec<String>>,
) -> Result<(), Error> {
    let mut features: BTreeMap<String, String> = BTreeMap::new();
    let mut prefix = String::new();
    for (index, lookup) in lookups.iter().enumerate() {
        let all: Vec<_> = lookup
            .subtables
            .iter()
            .flat_map(|table| rules.get(table).into_iter().flatten())
            .collect();
        if all.is_empty() {
            continue;
        }
        if lookup.kind >= 259 {
            return Err(error(
                "explicit anchor lookups must use supported native anchor records",
            ));
        }
        let name = format!("SFDLookup{index}");
        prefix.push_str(&format!("lookup {name} {{\n"));
        if lookup.flags != 0 {
            prefix.push_str(&format!("lookupflag {};\n", lookup.flags));
        }
        for table in &lookup.subtables {
            if let Some(rules) = rules.get(table) {
                for rule in rules {
                    prefix.push_str(rule);
                    prefix.push('\n');
                }
            }
        }
        prefix.push_str(&format!("}} {name};\n"));
        for (feature, scripts) in &lookup.features {
            let code = features.entry(feature.clone()).or_default();
            for (script, languages) in scripts {
                code.push_str(&format!("script {};\n", script.trim()));
                for language in languages {
                    code.push_str(&format!(
                        "language {} exclude_dflt;\nlookup {name};\n",
                        language.trim()
                    ));
                }
            }
        }
    }
    if !prefix.is_empty() {
        font.features.prefixes.push(FeatureBlock {
            tag: "SFD native lookups".into(),
            code: prefix,
            ..Default::default()
        });
    }
    for (tag, code) in features {
        font.features.features.push(FeatureBlock {
            tag,
            code,
            ..Default::default()
        });
    }
    Ok(())
}

pub(super) fn export(font: &Font) -> Result<Output, Error> {
    let source = font.features.to_fea();
    if source.len() > 8 * 1024 * 1024 {
        return Err(error("feature source exceeds 8 MiB limit"));
    }
    let tokens = fea_tokens(&source)?;
    let mut cursor = Cursor::new(tokens);
    let mut out = Output::default();
    let mut named = BTreeMap::<String, Vec<usize>>::new();
    let mut classes = BTreeMap::<String, Vec<String>>::new();
    let mut systems = Vec::new();
    while let Some(token) = cursor.peek() {
        match token {
            "languagesystem" => {
                cursor.next()?;
                let script = cursor.next()?;
                tag(&script)?;
                let language = language_tag(&cursor.next()?)?;
                systems.push((script, language));
                cursor.eat(";")?;
            }
            "lookup" => {
                cursor.next()?;
                let name = cursor.next()?;
                cursor.eat("{")?;
                let indices = block(
                    font,
                    &mut cursor,
                    &mut out,
                    &classes,
                    None,
                    &named,
                    &systems,
                )?;
                cursor.eat("}")?;
                cursor.eat(&name)?;
                cursor.eat(";")?;
                if named.insert(name, indices).is_some() {
                    return Err(error("duplicate FEA lookup name"));
                }
            }
            "feature" => {
                cursor.next()?;
                let feature = cursor.next()?;
                tag(&feature)?;
                cursor.eat("{")?;
                block(
                    font,
                    &mut cursor,
                    &mut out,
                    &classes,
                    Some(feature.clone()),
                    &named,
                    &systems,
                )?;
                cursor.eat("}")?;
                cursor.eat(&feature)?;
                cursor.eat(";")?;
            }
            token if token.starts_with('@') => {
                let name = cursor.next()?;
                cursor.eat("=")?;
                let members = glyph_set(font, &mut cursor, &classes)?;
                cursor.eat(";")?;
                if classes.insert(name, members).is_some() {
                    return Err(error("duplicate FEA class"));
                }
            }
            _ => {
                return Err(error(format!(
                    "unsupported SFD feature syntax near {token:?}"
                )))
            }
        }
    }
    Ok(out)
}

fn block(
    font: &Font,
    t: &mut Cursor,
    out: &mut Output,
    classes: &BTreeMap<String, Vec<String>>,
    feature: Option<String>,
    named: &BTreeMap<String, Vec<usize>>,
    systems: &[(String, String)],
) -> Result<Vec<usize>, Error> {
    let mut indices = Vec::new();
    let mut script = "DFLT".to_owned();
    let mut language = "dflt".to_owned();
    let mut implicit_systems = true;
    let mut flags = 0;
    let mut current: Option<usize> = None;
    while t.peek() != Some("}") {
        match t.peek() {
            Some("script") if feature.is_some() => {
                t.next()?;
                script = t.next()?;
                tag(&script)?;
                language = "dflt".into();
                implicit_systems = false;
                t.eat(";")?;
                current = None;
            }
            Some("language") if feature.is_some() => {
                t.next()?;
                language = language_tag(&t.next()?)?;
                implicit_systems = false;
                if t.peek() == Some("exclude_dflt") {
                    t.next()?;
                } else if language != "dflt" {
                    return Err(error(
                        "language inheritance requires explicit exclude_dflt for SFD export",
                    ));
                }
                t.eat(";")?;
                current = None;
            }
            Some("lookupflag") => {
                t.next()?;
                flags = integer(&t.next()?)?;
                if flags & !15 != 0 {
                    return Err(error("unsupported lookup flags"));
                }
                t.eat(";")?;
                current = None;
            }
            Some("lookup") if feature.is_some() => {
                t.next()?;
                let name = t.next()?;
                t.eat(";")?;
                let referenced = named
                    .get(&name)
                    .ok_or_else(|| error(format!("undefined lookup {name}")))?;
                for index in referenced {
                    if implicit_systems && !systems.is_empty() {
                        for (script, language) in systems {
                            attach(
                                &mut out.lookups[*index],
                                feature.as_deref(),
                                script,
                                language,
                            );
                        }
                    } else {
                        attach(
                            &mut out.lookups[*index],
                            feature.as_deref(),
                            &script,
                            &language,
                        );
                    }
                }
                current = None;
            }
            Some("sub" | "substitute" | "pos" | "position") => {
                let rule = parse_rule(font, t, classes)?;
                let index = match current {
                    Some(index) if out.lookups[index].kind == rule.0 => index,
                    _ => {
                        let index = out.lookups.len();
                        let name = format!("FontCarver.lookup.{index}");
                        let mut lookup = Lookup {
                            kind: rule.0,
                            flags,
                            name: name.clone(),
                            subtables: vec![name],
                            features: Vec::new(),
                        };
                        if implicit_systems && !systems.is_empty() {
                            for (script, language) in systems {
                                attach(&mut lookup, feature.as_deref(), script, language);
                            }
                        } else {
                            attach(&mut lookup, feature.as_deref(), &script, &language);
                        }
                        out.lookups.push(lookup);
                        indices.push(index);
                        current = Some(index);
                        index
                    }
                };
                let table = strings::quote(&out.lookups[index].subtables[0]);
                for (glyph, key, body) in rule.1 {
                    out.glyph_lines
                        .entry(glyph)
                        .or_default()
                        .push(format!("{key}: {table} {body}\n"));
                }
                if out.glyph_lines.values().map(Vec::len).sum::<usize>() > MAX_ITEMS {
                    return Err(error("layout expansion limit exceeded"));
                }
            }
            token => {
                return Err(error(format!(
                    "unsupported SFD feature rule near {token:?}"
                )))
            }
        }
    }
    Ok(indices)
}

fn attach(lookup: &mut Lookup, feature: Option<&str>, script: &str, language: &str) {
    let Some(feature) = feature else {
        return;
    };
    let item = (
        feature.to_owned(),
        vec![(script.to_owned(), vec![language.to_owned()])],
    );
    if !lookup.features.contains(&item) {
        lookup.features.push(item);
    }
}

type Rules = (i64, Vec<(String, &'static str, String)>);

fn parse_rule(
    font: &Font,
    t: &mut Cursor,
    classes: &BTreeMap<String, Vec<String>>,
) -> Result<Rules, Error> {
    let op = t.next()?;
    let first = glyph_set(font, t, classes)?;
    if op == "pos" || op == "position" {
        if t.peek()
            .is_some_and(|v| v == "<" || v.parse::<f64>().is_ok())
        {
            let value = fea_value(t)?;
            if t.peek() == Some(";") {
                t.next()?;
                return Ok((
                    257,
                    first
                        .into_iter()
                        .map(|g| (g, "Position2", native_value(value)))
                        .collect(),
                ));
            }
            let second = glyph_set(font, t, classes)?;
            let value2 = fea_value(t)?;
            t.eat(";")?;
            return pair_rules(first, second, value, value2);
        }
        let second = glyph_set(font, t, classes)?;
        let value = fea_value(t)?;
        t.eat(";")?;
        return pair_rules(first, second, value, [0.0; 4]);
    }
    if t.peek() == Some("from") {
        t.next()?;
        let alternatives = glyph_set(font, t, classes)?;
        t.eat(";")?;
        return Ok((
            3,
            first
                .into_iter()
                .map(|g| (g, "AlternateSubs2", alternatives.join(" ")))
                .collect(),
        ));
    }
    let mut inputs = vec![first];
    while t.peek() != Some("by") {
        inputs.push(glyph_set(font, t, classes)?);
        if inputs.len() > 64 {
            return Err(error("ligature input limit exceeded"));
        }
    }
    t.eat("by")?;
    let output = glyph_set(font, t, classes)?;
    let mut extra = Vec::new();
    while t.peek() != Some(";") {
        extra.extend(glyph_set(font, t, classes)?);
    }
    t.eat(";")?;
    if inputs.len() > 1 {
        if output.len() != 1 || !extra.is_empty() {
            return Err(error("ligature needs one output glyph"));
        }
        let mut combinations = vec![Vec::<String>::new()];
        for input in inputs {
            if combinations.len().saturating_mul(input.len()) > MAX_ITEMS {
                return Err(error("ligature expansion limit exceeded"));
            }
            combinations = combinations
                .into_iter()
                .flat_map(|prefix| {
                    input.iter().map(move |g| {
                        let mut p = prefix.clone();
                        p.push(g.clone());
                        p
                    })
                })
                .collect();
        }
        return Ok((
            4,
            combinations
                .into_iter()
                .map(|input| (output[0].clone(), "Ligature2", input.join(" ")))
                .collect(),
        ));
    }
    let first = inputs.remove(0);
    if !extra.is_empty() {
        let mut all = output;
        all.extend(extra);
        if first.len() != 1 {
            return Err(error("multiple substitution requires one input"));
        }
        return Ok((2, vec![(first[0].clone(), "MultipleSubs2", all.join(" "))]));
    }
    if output.len() != 1 && output.len() != first.len() {
        return Err(error("mismatched substitution classes"));
    }
    Ok((
        1,
        first
            .into_iter()
            .enumerate()
            .map(|(i, g)| {
                (
                    g,
                    "Substitution2",
                    output[if output.len() == 1 { 0 } else { i }].clone(),
                )
            })
            .collect(),
    ))
}

fn pair_rules(
    first: Vec<String>,
    second: Vec<String>,
    a: [f64; 4],
    b: [f64; 4],
) -> Result<Rules, Error> {
    if first.len().saturating_mul(second.len()) > MAX_ITEMS {
        return Err(error("pair expansion limit exceeded"));
    }
    Ok((
        258,
        first
            .into_iter()
            .flat_map(|left| {
                second.iter().map(move |right| {
                    (
                        left.clone(),
                        "PairPos2",
                        format!("{right} {} {}", native_value(a), native_value(b)),
                    )
                })
            })
            .collect(),
    ))
}

fn native_value(v: [f64; 4]) -> String {
    format!("dx={} dy={} dh={} dv={}", v[0], v[1], v[2], v[3])
}
fn fea_value(t: &mut Cursor) -> Result<[f64; 4], Error> {
    if t.peek() == Some("<") {
        t.next()?;
        let a = [
            number(&t.next()?)?,
            number(&t.next()?)?,
            number(&t.next()?)?,
            number(&t.next()?)?,
        ];
        t.eat(">")?;
        Ok(a)
    } else {
        Ok([0.0, 0.0, number(&t.next()?)?, 0.0])
    }
}

fn glyph_set(
    font: &Font,
    t: &mut Cursor,
    classes: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, Error> {
    let token = t.next()?;
    if token == "[" {
        let mut result = Vec::new();
        while t.peek() != Some("]") {
            let token = t.next()?;
            result.extend(resolve(font, &token, classes)?);
            if result.len() > MAX_ITEMS {
                return Err(error("class expansion limit exceeded"));
            }
        }
        t.eat("]")?;
        if result.is_empty() {
            return Err(error("empty glyph class"));
        }
        Ok(result)
    } else {
        resolve(font, &token, classes)
    }
}
fn resolve(
    font: &Font,
    token: &str,
    classes: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, Error> {
    if token.starts_with('@') {
        return classes
            .get(token)
            .cloned()
            .ok_or_else(|| error(format!("undefined glyph class {token}")));
    }
    let token = token.strip_prefix('\\').unwrap_or(token);
    glyph_token(token)?;
    if font.glyph(token).is_none() {
        return Err(error(format!("unknown feature glyph {token}")));
    }
    Ok(vec![token.into()])
}
pub(super) fn glyph_token(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name
            .chars()
            .any(|c| c.is_whitespace() || "{}[]();'<>\\\"@=#".contains(c))
    {
        Err(error(format!(
            "glyph name cannot be expressed in this SFD layout subset: {name:?}"
        )))
    } else {
        Ok(())
    }
}
fn tag(tag: &str) -> Result<(), Error> {
    if tag.len() != 4
        || !tag.is_ascii()
        || tag.chars().any(|c| !c.is_ascii_alphanumeric() && c != ' ')
    {
        Err(error(format!("invalid layout tag {tag:?}")))
    } else {
        Ok(())
    }
}

fn language_tag(value: &str) -> Result<String, Error> {
    let padded = if value.len() == 3 {
        format!("{value} ")
    } else {
        value.to_owned()
    };
    tag(&padded)?;
    Ok(padded)
}

fn fea_tokens(source: &str) -> Result<Vec<String>, Error> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut comment = false;
    for c in source.chars() {
        if comment {
            if c == '\n' || c == '\r' {
                comment = false;
            } else {
                continue;
            }
        }
        if c == '#' {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            comment = true;
            continue;
        }
        if c.is_whitespace() || "{}[];=<>".contains(c) {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            if !c.is_whitespace() {
                tokens.push(c.to_string());
            }
        } else {
            current.push(c);
        }
        if tokens.len() > MAX_ITEMS {
            return Err(error("feature token limit exceeded"));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

struct Cursor {
    tokens: Vec<String>,
    position: usize,
}
impl Cursor {
    fn new(tokens: Vec<String>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.position).map(String::as_str)
    }
    fn next(&mut self) -> Result<String, Error> {
        let value = self
            .tokens
            .get(self.position)
            .cloned()
            .ok_or_else(|| error("unexpected end of layout record"))?;
        self.position += 1;
        Ok(value)
    }
    fn eat(&mut self, value: &str) -> Result<(), Error> {
        let actual = self.next()?;
        if actual != value {
            Err(error(format!("expected {value:?}, got {actual:?}")))
        } else {
            Ok(())
        }
    }
}
