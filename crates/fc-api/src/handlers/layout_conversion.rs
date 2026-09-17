//! Server-held proof tokens and atomic imported-layout ownership transitions.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use fc_doc::{Change, DocId, Document, PreparedFeatureChange};
use fc_layout::{
    conversion::{self, ProvenConversion},
    LayoutImport,
};
use fc_model::{FeatureSourceId, FeaturesSource, Font, MasterId};

use crate::{
    Api, ApiError, ApplyLayoutConversionParams, LayoutConversionPreview,
    PrepareLayoutConversionParams, RevertOriginalLayoutParams,
};

const TOKEN_SECONDS: u32 = 300;
const MAX_TOKENS: usize = 16;
const MAX_MESSAGES: usize = 16;
const MAX_MESSAGE_CHARS: usize = 4096;

/// Bounded process-local proofs; never part of document serialization.
#[derive(Default)]
pub(crate) struct ConversionTokens {
    entries: VecDeque<Token>,
}

struct Token {
    id: String,
    created: Instant,
    identity: Identity,
    proof: ProvenConversion,
}

#[derive(Clone)]
struct Identity {
    doc: DocId,
    master: MasterId,
    generation: u64,
    compile_revision: u64,
    draft_revision: u64,
    fingerprint: String,
}

impl ConversionTokens {
    pub(crate) fn remove_doc(&mut self, doc: DocId) {
        self.entries.retain(|token| token.identity.doc != doc);
    }

    fn prune(&mut self) {
        self.entries.retain(|token| {
            token.created.elapsed() < Duration::from_secs(u64::from(TOKEN_SECONDS))
        });
    }

    fn insert(&mut self, identity: Identity, proof: ProvenConversion) -> String {
        self.prune();
        self.remove_doc(identity.doc);
        while self.entries.len() >= MAX_TOKENS {
            self.entries.pop_front();
        }
        let id = FeatureSourceId::default().0;
        self.entries.push_back(Token {
            id: id.clone(),
            created: Instant::now(),
            identity,
            proof,
        });
        id
    }

    fn take(&mut self, doc: DocId, id: &str) -> Result<Token, ApiError> {
        self.prune();
        let position = self.entries.iter().position(|token| token.id == id && token.identity.doc == doc)
            .ok_or_else(|| error("invalid_conversion_token", "Conversion token is missing, expired, already used, or belongs to another document"))?;
        self.entries
            .remove(position)
            .ok_or_else(|| ApiError::internal("conversion token disappeared"))
    }
}

pub(crate) fn prepare_conversion(
    api: &Api,
    params: PrepareLayoutConversionParams,
) -> Result<LayoutConversionPreview, ApiError> {
    let (font, mut identity) = api.session.with_doc(params.doc, |document| {
        check_revisions(
            document,
            params.expected_generation,
            params.expected_compile_revision,
            params.expected_draft_revision,
        )?;
        let master = params
            .master
            .clone()
            .unwrap_or_else(|| document.font.default_master().id.clone());
        if document.font.master(&master).is_none() {
            return Err(ApiError::not_found("conversion master does not exist"));
        }
        Ok((
            document.font.clone(),
            Identity {
                doc: params.doc,
                master,
                generation: document.generation,
                compile_revision: document.compile_revision(),
                draft_revision: document.font.feature_revision,
                fingerprint: String::new(),
            },
        ))
    })??;
    identity.fingerprint = LayoutImport::fingerprint_for_font(&font);
    // Parsing, decompilation, every compiler run and shape proof are outside
    // the session lock. Only the exact immutable result can enter the token.
    let result = conversion::prepare_conversion(&font, params.prefer_model_lift);
    let mut preview = LayoutConversionPreview {
        doc: params.doc,
        master: identity.master.clone(),
        generation: identity.generation,
        compile_revision: identity.compile_revision,
        draft_revision: identity.draft_revision,
        fingerprint: identity.fingerprint.clone(),
        token: None,
        expires_in_seconds: 0,
        source: None,
        kerning_lifted: false,
        anchors_lifted: false,
        messages: Vec::new(),
        blockers: Vec::new(),
    };
    // Keep the identity check and token insertion under one document read lock;
    // an edit cannot slip between the check and token publication.
    api.session.with_doc(params.doc, |document| {
        check_identity(document, &identity)?;
        match result {
            Ok(proof) => {
                preview.source = Some(proof.source.clone());
                preview.kerning_lifted = proof.kerning_lifted;
                preview.anchors_lifted = proof.anchors_lifted;
                preview.messages = bounded_messages(proof.messages.clone());
                preview.token = Some(
                    api.conversion_tokens
                        .lock()
                        .map_err(|_| ApiError::internal("conversion token lock is poisoned"))?
                        .insert(identity, proof),
                );
                preview.expires_in_seconds = TOKEN_SECONDS;
            }
            Err(error) => preview.blockers = bounded_messages(vec![error.to_string()]),
        }
        Ok::<_, ApiError>(())
    })??;
    Ok(preview)
}

