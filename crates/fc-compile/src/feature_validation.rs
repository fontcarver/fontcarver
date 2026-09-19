//! Bounded, file-access-free FEA diagnostics and real candidate compilation.

mod work_budget;

use std::{collections::BTreeMap, ops::Range, path::Path, sync::Arc};

use fc_model::{
    AssembledFea, FeatureGenerationSettings, FeatureSourceId, Features, Font, MasterId,
};
use fea_rs::typed::AstNode;
use fea_rs::{compile::NopVariationInfo, parse::SourceLoadError, GlyphMap, ParseTree};

use crate::{compile_static, CompileOptions};

const MAX_SOURCE_BYTES: usize = 1_048_576;
const MAX_RULES: usize = 50_000;
const MAX_DEPTH: usize = 64;
const MAX_DIAGNOSTICS: usize = 100;
const ROOT: &str = "fontcarver-memory.fea";

/// Diagnostic severity independent of a particular parser implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureDiagnosticSeverity {
    /// The candidate cannot be activated.
    Error,
    /// The candidate compiles but warrants attention.
    Warning,
    /// Informational source feedback.
    Info,
}

/// A half-open UTF-16 range in one source item's editable code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureDiagnosticSpan {
    /// Starting UTF-16 code unit.
    pub start: u32,
    /// Exclusive ending UTF-16 code unit.
    pub end: u32,
}

/// A located, bounded source or whole-compilation diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureDiagnostic {
    /// Severity used to decide whether activation is permitted.
    pub severity: FeatureDiagnosticSeverity,
    /// Stable machine-readable category.
    pub code: String,
    /// Compiler detail suitable for an editor diagnostics pane.
    pub message: String,
    /// Source item identity, absent for whole-font failures.
    pub source_id: Option<FeatureSourceId>,
    /// Item-local UTF-16 span; synthesized wrappers have no editable span.
    pub span: Option<FeatureDiagnosticSpan>,
    /// Whether the location belongs to synthesized wrapper syntax.
    pub generated: bool,
    /// Master whose real compilation failed, if applicable.
    pub master: Option<MasterId>,
}

impl FeatureDiagnostic {
    pub(crate) fn error(code: &str, message: impl Into<String>) -> Self {
        Self {
            severity: FeatureDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            source_id: None,
            span: None,
            generated: false,
            master: None,
        }
    }
}

/// Validation of a frozen model snapshot and candidate source/settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeatureValidation {
    /// At most one hundred structured diagnostics.
    pub diagnostics: Vec<FeatureDiagnostic>,
    /// Additional diagnostics were omitted from this bounded result.
    pub truncated: bool,
    /// Masters successfully compiled with the exact export writer policy.
    pub compiled_masters: Vec<MasterId>,
}

impl FeatureValidation {
    /// Whether all source checks and every master compilation succeeded.
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|d| d.severity == FeatureDiagnosticSeverity::Error)
            && !self.compiled_masters.is_empty()
    }

    fn push(&mut self, diagnostic: FeatureDiagnostic) {
        if self.diagnostics.len() < MAX_DIAGNOSTICS {
            self.diagnostics.push(diagnostic);
        } else {
            self.truncated = true;
            // A full warning list must never hide the first fatal error and
            // accidentally turn a failed candidate into successful activation.
            if diagnostic.severity == FeatureDiagnosticSeverity::Error && !self.has_errors() {
                if let Some(last) = self.diagnostics.last_mut() {
                    *last = diagnostic;
                }
            }
        }
    }

    fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == FeatureDiagnosticSeverity::Error)
    }
}

/// Validate and actually compile a candidate for every master without mutation.
///
/// Callers take a snapshot under their document lock, call this outside the lock,
/// then recheck generation/source identity before committing. This is neither an
/// authority-switch permission nor a reusable proof after any model change.
/// Generation is intentionally separate: this validates the exact materialized
/// source and writer settings passed by the caller. Includes never access files.
pub fn validate_feature_candidate(
    font: &Font,
    source: &Features,
    settings: &FeatureGenerationSettings,
) -> FeatureValidation {
    let assembled = source.assemble_fea();
    let mut result = FeatureValidation::default();
    let tree = parse_source(font, &assembled, &mut result);
    if let Some((tree, glyphs)) = tree.filter(|_| !result.has_errors()) {
        for diagnostic in writer_conflicts(font, settings, &tree, &assembled) {
            result.push(diagnostic);
        }
        let diagnostics = fea_rs::compile::validate::<NopVariationInfo>(&tree, &glyphs, None);
        for diagnostic in diagnostics.diagnostics() {
            result.push(map_diagnostic(diagnostic, &assembled, "feature-semantic"));
        }
    }
    if result.has_errors() {
        return result;
    }
    let mut candidate = font.clone();
    candidate.features = source.clone();
    candidate.feature_settings = settings.clone();
    for master in &font.masters {
        match compile_static(&candidate, &master.id, &CompileOptions::default()) {
            Ok(_) => result.compiled_masters.push(master.id.clone()),
            Err(error) => {
                let mut diagnostic =
                    FeatureDiagnostic::error("feature-compile-failed", error.to_string());
                diagnostic.master = Some(master.id.clone());
                result.push(diagnostic);
            }
        }
    }
    if font.masters.is_empty() {
        result.push(FeatureDiagnostic::error(
            "feature-no-masters",
            "The font has no masters to compile.",
        ));
    }
    result
}

