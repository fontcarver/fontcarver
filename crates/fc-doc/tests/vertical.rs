mod common;
use fc_doc::{Command, Session};
use fc_model::{FeatureBlock, KernSide};

#[test]
fn vertical_edits_are_atomic_undoable_and_independent_of_horizontal_source() {
    let mut font = common::test_font();
    let master = font.masters[0].id.clone();
    let layer = font.glyph("base").unwrap().layers[0].id.clone();
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "pos [\\base] [composite] -20;".into(),
        ..Default::default()
    });
    let pair = (
        KernSide::Glyph("base".into()),
        KernSide::Glyph("composite".into()),
    );
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            doc.apply(
                Command::Batch(vec![
                    Command::SetVerticalKerning {
                        master: master.clone(),
                        left: pair.0.clone(),
                        right: pair.1.clone(),
                        value: Some(-80.),
                    },
                    Command::SetVerticalMetrics {
                        glyph: "base".into(),
                        layer: layer.clone(),
                        advance: Some(1000.),
                        origin: Some(880.),
                    },
                ]),
                "Vertical spacing",
            )
            .unwrap();
            assert_eq!(doc.font.masters[0].kerning_v.0.get(&pair), Some(&-80.));
            assert_eq!(
                fc_doc::kerning::resolve_stored_vertical(
                    &doc.font,
                    &"base".into(),
                    &"composite".into(),
                    &master
                )
                .unwrap()
                .unwrap()
                .value,
                -80.
            );
            let after = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
            assert!(doc
                .apply(
                    Command::SetVerticalMetrics {
                        glyph: "base".into(),
                        layer,
                        advance: Some(-1.),
                        origin: None
                    },
                    "Invalid"
                )
                .is_err());
            assert_eq!(doc.font, after);
            doc.font.features.features.push(FeatureBlock {
                tag: "vkrn".into(),
                code: "pos [\\base] [composite] 40;".into(),
                ..Default::default()
            });
            let before = doc.font.clone();
            assert!(doc
                .apply(
                    Command::SetVerticalKerning {
                        master,
                        left: pair.0,
                        right: pair.1,
                        value: None
                    },
                    "Source owned"
                )
                .is_err());
            assert_eq!(doc.font, before);
        })
        .unwrap();
}

#[test]
fn group_rename_and_removal_include_vertical_records_and_undo() {
    let session = Session::new();
    let mut font = common::test_font();
    font.kerning_groups
        .side1
        .insert("public.kern1.Top".into(), ["base".into()].into());
    font.masters[0].kerning_v.0.insert(
        (
            KernSide::Group("public.kern1.Top".into()),
            KernSide::Glyph("composite".into()),
        ),
        -90.,
    );
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            doc.apply(
                Command::RenameKerningGroup {
                    side: fc_doc::kerning::KerningGroupSide::Side1,
                    from: "public.kern1.Top".into(),
                    to: "public.kern1.New".into(),
                },
                "Rename",
            )
            .unwrap();
            assert!(doc.font.masters[0].kerning_v.0.contains_key(&(
                KernSide::Group("public.kern1.New".into()),
                KernSide::Glyph("composite".into())
            )));
            assert!(doc
                .apply(
                    Command::RemoveKerningGroup {
                        side: fc_doc::kerning::KerningGroupSide::Side1,
                        name: "public.kern1.New".into(),
                        remove_pairs: false
                    },
                    "Remove"
                )
                .is_err());
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
        })
        .unwrap();
}

/// Row 19: the inspector's vertical fields must obey the same guards as every
/// other editing command, and survive a native save and reopen.
#[test]
fn vertical_metric_guards_hold_and_edited_bearings_survive_fcf() {
    use fc_doc::command::EditorMetadataCommand;
    let session = Session::new();
    let font = common::test_font();
    let layer = font.glyph("base").unwrap().layers[0].id.clone();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let set = |advance: Option<f64>, origin: Option<f64>| Command::SetVerticalMetrics {
                glyph: "base".into(),
                layer: layer.clone(),
                advance,
                origin,
            };

            // Out of range in either field, in both directions, is refused and
            // leaves the document exactly as it was — the same bytes a save
            // would write, and no history entry for the refusal.
            let serialized =
                |doc: &fc_doc::Document| serde_json::to_vec(&doc.font).expect("serialize font");
            let pristine = doc.font.clone();
            let pristine_bytes = serialized(doc);
            let entries = doc.undo_len();
            for command in [
                set(Some(65_536.), None),
                set(Some(-1.), None),
                set(None, Some(32_768.)),
                set(None, Some(-32_769.)),
                set(Some(f64::NAN), None),
                set(None, Some(f64::INFINITY)),
            ] {
                assert!(doc.apply(command, "Invalid").is_err());
                assert_eq!(doc.font, pristine);
                assert_eq!(serialized(doc), pristine_bytes);
                assert_eq!(doc.undo_len(), entries);
            }

            // A TSB edit as the inspector sends it: origin only, advance kept.
            doc.apply(set(Some(1000.), Some(880.)), "Vertical metrics")
                .unwrap();
            doc.apply(set(Some(1000.), Some(905.)), "Vertical metrics")
                .unwrap();
            let edited = doc.font.clone();
            let stored = &edited.glyph("base").unwrap().layers[0];
            assert_eq!(stored.vertical_width, Some(1000.));
            assert_eq!(stored.vertical_origin, Some(905.));

            // Native save and reopen keep both overrides.
            let dir = common::temp_dir("vertical-metrics");
            let path = dir.join("font.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), edited);
            std::fs::remove_dir_all(dir).unwrap();

            // A locked glyph refuses the edit; unlocking restores it.
            doc.apply(
                Command::EditorMetadata(EditorMetadataCommand::GlyphLock {
                    glyph: "base".into(),
                    locked: true,
                }),
                "Lock",
            )
            .unwrap();
            let locked = doc.font.clone();
            let error = doc.apply(set(Some(1100.), Some(880.)), "Blocked");
            assert!(
                error.is_err(),
                "a locked glyph must refuse vertical metrics"
            );
            assert_eq!(doc.font, locked);
            doc.apply(
                Command::EditorMetadata(EditorMetadataCommand::GlyphLock {
                    glyph: "base".into(),
                    locked: false,
                }),
                "Unlock",
            )
            .unwrap();

            // A read-only document refuses it too, without a history entry.
            doc.set_read_only(true);
            let frozen = doc.font.clone();
            let entries = doc.undo_len();
            assert!(doc.apply(set(Some(1200.), Some(880.)), "Blocked").is_err());
            assert_eq!(doc.font, frozen);
            assert_eq!(doc.undo_len(), entries);
            doc.set_read_only(false);

            // Clearing both overrides is one undoable step back to inheritance.
            doc.apply(set(None, None), "Use master vertical metrics")
                .unwrap();
            let cleared = &doc.font.glyph("base").unwrap().layers[0];
            assert_eq!(cleared.vertical_width, None);
            assert_eq!(cleared.vertical_origin, None);
            doc.undo().unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].vertical_origin,
                Some(905.)
            );
        })
        .unwrap();
}