pub(crate) fn apply_conversion(
    api: &Api,
    params: ApplyLayoutConversionParams,
) -> Result<Change, ApiError> {
    let token = api
        .conversion_tokens
        .lock()
        .map_err(|_| ApiError::internal("conversion token lock is poisoned"))?
        .take(params.doc, &params.token)?;
    api.session.with_doc_mut(params.doc, |document| {
        check_identity(document, &token.identity)?;
        if document.font.features.source_of_truth != FeaturesSource::ImportedTables
            || document.font.feature_draft.is_some()
        {
            return Err(stale(
                "active authority or draft changed after conversion proof",
            ));
        }
        let proof = token.proof;
        let mut prepared = PreparedFeatureChange::new(proof.source, proof.settings)
            .with_conversion(Some(proof.backup))
            .with_anchors(proof.anchors);
        if let Some(kerning) = proof.kerning {
            prepared = prepared.with_kerning(kerning);
        }
        document
            .commit_prepared_features(
                token.identity.generation,
                token.identity.compile_revision,
                prepared,
                "Convert imported layout",
            )
            .map_err(ApiError::from)
    })?
}

pub(crate) fn revert_original(
    api: &Api,
    params: RevertOriginalLayoutParams,
) -> Result<Change, ApiError> {
    let font: Font = api.session.with_doc(params.doc, |document| {
        check_revisions(
            document,
            params.expected_generation,
            params.expected_compile_revision,
            params.expected_draft_revision,
        )?;
        Ok::<_, ApiError>(document.font.clone())
    })??;
    let backup = conversion::revert_eligibility(&font)
        .map_err(|error| error_code("layout_revert_unavailable", error.to_string()))?
        .clone();
    let fingerprint = backup.baseline_fingerprint.clone();
    let mut prepared =
        PreparedFeatureChange::new(backup.original_features, backup.original_settings)
            .with_conversion(None)
            .with_anchors(backup.lifted_anchors);
    if let Some(kerning) = backup.lifted_kerning {
        prepared = prepared.with_kerning(kerning);
    }
    api.conversion_tokens
        .lock()
        .map_err(|_| ApiError::internal("conversion token lock is poisoned"))?
        .remove_doc(params.doc);
    api.session.with_doc_mut(params.doc, |document| {
        check_revisions(
            document,
            params.expected_generation,
            params.expected_compile_revision,
            params.expected_draft_revision,
        )?;
        if LayoutImport::fingerprint_for_font(&document.font) != fingerprint {
            return Err(stale("original layout provenance changed during revert"));
        }
        document
            .commit_prepared_features(
                params.expected_generation,
                params.expected_compile_revision,
                prepared,
                "Restore original layout",
            )
            .map_err(ApiError::from)
    })?
}

