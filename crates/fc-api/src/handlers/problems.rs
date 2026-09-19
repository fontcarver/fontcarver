//! Snapshot-based, non-mutating document and feature diagnostics.

use std::collections::HashMap;

use fc_doc::problems::{Issue, Severity};
use fc_model::FeaturesSource;

use crate::{Api, ApiError, FindProblemsParams, FindProblemsResult, ProblemIssue, ProblemSeverity};

pub(crate) fn find(api: &Api, params: FindProblemsParams) -> Result<FindProblemsResult, ApiError> {
    let limit = params.limit.unwrap_or(200);
    if !(1..=fc_doc::problems::MAX_ISSUES).contains(&limit) {
        return Err(ApiError::invalid_params("limit must be between 1 and 2000"));
    }
    let (font, generation, handles) = api.session.with_doc(params.doc, |document| {
        let handles: HashMap<_, _> = document
            .font
            .glyphs
            .keys()
            .filter_map(|name| document.glyph_id(name).map(|id| (name.clone(), id)))
            .collect();
        (document.font.clone(), document.generation, handles)
    })?;
    // Expensive scanning and real compilation never hold the session lock.
    let report = fc_doc::problems::scan(&font, limit);
    let structural_errors = report.has_errors;
    let mut result = FindProblemsResult {
        doc: params.doc,
        generation,
        total: report.total,
        truncated: report.truncated,
        has_errors: report.has_errors,
        issues: report
            .issues
            .into_iter()
            .map(|issue| ProblemIssue {
                glyph: issue
                    .glyph
                    .as_ref()
                    .and_then(|name| handles.get(name))
                    .copied(),
                glyph_name: issue.glyph,
                master: issue.master,
                layer: issue.layer,
                contour: issue.contour,
                point: issue.point,
                color_layer: issue.color_layer,
                code: issue.code,
                severity: severity(issue.severity),
                message: issue.message,
            })
            .collect(),
    };
    let (source, settings) = font
        .feature_draft
        .as_ref()
        .map_or((&font.features, &font.feature_settings), |draft| {
            (&draft.source, &draft.settings)
        });
    if font.has_unapplied_feature_draft() {
        append(&mut result, limit, simple("unapplied-feature-draft", Severity::Warning, "Feature source has unapplied changes; exports require applying or discarding the draft."));
    }
    if source.source_of_truth == FeaturesSource::Fea {
        if structural_errors {
            append(
                &mut result,
                limit,
                simple(
                    "feature-validation-skipped",
                    Severity::Info,
                    "Repair structural errors and scan again to validate feature compilation.",
                ),
            );
        } else {
            let generated = fc_compile::generate_feature_source(&font, source, settings);
            result.truncated |= generated
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "feature-generation-diagnostics-limit");
            if fc_compile::generation_is_stale(&font, settings) {
                append(&mut result, limit, simple("stale-generated-features", Severity::Warning, "Automatic feature source predates the current glyph set; review and compile feature source again."));
            }
            // Active source is what export actually compiles. Regenerating an
            // owned block here could hide invalid stored source. A saved draft
            // follows the existing feature editor's candidate semantics.
            let (compile_source, compile_settings) = if font.feature_draft.is_some() {
                (&generated.source, &generated.settings)
            } else {
                (source, settings)
            };
            let validation =
                fc_compile::validate_feature_candidate(&font, compile_source, compile_settings);
            result.truncated |= validation.truncated;
            for diagnostic in generated
                .diagnostics
                .into_iter()
                .chain(validation.diagnostics)
            {
                let mut issue = simple(
                    &diagnostic.code,
                    match diagnostic.severity {
                        fc_compile::FeatureDiagnosticSeverity::Error => Severity::Error,
                        fc_compile::FeatureDiagnosticSeverity::Warning => Severity::Warning,
                        fc_compile::FeatureDiagnosticSeverity::Info => Severity::Info,
                    },
                    &diagnostic.message,
                );
                issue.master = diagnostic.master;
                append(&mut result, limit, issue);
            }
        }
    }
    api.session.with_doc(params.doc, |document| {
        if document.generation != generation {
            Err(ApiError {
                code: "stale_problems".into(),
                message: "Document changed during the scan; run Find problems again.".into(),
            })
        } else {
            Ok(result)
        }
    })?
}

fn simple(code: &str, severity: Severity, message: &str) -> Issue {
    Issue {
        code: code.into(),
        severity,
        message: message.into(),
        glyph: None,
        master: None,
        layer: None,
        color_layer: None,
        contour: None,
        point: None,
    }
}

fn severity(value: Severity) -> ProblemSeverity {
    match value {
        Severity::Error => ProblemSeverity::Error,
        Severity::Warning => ProblemSeverity::Warning,
        Severity::Info => ProblemSeverity::Info,
    }
}

fn append(result: &mut FindProblemsResult, limit: usize, issue: Issue) {
    result.total += 1;
    result.has_errors |= issue.severity == Severity::Error;
    if result.issues.len() == limit {
        result.truncated = true;
    } else {
        result.issues.push(ProblemIssue {
            code: issue.code,
            severity: severity(issue.severity),
            message: issue.message,
            glyph: None,
            glyph_name: issue.glyph,
            master: issue.master,
            layer: issue.layer,
            color_layer: issue.color_layer,
            contour: issue.contour,
            point: issue.point,
        });
    }
}
