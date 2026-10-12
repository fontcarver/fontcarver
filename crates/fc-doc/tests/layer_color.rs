use fc_doc::{Command, Session};
use fc_model::{Font, Glyph};
use serde_json::json;

#[test]
fn layer_labels_are_independent_undoable_and_persisted() {
    let mut font = Font::new_empty("Labels", 1000);
    let mut glyph = Glyph::new("A");
    glyph.color_label = Some(2);
    font.insert_glyph(glyph).unwrap();
    let layer = font.glyph("A").unwrap().layers[0].id.clone();
    font.glyph_mut("A").unwrap().layers[0].user_data = json!({"vendor": [1, 2]});
    let session = Session::new();
    let doc = session.open_font(font, None);
    session
        .with_doc_mut(doc, |document| {
            let original = document.font.clone();
            let set = |color| Command::SetLayerColor {
                glyph: "A".into(),
                layer: layer.clone(),
                color,
            };
            document.apply(set(Some(11)), "Layer label").unwrap();
            let glyph = document.font.glyph("A").unwrap();
            assert_eq!(glyph.color_label, Some(2));
            assert_eq!(
                glyph.layers[0].user_data,
                json!({"vendor": [1, 2], "com.fontcarver.colorLabel": 11})
            );
            let directory =
                std::env::temp_dir().join(format!("layer-labels-{}", std::process::id()));
            std::fs::create_dir_all(&directory).unwrap();
            let path = directory.join("labels.fcf");
            fc_doc::fcf::write(&path, &document.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), document.font);
            std::fs::remove_dir_all(directory).unwrap();
            document.undo().unwrap();
            assert_eq!(document.font, original);
            document.redo().unwrap();
            document.apply(set(None), "Clear layer label").unwrap();
            assert_eq!(
                document.font.glyph("A").unwrap().layers[0].user_data,
                json!({"vendor": [1, 2]})
            );
            let before = document.font.clone();
            let generation = document.generation;
            assert!(document
                .apply(set(Some(12)), "Invalid layer label")
                .is_err());
            assert_eq!(document.font, before);
            assert_eq!(document.generation, generation);
        })
        .unwrap();
}

#[test]
fn non_object_layer_data_is_preserved_on_rejected_label_edit() {
    let mut font = Font::new_empty("Labels", 1000);
    font.insert_glyph(Glyph::new("A")).unwrap();
    let glyph = font.glyph_mut("A").unwrap();
    let layer = glyph.layers[0].id.clone();
    glyph.layers[0].user_data = json!(["vendor-data"]);
    let session = Session::new();
    let doc = session.open_font(font.clone(), None);
    session
        .with_doc_mut(doc, |document| {
            assert!(document
                .apply(
                    Command::SetLayerColor {
                        glyph: "A".into(),
                        layer,
                        color: Some(1)
                    },
                    "Label"
                )
                .is_err());
            assert_eq!(document.font, font);
        })
        .unwrap();
}
