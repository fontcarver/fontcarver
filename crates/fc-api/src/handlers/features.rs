//! Snapshot-based feature diagnostics and guarded source activation.

use fc_doc::{Command, Document, EditToken, PreparedFeatureChange};
use fc_model::{FeatureGenerationSettings, Features, FeaturesSource, Font};

use crate::{
    Api, ApiError, AutomaticEntryOption, DiscardFeatureDraftParams, DocId, DocParams,
    FeatureApplyResult, FeatureDiagnostic, FeatureDiagnosticSeverity, FeatureDiagnosticSpan,
    FeatureState, FeatureValidationParams, FeatureValidationResult, FreezeFeatureParams,
    SetFeatureDraftParams,
};

const MAX_DIAGNOSTICS: usize = 200;
const MAX_DRAFT_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn current_state(doc: DocId, document: &Document) -> FeatureState {
    FeatureState {
        doc,
        generation: document.generation,
        compile_revision: document.compile_revision(),
        draft_revision: document.font.feature_revision,
        authority: document.font.features.source_of_truth,
        active_source: document.font.features.clone(),
        active_settings: document.font.feature_settings.clone(),
        draft: document.font.feature_draft.clone(),
        has_unapplied_changes: document.font.has_unapplied_feature_draft(),
        original_layout_available: document.font.layout_conversion.is_some(),
        generation_stale: fc_compile::generation_is_stale(
            &document.font,
            &document.font.feature_settings,
        ),
        automatic_entries: automatic_entries(&document.font.feature_settings),
    }
}

/// Every feature tag and glyph class name a recipe family can materialize.
///
/// Automatic classes are opt-in, so the registry names are offered even when the
/// document has not requested them yet.
fn automatic_entries(settings: &FeatureGenerationSettings) -> Vec<AutomaticEntryOption> {
    let mut entries: Vec<AutomaticEntryOption> = Vec::new();
    for generator in fc_model::FEATURE_GENERATORS {
        let kind = fc_model::family_kind(generator);
        let names = if generator == fc_model::FeatureGenerator::Classes {
            fc_model::automatic_class_registry()
                .iter()
                .map(|name| (*name).to_owned())
                .collect()
        } else {
            fc_model::family_names(generator, settings)
        };
        for name in names {
            if entries.iter().any(|entry| entry.name == name) {
                continue;
            }
            entries.push(AutomaticEntryOption {
                name,
                generator,
                kind,
            });
        }
    }
    entries
}

pub(crate) fn get(api: &Api, params: DocParams) -> Result<FeatureState, ApiError> {
    api.session
        .with_doc(params.doc, |document| current_state(params.doc, document))
        .map_err(ApiError::from)
}

pub(crate) fn check_revisions(
    document: &Document,
    generation: u64,
    compile_revision: Option<u64>,
    draft_revision: u64,
) -> Result<(), ApiError> {
    if document.generation != generation
        || compile_revision.is_some_and(|revision| revision != document.compile_revision())
        || document.font.feature_revision != draft_revision
    {
        return Err(ApiError {
            code: "stale_features".to_owned(),
            message: "feature source or font inputs changed; refresh before retrying".to_owned(),
        });
    }
    Ok(())
}

