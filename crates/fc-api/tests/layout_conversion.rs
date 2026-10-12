use std::sync::Arc;

use fc_api::Api;
use fc_doc::{DocId, Session};
use fc_model::{FeatureBlock, FeaturesSource, Font, Glyph, Layer, Tag};
use serde_json::{json, Value};

fn imported() -> Font {
    let mut font = Font::new_empty("Conversion API", 1000);
    let master = font.default_master().id.clone();
    for (name, unicode) in [
        (".notdef", None),
        ("f", Some(102)),
        ("i", Some(105)),
        ("f_i", None),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 500.0;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.features.features.push(FeatureBlock {
        tag: "liga".into(),
        code: "sub f i by f_i;".into(),
        ..Default::default()
    });
    let bytes = fc_compile::export(&font, &|_| 0, &Default::default())
        .unwrap()
        .bytes;
    fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0)
}

fn setup(font: Font) -> (Arc<Session>, Api, DocId) {
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/p2b-conversion-test-config")
            .join(fc_model::FeatureSourceId::default().0),
    )
    .unwrap();
    (session, api, doc)
}

fn revisions(session: &Session, doc: DocId) -> Value {
    session
        .with_doc(doc, |document| {
            json!({
                "doc":doc, "expected_generation":document.generation,
                "expected_compile_revision":document.compile_revision(),
                "expected_draft_revision":document.font.feature_revision,
                "prefer_model_lift":false
            })
        })
        .unwrap()
}

fn prepare(api: &Api, session: &Session, doc: DocId) -> Value {
    api.call("layout.prepare_conversion", revisions(session, doc))
        .unwrap()
}

#[test]
fn prepare_is_read_only_and_exact_conversion_is_one_undo_step() {
    let font = imported();
    let (session, api, doc) = setup(font.clone());
    let preview = prepare(&api, &session, doc);
    assert!(preview["token"].is_string(), "{preview}");
    assert_eq!(preview["blockers"], json!([]));
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font, font);
            assert!(!document.dirty);
            assert_eq!(document.undo_len(), 0);
        })
        .unwrap();
    let accepted = api
        .call(
            "layout.apply_conversion",
            json!({"doc":doc,"token":preview["token"]}),
        )
        .unwrap();
    assert_eq!(accepted["layout"], true);
    assert_eq!(accepted["glyphs"], json!([]));
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.features.source_of_truth, FeaturesSource::Fea);
            assert_eq!(
                serde_json::to_value(&document.font.features).unwrap(),
                preview["source"]
            );
            assert!(document.font.layout_conversion.is_some());
            assert_eq!(document.undo_len(), 1);
            for name in document.font.glyph_order() {
                assert_eq!(
                    document.glyph_generation(document.glyph_id(name.as_ref()).unwrap()),
                    0
                );
            }
        })
        .unwrap();
    assert!(api
        .call(
            "layout.apply_conversion",
            json!({"doc":doc,"token":preview["token"]})
        )
        .is_err());
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(
        session
            .with_doc(doc, |document| document.font.features.source_of_truth)
            .unwrap(),
        FeaturesSource::ImportedTables
    );
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    assert_eq!(
        session
            .with_doc(doc, |document| document.font.features.source_of_truth)
            .unwrap(),
        FeaturesSource::Fea
    );
}

#[test]
fn forged_cross_document_and_stale_tokens_cannot_change_authority() {
    let (session, api, doc) = setup(imported());
    let other = session.open_font(imported(), None);
    assert_eq!(
        api.call(
            "layout.apply_conversion",
            json!({"doc":doc,"token":"forged"})
        )
        .unwrap_err()
        .code,
        "invalid_conversion_token"
    );
    let preview = prepare(&api, &session, doc);
    assert_eq!(
        api.call(
            "layout.apply_conversion",
            json!({"doc":other,"token":preview["token"]})
        )
        .unwrap_err()
        .code,
        "invalid_conversion_token"
    );
    let layer = session
        .with_doc(doc, |document| {
            document.font.glyph("f").unwrap().layers[0].id.clone()
        })
        .unwrap();
    api.call("doc.apply", json!({"doc":doc,"label":"width","command":{"SetWidth":{"glyph":"f","layer":layer,"width":600}}})).unwrap();
    assert_eq!(
        api.call(
            "layout.apply_conversion",
            json!({"doc":doc,"token":preview["token"]})
        )
        .unwrap_err()
        .code,
        "stale_layout_conversion"
    );
    assert_eq!(
        session
            .with_doc(doc, |document| document.font.features.source_of_truth)
            .unwrap(),
        FeaturesSource::ImportedTables
    );
}

