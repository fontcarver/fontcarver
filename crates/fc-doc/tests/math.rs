mod common;
use fc_doc::{fcf, Command, Session};
use fc_model::{MathConstruction, MathGlyphInfo, MathTable, MathVariant};

#[test]
fn math_rename_undo_and_native_save_are_lossless() {
    let mut font = common::test_font();
    font.math = Some(MathTable {
        glyphs: [(
            "base".into(),
            MathGlyphInfo {
                vertical: Some(MathConstruction {
                    variants: vec![MathVariant {
                        glyph: "base".into(),
                        advance: 500.,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )]
        .into(),
        ..Default::default()
    });
    font.math_table_owned = true;
    let original = font.clone();
    let session = Session::new();
    let id = session.open_font(font, None);
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
            let renamed = doc.font.clone();
            assert_eq!(
                renamed.math.as_ref().unwrap().glyphs["renamed"]
                    .vertical
                    .as_ref()
                    .unwrap()
                    .variants[0]
                    .glyph
                    .as_ref(),
                "renamed"
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.redo().unwrap();
            assert_eq!(doc.font, renamed);
            let dir = common::temp_dir("math");
            for name in ["math.fcf", "math.fcf.gz"] {
                let path = dir.join(name);
                fcf::write(&path, &doc.font).unwrap();
                assert_eq!(fcf::read(&path).unwrap(), renamed);
            }
            std::fs::remove_dir_all(dir).unwrap();
        })
        .unwrap();
}

#[test]
fn math_follows_glyph_deletion_and_duplication_with_undo() {
    let mut font = common::test_font();
    let variant = |glyph: &str| MathVariant {
        glyph: glyph.into(),
        advance: 500.,
    };
    font.math = Some(MathTable {
        glyphs: [
            (
                "base".into(),
                MathGlyphInfo {
                    extended_shape: true,
                    vertical: Some(MathConstruction {
                        variants: vec![variant("base"), variant("composite")],
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ),
            (
                "composite".into(),
                MathGlyphInfo {
                    extended_shape: true,
                    ..Default::default()
                },
            ),
        ]
        .into(),
        ..Default::default()
    });
    font.math_table_owned = true;
    let original = font.clone();
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["composite".into()],
                },
                "Delete",
            )
            .unwrap();
            let math = doc.font.math.as_ref().unwrap();
            assert!(!math.glyphs.contains_key("composite"));
            assert_eq!(
                math.glyphs["base"].vertical.as_ref().unwrap().variants,
                vec![variant("base")]
            );
            let deleted = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.redo().unwrap();
            assert_eq!(doc.font, deleted);
            doc.undo().unwrap();

            doc.apply(
                Command::DuplicateGlyph {
                    source: "base".into(),
                    name: "base.copy".into(),
                },
                "Duplicate",
            )
            .unwrap();
            let math = doc.font.math.as_ref().unwrap();
            assert_eq!(math.glyphs["base.copy"], math.glyphs["base"]);
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
        })
        .unwrap();
}

#[test]
fn math_commands_validate_atomically_and_undo() {
    use fc_doc::command::MathCommand;
    use fc_model::{MathConstants, MathValue};
    let font = common::test_font();
    let original = font.clone();
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let change = doc
                .apply(
                    Command::Math(MathCommand::SetGlobals {
                        constants: Some(MathConstants {
                            axis_height: MathValue::from(250.),
                            ..Default::default()
                        }),
                        min_connector_overlap: 10.,
                    }),
                    "MATH constants",
                )
                .unwrap();
            assert!(change.math);
            assert!(doc.font.math_table_owned);
            let with_constants = doc.font.clone();
            assert_eq!(
                with_constants
                    .math
                    .as_ref()
                    .unwrap()
                    .constants
                    .as_ref()
                    .unwrap()
                    .axis_height
                    .value,
                250.
            );

            // Unknown glyph references and invalid values leave the document unchanged.
            let dangling = MathGlyphInfo {
                vertical: Some(MathConstruction {
                    variants: vec![MathVariant {
                        glyph: "missing".into(),
                        advance: 1.,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            };
            let error = doc
                .apply(
                    Command::Math(MathCommand::SetGlyph {
                        glyph: "base".into(),
                        info: Some(dangling),
                    }),
                    "Bad",
                )
                .unwrap_err();
            assert!(error.to_string().contains("missing"), "{error}");
            let error = doc
                .apply(
                    Command::Math(MathCommand::SetGlyph {
                        glyph: "nothing".into(),
                        info: Some(MathGlyphInfo::default()),
                    }),
                    "Bad",
                )
                .unwrap_err();
            assert!(error.to_string().contains("nothing"), "{error}");
            assert_eq!(doc.font, with_constants);

            // A failing command later in a batch rolls back earlier MATH edits.
            let error = doc.apply(
                Command::Batch(vec![
                    Command::Math(MathCommand::SetGlyph {
                        glyph: "base".into(),
                        info: Some(MathGlyphInfo {
                            extended_shape: true,
                            ..Default::default()
                        }),
                    }),
                    Command::Math(MathCommand::SetGlobals {
                        constants: None,
                        min_connector_overlap: f64::NAN,
                    }),
                ]),
                "Batch",
            );
            assert!(error.is_err());
            assert_eq!(doc.font, with_constants);

            doc.apply(
                Command::Math(MathCommand::SetGlyph {
                    glyph: "base".into(),
                    info: Some(MathGlyphInfo {
                        top_accent: Some(MathValue::from(300.)),
                        ..Default::default()
                    }),
                }),
                "Accent",
            )
            .unwrap();
            doc.apply(Command::Math(MathCommand::Replace { math: None }), "Delete")
                .unwrap();
            assert!(doc.font.math.is_none());
            assert!(doc.font.math_table_owned);
            doc.undo().unwrap();
            assert_eq!(
                doc.font.math.as_ref().unwrap().glyphs["base"]
                    .top_accent
                    .as_ref()
                    .unwrap()
                    .value,
                300.
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, with_constants);
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
        })
        .unwrap();
}
