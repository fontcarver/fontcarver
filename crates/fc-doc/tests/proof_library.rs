mod common;

use std::path::PathBuf;

use fc_doc::{fcf, Command, Session};
use fc_model::Font;
use serde_json::{json, Value};

const LIBRARY_KEY: &str = "com.fontcarver.proofLibrary";

struct TestDirectory(PathBuf);

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn proof_library() -> Value {
    json!({
        "version": 1,
        "entries": [
            {
                "id": "latin-pairs",
                "name": "Latin pairs",
                "group": "Kerning",
                "script": "latn",
                "language": "en",
                "direction": "ltr",
                "text": " AV To Wa\nCafe\u{301} <&> \"proof\" "
            },
            {
                "id": "arabic-phrase",
                "name": "Arabic phrase",
                "group": "Phrases",
                "script": "arab",
                "language": "ar",
                "direction": "rtl",
                "text": "مرحبا بالعالم"
            },
            {
                "id": "han-phrase",
                "name": "汉字校样",
                "group": "Phrases",
                "script": "hani",
                "language": "zh",
                "direction": "ttb",
                "text": "天地玄黄，宇宙洪荒。"
            }
        ]
    })
}

fn font_with_unrelated_metadata() -> Font {
    let mut font = common::test_font();
    font.info.style_name = "Proof Regular".to_owned();
    font.info.user_data = json!({
        "com.example.editor": { "selection": ["base"], "zoom": 1.25 },
        "notes": "Keep this font metadata"
    });
    font.user_data = json!({ "formatMetadata": { "preserve": true } });
    font
}

#[test]
fn saved_proof_library_round_trips_plain_and_compressed_native_documents() {
    let directory = TestDirectory(common::temp_dir("proof-library"));
    let mut font = font_with_unrelated_metadata();
    font.info.user_data[LIBRARY_KEY] = proof_library();

    for filename in ["proof.fcf", "proof.fcf.gz"] {
        let path = directory.0.join(filename);
        fcf::write(&path, &font).unwrap();
        let reopened = fcf::read(&path).unwrap();

        assert_eq!(reopened.info.user_data[LIBRARY_KEY], proof_library());
        assert_eq!(reopened, font, "native round trip for {filename}");
    }
}

#[test]
fn proof_library_add_edit_and_clear_undo_without_changing_other_font_data() {
    let original = font_with_unrelated_metadata();
    let session = Session::new();
    let id = session.open_font(original.clone(), None);

    session
        .with_doc_mut(id, |doc| {
            let mut revised_library = proof_library();
            revised_library["entries"][0]["name"] = json!("Revised Latin pairs");
            revised_library["entries"][0]["text"] = json!("VA TA Yo");
            let changes = [
                ("Add proof strings", proof_library()),
                ("Edit proof string", revised_library),
                (
                    "Clear proof strings",
                    json!({ "version": 1, "entries": [] }),
                ),
            ];
            let mut snapshots = vec![original.clone()];

            for (index, (label, library)) in changes.iter().enumerate() {
                let mut info = doc.font.info.clone();
                info.user_data[LIBRARY_KEY] = library.clone();
                let change = doc
                    .apply(
                        Command::SetFontInfo {
                            info: Box::new(info),
                        },
                        *label,
                    )
                    .unwrap();

                let mut expected = original.clone();
                expected.info.user_data[LIBRARY_KEY] = library.clone();
                assert_eq!(doc.font, expected);
                assert!(change.font_info);
                assert!(doc.dirty);
                assert_eq!(doc.undo_len(), index + 1);
                assert_eq!(doc.undo_label(), Some(*label));
                snapshots.push(expected);
            }

            for (index, (label, _)) in changes.iter().enumerate().rev() {
                assert_eq!(doc.undo_label(), Some(*label));
                assert!(doc.undo().unwrap().unwrap().font_info);
                assert_eq!(doc.font, snapshots[index]);
                assert_eq!(doc.redo_label(), Some(*label));
            }
            assert!(!doc.dirty);
            assert!(doc.undo().unwrap().is_none());
            assert!(doc.font.info.user_data.get(LIBRARY_KEY).is_none());

            for (index, (label, _)) in changes.iter().enumerate() {
                assert_eq!(doc.redo_label(), Some(*label));
                assert!(doc.redo().unwrap().unwrap().font_info);
                assert_eq!(doc.font, snapshots[index + 1]);
                assert!(doc.dirty);
            }
            assert!(doc.redo().unwrap().is_none());
        })
        .unwrap();
}
