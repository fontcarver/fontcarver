#[allow(dead_code)]
mod common;

use fc_doc::{Command, Session};
use fc_model::{Contour, Font, Glyph, Layer, Point, PointKind};

fn fixture() -> Font {
    let mut font = Font::default();
    let mut source = Glyph::new("_cap");
    let mut source_layer = Layer::new_master(font.default_master().id.clone());
    source_layer.contours.push(Contour {
        closed: true,
        points: [(0., 0.), (20., 0.), (0., 20.)]
            .into_iter()
            .map(|(x, y)| Point {
                x,
                y,
                kind: PointKind::Line,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    });
    source.layers.push(source_layer);
    font.insert_glyph(source).unwrap();
    let mut glyph = Glyph::new("A");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.contours.push(Contour {
        closed: false,
        points: [(0., "start"), (100., "end")]
            .into_iter()
            .enumerate()
            .map(|(index, (x, id))| Point {
                id: Some(id.into()),
                x,
                kind: if index == 0 {
                    PointKind::Move
                } else {
                    PointKind::Line
                },
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    });
    layer.constructions = serde_json::from_value(serde_json::json!([{
        "id":"cap", "source":"_cap", "keep_carrier":true,
        "kind":{"Cap":{"at":{"id":"start"},"scale":1.0}}
    }]))
    .unwrap();
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_roundtrip_preserves_constructions_and_rejects_bad_node_refs_before_replacing_output() {
    let font = fixture();
    let scratch = Scratch(common::temp_dir("construction-persistence"));
    let path = scratch.0.join("construction.fcf");
    fc_doc::fcf::write(&path, &font).unwrap();
    let reopened = fc_doc::fcf::read(&path).unwrap();
    assert_eq!(font, reopened);
    let before = std::fs::read(&path).unwrap();
    let mut invalid = font;
    invalid.glyph_mut("A").unwrap().layers[0].contours[0].points[0].id = None;
    assert!(fc_doc::fcf::write(&path, &invalid).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut malformed: serde_json::Value = serde_json::from_slice(&before).unwrap();
    malformed["font"]["glyphs"]["A"]["layers"][0]["contours"][0]["points"][0]["id"] =
        serde_json::Value::Null;
    std::fs::write(&path, serde_json::to_vec(&malformed).unwrap()).unwrap();
    assert!(fc_doc::fcf::read(&path).is_err());
}

#[test]
fn construction_source_rename_undo_redo_and_removal_are_atomic() {
    let session = Session::new();
    let doc_id = session.open_font(fixture(), None);
    session
        .with_doc_mut(doc_id, |doc| {
            doc.apply(
                Command::RenameGlyph {
                    from: "_cap".into(),
                    to: "_cap.alt".into(),
                },
                "Rename source",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph("A").unwrap().layers[0].constructions[0].source,
                "_cap.alt".into()
            );
            doc.undo().unwrap();
            assert_eq!(
                doc.font.glyph("A").unwrap().layers[0].constructions[0].source,
                "_cap".into()
            );
            doc.redo().unwrap();
            assert_eq!(
                doc.font.glyph("A").unwrap().layers[0].constructions[0].source,
                "_cap.alt".into()
            );
            let before = doc.font.clone();
            let generation = doc.generation;
            assert!(doc
                .apply(
                    Command::RemoveGlyphs {
                        names: vec!["_cap.alt".into()]
                    },
                    "Remove source"
                )
                .is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, generation);
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["_cap.alt".into(), "A".into()],
                },
                "Remove both",
            )
            .unwrap();
            assert!(doc.font.glyphs.is_empty());
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
        })
        .unwrap();
}
