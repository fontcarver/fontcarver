mod common;
use fc_doc::{command::BitmapCommand, Command, Session};
use fc_model::{Bitmap, BitmapGlyph, BitmapStrike};

fn strike() -> BitmapStrike {
    let mut strike = BitmapStrike {
        ppem: 16,
        depth: 1,
        ascent: 12,
        descent: 4,
        ..Default::default()
    };
    strike.glyphs.insert(
        "base".into(),
        BitmapGlyph {
            bitmap: Bitmap::new(8, 16, 1).unwrap(),
            advance: 8,
            ..Default::default()
        },
    );
    strike
}

#[test]
fn pixel_edits_undo_redo_without_touching_outline_generations() {
    let session = Session::new();
    let mut font = common::test_font();
    font.bitmap_strikes.push(strike());
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.glyph("base").unwrap().clone();
            let gid = doc.glyph_id("base").unwrap();
            let generation = doc.glyph_generation(gid);
            let mut value = doc.font.bitmap_strikes[0].glyphs["base"].clone();
            value.bitmap.set_pixel(2, 3, 1).unwrap();
            let change = doc
                .apply(
                    Command::Bitmap(BitmapCommand::SetGlyph {
                        ppem: 16,
                        depth: 1,
                        glyph: "base".into(),
                        value: Some(value),
                    }),
                    "Draw pixels",
                )
                .unwrap();
            assert!(change.bitmaps);
            assert!(change.glyphs.is_empty());
            assert!(!change.affects_compile());
            assert_eq!(doc.glyph_generation(gid), generation);
            assert_eq!(doc.font.glyph("base").unwrap(), &original);
            doc.undo().unwrap();
            assert_eq!(
                doc.font.bitmap_strikes[0].glyphs["base"].bitmap.pixel(2, 3),
                0
            );
            doc.redo().unwrap();
            assert_eq!(
                doc.font.bitmap_strikes[0].glyphs["base"].bitmap.pixel(2, 3),
                1
            );
        })
        .unwrap();
}

#[test]
fn failed_bitmap_batch_restores_strikes_and_does_not_create_history() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let generation = doc.generation;
            let result = doc.apply(
                Command::Batch(vec![
                    Command::Bitmap(BitmapCommand::SetStrike { strike: strike() }),
                    Command::Bitmap(BitmapCommand::SetGlyph {
                        ppem: 16,
                        depth: 1,
                        glyph: "missing".into(),
                        value: None,
                    }),
                ]),
                "Invalid",
            );
            assert!(result.is_err());
            assert!(doc.font.bitmap_strikes.is_empty());
            assert_eq!(doc.generation, generation);
            assert_eq!(doc.undo_len(), 0);
        })
        .unwrap();
}

#[test]
fn rename_duplicate_delete_and_native_roundtrip_keep_bitmap_ownership() {
    let session = Session::new();
    let mut font = common::test_font();
    font.bitmap_strikes.push(strike());
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
            doc.font.validate_bitmaps().unwrap();
            assert!(doc.font.bitmap_strikes[0].glyphs.contains_key("renamed"));
            doc.undo().unwrap();
            doc.font.validate_bitmaps().unwrap();
            assert!(doc.font.bitmap_strikes[0].glyphs.contains_key("base"));
            doc.apply(
                Command::DuplicateGlyph {
                    source: "base".into(),
                    name: "copy".into(),
                },
                "Duplicate",
            )
            .unwrap();
            assert!(doc.font.bitmap_strikes[0].glyphs.contains_key("copy"));
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["copy".into()],
                },
                "Delete",
            )
            .unwrap();
            assert!(!doc.font.bitmap_strikes[0].glyphs.contains_key("copy"));
            doc.undo().unwrap();
            assert!(doc.font.bitmap_strikes[0].glyphs.contains_key("copy"));
            let dir = common::temp_dir("bitmap");
            let path = dir.join("bitmap.fcf");
            doc.save(&path).unwrap();
            assert_eq!(
                fc_doc::fcf::read(&path).unwrap().bitmap_strikes,
                doc.font.bitmap_strikes
            );
            std::fs::remove_dir_all(dir).unwrap();
        })
        .unwrap();
}

#[test]
fn mixed_full_strike_and_pixel_gestures_restore_both_endpoints() {
    for full_first in [false, true] {
        let session = Session::new();
        let mut font = common::test_font();
        font.bitmap_strikes.push(strike());
        let id = session.open_font(font, None);
        session
            .with_doc_mut(id, |doc| {
                let before = doc.font.bitmap_strikes.clone();
                let owned = doc.font.bitmap_tables_owned;
                let token = doc.begin_edit("mixed");
                for bulk in [full_first, !full_first] {
                    let mut strike = doc.font.bitmap_strikes[0].clone();
                    let glyph = strike.glyphs.get_mut("base").unwrap();
                    glyph
                        .bitmap
                        .set_pixel(if bulk { 1 } else { 2 }, 3, 1)
                        .unwrap();
                    let command = if bulk {
                        BitmapCommand::SetStrike { strike }
                    } else {
                        BitmapCommand::SetGlyph {
                            ppem: 16,
                            depth: 1,
                            glyph: "base".into(),
                            value: Some(glyph.clone()),
                        }
                    };
                    doc.apply_in_edit(&token, Command::Bitmap(command)).unwrap();
                }
                doc.end_edit(token);
                let after = doc.font.bitmap_strikes.clone();
                doc.undo().unwrap();
                assert_eq!(doc.font.bitmap_strikes, before);
                assert_eq!(doc.font.bitmap_tables_owned, owned);
                doc.redo().unwrap();
                assert_eq!(doc.font.bitmap_strikes, after);
            })
            .unwrap();
    }
}
