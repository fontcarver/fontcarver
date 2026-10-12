mod common;

use fc_doc::{
    command::{AnnotationKind, EditorMetadataCommand, LayerAnnotation},
    Command, Document, PointRef, Session,
};
use fc_model::{Layer, LayerId};
use serde_json::json;

const KEY: &str = "com.fontcarver.editor";

fn with_document(test: impl FnOnce(&mut Document, LayerId)) {
    let mut font = common::test_font();
    let layer = &mut font.glyph_mut("base").unwrap().layers[0];
    layer.contours.push(layer.contours[0].clone());
    layer.user_data = json!({"foreign": {"keep": true}, KEY: {"future": 42}});
    let layer_id = layer.id.clone();
    let session = Session::new();
    let id = session.open_font(font, None);
    session.with_doc_mut(id, |doc| test(doc, layer_id)).unwrap();
}

fn layer(doc: &Document) -> &Layer {
    &doc.font.glyph("base").unwrap().layers[0]
}

fn paths(layer: &LayerId, contours: Vec<usize>, locked: bool) -> Command {
    Command::EditorMetadata(EditorMetadataCommand::PathLocks {
        glyph: "base".into(),
        layer: layer.clone(),
        contours,
        locked,
    })
}

fn annotation(id: &str) -> LayerAnnotation {
    LayerAnnotation {
        id: id.into(),
        kind: AnnotationKind::Text,
        x: 10.,
        y: 20.,
        end_x: 40.,
        end_y: 60.,
        text: "Check the curve".into(),
    }
}

fn annotations(layer: &LayerId, annotations: Vec<LayerAnnotation>) -> Command {
    Command::EditorMetadata(EditorMetadataCommand::Annotations {
        glyph: "base".into(),
        layer: layer.clone(),
        annotations,
    })
}

fn point(layer: &LayerId, contour: usize) -> Command {
    Command::SetPointPosition {
        glyph: "base".into(),
        layer: layer.clone(),
        point: PointRef { contour, point: 0 },
        x: 25.,
        y: 15.,
    }
}

#[test]
fn annotations_and_locks_preserve_metadata_history_and_native_storage() {
    with_document(|doc, id| {
        let original = doc.font.clone();
        doc.apply(
            Command::Batch(vec![
                annotations(&id, vec![annotation("note")]),
                paths(&id, vec![0], true),
                Command::EditorMetadata(EditorMetadataCommand::GlyphLock {
                    glyph: "base".into(),
                    locked: true,
                }),
            ]),
            "Annotate and lock",
        )
        .unwrap();
        let updated = doc.font.clone();
        let stored = &layer(doc).user_data;
        assert_eq!(stored["foreign"], json!({"keep": true}));
        assert_eq!(stored[KEY]["future"], 42);
        assert_eq!(stored[KEY]["annotations"][0]["id"], "note");
        assert_eq!(
            stored[KEY]["locked_paths"][0],
            layer(doc).contours[0].id.as_ref().unwrap().as_str()
        );
        let dir = common::temp_dir("editor-metadata");
        let path = dir.join("font.fcf");
        fc_doc::fcf::write(&path, &doc.font).unwrap();
        assert_eq!(fc_doc::fcf::read(&path).unwrap(), updated);
        std::fs::remove_dir_all(dir).unwrap();
        doc.undo().unwrap();
        assert_eq!(doc.font, original);
        doc.redo().unwrap();
        assert_eq!(doc.font, updated);
    });
}

#[test]
fn invalid_annotations_and_path_indexes_do_not_mutate_or_add_history() {
    with_document(|doc, id| {
        let before = doc.font.clone();
        let mut nonfinite = annotation("bad");
        nonfinite.end_y = f64::NAN;
        for command in [
            annotations(&id, vec![annotation("same"), annotation("same")]),
            annotations(&id, vec![annotation("")]),
            annotations(&id, vec![nonfinite]),
            paths(&id, vec![0, 99], true),
        ] {
            assert!(doc.apply(command, "Invalid metadata").is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, 0);
        }
        assert!(doc.undo().unwrap().is_none());
    });
}

#[test]
fn locked_path_rejects_changes_and_deletion_but_other_paths_remain_editable() {
    with_document(|doc, id| {
        doc.apply(paths(&id, vec![0], true), "Lock").unwrap();
        let before = doc.font.clone();
        for command in [
            point(&id, 0),
            Command::DeleteContours {
                glyph: "base".into(),
                layer: id.clone(),
                contours: vec![0],
            },
            Command::RemoveLayer {
                glyph: "base".into(),
                layer: id.clone(),
            },
        ] {
            assert!(doc.apply(command, "Locked edit").is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, 1);
        }
        doc.apply(point(&id, 1), "Other path").unwrap();
        doc.apply(paths(&id, vec![0], false), "Unlock").unwrap();
        doc.apply(point(&id, 0), "Unlocked edit").unwrap();
    });
}

