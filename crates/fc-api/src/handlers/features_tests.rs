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
