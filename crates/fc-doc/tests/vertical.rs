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