#[test]
fn lock_identity_survives_reordering_and_rejects_duplicate_identity() {
    with_document(|doc, id| {
        doc.apply(paths(&id, vec![0], true), "Lock").unwrap();
        let mut contours = layer(doc).contours.clone();
        contours.reverse();
        doc.apply(
            Command::ReplaceLayerOutline {
                glyph: "base".into(),
                layer: id.clone(),
                contours,
                components: vec![],
                anchors: vec![],
            },
            "Reorder",
        )
        .unwrap();
        doc.apply(point(&id, 0), "Unlocked moved index").unwrap();
        let before = doc.font.clone();
        assert!(doc.apply(point(&id, 1), "Locked moved index").is_err());
        let duplicate = layer(doc).contours[1].clone();
        assert!(doc
            .apply(
                Command::AddContour {
                    glyph: "base".into(),
                    layer: id,
                    contour: duplicate
                },
                "Duplicate identity"
            )
            .is_err());
        assert_eq!(doc.font, before);
    });
}

#[test]
fn batch_checks_new_locks_and_restores_every_preceding_mutation() {
    with_document(|doc, id| {
        let before = doc.font.clone();
        assert!(doc
            .apply(
                Command::Batch(vec![
                    annotations(&id, vec![annotation("note")]),
                    paths(&id, vec![0], true),
                    Command::Batch(vec![point(&id, 0)])
                ]),
                "Failed batch"
            )
            .is_err());
        assert_eq!(doc.font, before);
        assert_eq!(doc.generation, 0);
    });
}

#[test]
fn glyph_lock_blocks_geometry_and_metadata_and_can_be_undone_or_unlocked() {
    with_document(|doc, id| {
        let lock = |locked| {
            Command::EditorMetadata(EditorMetadataCommand::GlyphLock {
                glyph: "base".into(),
                locked,
            })
        };
        doc.apply(lock(true), "Lock glyph").unwrap();
        let before = doc.font.clone();
        for command in [
            point(&id, 0),
            annotations(&id, vec![annotation("note")]),
            paths(&id, vec![0], true),
            Command::RemoveGlyphs {
                names: vec!["base".into()],
            },
            Command::RenameGlyph {
                from: "base".into(),
                to: "renamed".into(),
            },
        ] {
            assert!(doc.apply(command, "Locked glyph edit").is_err());
            assert_eq!(doc.font, before);
        }
        doc.undo().unwrap();
        assert!(!doc.font.glyph("base").unwrap().locked);
        doc.redo().unwrap();
        assert!(doc.font.glyph("base").unwrap().locked);
        doc.apply(lock(false), "Unlock glyph").unwrap();
        doc.apply(point(&id, 0), "Allowed edit").unwrap();
    });
}

#[test]
fn foreign_scalar_metadata_is_preserved_on_rejection() {
    with_document(|doc, id| {
        doc.font.glyph_mut("base").unwrap().layers[0].user_data = json!("foreign");
        let before = doc.font.clone();
        assert!(doc
            .apply(annotations(&id, vec![annotation("note")]), "Annotate")
            .is_err());
        assert_eq!(doc.font, before);
    });
}

#[test]
fn rejecting_an_edit_after_rename_restores_names_references_and_locks() {
    with_document(|doc, id| {
        doc.apply(paths(&id, vec![0], true), "Lock").unwrap();
        let before = doc.font.clone();
        let mut edit = point(&id, 0);
        if let Command::SetPointPosition { glyph, .. } = &mut edit {
            *glyph = "renamed".into();
        }
        assert!(doc
            .apply(
                Command::Batch(vec![
                    Command::RenameGlyph {
                        from: "base".into(),
                        to: "renamed".into()
                    },
                    edit,
                ]),
                "Rename then locked edit"
            )
            .is_err());
        assert_eq!(doc.font, before);
        assert!(doc.glyph_id("base").is_some());
        assert!(doc.glyph_id("renamed").is_none());
    });
}

#[test]
fn locking_ambiguous_contour_ids_assigns_distinct_persistent_identities() {
    with_document(|doc, id| {
        for contour in &mut doc.font.glyph_mut("base").unwrap().layers[0].contours {
            contour.id = Some("imported-duplicate".into());
        }
        doc.apply(paths(&id, vec![0, 1], true), "Lock ambiguous contours")
            .unwrap();
        let target = layer(doc);
        assert_ne!(target.contours[0].id, target.contours[1].id);
        assert_eq!(
            target.user_data[KEY]["locked_paths"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(doc.apply(point(&id, 0), "Locked").is_err());
        assert!(doc.apply(point(&id, 1), "Locked").is_err());
    });
}

#[test]
fn layer_effects_cannot_bypass_path_geometry_locks() {
    with_document(|doc, id| {
        doc.apply(paths(&id, vec![0], true), "Lock").unwrap();
        let before = doc.font.clone();
        assert!(doc
            .apply(
                Command::SetLayerEffects {
                    glyph: "base".into(),
                    layer: id,
                    effects: vec![fc_model::LayerEffect {
                        enabled: true,
                        effect: fc_model::LayerEffectKind::Boolean {
                            operation: fc_model::EffectBoolean::Union,
                            path: "M100 0L150 0L150 100L100 100Z".into(),
                        }
                    }]
                },
                "Change locked geometry"
            )
            .is_err());
        assert_eq!(doc.font, before);
    });
}
