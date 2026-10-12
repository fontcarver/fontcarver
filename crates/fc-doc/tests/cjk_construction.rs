use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use fc_doc::{Command, Document, Error, Session};
use fc_model::{Component, Contour, Font, Glyph, Layer, Point};
use kurbo::Affine;

const WOOD: &str = "uni6728";
const FOREST: &str = "uni6797";
const LEFT: [f64; 6] = [0.5, 0.0, 0.0, 1.0, 0.0, 0.0];
const RIGHT: [f64; 6] = [0.5, 0.0, 0.0, 1.0, 500.0, 0.0];

fn test_font() -> Font {
    let mut font = Font::new_empty("CJK Construction Test", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicode, left, bottom, right, top) in [
        (WOOD, 0x6728, 100.0, 0.0, 900.0, 800.0),
        (FOREST, 0x6797, 450.0, -100.0, 550.0, -50.0),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 1_000.0;
        layer.contours.push(Contour {
            closed: true,
            points: [(left, bottom), (right, bottom), (right, top), (left, top)]
                .into_iter()
                .map(|(x, y)| Point {
                    x,
                    y,
                    ..Point::default()
                })
                .collect(),
            ..Contour::default()
        });
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font
}

fn component(base: &str, transform: [f64; 6]) -> Component {
    Component {
        base: base.into(),
        transform: Affine::new(transform),
        ..Component::default()
    }
}

fn construction(doc: &Document, second_base: &str) -> Command {
    let layer = doc.font.glyph(FOREST).unwrap().layers[0].id.clone();
    // The two editable references generated for the IDS expression ⿰木木.
    Command::Batch(
        [component(WOOD, LEFT), component(second_base, RIGHT)]
            .into_iter()
            .map(|component| Command::AddComponent {
                glyph: FOREST.into(),
                layer: layer.clone(),
                component,
            })
            .collect(),
    )
}

#[test]
fn horizontal_ids_construction_is_one_undoable_batch() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            doc.apply(construction(doc, WOOD), "Construct ⿰木木")
                .unwrap();
            let constructed = doc.font.clone();
            let target = &doc.font.glyph(FOREST).unwrap().layers[0];
            assert_eq!(target.width, 1_000.0);
            assert_eq!(
                target.components,
                vec![component(WOOD, LEFT), component(WOOD, RIGHT)]
            );
            assert_eq!(
                target.contours,
                original.glyph(FOREST).unwrap().layers[0].contours
            );
            assert_eq!(doc.font.glyph(WOOD), original.glyph(WOOD));
            assert_eq!(doc.undo_len(), 1);
            assert_eq!(doc.undo_label(), Some("Construct ⿰木木"));

            assert!(doc.undo().unwrap().is_some());
            assert_eq!(doc.font, original);
            assert!(!doc.dirty);
            assert!(doc.undo().unwrap().is_none());
            assert_eq!(doc.redo_label(), Some("Construct ⿰木木"));
            assert!(doc.redo().unwrap().is_some());
            assert_eq!(doc.font, constructed);
            assert!(doc.redo().unwrap().is_none());
        })
        .unwrap();
}

fn assert_failed_construction_preserves_history(font: Font, invalid_base: &str) -> Error {
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let layer = doc.font.glyph(WOOD).unwrap().layers[0].id.clone();
            for (width, label) in [(950.0, "Previous edit"), (1_050.0, "Future edit")] {
                doc.apply(
                    Command::SetWidth {
                        glyph: WOOD.into(),
                        layer: layer.clone(),
                        width,
                    },
                    label,
                )
                .unwrap();
            }
            let future = doc.font.clone();
            doc.undo().unwrap();
            let before = doc.font.clone();
            let generation = doc.generation;
            let dirty = doc.dirty;
            let compile_revision = doc.compile_revision();
            let target_id = doc.glyph_id(FOREST).unwrap();
            let target_generation = doc.glyph_generation(target_id);
            let error = doc
                .apply(construction(doc, invalid_base), "Invalid construction")
                .unwrap_err();

            // The valid first AddComponent must roll back when the second fails.
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, generation);
            assert_eq!(doc.compile_revision(), compile_revision);
            assert_eq!(doc.glyph_generation(target_id), target_generation);
            assert_eq!(doc.dirty, dirty);
            assert_eq!(doc.undo_len(), 2);
            assert_eq!(doc.undo_label(), Some("Previous edit"));
            assert_eq!(doc.redo_label(), Some("Future edit"));
            assert!(doc.redo().unwrap().is_some());
            assert_eq!(doc.font, future);
            assert!(doc.undo().unwrap().is_some());
            assert_eq!(doc.font, before);
            assert!(doc.undo().unwrap().is_some());
            assert_eq!(doc.font, original);
            assert!(doc.undo().unwrap().is_none());
            error
        })
        .unwrap()
}

#[test]
fn missing_radical_rolls_back_the_batch_without_changing_history() {
    let error = assert_failed_construction_preserves_history(test_font(), "missing-radical");
    assert!(matches!(
        error,
        Error::Model(fc_model::Error::MissingGlyph { glyph }) if glyph.as_ref() == "missing-radical"
    ));
}

