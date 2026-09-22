mod common;
use fc_doc::{Command, Session};
fn names(doc: &fc_doc::Document, cp: u32) -> Vec<String> {
    doc.unicode_index()
        .glyphs(cp)
        .map(ToString::to_string)
        .collect()
}
#[test]
fn index_tracks_reencoding_rename_remove_undo_redo_and_failed_batches() {
    let mut font = common::test_font();
    font.glyph_mut("base").unwrap().unicodes = vec![0x41, 0x42];
    font.glyph_mut("composite").unwrap().unicodes = vec![0x41];
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            assert_eq!(names(doc, 0x41), ["base", "composite"]);
            doc.apply(
                Command::SetUnicodes {
                    glyph: "base".into(),
                    unicodes: vec![0x43],
                },
                "Encode",
            )
            .unwrap();
            assert_eq!(names(doc, 0x41), ["composite"]);
            assert!(names(doc, 0x42).is_empty());
            doc.apply(
                Command::RenameGlyph {
                    from: "base".into(),
                    to: "new".into(),
                },
                "Rename",
            )
            .unwrap();
            assert_eq!(names(doc, 0x43), ["new"]);
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["new".into()],
                },
                "Remove",
            )
            .unwrap();
            assert!(names(doc, 0x43).is_empty());
            doc.undo().unwrap();
            assert_eq!(names(doc, 0x43), ["new"]);
            doc.undo().unwrap();
            assert_eq!(names(doc, 0x43), ["base"]);
            doc.undo().unwrap();
            assert_eq!(names(doc, 0x41), ["base", "composite"]);
            doc.redo().unwrap();
            assert_eq!(names(doc, 0x43), ["base"]);
            assert!(doc
                .apply(
                    Command::Batch(vec![
                        Command::SetUnicodes {
                            glyph: "base".into(),
                            unicodes: vec![0x44]
                        },
                        Command::RemoveGlyphs {
                            names: vec!["absent".into()]
                        }
                    ]),
                    "Fail"
                )
                .is_err());
            assert_eq!(names(doc, 0x43), ["base"]);
            assert!(names(doc, 0x44).is_empty());
        })
        .unwrap();
}
