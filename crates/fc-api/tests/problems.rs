use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{FeatureBlock, Font, Glyph, Layer};
use serde_json::json;

fn setup() -> (Arc<Session>, Api, fc_doc::DocId) {
    let mut font = Font::new_empty("Problem API", 1000);
    for name in [".notdef", "A"] {
        let mut glyph = Glyph::new(name);
        glyph
            .layers
            .push(Layer::new_master(font.default_master().id.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    (session, api, doc)
}

#[test]
fn report_is_generation_bound_and_does_not_mutate_document() {
    let (session, api, doc) = setup();
    session
        .with_doc_mut(doc, |document| {
            document.font.glyphs.get_mut("A").unwrap().unicodes = vec![0xd800, 0x110000];
        })
        .unwrap();
    let before = session
        .with_doc(doc, |document| {
            (
                document.font.clone(),
                document.generation,
                document.dirty,
                document.undo_len(),
            )
        })
        .unwrap();
    let report = api
        .call("doc.find_problems", json!({"doc":doc,"limit":1}))
        .unwrap();
    assert_eq!(report["generation"], before.1);
    assert_eq!(report["issues"].as_array().unwrap().len(), 1);
    assert_eq!(report["issues"][0]["code"], "invalid-unicode");
    assert_eq!(report["issues"][0]["glyph"], 1);
    assert_eq!(report["issues"][0]["glyph_name"], "A");
    assert_eq!(report["truncated"], true);
    assert_eq!(report["has_errors"], true);
    let after = session
        .with_doc(doc, |document| {
            (
                document.font.clone(),
                document.generation,
                document.dirty,
                document.undo_len(),
            )
        })
        .unwrap();
    assert_eq!(before, after);
}

#[test]
fn validates_feature_source_without_applying_it() {
    let (session, api, doc) = setup();
    session
        .with_doc_mut(doc, |document| {
            document.font.features.features.push(FeatureBlock {
                tag: "liga".into(),
                code: "sub Missing by A;".into(),
                ..FeatureBlock::default()
            });
        })
        .unwrap();
    let report = api.call("doc.find_problems", json!({"doc":doc})).unwrap();
    assert_eq!(report["has_errors"], true, "{report}");
    assert!(
        report["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"].as_str().unwrap().starts_with("feature-")),
        "{report}"
    );
    assert!(!session.with_doc(doc, |document| document.dirty).unwrap());
}

#[test]
fn validates_saved_draft_without_changing_active_features() {
    let (session, api, doc) = setup();
    session
        .with_doc_mut(doc, |document| {
            let mut draft = fc_model::FeatureDraft::default();
            draft.source.features.push(FeatureBlock {
                tag: "liga".into(),
                code: "sub Missing by A;".into(),
                ..FeatureBlock::default()
            });
            document.font.feature_draft = Some(draft);
        })
        .unwrap();
    let before = session
        .with_doc(doc, |document| document.font.clone())
        .unwrap();
    let report = api
        .call("doc.find_problems", json!({"doc":doc,"limit":1}))
        .unwrap();
    assert_eq!(report["has_errors"], true, "{report}");
    assert_eq!(report["truncated"], true);
    assert_eq!(report["issues"][0]["severity"], "warning");
    assert!(report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "unapplied-feature-draft"));
    assert_eq!(
        session
            .with_doc(doc, |document| document.font.clone())
            .unwrap(),
        before
    );
}

#[test]
fn rejects_invalid_limits_and_clean_font_has_no_errors() {
    let (_, api, doc) = setup();
    for limit in [0, 2001] {
        let error = api
            .call("doc.find_problems", json!({"doc":doc,"limit":limit}))
            .unwrap_err();
        assert_eq!(error.code, "invalid_params");
    }
    let report = api.call("doc.find_problems", json!({"doc":doc})).unwrap();
    assert_eq!(report["has_errors"], false, "{report}");
    assert_eq!(report["total"], 0, "{report}");
}

#[test]
fn generator_truncation_is_visible_even_with_a_large_response_limit() {
    let (session, api, doc) = setup();
    session
        .with_doc_mut(doc, |document| {
            document.font.feature_settings.small_caps = true;
            for index in 0..120 {
                let mut glyph = Glyph::new(format!("orphan{index}.sc"));
                glyph
                    .layers
                    .push(Layer::new_master(document.font.default_master().id.clone()));
                document.font.insert_glyph(glyph).unwrap();
            }
        })
        .unwrap();
    let report = api
        .call("doc.find_problems", json!({"doc":doc,"limit":2000}))
        .unwrap();
    assert_eq!(report["truncated"], true, "{report}");
    assert_eq!(report["has_errors"], true);
    assert!(report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "feature-generation-diagnostics-limit"));
}

#[test]
fn active_generated_source_is_validated_exactly_as_stored() {
    let (session, api, doc) = setup();
    session
        .with_doc_mut(doc, |document| {
            let mut glyph = Glyph::new("A.sc");
            glyph
                .layers
                .push(Layer::new_master(document.font.default_master().id.clone()));
            document.font.insert_glyph(glyph).unwrap();
            document.font.feature_settings.small_caps = true;
            let block = FeatureBlock {
                tag: "smcp".into(),
                code: "sub Missing by A.sc;".into(),
                ..FeatureBlock::default()
            };
            document
                .font
                .feature_settings
                .generated
                .push(fc_model::GeneratedFeatureRecord {
                    generator: fc_model::FeatureGenerator::SmallCaps,
                    source_id: block.id.clone(),
                    glyphs: vec!["A".into(), "A.sc".into()],
                });
            document.font.features.features.push(block);
        })
        .unwrap();
    let report = api.call("doc.find_problems", json!({"doc":doc})).unwrap();
    assert_eq!(report["has_errors"], true, "{report}");
    assert!(
        report["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "feature-semantic" || issue["code"] == "feature-syntax"),
        "{report}"
    );
}

#[test]
fn exponential_component_dag_never_reaches_full_compilation() {
    let (session, api, doc) = setup();
    session
        .with_doc_mut(doc, |document| {
            for index in 0..24 {
                let mut glyph = Glyph::new(format!("branch{index}"));
                let mut layer = Layer::new_master(document.font.default_master().id.clone());
                layer.contours.push(fc_model::Contour {
                    closed: true,
                    points: vec![fc_model::Point::default(); 3],
                    ..fc_model::Contour::default()
                });
                if index > 0 {
                    layer.components = vec![
                        fc_model::Component {
                            base: format!("branch{}", index - 1).into(),
                            ..fc_model::Component::default()
                        };
                        2
                    ];
                }
                glyph.layers.push(layer);
                document.font.glyphs.insert(glyph.name.clone(), glyph);
            }
        })
        .unwrap();
    let started = std::time::Instant::now();
    let report = api.call("doc.find_problems", json!({"doc":doc})).unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    assert_eq!(report["has_errors"], true);
    assert!(report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "component-expansion-limit"));
    assert!(report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "feature-validation-skipped"));
}
