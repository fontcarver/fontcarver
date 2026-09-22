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
