mod common;
use fc_doc::{command::ColorCommand, fcf, Command, Session};
use fc_model::{ColorPalette, ColorTable, ColrLayer, PaletteColor};

fn colors() -> ColorTable {
    ColorTable {
        graph: None,
        palettes: vec![ColorPalette {
            colors: vec![PaletteColor {
                red: 240,
                green: 20,
                blue: 10,
                alpha: 128,
            }],
            ..Default::default()
        }],
        glyphs: [(
            "base".into(),
            vec![
                ColrLayer {
                    glyph: "base".into(),
                    palette_index: Some(0),
                },
                ColrLayer {
                    glyph: "composite".into(),
                    palette_index: None,
                },
            ],
        )]
        .into(),
        ..Default::default()
    }
}

#[test]
fn color_references_follow_rename_duplicate_delete_and_undo() {
    let session = Session::new();
    let mut font = common::test_font();
    font.color = Some(colors());
    font.color_tables_owned = true;
    let id = session.open_font(font.clone(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(
                Command::RenameGlyph {
                    from: "base".into(),
                    to: "renamed".into(),
                },
                "Rename",
            )
            .unwrap();
            let table = doc.font.color.as_ref().unwrap();
            assert_eq!(table.glyphs["renamed"][0].glyph.as_ref(), "renamed");
            doc.undo().unwrap();
            assert_eq!(doc.font, font);
            doc.redo().unwrap();
            doc.undo().unwrap();
            doc.apply(
                Command::DuplicateGlyph {
                    source: "base".into(),
                    name: "copy".into(),
                },
                "Copy",
            )
            .unwrap();
            assert_eq!(
                doc.font.color.as_ref().unwrap().glyphs["copy"],
                colors().glyphs["base"]
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, font);
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["composite".into()],
                },
                "Delete",
            )
            .unwrap();
            assert_eq!(doc.font.color.as_ref().unwrap().glyphs["base"].len(), 1);
            doc.undo().unwrap();
            assert_eq!(doc.font, font);
        })
        .unwrap();
}

#[test]
fn edits_are_atomic_and_owned_deletion_survives_native_save() {
    let session = Session::new();
    let font = common::test_font();
    let id = session.open_font(font.clone(), None);
    session
        .with_doc_mut(id, |doc| {
            let change = doc
                .apply(
                    Command::Color(ColorCommand::Replace {
                        color: Some(colors()),
                    }),
                    "Colors",
                )
                .unwrap();
            assert!(change.color);
            let authored = doc.font.clone();
            let mut invalid = colors();
            invalid.glyphs.get_mut("base").unwrap()[0].glyph = "missing".into();
            assert!(doc
                .apply(
                    Command::Color(ColorCommand::Replace {
                        color: Some(invalid)
                    }),
                    "Invalid"
                )
                .is_err());
            assert_eq!(doc.font, authored);
            let dir = common::temp_dir("color");
            for file in ["color.fcf", "color.fcf.gz"] {
                let path = dir.join(file);
                fcf::write(&path, &doc.font).unwrap();
                assert_eq!(fcf::read(&path).unwrap(), authored);
            }
            doc.apply(
                Command::Color(ColorCommand::Replace { color: None }),
                "Delete colors",
            )
            .unwrap();
            assert!(doc.font.color.is_none() && doc.font.color_tables_owned);
            doc.undo().unwrap();
            assert_eq!(doc.font, authored);
            doc.undo().unwrap();
            assert_eq!(doc.font, font);
            std::fs::remove_dir_all(dir).unwrap();
        })
        .unwrap();
}

#[test]
fn graph_dependencies_follow_document_commands_and_undo() {
    use fc_model::{ColrGraph, ColrPaint};
    let session = Session::new();
    let mut font = common::test_font();
    let mut color = colors();
    color.graph = Some(ColrGraph {
        glyphs: [("base".into(), 0), ("composite".into(), 2)].into(),
        paints: vec![
            ColrPaint::Glyph {
                glyph: "base".into(),
                paint: 1,
            },
            ColrPaint::Solid {
                palette_index: Some(0),
                alpha: 1.,
            },
            ColrPaint::ColrGlyph {
                glyph: "base".into(),
            },
        ],
        clips: [("base".into(), [0, 0, 600, 700])].into(),
    });
    font.color = Some(color);
    let original = font.clone();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            assert!(fc_model::query::dependents(&doc.font)["base"].contains(&"composite".into()));
            doc.apply(
                Command::RenameGlyph {
                    from: "base".into(),
                    to: "renamed".into(),
                },
                "Rename",
            )
            .unwrap();
            let graph = doc.font.color.as_ref().unwrap().graph.as_ref().unwrap();
            assert!(graph.glyphs.contains_key("renamed"));
            assert!(graph.clips.contains_key("renamed"));
            assert_eq!(
                graph.paints[2],
                ColrPaint::ColrGlyph {
                    glyph: "renamed".into()
                }
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.apply(
                Command::DuplicateGlyph {
                    source: "base".into(),
                    name: "copy".into(),
                },
                "Copy",
            )
            .unwrap();
            let graph = doc.font.color.as_ref().unwrap().graph.as_ref().unwrap();
            assert_eq!(graph.glyphs["copy"], graph.glyphs["base"]);
            assert_eq!(graph.clips["copy"], graph.clips["base"]);
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["base".into()],
                },
                "Delete",
            )
            .unwrap();
            assert!(doc
                .font
                .color
                .as_ref()
                .unwrap()
                .graph
                .as_ref()
                .unwrap()
                .glyphs
                .is_empty());
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
        })
        .unwrap();
}