/// Apply source work limits and ownership checks at direct compilation too.
pub(crate) fn check_writer_conflicts(font: &Font) -> Result<(), crate::Error> {
    let assembled = font.features.assemble_fea();
    if assembled.text.trim().is_empty() {
        return Ok(());
    }
    let mut result = FeatureValidation::default();
    let parsed = parse_source(font, &assembled, &mut result);
    if let Some(error) = result
        .diagnostics
        .iter()
        .find(|d| d.severity == FeatureDiagnosticSeverity::Error)
    {
        return Err(crate::Error::InvalidSource(error.message.clone()));
    }
    if let Some((tree, _)) = parsed.filter(|_| !result.has_errors()) {
        if let Some(diagnostic) =
            writer_conflicts(font, &font.feature_settings, &tree, &assembled).first()
        {
            return Err(crate::Error::InvalidSource(diagnostic.message.clone()));
        }
    }
    Ok(())
}

fn writer_conflicts(
    font: &Font,
    settings: &FeatureGenerationSettings,
    tree: &ParseTree,
    assembled: &AssembledFea,
) -> Vec<FeatureDiagnostic> {
    let kern =
        settings.writer_policy.kerning && font.masters.iter().any(|m| !m.kerning_h.0.is_empty());
    let anchors = settings.writer_policy.anchors
        && font
            .glyphs
            .values()
            .any(|glyph| glyph.layers.iter().any(|layer| !layer.anchors.is_empty()));
    let mut features = BTreeMap::<String, (bool, Option<Range<usize>>)>::new();
    for feature in tree
        .typed_root()
        .statements()
        .filter_map(fea_rs::typed::Feature::cast)
    {
        let tag = feature.tag().to_raw().to_string();
        let enabled = match tag.as_str() {
            "kern" | "dist" => kern,
            "mark" | "mkmk" | "abvm" | "blwm" | "curs" => anchors,
            _ => false,
        };
        if !enabled {
            continue;
        }
        let entry = features.entry(tag).or_default();
        entry.0 |= feature.has_insert_marker();
        // Script/language/flags select generated rules; other AST statements
        // author or reference manual rules. Check all blocks sharing a tag so
        // splitting manual rules from their marker cannot bypass ownership.
        let manual = feature
            .iter()
            .skip_while(|item| item.kind() != fea_rs::Kind::LBrace)
            .skip(1)
            .take_while(|item| item.kind() != fea_rs::Kind::RBrace)
            .any(|item| {
                item.as_node().is_some()
                    && !matches!(
                        item.kind(),
                        fea_rs::Kind::ScriptNode
                            | fea_rs::Kind::LanguageNode
                            | fea_rs::Kind::LookupFlagNode
                    )
            });
        if manual {
            entry.1 = Some(feature.range());
        }
    }
    features.into_iter().filter_map(|(tag,(marker,manual))| {
        let range = manual.filter(|_|marker)?;
        Some(locate_diagnostic(FeatureDiagnostic::error("feature-writer-conflict",format!("The {tag} feature combines manual rules and an Automatic Code marker while its model writer is enabled. Disable that model writer or remove the marker before compiling.")), &assembled.text, assembled, range))
    }).collect()
}