#[test]
fn adding_a_resolved_radical_preserves_preexisting_unresolved_imported_references() {
    let session = Session::new();
    let mut font = test_font();
    font.glyph_mut(FOREST).unwrap().layers[0]
        .components
        .push(component("imported-missing-radical", RIGHT));
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let layer = doc.font.glyph(FOREST).unwrap().layers[0].id.clone();
            doc.apply(
                Command::AddComponent {
                    glyph: FOREST.into(),
                    layer,
                    component: component(WOOD, LEFT),
                },
                "Add resolved radical",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph(FOREST).unwrap().layers[0].components,
                vec![
                    component("imported-missing-radical", RIGHT),
                    component(WOOD, LEFT)
                ]
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
        })
        .unwrap();
}

#[test]
fn indirect_component_cycle_rolls_back_the_batch_without_changing_history() {
    let mut font = test_font();
    let mut dependent = Glyph::new("forest-dependent");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer
        .components
        .push(component(FOREST, Affine::IDENTITY.as_coeffs()));
    dependent.layers.push(layer);
    font.insert_glyph(dependent).unwrap();

    let error = assert_failed_construction_preserves_history(font, "forest-dependent");
    assert!(matches!(
        error,
        Error::Model(fc_model::Error::ComponentCycle { .. })
    ));
}

#[test]
fn constructed_component_transform_remains_independently_editable() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(construction(doc, WOOD), "Construct ⿰木木")
                .unwrap();
            let constructed = doc.font.clone();
            let layer = doc.font.glyph(FOREST).unwrap().layers[0].id.clone();
            let edited = component(WOOD, [0.45, 0.1, -0.05, 0.9, 525.0, 20.0]);
            doc.apply(
                Command::SetComponent {
                    glyph: FOREST.into(),
                    layer,
                    index: 1,
                    component: edited.clone(),
                },
                "Adjust right radical",
            )
            .unwrap();
            let adjusted = doc.font.clone();
            let target = &doc.font.glyph(FOREST).unwrap().layers[0];
            assert_eq!(target.components, vec![component(WOOD, LEFT), edited]);
            assert_eq!(
                target.contours,
                constructed.glyph(FOREST).unwrap().layers[0].contours
            );
            assert_eq!(doc.font.glyph(WOOD), constructed.glyph(WOOD));
            assert_eq!(doc.undo_len(), 2);
            assert_eq!(doc.undo_label(), Some("Adjust right radical"));
            assert!(doc.undo().unwrap().is_some());
            assert_eq!(doc.font, constructed);
            assert_eq!(doc.undo_label(), Some("Construct ⿰木木"));
            assert!(doc.redo().unwrap().is_some());
            assert_eq!(doc.font, adjusted);
        })
        .unwrap();
}

struct TemporaryDocument(PathBuf);

impl TemporaryDocument {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "fontcarver-cjk-construction-{}-{nonce}.fcf",
            std::process::id()
        )))
    }
}

impl Drop for TemporaryDocument {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(self.0.with_extension("fcf.tmp"));
    }
}

#[test]
fn native_save_reopens_transformed_references_and_original_contours() {
    let file = TemporaryDocument::new();
    let session = Session::new();
    let original = test_font();
    let id = session.open_font(original.clone(), None);
    let saved = session
        .with_doc_mut(id, |doc| {
            doc.apply(construction(doc, WOOD), "Construct ⿰木木")
                .unwrap();
            doc.save(&file.0).unwrap();
            assert!(!doc.dirty);
            doc.font.clone()
        })
        .unwrap();
    session.close(id).unwrap();

    let reopened = session.open_path(&file.0, None).unwrap();
    session
        .with_doc(reopened, |doc| {
            assert_eq!(doc.font, saved);
            let target = &doc.font.glyph(FOREST).unwrap().layers[0];
            assert_eq!(target.width, 1_000.0);
            assert_eq!(
                target.components,
                vec![component(WOOD, LEFT), component(WOOD, RIGHT)]
            );
            assert_eq!(
                target.contours,
                original.glyph(FOREST).unwrap().layers[0].contours
            );
            assert_eq!(doc.font.glyph(WOOD), original.glyph(WOOD));
            assert!(!doc.dirty);
            assert_eq!(doc.undo_len(), 0);
        })
        .unwrap();
}

/// Row 112: the transform grid applies every staged component in one batch, so
/// a locked component must stop the whole set rather than let half of it land.
#[test]
fn a_locked_component_refuses_the_whole_transform_batch() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(construction(doc, WOOD), "Construct ⿰木木")
                .unwrap();
            let layer = doc.font.glyph(FOREST).unwrap().layers[0].id.clone();
            doc.font.glyph_mut(FOREST).unwrap().layers[0].components[1].locked = true;
            let constructed = doc.font.clone();
            let entries = doc.undo_len();

            // Exactly what the grid sends: one SetComponent per changed row.
            let batch = Command::Batch(vec![
                Command::SetComponent {
                    glyph: FOREST.into(),
                    layer: layer.clone(),
                    index: 0,
                    component: component(WOOD, [0.52, 0.0, 0.0, 1.0, 0.0, 20.0]),
                },
                Command::SetComponent {
                    glyph: FOREST.into(),
                    layer,
                    index: 1,
                    component: component(WOOD, [0.5, 0.0, 0.0, 1.0, 480.0, 0.0]),
                },
            ]);
            let error = doc
                .apply(batch, "Transform CJK components")
                .expect_err("a locked component must refuse the batch");
            assert!(
                error.to_string().contains("locked"),
                "unexpected error: {error}"
            );
            // Neither row was written and no history entry was added.
            assert_eq!(doc.font, constructed);
            assert_eq!(doc.undo_len(), entries);
        })
        .unwrap();
}
