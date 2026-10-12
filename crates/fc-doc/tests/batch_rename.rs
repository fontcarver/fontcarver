mod common;

use fc_doc::{Command, Session};
use fc_model::{GlyphName, KernSide};

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn swap_command() -> Command {
    Command::Batch(
        [
            ("base", "_fcRename0"),
            ("composite", "_fcRename1"),
            ("_fcRename0", "composite"),
            ("_fcRename1", "base"),
        ]
        .into_iter()
        .map(|(from, to)| Command::RenameGlyph {
            from: from.into(),
            to: to.into(),
        })
        .collect(),
    )
}

#[test]
fn simultaneous_rename_preserves_handles_references_undo_and_persistence() {
    let mut font = common::test_font();
    font.masters[0].kerning_h.0.insert(
        (
            KernSide::Glyph("base".into()),
            KernSide::Glyph("composite".into()),
        ),
        -35.0,
    );
    let original = serde_json::to_value(&font).unwrap();
    let session = Session::new();
    let doc = session.open_font(font, None);
    session
        .with_doc_mut(doc, |document| {
            let base_id = document.glyph_id("base").unwrap();
            let composite_id = document.glyph_id("composite").unwrap();
            document
                .apply(swap_command(), "Batch rename glyphs")
                .unwrap();
            assert_eq!(document.undo_len(), 1);
            assert_eq!(document.glyph_id("composite"), Some(base_id));
            assert_eq!(document.glyph_id("base"), Some(composite_id));
            assert_eq!(
                document.font.glyph("base").unwrap().layers[0].components[0].base,
                GlyphName::from("composite")
            );
            assert_eq!(
                document.font.masters[0].kerning_h.0.get(&(
                    KernSide::Glyph("composite".into()),
                    KernSide::Glyph("base".into()),
                )),
                Some(&-35.0)
            );
            let renamed = serde_json::to_value(&document.font).unwrap();
            let dir = Scratch(common::temp_dir("batch-rename"));
            let path = dir.0.join("renamed.fcf");
            fc_doc::fcf::write(&path, &document.font).unwrap();
            assert_eq!(
                serde_json::to_value(fc_doc::fcf::read(&path).unwrap()).unwrap(),
                renamed
            );
            document.undo().unwrap();
            assert_eq!(serde_json::to_value(&document.font).unwrap(), original);
            assert_eq!(document.glyph_id("base"), Some(base_id));
            assert_eq!(document.glyph_id("composite"), Some(composite_id));
            document.redo().unwrap();
            assert_eq!(serde_json::to_value(&document.font).unwrap(), renamed);
            assert_eq!(document.glyph_id("composite"), Some(base_id));
        })
        .unwrap();
}

#[test]
fn collision_in_final_phase_rolls_back_every_temporary_rename() {
    let session = Session::new();
    let doc = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(doc, |document| {
            let original = serde_json::to_value(&document.font).unwrap();
            let handle = document.glyph_id("base");
            let result = document.apply(
                Command::Batch(vec![
                    Command::RenameGlyph {
                        from: "base".into(),
                        to: "_fcRename0".into(),
                    },
                    Command::RenameGlyph {
                        from: "_fcRename0".into(),
                        to: "composite".into(),
                    },
                ]),
                "Batch rename glyphs",
            );
            assert!(result.is_err());
            assert_eq!(serde_json::to_value(&document.font).unwrap(), original);
            assert_eq!(document.glyph_id("base"), handle);
            assert!(document.glyph_id("_fcRename0").is_none());
            assert_eq!(document.undo_len(), 0);
            assert!(!document.dirty);
        })
        .unwrap();
}