#[test]
fn unsupported_competitor_blocks_without_dirty_or_undo() {
    let mut font = imported();
    font.extra
        .raw_tables
        .insert(Tag(*b"JSTF"), vec![0, 1, 0, 0]);
    let (session, api, doc) = setup(font);
    let preview = prepare(&api, &session, doc);
    assert!(preview["token"].is_null());
    assert!(!preview["blockers"].as_array().unwrap().is_empty());
    session
        .with_doc(doc, |document| {
            assert!(!document.dirty);
            assert_eq!(document.undo_len(), 0);
            assert_eq!(
                document.font.features.source_of_truth,
                FeaturesSource::ImportedTables
            );
        })
        .unwrap();
}

#[test]
fn native_reopen_revert_retains_other_edits_and_undo_restores_the_draft() {
    let (session, api, doc) = setup(imported());
    let preview = prepare(&api, &session, doc);
    api.call(
        "layout.apply_conversion",
        json!({"doc":doc,"token":preview["token"]}),
    )
    .unwrap();
    let layer = session
        .with_doc(doc, |document| {
            document.font.glyph("f").unwrap().layers[0].id.clone()
        })
        .unwrap();
    api.call("doc.apply", json!({"doc":doc,"label":"width","command":{"SetWidth":{"glyph":"f","layer":layer,"width":650}}})).unwrap();
    let state = api.call("features.get", json!({"doc":doc})).unwrap();
    api.call("features.set_draft", json!({
        "doc":doc,"expected_generation":state["generation"],"expected_draft_revision":state["draft_revision"],
        "source":{"prefixes":[{"id":"repair","tag":"Repair","code":"invalid text;"}]},"settings":state["active_settings"]
    })).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("converted.fcf");
    api.call("session.save", json!({"doc":doc,"path":path}))
        .unwrap();
    api.call("session.close", json!({"doc":doc})).unwrap();
    let reopened: DocId = serde_json::from_value(
        api.call("session.open", json!({"path":path})).unwrap()["doc"].clone(),
    )
    .unwrap();
    let before = session
        .with_doc(reopened, |document| document.font.clone())
        .unwrap();
    assert!(before
        .extra
        .source
        .as_ref()
        .unwrap()
        .original_bytes
        .is_none());
    assert!(before.feature_draft.is_some());
    let change = api
        .call("layout.revert_original", revisions(&session, reopened))
        .unwrap();
    assert_eq!(change["glyphs"], json!([]));
    session
        .with_doc(reopened, |document| {
            assert_eq!(
                document.font.features.source_of_truth,
                FeaturesSource::ImportedTables
            );
            assert!(document.font.feature_draft.is_none());
            assert!(document.font.layout_conversion.is_none());
            assert_eq!(document.font.glyph("f").unwrap().layers[0].width, 650.0);
            assert_eq!(document.font.extra.raw_tables, before.extra.raw_tables);
            assert_eq!(
                document.glyph_generation(document.glyph_id("f").unwrap()),
                0
            );
        })
        .unwrap();
    api.call("doc.undo", json!({"doc":reopened})).unwrap();
    session
        .with_doc(reopened, |document| {
            assert_eq!(document.font.features, before.features);
            assert_eq!(
                document.font.feature_draft.as_ref().unwrap().source,
                before.feature_draft.as_ref().unwrap().source
            );
            assert_eq!(document.font.layout_conversion, before.layout_conversion);
            assert_eq!(document.font.glyph("f").unwrap().layers[0].width, 650.0);
        })
        .unwrap();
}