fn require_source_authority(document: &Document) -> Result<(), ApiError> {
    if document.font.features.source_of_truth != FeaturesSource::Fea {
        return Err(ApiError {
            code: "layout_conversion_required".to_owned(),
            message: "convert the complete imported layout before applying editable source"
                .to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn set_draft(
    api: &Api,
    mut params: SetFeatureDraftParams,
) -> Result<FeatureState, ApiError> {
    // Drafts may contain invalid syntax, but must remain bounded before persistence.
    let bytes = serde_json::to_vec(&(&params.source, &params.settings))
        .map_err(|error| ApiError::invalid_params(error.to_string()))?;
    if bytes.len() > MAX_DRAFT_BYTES {
        return Err(ApiError::invalid_params(
            "feature draft exceeds the 4 MiB limit",
        ));
    }
    api.session
        .with_doc_mut(params.doc, |document| {
            check_revisions(
                document,
                params.expected_generation,
                None,
                params.expected_draft_revision,
            )?;
            // Materialization identities are trusted server state, not editor options.
            let (_, current_settings) = candidate(&document.font);
            params.settings.generated = current_settings.generated.clone();
            params.settings.glyph_fingerprint = current_settings.glyph_fingerprint.clone();
            let command = Command::SetFeatureDraft {
                expected_revision: params.expected_draft_revision,
                source: params.source,
                settings: params.settings,
            };
            if let Some(token) = params.token {
                document.apply_in_edit(&EditToken(token.0), command)?;
            } else {
                document.apply(command, "Edit feature source")?;
            }
            Ok(current_state(params.doc, document))
        })
        .map_err(ApiError::from)?
}

pub(crate) fn discard_draft(
    api: &Api,
    params: DiscardFeatureDraftParams,
) -> Result<FeatureState, ApiError> {
    api.session
        .with_doc_mut(params.doc, |document| {
            check_revisions(
                document,
                params.expected_generation,
                None,
                params.expected_draft_revision,
            )?;
            document.apply(
                Command::DiscardFeatureDraft {
                    expected_revision: params.expected_draft_revision,
                },
                "Discard feature draft",
            )?;
            Ok(current_state(params.doc, document))
        })
        .map_err(ApiError::from)?
}

pub(crate) fn freeze(api: &Api, params: FreezeFeatureParams) -> Result<FeatureState, ApiError> {
    api.session
        .with_doc_mut(params.doc, |document| {
            check_revisions(
                document,
                params.expected_generation,
                Some(params.expected_compile_revision),
                params.expected_draft_revision,
            )?;
            require_source_authority(document)?;
            let (source, settings) = candidate(&document.font);
            let frozen = fc_compile::freeze_feature_source(
                source,
                settings,
                params.generator,
                params.entry.as_deref(),
            );
            if let Some(diagnostic) = frozen.diagnostics.first() {
                return Err(ApiError {
                    code: diagnostic.code.clone(),
                    message: diagnostic.message.clone(),
                });
            }
            document.apply(
                Command::SetFeatureDraft {
                    expected_revision: params.expected_draft_revision,
                    source: frozen.source,
                    settings: frozen.settings,
                },
                "Freeze automatic feature source",
            )?;
            Ok(current_state(params.doc, document))
        })
        .map_err(ApiError::from)?
}

pub(crate) fn validate(
    api: &Api,
    params: FeatureValidationParams,
) -> Result<FeatureValidationResult, ApiError> {
    validate_with(api, params, compile_candidate)
}

fn validate_with(
    api: &Api,
    params: FeatureValidationParams,
    compile: impl FnOnce(&Font, &FeatureValidationParams) -> FeatureValidationResult,
) -> Result<FeatureValidationResult, ApiError> {
    let font = snapshot(api, &params)?;
    let validation = compile(&font, &params);
    api.session
        .with_doc(params.doc, |document| {
            check_request(document, &params)?;
            Ok(validation)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn apply(
    api: &Api,
    params: FeatureValidationParams,
) -> Result<FeatureApplyResult, ApiError> {
    apply_with(api, params, compile_candidate)
}

fn apply_with(
    api: &Api,
    params: FeatureValidationParams,
    compile: impl FnOnce(&Font, &FeatureValidationParams) -> FeatureValidationResult,
) -> Result<FeatureApplyResult, ApiError> {
    let font = snapshot(api, &params)?;
    // Expensive generation, parsing and actual fontc compilation are deliberately
    // outside the session lock; concurrent edits are rejected at the commit point.
    let validation = compile(&font, &params);
    api.session
        .with_doc_mut(params.doc, |document| {
            check_request(document, &params)?;
            if validation.valid {
                let prepared = PreparedFeatureChange::new(
                    validation.generated_source.clone(),
                    validation.generated_settings.clone(),
                );
                document.commit_prepared_features(
                    params.expected_generation,
                    params.expected_compile_revision,
                    prepared,
                    "Compile feature source",
                )?;
            }
            Ok(FeatureApplyResult {
                applied: validation.valid,
                validation,
                state: current_state(params.doc, document),
            })
        })
        .map_err(ApiError::from)?
}

fn snapshot(api: &Api, params: &FeatureValidationParams) -> Result<Font, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            check_request(document, params)?;
            Ok(document.font.clone())
        })
        .map_err(ApiError::from)?
}

fn check_request(document: &Document, params: &FeatureValidationParams) -> Result<(), ApiError> {
    check_revisions(
        document,
        params.expected_generation,
        Some(params.expected_compile_revision),
        params.expected_draft_revision,
    )?;
    require_source_authority(document)?;
    if candidate(&document.font).0.source_of_truth != FeaturesSource::Fea {
        return Err(ApiError::invalid_params(
            "saved feature draft must contain editable FEA source",
        ));
    }
    Ok(())
}

fn candidate(font: &Font) -> (&Features, &FeatureGenerationSettings) {
    font.feature_draft
        .as_ref()
        .map_or((&font.features, &font.feature_settings), |draft| {
            (&draft.source, &draft.settings)
        })
}

fn compile_candidate(font: &Font, params: &FeatureValidationParams) -> FeatureValidationResult {
    let (source, settings) = candidate(font);
    let generated = fc_compile::generate_feature_source(font, source, settings);
    let validation =
        fc_compile::validate_feature_candidate(font, &generated.source, &generated.settings);
    let valid = validation.is_valid()
        && !generated.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.severity,
                fc_compile::FeatureDiagnosticSeverity::Error
            )
        });
    let mut diagnostics = generated.diagnostics;
    diagnostics.extend(validation.diagnostics);
    let truncated = validation.truncated
        || diagnostics.len() > MAX_DIAGNOSTICS
        || diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "feature-generation-diagnostics-limit"
                || diagnostic.message.chars().count() > 4096
        });
    diagnostics.truncate(MAX_DIAGNOSTICS);
    FeatureValidationResult {
        doc: params.doc,
        generation: params.expected_generation,
        compile_revision: params.expected_compile_revision,
        draft_revision: params.expected_draft_revision,
        valid,
        diagnostics: diagnostics.into_iter().map(diagnostic_view).collect(),
        truncated,
        compiled_masters: validation.compiled_masters,
        generated_source: generated.source,
        generated_settings: generated.settings,
    }
}

fn diagnostic_view(value: fc_compile::FeatureDiagnostic) -> FeatureDiagnostic {
    FeatureDiagnostic {
        severity: match value.severity {
            fc_compile::FeatureDiagnosticSeverity::Error => FeatureDiagnosticSeverity::Error,
            fc_compile::FeatureDiagnosticSeverity::Warning => FeatureDiagnosticSeverity::Warning,
            fc_compile::FeatureDiagnosticSeverity::Info => FeatureDiagnosticSeverity::Info,
        },
        code: value.code,
        message: value.message.chars().take(4096).collect(),
        source_id: value.source_id,
        span: value.span.map(|span| FeatureDiagnosticSpan {
            start: span.start,
            end: span.end,
        }),
        generated: value.generated,
        master: value.master,
    }
}

#[cfg(test)]
#[path = "features_tests.rs"]
mod tests;