pub(crate) fn parse_source(
    font: &Font,
    assembled: &AssembledFea,
    result: &mut FeatureValidation,
) -> Option<(ParseTree, GlyphMap)> {
    if let Some(message) = source_limit(&assembled.text) {
        result.push(FeatureDiagnostic::error("feature-source-limit", message));
        return None;
    }
    let mut names: Vec<&str> = font
        .glyph_order()
        .filter_map(|name| {
            font.glyphs
                .get(name)
                .filter(|glyph| glyph.export)
                .map(|glyph| glyph.name.as_ref())
        })
        .collect();
    if !names.contains(&".notdef") {
        names.insert(0, ".notdef");
    }
    let glyphs = match GlyphMap::new(names) {
        Ok(map) => map,
        Err(error) => {
            result.push(FeatureDiagnostic::error(
                "feature-glyph-limit",
                error.to_string(),
            ));
            return None;
        }
    };
    let text: Arc<str> = assembled.text.clone().into();
    let resolver = move |path: &Path| {
        if path == Path::new(ROOT) {
            Ok(Arc::clone(&text))
        } else {
            Err(SourceLoadError::new(
                path.to_path_buf(),
                "External includes are disabled in the feature editor.",
            ))
        }
    };
    let (tree, diagnostics) =
        match fea_rs::parse::parse_root(ROOT.into(), Some(&glyphs), Box::new(resolver)) {
            Ok(parsed) => parsed,
            Err(error) => {
                result.push(FeatureDiagnostic::error(
                    "feature-parse-failed",
                    error.to_string(),
                ));
                return None;
            }
        };
    let mut includes = false;
    for token in tree
        .root()
        .iter_tokens()
        .filter(|token| token.kind == fea_rs::Kind::IncludeKw)
    {
        includes = true;
        let diagnostic = locate_diagnostic(
            FeatureDiagnostic::error(
                "feature-include-disabled",
                "External includes are disabled in the feature editor.",
            ),
            &assembled.text,
            assembled,
            token.range(),
        );
        result.push(diagnostic);
    }
    for diagnostic in diagnostics.diagnostics() {
        let code = if diagnostic.text().contains("includes are disabled") || includes {
            "feature-include-disabled"
        } else {
            "feature-syntax"
        };
        result.push(map_diagnostic(diagnostic, assembled, code));
    }
    if !result.has_errors() {
        if let Some(range) = work_budget::exceeded(&tree) {
            result.push(locate_diagnostic(
                FeatureDiagnostic::error(
                    "feature-source-limit",
                    "Feature class/rule expansion exceeds the safe work limit (100000 records per class or rule, 1000000 total).",
                ),
                &assembled.text,
                assembled,
                range,
            ));
        }
    }
    Some((tree, glyphs))
}

fn map_diagnostic(
    diagnostic: &fea_rs::Diagnostic,
    assembled: &AssembledFea,
    code: &str,
) -> FeatureDiagnostic {
    let severity = match diagnostic.level {
        fea_rs::Level::Error => FeatureDiagnosticSeverity::Error,
        fea_rs::Level::Warning => FeatureDiagnosticSeverity::Warning,
        fea_rs::Level::Info => FeatureDiagnosticSeverity::Info,
    };
    let mut mapped = FeatureDiagnostic::error(code, diagnostic.text());
    mapped.severity = severity;
    locate_diagnostic(mapped, &assembled.text, assembled, diagnostic.span())
}

fn locate_diagnostic(
    mut diagnostic: FeatureDiagnostic,
    text: &str,
    assembled: &AssembledFea,
    range: Range<usize>,
) -> FeatureDiagnostic {
    if let Some(location) = assembled.locate(range.start) {
        diagnostic.source_id = Some(location.item.clone());
        diagnostic.generated = location.byte_offset.is_none();
        if let Some(start) = location.utf16_offset {
            // Clamp a cross-item diagnostic to its first copied region, so an
            // error in a synthesized closing delimiter never points into a
            // different item's editable code.
            if let Some(mapping) = assembled.mappings.iter().find(|m| {
                m.item == location.item
                    && m.source_range.is_some()
                    && m.range.start <= range.start
                    && range.start <= m.range.end
            }) {
                let mut end = range.end.min(mapping.range.end).max(range.start);
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                diagnostic.span = Some(FeatureDiagnosticSpan {
                    start: start as u32,
                    end: (start + text[range.start..end].encode_utf16().count()) as u32,
                });
            }
        }
    }
    diagnostic
}

fn source_limit(text: &str) -> Option<&'static str> {
    if text.len() > MAX_SOURCE_BYTES {
        return Some("Feature source exceeds the 1 MiB editor limit.");
    }
    let (mut depth, mut rules, mut comment, mut quoted, mut escaped) =
        (0usize, 0usize, false, false, false);
    for c in text.chars() {
        if comment {
            comment = !matches!(c, '\n' | '\r');
            continue;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            quoted = !quoted;
            continue;
        }
        if quoted {
            continue;
        }
        match c {
            '#' => comment = true,
            '{' | '[' | '(' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Some("Feature source exceeds the nesting limit of 64.");
                }
            }
            '}' | ']' | ')' => depth = depth.saturating_sub(1),
            ';' => {
                rules += 1;
                if rules > MAX_RULES {
                    return Some("Feature source exceeds the rule limit of 50000.");
                }
            }
            _ => {}
        }
    }
    None
}