#[test]
fn changed_mapping_missing_tables_or_corrupt_provenance_blocks_revert_without_mutation() {
    for corruption in 0..3 {
        let (session, api, doc) = setup(imported());
        let preview = prepare(&api, &session, doc);
        api.call(
            "layout.apply_conversion",
            json!({"doc":doc,"token":preview["token"]}),
        )
        .unwrap();
        session
            .with_doc_mut(doc, |document| match corruption {
                0 => document.font.glyph_mut("f").unwrap().export = false,
                1 => {
                    document.font.extra.raw_tables.remove(&Tag(*b"GSUB"));
                }
                _ => {
                    let source = document.font.extra.source.as_mut().unwrap();
                    assert!(source.original_bytes.is_some());
                    source.sha256 = "not-the-original-digest".into();
                }
            })
            .unwrap();
        let before = session
            .with_doc(doc, |document| (document.font.clone(), document.undo_len()))
            .unwrap();
        assert_eq!(
            api.call("layout.revert_original", revisions(&session, doc))
                .unwrap_err()
                .code,
            "layout_revert_unavailable"
        );
        assert_eq!(
            session
                .with_doc(doc, |document| (document.font.clone(), document.undo_len()))
                .unwrap(),
            before
        );
    }
}

#[test]
fn corrupt_original_digest_cannot_turn_the_shape_proof_into_a_self_comparison() {
    let mut font = imported();
    font.extra.source.as_mut().unwrap().sha256 = "mismatched-original-bytes".into();
    let (session, api, doc) = setup(font);
    let preview = prepare(&api, &session, doc);
    assert!(preview["token"].is_null(), "{preview}");
    assert!(
        preview["blockers"]
            .to_string()
            .contains("provenance digest"),
        "{preview}"
    );
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        0
    );
}

#[test]
fn close_reopen_invalidates_capabilities_and_compiler_errors_do_not_publish_them() {
    let (session, api, doc) = setup(imported());
    let preview = prepare(&api, &session, doc);
    let font = session
        .with_doc(doc, |document| document.font.clone())
        .unwrap();
    api.call("session.close", json!({"doc":doc})).unwrap();
    let reopened = session.open_font(font, None);
    assert_eq!(
        api.call(
            "layout.apply_conversion",
            json!({"doc":reopened,"token":preview["token"]})
        )
        .unwrap_err()
        .code,
        "invalid_conversion_token"
    );
    session
        .with_doc_mut(reopened, |document| document.font.info.units_per_em = 1)
        .unwrap();
    let rejected = prepare(&api, &session, reopened);
    assert!(rejected["token"].is_null(), "{rejected}");
    assert!(
        rejected["blockers"].to_string().contains("units per em"),
        "{rejected}"
    );
    session
        .with_doc(reopened, |document| {
            assert!(!document.dirty);
            assert_eq!(document.undo_len(), 0);
        })
        .unwrap();
}

fn imported_model_layout() -> Font {
    let mut font = Font::new_empty("Writer conversion", 1000);
    for (name, unicode) in [
        (".notdef", None),
        ("A", Some(65)),
        ("V", Some(86)),
        ("acutecomb", Some(0x301)),
        ("gravecomb", Some(0x300)),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        font.insert_glyph(glyph).unwrap();
    }
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].width = if glyph.name.ends_with("comb") {
            0.0
        } else {
            600.0
        };
    }
    font.masters[0].kerning_h.0.insert(
        (
            fc_model::KernSide::Glyph("A".into()),
            fc_model::KernSide::Glyph("V".into()),
        ),
        -80.0,
    );
    font.glyph_mut("A").unwrap().layers[0]
        .anchors
        .push(fc_model::Anchor {
            name: "top".into(),
            x: 300.0,
            y: 700.0,
        });
    for name in ["acutecomb", "gravecomb"] {
        font.glyph_mut(name).unwrap().layers[0].anchors = vec![
            fc_model::Anchor {
                name: "_top".into(),
                x: 0.0,
                y: 0.0,
            },
            fc_model::Anchor {
                name: "top".into(),
                x: 50.0,
                y: 200.0,
            },
        ];
    }
    let bytes = fc_compile::export(&font, &|_| 0, &Default::default())
        .unwrap()
        .bytes;
    fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0)
}

