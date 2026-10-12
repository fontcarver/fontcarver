use std::sync::Arc;

use fc_doc::{Command, Session};
use fc_model::{FeatureBlock, FeatureGenerationSettings, Features, Font, Glyph, Layer};

use super::*;

fn setup() -> (Arc<Session>, Api, FeatureValidationParams) {
    let mut font = Font::new_empty("Feature races", 1000);
    let master = font.default_master().id.clone();
    for name in [".notdef", "A", "A.sc"] {
        let mut glyph = Glyph::new(name);
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(Arc::clone(&session));
    let state = set_draft(
        &api,
        SetFeatureDraftParams {
            doc,
            expected_generation: 0,
            expected_draft_revision: 0,
            source: Features {
                features: vec![FeatureBlock {
                    tag: "smcp".into(),
                    code: "sub A by A.sc;".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            settings: FeatureGenerationSettings::default(),
            token: None,
        },
    )
    .unwrap();
    let params = FeatureValidationParams {
        doc,
        expected_generation: state.generation,
        expected_compile_revision: state.compile_revision,
        expected_draft_revision: state.draft_revision,
    };
    (session, api, params)
}

#[test]
fn glyph_mutation_during_actual_compile_cannot_activate_stale_source() {
    let (session, api, params) = setup();
    let doc = params.doc;
    let error = apply_with(&api, params, |font, params| {
        let result = compile_candidate(font, params);
        assert!(result.valid);
        session
            .with_doc_mut(doc, |document| {
                document.apply(
                    Command::RemoveGlyphs {
                        names: vec!["A.sc".into()],
                    },
                    "Delete substitution target",
                )
            })
            .unwrap()
            .unwrap();
        result
    })
    .unwrap_err();
    assert_eq!(error.code, "stale_features");
    session
        .with_doc(doc, |document| {
            assert!(document.font.features.features.is_empty());
            assert!(document.font.feature_draft.is_some());
            assert_eq!(document.undo_label(), Some("Delete substitution target"));
        })
        .unwrap();
}

#[test]
fn delayed_validation_after_glyph_undo_redo_is_not_current() {
    let (session, api, params) = setup();
    let doc = params.doc;
    let expected_draft = params.expected_draft_revision;
    let error = validate_with(&api, params, |font, params| {
        let result = compile_candidate(font, params);
        assert!(result.valid);
        session
            .with_doc_mut(doc, |document| {
                document
                    .apply(
                        Command::RemoveGlyphs {
                            names: vec!["A.sc".into()],
                        },
                        "Delete target",
                    )
                    .unwrap();
                document.undo().unwrap();
                document.redo().unwrap();
                assert_eq!(document.font.feature_revision, expected_draft);
            })
            .unwrap();
        result
    })
    .unwrap_err();
    assert_eq!(error.code, "stale_features");
}

#[test]
fn closing_during_validation_returns_not_found_and_never_recreates_document() {
    let (session, api, params) = setup();
    let doc = params.doc;
    let error = apply_with(&api, params, |font, params| {
        let result = compile_candidate(font, params);
        assert!(result.valid);
        session.close(doc).unwrap();
        result
    })
    .unwrap_err();
    assert_eq!(error.code, "not_found");
    assert!(session.ids().is_empty());
}

#[test]
fn new_draft_during_validation_is_retained() {
    let (session, api, params) = setup();
    let doc = params.doc;
    let error = apply_with(&api, params, |font, params| {
        let result = compile_candidate(font, params);
        assert!(result.valid);
        session
            .with_doc_mut(doc, |document| {
                let mut source = document.font.feature_draft.as_ref().unwrap().source.clone();
                source.features[0].code = "invalid newer text".into();
                document
                    .apply(
                        Command::SetFeatureDraft {
                            expected_revision: document.font.feature_revision,
                            source,
                            settings: FeatureGenerationSettings::default(),
                        },
                        "Newer draft",
                    )
                    .unwrap();
            })
            .unwrap();
        result
    })
    .unwrap_err();
    assert_eq!(error.code, "stale_features");
    let current = get(&api, DocParams { doc }).unwrap();
    assert_eq!(
        current.draft.unwrap().source.features[0].code,
        "invalid newer text"
    );
}

/// Row 47: the per-entry automatic switch is one checked, undoable transaction.
#[test]
fn freezing_one_entry_keeps_its_text_and_leaves_the_recipe_enabled() {
    let (session, api, params) = setup();
    let doc = params.doc;
    // Hand `smcp` to its recipe: drop the user block and enable small caps.
    let state = set_draft(
        &api,
        SetFeatureDraftParams {
            doc,
            expected_generation: params.expected_generation,
            expected_draft_revision: params.expected_draft_revision,
            source: Features::default(),
            settings: FeatureGenerationSettings {
                small_caps: true,
                stylistic_sets: true,
                ..FeatureGenerationSettings::default()
            },
            token: None,
        },
    )
    .unwrap();
    let applied = apply(
        &api,
        FeatureValidationParams {
            doc,
            expected_generation: state.generation,
            expected_compile_revision: state.compile_revision,
            expected_draft_revision: state.draft_revision,
        },
    )
    .unwrap();
    assert!(applied.applied, "{:?}", applied.validation.diagnostics);
    let state = applied.state;
    let generated = state
        .active_source
        .features
        .iter()
        .find(|block| block.tag == "smcp")
        .expect("generated smcp");
    assert!(generated.automatic);
    let code = generated.code.clone();
    assert!(state
        .active_settings
        .generated
        .iter()
        .any(|record| record.generator == fc_model::FeatureGenerator::SmallCaps));
    // Every recipe name is offered to the editor's per-entry switch.
    assert!(state
        .automatic_entries
        .iter()
        .any(|entry| entry.name == "smcp"));
    assert!(
        state
            .automatic_entries
            .iter()
            .any(|entry| entry.name == "Uppercase"
                && entry.kind == fc_model::GeneratedItemKind::Class)
    );

    let frozen = freeze(
        &api,
        FreezeFeatureParams {
            doc,
            expected_generation: state.generation,
            expected_compile_revision: state.compile_revision,
            expected_draft_revision: state.draft_revision,
            generator: fc_model::FeatureGenerator::SmallCaps,
            entry: Some("smcp".into()),
        },
    )
    .unwrap();
    let draft = frozen.draft.as_ref().expect("freezing writes a draft");
    // The text survives, the entry stops regenerating, the recipe stays on.
    let kept = draft
        .source
        .features
        .iter()
        .find(|block| block.tag == "smcp")
        .expect("frozen smcp");
    assert_eq!(kept.code, code);
    assert!(!kept.automatic);
    assert!(draft.settings.small_caps);
    assert!(draft.settings.entry_is_manual("smcp"));
    assert!(draft.settings.generated.is_empty());

    // One gesture, one undo unit: undo restores the generated entry.
    session
        .with_doc_mut(doc, |document| document.undo().unwrap())
        .unwrap();
    let restored = get(&api, DocParams { doc }).unwrap();
    assert!(restored
        .active_settings
        .generated
        .iter()
        .any(|record| record.generator == fc_model::FeatureGenerator::SmallCaps));
    assert!(restored.draft.is_none());
}

/// Freezing an entry a recipe does not own is refused without mutating anything.
#[test]
fn freezing_an_unowned_entry_is_refused_and_leaves_the_document_intact() {
    let (_session, api, params) = setup();
    let doc = params.doc;
    let before = get(&api, DocParams { doc }).unwrap();
    let error = freeze(
        &api,
        FreezeFeatureParams {
            doc,
            expected_generation: before.generation,
            expected_compile_revision: before.compile_revision,
            expected_draft_revision: before.draft_revision,
            generator: fc_model::FeatureGenerator::SmallCaps,
            entry: Some("smcp".into()),
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "feature-generation-not-generated");
    let after = get(&api, DocParams { doc }).unwrap();
    assert_eq!(after.generation, before.generation);
    assert_eq!(after.draft_revision, before.draft_revision);
    assert_eq!(
        after.draft.map(|draft| draft.source),
        before.draft.map(|draft| draft.source)
    );
}
