mod common;

use fc_doc::{Command, Document, PreparedFeatureChange, Session};
use fc_model::{FeatureBlock, FeatureGenerationSettings, Features, FeaturesSource};

fn document(test: impl FnOnce(&mut Document)) {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session.with_doc_mut(id, test).unwrap();
}

fn source(text: &str) -> Features {
    Features {
        prefixes: vec![FeatureBlock {
            code: text.into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn draft(doc: &Document, source: Features) -> Command {
    Command::SetFeatureDraft {
        expected_revision: doc.font.feature_revision,
        source,
        settings: FeatureGenerationSettings::default(),
    }
}

#[test]
fn invalid_draft_is_undoable_without_changing_active_layout_or_outlines() {
    document(|doc| {
        let active = doc.font.features.clone();
        let glyphs = doc.font.glyphs.clone();
        let source = source("# 中文\r\nfeature kern { broken");
        let change = doc.apply(draft(doc, source.clone()), "Draft").unwrap();
        assert!(change.feature_source);
        assert!(!change.layout);
        assert!(change.glyphs.is_empty());
        assert_eq!(doc.compile_revision(), 0);
        assert_eq!(doc.font.features, active);
        assert_eq!(doc.font.glyphs, glyphs);
        assert!(doc.font.has_unapplied_feature_draft());
        let revision = doc.font.feature_revision;
        doc.undo().unwrap();
        assert!(doc.font.feature_draft.is_none());
        assert!(doc.font.feature_revision > revision);
        assert!(!doc.dirty);
        doc.redo().unwrap();
        assert_eq!(doc.font.feature_draft.as_ref().unwrap().source, source);
        assert_eq!(doc.compile_revision(), 0);
    });
}

#[test]
fn unchanged_source_and_repeated_draft_are_no_ops() {
    document(|doc| {
        doc.apply(draft(doc, doc.font.features.clone()), "Open source")
            .unwrap();
        assert!(!doc.dirty);
        assert_eq!(doc.generation, 0);
        assert_eq!(doc.undo_len(), 0);
        let text = source("bad");
        doc.apply(draft(doc, text.clone()), "Type").unwrap();
        let state = (doc.generation, doc.font.feature_revision, doc.undo_len());
        doc.apply(draft(doc, text), "Same").unwrap();
        assert_eq!(
            state,
            (doc.generation, doc.font.feature_revision, doc.undo_len())
        );
    });
}

#[test]
fn draft_coalesces_and_stale_revision_or_authority_override_is_atomic() {
    document(|doc| {
        let edit = doc.begin_edit("Type features");
        doc.apply_in_edit(&edit, draft(doc, source("a"))).unwrap();
        doc.apply_in_edit(&edit, draft(doc, source("ab"))).unwrap();
        doc.end_edit(edit);
        assert_eq!(doc.undo_len(), 1);
        let before = doc.font.clone();
        assert!(doc
            .apply(
                Command::SetFeatureDraft {
                    expected_revision: 0,
                    source: source("stale"),
                    settings: Default::default()
                },
                "Stale"
            )
            .is_err());
        let mut imported = source("override");
        imported.source_of_truth = FeaturesSource::ImportedTables;
        assert!(doc.apply(draft(doc, imported), "Override").is_err());
        assert_eq!(doc.font, before);
        doc.undo().unwrap();
        assert!(doc.font.feature_draft.is_none());
    });
}

#[test]
fn failed_batch_restores_draft_and_revision() {
    document(|doc| {
        let before = doc.font.clone();
        let command = Command::Batch(vec![
            draft(doc, source("invalid")),
            Command::RemoveGlyphs {
                names: vec!["missing".into()],
            },
        ]);
        assert!(doc.apply(command, "Rejected").is_err());
        assert_eq!(doc.font, before);
        assert_eq!(doc.generation, 0);
        assert_eq!(doc.compile_revision(), 0);
        assert_eq!(doc.undo_len(), 0);
    });
}

#[test]
fn trusted_apply_is_one_atomic_entry_and_rechecks_revisions() {
    document(|doc| {
        let active = doc.font.features.clone();
        let source = source("feature liga { sub base base by composite; } liga;");
        doc.apply(draft(doc, source.clone()), "Draft").unwrap();
        let prepared = PreparedFeatureChange::new(source.clone(), Default::default());
        assert!(doc
            .commit_prepared_features(0, 0, prepared.clone(), "Apply")
            .is_err());
        assert_eq!(doc.font.features, active);
        doc.commit_prepared_features(doc.generation, doc.compile_revision(), prepared, "Apply")
            .unwrap();
        assert_eq!(doc.font.features, source);
        assert!(doc.font.feature_draft.is_none());
        assert_eq!(doc.compile_revision(), 1);
        assert_eq!(doc.undo_len(), 2);
        doc.undo().unwrap();
        assert_eq!(doc.font.features, active);
        assert_eq!(doc.font.feature_draft.as_ref().unwrap().source, source);
        assert_eq!(doc.compile_revision(), 2);
    });
}

#[test]
fn invalid_unicode_draft_survives_plain_gzip_and_reopen() {
    document(|doc| {
        let source = source("# 中文😀\r\nfeature kern { invalid");
        doc.apply(draft(doc, source.clone()), "Draft").unwrap();
        let dir = common::temp_dir("feature-draft");
        for name in ["draft.fcf", "draft.fcf.gz"] {
            let path = dir.join(name);
            doc.save(&path).unwrap();
            let font = fc_doc::fcf::read(&path).unwrap();
            assert_eq!(font.feature_draft.as_ref().unwrap().source, source);
            assert_eq!(font.features, Features::default());
            assert!(font.has_unapplied_feature_draft());
        }
        std::fs::remove_dir_all(dir).unwrap();
    });
}

#[test]
fn discard_is_undoable_and_persistent_revision_never_reuses_undo_revision() {
    document(|doc| {
        doc.apply(draft(doc, source("bad")), "Type").unwrap();
        let stale = doc.font.feature_revision;
        doc.apply(
            Command::DiscardFeatureDraft {
                expected_revision: stale,
            },
            "Discard",
        )
        .unwrap();
        assert!(doc.font.feature_draft.is_none());
        doc.undo().unwrap();
        assert!(doc.font.feature_draft.is_some());
        assert!(doc.font.feature_revision > stale);
        assert!(doc
            .apply(
                Command::DiscardFeatureDraft {
                    expected_revision: stale
                },
                "Stale discard"
            )
            .is_err());
        let revision = doc.font.feature_revision;
        assert_eq!(doc.font.feature_draft.as_ref().unwrap().revision, revision);
    });
}

#[test]
fn compile_revision_tracks_all_existing_mutation_causes_and_history() {
    document(|doc| {
        let master = doc.font.default_master().id.clone();
        let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
        let mut replacement_master = doc.font.default_master().clone();
        replacement_master.name = "Changed master".into();
        let commands = [
            Command::SetWidth {
                glyph: "base".into(),
                layer,
                width: 550.0,
            },
            Command::SetUnicodes {
                glyph: "base".into(),
                unicodes: vec![65],
            },
            Command::SetMaster {
                master: replacement_master,
            },
            Command::SetHorizontalKerning {
                master,
                left: fc_model::KernSide::Glyph("base".into()),
                right: fc_model::KernSide::Glyph("composite".into()),
                value: Some(-40.0),
            },
        ];
        for (index, command) in commands.into_iter().enumerate() {
            doc.apply(command, "Compile input").unwrap();
            assert_eq!(doc.compile_revision(), index as u64 + 1);
        }
        doc.undo().unwrap();
        assert_eq!(doc.compile_revision(), 5);
        doc.redo().unwrap();
        assert_eq!(doc.compile_revision(), 6);
        doc.apply(draft(doc, source("broken")), "Draft only")
            .unwrap();
        assert_eq!(doc.compile_revision(), 6);
    });
}

#[test]
fn anchor_activation_and_conversion_backup_restore_without_outlines() {
    document(|doc| {
        let before = doc.font.glyph("base").unwrap().clone();
        let layer = before.layers[0].id.clone();
        let glyph_id = doc.glyph_id("base").unwrap();
        let backup = fc_model::LayoutConversion {
            original_features: doc.font.features.clone(),
            original_settings: doc.font.feature_settings.clone(),
            original_glyph_order: doc.font.glyph_order().cloned().collect(),
            baseline_fingerprint: "test baseline".into(),
            lifted_anchors: vec![fc_model::FeatureAnchorState {
                glyph: "base".into(),
                layer: layer.clone(),
                anchors: before.layers[0].anchors.clone(),
            }],
            ..Default::default()
        };
        let anchors = vec![fc_model::Anchor {
            name: "top".into(),
            x: 50.0,
            y: 700.0,
        }];
        let prepared = PreparedFeatureChange::new(
            doc.font.features.clone(),
            doc.font.feature_settings.clone(),
        )
        .with_conversion(Some(backup.clone()))
        .with_anchors(vec![fc_model::FeatureAnchorState {
            glyph: "base".into(),
            layer,
            anchors: anchors.clone(),
        }]);
        let change = doc
            .commit_prepared_features(0, 0, prepared, "Lift")
            .unwrap();
        assert!(change.layout);
        assert!(change.glyphs.is_empty());
        assert_eq!(doc.glyph_generation(glyph_id), 0);
        let after = doc.font.glyph("base").unwrap();
        assert_eq!(after.layers[0].anchors, anchors);
        assert_eq!(after.layers[0].contours, before.layers[0].contours);
        assert_eq!(after.layers[0].components, before.layers[0].components);
        assert_eq!(after.user_data, before.user_data);
        assert_eq!(doc.font.layout_conversion, Some(backup));
        let dir = common::temp_dir("feature-conversion");
        let path = dir.join("converted.fcf");
        fc_doc::fcf::write(&path, &doc.font).unwrap();
        let restored = fc_doc::fcf::read(&path).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert_eq!(restored.layout_conversion, doc.font.layout_conversion);
        doc.undo().unwrap();
        assert_eq!(doc.font.glyph("base").unwrap(), &before);
        assert!(doc.font.layout_conversion.is_none());
        doc.redo().unwrap();
        assert_eq!(doc.font.glyph("base").unwrap().layers[0].anchors, anchors);
    });
}

#[test]
fn activation_rejects_bad_references_before_any_change_and_noop_stays_clean() {
    document(|doc| {
        let prepared = PreparedFeatureChange::new(
            doc.font.features.clone(),
            doc.font.feature_settings.clone(),
        );
        doc.commit_prepared_features(0, 0, prepared.clone(), "Noop")
            .unwrap();
        assert_eq!(doc.generation, 0);
        assert!(!doc.dirty);
        let prepared = prepared.with_anchors(vec![fc_model::FeatureAnchorState {
            glyph: "missing".into(),
            layer: doc.font.glyph("base").unwrap().layers[0].id.clone(),
            anchors: vec![],
        }]);
        let before = doc.font.clone();
        assert!(doc
            .commit_prepared_features(0, 0, prepared, "Invalid")
            .is_err());
        assert_eq!(doc.font, before);
        assert_eq!(doc.undo_len(), 0);
    });
}

#[test]
fn native_autosave_retains_invalid_draft_without_activating_it() {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };
    let session = Arc::new(Session::new());
    let id = session.open_font(common::test_font(), None);
    let source = source("# 保存失败语法😀\nfeature invalid {");
    session
        .with_doc_mut(id, |doc| {
            doc.apply(draft(doc, source.clone()), "Draft").unwrap()
        })
        .unwrap();
    let dir = common::temp_dir("feature-autosave");
    let handle = fc_doc::autosave::spawn(
        Arc::clone(&session),
        fc_doc::autosave::AutosaveConfig {
            interval: Duration::from_millis(10),
            dir: dir.clone(),
        },
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    let path = loop {
        if let Some(entry) = fc_doc::autosave::list_recoverable(&dir).first() {
            break entry.autosave_path.clone();
        }
        assert!(
            Instant::now() < deadline,
            "autosave did not persist the draft"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    handle.stop();
    let font = fc_doc::fcf::read(&path).unwrap();
    assert_eq!(font.feature_draft.unwrap().source, source);
    assert_eq!(font.features, Features::default());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ordinary_wire_commands_cannot_deserialize_an_authority_activation() {
    assert!(
        serde_json::from_str::<Command>(r#"{"ApplyFeatures":{"source_of_truth":"Fea"}}"#).is_err()
    );
    assert!(serde_json::from_str::<Command>(
        r#"{"CommitPreparedFeatures":{"source_of_truth":"Fea"}}"#
    )
    .is_err());
}

#[test]
fn saved_draft_cannot_change_imported_authority_or_active_writer_policy() {
    document(|doc| {
        doc.font.features.source_of_truth = FeaturesSource::ImportedTables;
        let active = doc.font.features.clone();
        let active_settings = doc.font.feature_settings.clone();
        let mut settings = active_settings.clone();
        settings.small_caps = true;
        doc.apply(
            Command::SetFeatureDraft {
                expected_revision: 0,
                source: source("invalid draft"),
                settings,
            },
            "Imported draft",
        )
        .unwrap();
        assert_eq!(doc.font.features, active);
        assert_eq!(doc.font.feature_settings, active_settings);
        assert_eq!(doc.compile_revision(), 0);
    });
}

#[test]
fn legacy_native_documents_default_to_clean_source_and_old_writer_policy() {
    let dir = common::temp_dir("feature-old-document");
    let path = dir.join("legacy.fcf");
    std::fs::write(&path, r#"{"format":"fontcarver","version":1,"font":{"features":{"features":[{"tag":"liga","automatic":true,"code":"sub base base by composite;"}]}}}"#).unwrap();
    let font = fc_doc::fcf::read(&path).unwrap();
    assert!(font.feature_draft.is_none());
    assert_eq!(font.feature_revision, 0);
    assert!(!font.has_unapplied_feature_draft());
    assert!(!font.feature_settings.small_caps);
    assert!(!font.feature_settings.ligatures);
    assert!(font.feature_settings.writer_policy.kerning);
    assert!(font.feature_settings.writer_policy.anchors);
    assert!(font.layout_conversion.is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn exhausted_saved_revision_cannot_reuse_stale_writes_or_partially_activate() {
    document(|doc| {
        doc.font.feature_revision = u64::MAX;
        let before = doc.font.clone();
        assert!(doc.apply(draft(doc, source("first")), "Exhausted").is_err());
        assert_eq!(doc.font, before);
        let prepared = PreparedFeatureChange::new(source("valid source"), Default::default());
        assert!(doc
            .commit_prepared_features(0, 0, prepared, "Exhausted apply")
            .is_err());
        assert_eq!(doc.font, before);
        assert_eq!(doc.generation, 0);
        assert_eq!(doc.undo_len(), 0);
    });
}

#[test]
fn exhausted_revision_rejects_history_atomically() {
    document(|doc| {
        doc.font.feature_revision = u64::MAX - 1;
        doc.apply(draft(doc, source("last revision")), "Type")
            .unwrap();
        let before = doc.font.clone();
        assert!(doc.undo().is_err());
        assert_eq!(doc.font, before);
        assert_eq!(doc.undo_label(), Some("Type"));
        assert!(doc.redo_label().is_none());
    });
}

#[test]
fn document_open_reconciles_inconsistent_persisted_draft_revisions() {
    let mut font = common::test_font();
    font.feature_revision = 2;
    font.feature_draft = Some(fc_model::FeatureDraft {
        source: source("saved"),
        settings: Default::default(),
        revision: 9,
    });
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            assert_eq!(doc.font.feature_revision, 9);
            assert_eq!(doc.font.feature_draft.as_ref().unwrap().revision, 9);
            assert!(!doc.dirty);
            assert!(doc
                .apply(
                    Command::DiscardFeatureDraft {
                        expected_revision: 2
                    },
                    "Stale"
                )
                .is_err());
        })
        .unwrap();
}

#[test]
fn partial_kerning_activation_rejects_dangling_other_master_and_membership_data() {
    document(|doc| {
        use fc_model::{FeatureKerningState, KernSide};
        doc.apply(
            Command::CreateKerningGroup {
                side: fc_doc::kerning::KerningGroupSide::Side1,
                name: "public.kern1.base".into(),
                members: vec!["base".into()],
            },
            "Group",
        )
        .unwrap();
        let master = doc.font.default_master().id.clone();
        doc.apply(
            Command::SetHorizontalKerning {
                master,
                left: KernSide::Group("public.kern1.base".into()),
                right: KernSide::Glyph("composite".into()),
                value: Some(-30.0),
            },
            "Pair",
        )
        .unwrap();
        let before = doc.font.clone();
        let prepared = PreparedFeatureChange::new(
            doc.font.features.clone(),
            doc.font.feature_settings.clone(),
        )
        .with_kerning(FeatureKerningState::default());
        assert!(doc
            .commit_prepared_features(
                doc.generation,
                doc.compile_revision(),
                prepared,
                "Dangling lift"
            )
            .is_err());
        assert_eq!(doc.font, before);
        // Even if redundant glyph fields are cleared too, a master omitted
        // from the replacement still cannot refer to the removed group.
        let prepared = PreparedFeatureChange::new(
            doc.font.features.clone(),
            doc.font.feature_settings.clone(),
        )
        .with_kerning(FeatureKerningState {
            memberships: vec![fc_model::FeatureGlyphMembership {
                glyph: "base".into(),
                side1: None,
                side2: None,
            }],
            ..Default::default()
        });
        assert!(doc
            .commit_prepared_features(
                doc.generation,
                doc.compile_revision(),
                prepared,
                "Dangling master"
            )
            .is_err());
        assert_eq!(doc.font, before);
    });
}

#[test]
fn validated_kerning_lift_installs_consistent_groups_and_retains_vertical_pairs() {
    document(|doc| {
        use fc_model::{
            FeatureGlyphMembership, FeatureKerningState, FeatureMasterKerning, KernSide,
            KerningGroups,
        };
        let master = doc.font.default_master().id.clone();
        let mut groups = KerningGroups::default();
        groups
            .side1
            .insert("public.kern1.base".into(), ["base".into()].into());
        let mut pairs = fc_model::Kerning::default();
        pairs.0.insert(
            (
                KernSide::Group("public.kern1.base".into()),
                KernSide::Glyph("composite".into()),
            ),
            -30.0,
        );
        doc.font.masters[0].kerning_v.0.insert(
            (
                KernSide::Glyph("base".into()),
                KernSide::Glyph("composite".into()),
            ),
            20.0,
        );
        let before = doc.font.clone();
        let state = FeatureKerningState {
            masters: vec![FeatureMasterKerning {
                master,
                kerning: pairs.clone(),
            }],
            groups,
            memberships: vec![FeatureGlyphMembership {
                glyph: "base".into(),
                side1: Some("public.kern1.base".into()),
                side2: None,
            }],
        };
        let prepared = PreparedFeatureChange::new(
            doc.font.features.clone(),
            doc.font.feature_settings.clone(),
        )
        .with_kerning(state);
        let change = doc
            .commit_prepared_features(0, 0, prepared, "Lift pairs")
            .unwrap();
        assert!(change.layout);
        assert!(change.glyphs.is_empty());
        assert_eq!(doc.font.masters[0].kerning_h, pairs);
        assert_eq!(doc.font.masters[0].kerning_v, before.masters[0].kerning_v);
        assert_eq!(
            doc.font.glyph("base").unwrap().kern_group_side1.as_deref(),
            Some("public.kern1.base")
        );
        doc.undo().unwrap();
        assert_eq!(doc.font.masters, before.masters);
        assert_eq!(doc.font.glyphs, before.glyphs);
        assert_eq!(doc.font.kerning_groups, before.kerning_groups);
    });
}

#[test]
fn renaming_a_glyph_follows_into_the_saved_draft() {
    document(|doc| {
        // `base` is a FEA keyword, which a rename deliberately leaves alone.
        let text = "@renameTargets = [base composite];";
        doc.apply(draft(doc, source(text)), "Draft").unwrap();
        doc.apply(
            Command::RenameGlyph {
                from: "composite".into(),
                to: "composite.renamed".into(),
            },
            "Rename glyph",
        )
        .unwrap();

        let renamed = doc.font.feature_draft.as_ref().unwrap().source.prefixes[0]
            .code
            .clone();
        assert_eq!(
            renamed, "@renameTargets = [base composite.renamed];",
            "a rename must follow into the draft the editor is still holding"
        );
        doc.undo().unwrap();
        assert_eq!(
            doc.font.feature_draft.as_ref().unwrap().source.prefixes[0].code,
            text
        );
    });
}