#[test]
fn combined_kerning_mark_and_mkmk_lift_revert_and_undo_restore_only_owned_data() {
    let original = imported_model_layout();
    let (session, api, doc) = setup(original.clone());
    let mut request = revisions(&session, doc);
    request["prefer_model_lift"] = json!(true);
    let preview = api.call("layout.prepare_conversion", request).unwrap();
    assert!(preview["token"].is_string(), "{preview}");
    assert_eq!(preview["kerning_lifted"], true, "{preview}");
    assert_eq!(preview["anchors_lifted"], true, "{preview}");
    api.call(
        "layout.apply_conversion",
        json!({"doc":doc,"token":preview["token"],"source":{"source_of_truth":"ImportedTables"}}),
    )
    .unwrap();
    let applied = session
        .with_doc(doc, |document| document.font.clone())
        .unwrap();
    let master = &applied.masters[0].id;
    api.call("doc.apply", json!({"doc":doc,"label":"spacing","command":{"SetHorizontalKerning":{"master":master,"left":{"glyph":"A"},"right":{"glyph":"V"},"value":-125}}})).unwrap();
    let layer = &applied.glyph("A").unwrap().layers[0].id;
    api.call("doc.apply", json!({"doc":doc,"label":"anchor","command":{"SetAnchor":{"glyph":"A","layer":layer,"index":0,"anchor":{"name":"top","x":350,"y":710}}}})).unwrap();
    let before = session
        .with_doc(doc, |document| document.font.clone())
        .unwrap();
    let bytes = fc_compile::export(&before, &|_| 1, &Default::default())
        .unwrap()
        .bytes;
    let binary = fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert!(binary.layout_conversion.is_none());
    let shape = |font: &Font| {
        fc_layout::Shaper::new()
            .shape(
                font,
                &fc_layout::FontKey {
                    doc: 1,
                    master: font.default_master().id.clone(),
                },
                0,
                &fc_layout::ShapeRequest {
                    text: "AV A\u{301}\u{300}".into(),
                    ..Default::default()
                },
            )
            .unwrap()
    };
    let before_run = shape(&before);
    let binary_run = shape(&binary);
    assert_eq!(before_run.names, binary_run.names);
    assert_eq!(before_run.glyphs, binary_run.glyphs);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("lifted.fcf");
    api.call("session.save", json!({"doc":doc,"path":path}))
        .unwrap();
    api.call("session.close", json!({"doc":doc})).unwrap();
    let doc: DocId = serde_json::from_value(
        api.call("session.open", json!({"path":path})).unwrap()["doc"].clone(),
    )
    .unwrap();
    let generations = session
        .with_doc(doc, |document| {
            document
                .font
                .glyph_order()
                .map(|name| document.glyph_generation(document.glyph_id(name.as_ref()).unwrap()))
                .collect::<Vec<_>>()
        })
        .unwrap();
    api.call("layout.revert_original", revisions(&session, doc))
        .unwrap();
    session.with_doc(doc, |document| {
        assert_eq!(document.font.masters[0].kerning_h, original.masters[0].kerning_h);
        for name in ["A", "acutecomb", "gravecomb"] { assert_eq!(document.font.glyph(name).unwrap().layers[0].anchors, original.glyph(name).unwrap().layers[0].anchors); }
        assert_eq!(document.font.glyph_order().map(|name| document.glyph_generation(document.glyph_id(name.as_ref()).unwrap())).collect::<Vec<_>>(), generations);
    }).unwrap();
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    session
        .with_doc(doc, |document| {
            assert_eq!(
                document.font.masters[0].kerning_h,
                before.masters[0].kerning_h
            );
            assert_eq!(
                document.font.glyph("A").unwrap().layers[0].anchors,
                before.glyph("A").unwrap().layers[0].anchors
            );
            assert_eq!(document.font.features, before.features);
        })
        .unwrap();
}

#[test]
fn every_master_must_compile_before_a_conversion_token_is_published() {
    let mut font = imported();
    let master = fc_model::Master::default();
    font.masters.push(master.clone());
    let (session, api, doc) = setup(font);
    let rejected = prepare(&api, &session, doc);
    assert!(rejected["token"].is_null(), "{rejected}");
    assert!(!rejected["blockers"].as_array().unwrap().is_empty());
    session
        .with_doc_mut(doc, |document| {
            for glyph in document.font.glyphs.values_mut() {
                let mut layer = glyph.layers[0].clone();
                layer.id = fc_model::LayerId::new();
                layer.kind = fc_model::LayerKind::Master(master.id.clone());
                glyph.layers.push(layer);
            }
        })
        .unwrap();
    let accepted = prepare(&api, &session, doc);
    assert!(accepted["token"].is_string(), "{accepted}");
    assert!(
        accepted["messages"].to_string().contains("2 master"),
        "{accepted}"
    );
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        0
    );
}
