mod common;

use std::{collections::BTreeSet, sync::Arc, time::Instant};

use fw_doc::{Command, Session};
use fw_model::{Anchor, Component, Contour, Glyph, GlyphName};

use common::{point, synthetic_font, test_font};

#[test]
fn set_width_undo_and_redo_restore_width_and_dirty_state() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.apply(
                Command::SetWidth {
                    glyph: "base".into(),
                    layer,
                    width: 720.0,
                },
                "Set width",
            )
            .unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 720.0);
            assert!(doc.dirty);

            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 500.0);
            assert!(!doc.dirty);

            doc.redo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 720.0);
            assert!(doc.dirty);
        })
        .unwrap();
}

#[test]
fn interactive_edit_coalesces_into_one_undo_entry() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let token = doc.begin_edit("Drag width");
            for step in 0..10 {
                doc.apply_in_edit(
                    &token,
                    Command::SetWidth {
                        glyph: "base".into(),
                        layer: layer.clone(),
                        width: 501.0 + f64::from(step),
                    },
                )
                .unwrap();
            }
            doc.end_edit(token);
            assert_eq!(doc.undo_len(), 1);
            assert_eq!(doc.undo_label(), Some("Drag width"));
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 500.0);
        })
        .unwrap();
}

#[test]
fn rename_undo_restores_stable_id_and_component_references() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let original_id = doc.glyph_id("base").unwrap();
            doc.apply(
                Command::RenameGlyph {
                    from: "base".into(),
                    to: "renamed".into(),
                },
                "Rename glyph",
            )
            .unwrap();
            assert_eq!(doc.glyph_id("renamed"), Some(original_id));
            assert_eq!(
                doc.font.glyph("composite").unwrap().layers[0].components[0].base,
                GlyphName::from("renamed")
            );

            doc.undo().unwrap();
            assert_eq!(doc.glyph_id("base"), Some(original_id));
            assert_eq!(
                doc.font.glyph("composite").unwrap().layers[0].components[0].base,
                GlyphName::from("base")
            );
        })
        .unwrap();
}

#[test]
fn remove_undo_restores_original_glyph_order() {
    let session = Session::new();
    let mut font = test_font();
    font.insert_glyph(Glyph::new("last")).unwrap();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let expected: Vec<_> = doc.font.glyph_order().cloned().collect();
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["composite".into()],
                },
                "Remove glyph",
            )
            .unwrap();
            doc.undo().unwrap();
            assert_eq!(
                doc.font.glyph_order().cloned().collect::<Vec<_>>(),
                expected
            );
        })
        .unwrap();
}

#[test]
fn undo_order_restore_skips_glyphs_removed_outside_commands() {
    let session = Session::new();
    let mut font = test_font();
    font.insert_glyph(Glyph::new("last")).unwrap();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["composite".into()],
                },
                "Remove glyph",
            )
            .unwrap();
            doc.font.glyphs.shift_remove("base");

            doc.undo().unwrap();

            assert_eq!(
                doc.font.glyph_order().cloned().collect::<Vec<_>>(),
                vec![GlyphName::from("composite"), GlyphName::from("last")]
            );
        })
        .unwrap();
}

#[test]
fn flattened_path_cache_invalidates_when_component_base_changes() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let base_id = doc.glyph_id("base").unwrap();
            let composite_id = doc.glyph_id("composite").unwrap();
            let master = doc.font.default_master().id.clone();
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let first = doc.flattened_path(composite_id, &master).unwrap();
            let generation = doc.glyph_generation(composite_id);
            doc.apply(
                Command::ReplaceLayerOutline {
                    glyph: "base".into(),
                    layer,
                    contours: vec![Contour {
                        closed: true,
                        points: vec![
                            point(0.0, 0.0),
                            point(200.0, 0.0),
                            point(200.0, 200.0),
                            point(0.0, 200.0),
                        ],
                        ..Contour::default()
                    }],
                    components: Vec::<Component>::new(),
                    anchors: Vec::<Anchor>::new(),
                },
                "Resize base",
            )
            .unwrap();
            let second = doc.flattened_path(composite_id, &master).unwrap();
            assert!(!Arc::ptr_eq(&first, &second));
            assert_ne!(first, second);
            assert!(doc.glyph_generation(base_id) > 0);
            assert!(doc.glyph_generation(composite_id) > generation);
        })
        .unwrap();
}

#[test]
fn metadata_command_updates_only_requested_fields() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(
                Command::SetGlyphMeta {
                    glyph: "base".into(),
                    export: Some(false),
                    color_label: Some(Some(3)),
                    tags: Some(BTreeSet::from(["review".to_owned()])),
                    note: Some("check curves".to_owned()),
                },
                "Set metadata",
            )
            .unwrap();
            let glyph = doc.font.glyph("base").unwrap();
            assert!(!glyph.export);
            assert_eq!(glyph.color_label, Some(3));
            assert!(glyph.tags.contains("review"));
            assert_eq!(glyph.note, "check curves");
        })
        .unwrap();
}

#[test]
fn add_and_remove_history_preserves_allocated_handles() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(
                Command::AddGlyph {
                    glyph: Glyph::new("added"),
                },
                "Add glyph",
            )
            .unwrap();
            let added_id = doc.glyph_id("added").unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.glyph_name(added_id), None);
            doc.redo().unwrap();
            assert_eq!(doc.glyph_id("added"), Some(added_id));

            let base_id = doc.glyph_id("base").unwrap();
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["base".into()],
                },
                "Remove base",
            )
            .unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.glyph_id("base"), Some(base_id));
        })
        .unwrap();
}