fn check_identity(document: &Document, identity: &Identity) -> Result<(), ApiError> {
    check_revisions(
        document,
        identity.generation,
        identity.compile_revision,
        identity.draft_revision,
    )?;
    if document.font.master(&identity.master).is_none()
        || LayoutImport::fingerprint_for_font(&document.font) != identity.fingerprint
    {
        return Err(stale(
            "original layout or master changed after conversion proof",
        ));
    }
    Ok(())
}

fn check_revisions(
    document: &Document,
    generation: u64,
    compile_revision: u64,
    draft_revision: u64,
) -> Result<(), ApiError> {
    if document.generation != generation
        || document.compile_revision() != compile_revision
        || document.font.feature_revision != draft_revision
    {
        return Err(stale(
            "document or compile inputs changed; prepare conversion again",
        ));
    }
    Ok(())
}

fn error_code(code: &str, message: String) -> ApiError {
    ApiError {
        code: code.into(),
        message,
    }
}
fn error(code: &str, message: &str) -> ApiError {
    error_code(code, message.into())
}

fn stale(message: &str) -> ApiError {
    error("stale_layout_conversion", message)
}

fn bounded_messages(messages: Vec<String>) -> Vec<String> {
    const SUFFIX: &str = "… (truncated)";
    let omitted = messages.len() > MAX_MESSAGES;
    let count = if omitted {
        MAX_MESSAGES - 1
    } else {
        MAX_MESSAGES
    };
    let mut result: Vec<_> = messages
        .into_iter()
        .take(count)
        .map(|message| {
            let mut chars = message.chars();
            let mut bounded: String = chars
                .by_ref()
                .take(MAX_MESSAGE_CHARS - SUFFIX.chars().count())
                .collect();
            if chars.next().is_some() {
                bounded.push_str(SUFFIX);
            }
            bounded
        })
        .collect();
    if omitted {
        result.push("Additional conversion details were truncated".into());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use fc_model::{FeatureGenerationSettings, Features, LayoutConversion};

    fn proof() -> ProvenConversion {
        ProvenConversion {
            source: Features::default(),
            settings: FeatureGenerationSettings::default(),
            backup: LayoutConversion::default(),
            kerning: None,
            anchors: Vec::new(),
            kerning_lifted: false,
            anchors_lifted: false,
            messages: Vec::new(),
        }
    }

    fn identity(doc: u32) -> Identity {
        Identity {
            doc: DocId(doc),
            master: MasterId::default(),
            generation: 0,
            compile_revision: 0,
            draft_revision: 0,
            fingerprint: String::new(),
        }
    }

    #[test]
    fn tokens_are_expiring_bounded_and_document_scoped() {
        let mut tokens = ConversionTokens::default();
        let first = tokens.insert(identity(0), proof());
        assert!(tokens.take(DocId(1), &first).is_err());
        assert_eq!(tokens.entries.len(), 1);
        tokens.entries[0].created = Instant::now() - Duration::from_secs(301);
        assert!(tokens.take(DocId(0), &first).is_err());
        for doc in 0..20 {
            tokens.insert(identity(doc), proof());
        }
        assert_eq!(tokens.entries.len(), MAX_TOKENS);
        tokens.remove_doc(DocId(19));
        assert_eq!(tokens.entries.len(), MAX_TOKENS - 1);
        tokens.insert(identity(18), proof());
        assert_eq!(tokens.entries.len(), MAX_TOKENS - 1);
    }

    #[test]
    fn preview_details_are_bounded_and_report_unicode_safe_truncation() {
        let details = bounded_messages(vec!["诊断".repeat(10_000); 100]);
        assert_eq!(details.len(), MAX_MESSAGES);
        assert!(details
            .iter()
            .all(|detail| detail.chars().count() <= MAX_MESSAGE_CHARS));
        assert!(details[0].ends_with("… (truncated)"));
        assert!(details.last().unwrap().contains("truncated"));
        assert_eq!(
            bounded_messages(vec!["Short detail".into()]),
            ["Short detail"]
        );
    }
}