#[test]
fn failed_batch_is_atomic_and_successful_batch_has_one_entry() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let failed = doc.apply(
                Command::Batch(vec![
                    Command::SetWidth {
                        glyph: "base".into(),
                        layer: layer.clone(),
                        width: 900.0,
                    },
                    Command::SetUnicodes {
                        glyph: "missing".into(),
                        unicodes: vec![65],
                    },
                ]),
                "Bad batch",
            );
            assert!(failed.is_err());
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].width, 500.0);
            assert_eq!(doc.undo_len(), 0);

            let mut info = doc.font.info.clone();
            info.family_name = "Changed Family".to_owned();
            let master = doc.font.default_master().id.clone();
            let mut metrics = doc.font.default_master().metrics.clone();
            metrics.ascender = 800.0;
            doc.apply(
                Command::Batch(vec![
                    Command::SetUnicodes {
                        glyph: "base".into(),
                        unicodes: vec![65],
                    },
                    Command::SetFontInfo {
                        info: Box::new(info),
                    },
                    Command::SetMasterMetrics { master, metrics },
                ]),
                "Update font",
            )
            .unwrap();
            assert_eq!(doc.undo_len(), 1);
            assert_eq!(doc.font.info.family_name, "Changed Family");
            assert_eq!(doc.font.default_master().metrics.ascender, 800.0);
            doc.undo().unwrap();
            assert_eq!(doc.font.info.family_name, "Document Test");
            assert_eq!(doc.font.default_master().metrics.ascender, 0.0);
            assert!(doc.font.glyph("base").unwrap().unicodes.is_empty());
        })
        .unwrap();
}

#[test]
fn batch_rename_then_remove_can_be_undone() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(
                Command::Batch(vec![
                    Command::RenameGlyph {
                        from: "base".into(),
                        to: "renamed".into(),
                    },
                    Command::RemoveGlyphs {
                        names: vec!["renamed".into()],
                    },
                ]),
                "Rename and remove",
            )
            .unwrap();
            assert!(doc.font.glyph("base").is_none());
            assert!(doc.font.glyph("renamed").is_none());
            doc.undo().unwrap();
            assert!(doc.font.glyph("base").is_some());
            assert_eq!(
                doc.font.glyph("composite").unwrap().layers[0].components[0].base,
                GlyphName::from("base")
            );
        })
        .unwrap();
}

#[test]
fn outline_replacement_rejects_component_cycles_without_mutating() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("composite").unwrap().layers[0].id.clone();
            let original = doc.font.glyph("composite").unwrap().clone();
            let result = doc.apply(
                Command::ReplaceLayerOutline {
                    glyph: "composite".into(),
                    layer,
                    contours: Vec::new(),
                    components: vec![Component {
                        base: "composite".into(),
                        ..Component::default()
                    }],
                    anchors: Vec::new(),
                },
                "Create cycle",
            );
            assert!(result.is_err());
            assert_eq!(doc.font.glyph("composite"), Some(&original));
            assert_eq!(doc.undo_len(), 0);
            assert!(!doc.dirty);
        })
        .unwrap();
}

#[test]
fn regular_apply_finalizes_a_live_edit_and_clears_redo_history() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let token = doc.begin_edit("Drag width");
            doc.apply_in_edit(
                &token,
                Command::SetWidth {
                    glyph: "base".into(),
                    layer,
                    width: 510.0,
                },
            )
            .unwrap();
            doc.apply(
                Command::SetUnicodes {
                    glyph: "base".into(),
                    unicodes: vec![65],
                },
                "Set Unicode",
            )
            .unwrap();
            assert_eq!(doc.undo_len(), 2);

            doc.undo().unwrap();
            assert_eq!(doc.redo_label(), Some("Set Unicode"));
            doc.apply(
                Command::SetUnicodes {
                    glyph: "base".into(),
                    unicodes: vec![66],
                },
                "Set another Unicode",
            )
            .unwrap();
            assert_eq!(doc.redo().unwrap(), None);
        })
        .unwrap();
}

#[test]
fn coalesced_outline_drag_meets_large_font_budget() {
    let session = Session::new();
    let id = session.open_font(synthetic_font(20_000), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("glyph-19999");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let token = doc.begin_edit("Drag outline");
            let started = Instant::now();
            for step in 0..200 {
                doc.apply_in_edit(
                    &token,
                    Command::ReplaceLayerOutline {
                        glyph: glyph.clone(),
                        layer: layer.clone(),
                        contours: vec![Contour {
                            closed: false,
                            points: vec![point(f64::from(step), 0.0)],
                            ..Contour::default()
                        }],
                        components: Vec::new(),
                        anchors: Vec::new(),
                    },
                )
                .unwrap();
            }
            let elapsed = started.elapsed();
            doc.end_edit(token);
            if cfg!(not(debug_assertions)) {
                assert!(
                    elapsed.as_secs_f64() / 200.0 < 0.001,
                    "outline drag averaged {:?} per step",
                    elapsed / 200
                );
            }
        })
        .unwrap();
}
