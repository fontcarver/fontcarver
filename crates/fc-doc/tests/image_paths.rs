mod common;

use fc_doc::{image_paths, Command, Session};
use fc_model::Image;

struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(common::temp_dir("image-paths"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn linked_paths_resolve_relative_to_native_document_or_ufo_images() {
    let scratch = Scratch::new();
    let native = scratch.0.join("fonts/font.fcf");
    let ufo = scratch.0.join("fonts/source.ufo");
    let source = scratch.0.join("scans/a.png");
    assert_eq!(
        image_paths::resolve_image_path(Some(&native), "../scans/a.png").unwrap(),
        source
    );
    assert_eq!(
        image_paths::resolve_image_path(Some(&ufo), "a.png").unwrap(),
        ufo.join("images/a.png")
    );
    assert!(image_paths::resolve_image_path(None, "a.png").is_err());
    assert_eq!(
        image_paths::relative_image_path(&native, &source).unwrap(),
        std::path::Path::new("../scans/a.png").to_string_lossy()
    );
}

#[test]
fn imported_ufo_rebases_links_but_retains_embedded_names_on_native_save() {
    let scratch = Scratch::new();
    let old = scratch.0.join("Source.ufo");
    let new = scratch.0.join("font.fcf.gz");
    let mut font = common::test_font();
    font.glyph_mut("base").unwrap().layers[0].background_image = Some(Image {
        file_name: "scan.png".into(),
        ..Image::default()
    });
    font.glyph_mut("composite").unwrap().layers[0].background_image = Some(Image {
        file_name: "embedded.png".into(),
        data: Some(vec![1, 2, 3]),
        ..Image::default()
    });
    let session = Session::new();
    let id = session.open_font(font, Some(old.clone()));
    session
        .with_doc_mut(id, |doc| doc.save(&new))
        .unwrap()
        .unwrap();
    let saved = fc_doc::fcf::read(&new).unwrap();
    let linked = saved.glyph("base").unwrap().layers[0]
        .background_image
        .as_ref()
        .unwrap();
    assert_eq!(
        image_paths::resolve_image_path(Some(&new), &linked.file_name).unwrap(),
        old.join("images/scan.png")
    );
    let embedded = saved.glyph("composite").unwrap().layers[0]
        .background_image
        .as_ref()
        .unwrap();
    assert_eq!(embedded.file_name, "embedded.png");
    assert_eq!(embedded.data, Some(vec![1, 2, 3]));
}

#[test]
fn save_as_rebases_links_and_history_without_changing_the_source() {
    let scratch = Scratch::new();
    let old = scratch.0.join("source/font.fcf");
    let new = scratch.0.join("moved/font.fcf");
    std::fs::create_dir_all(old.parent().unwrap()).unwrap();
    std::fs::create_dir_all(new.parent().unwrap()).unwrap();
    let mut font = common::test_font();
    font.glyph_mut("base").unwrap().layers[0].background_image = Some(Image {
        file_name: "scan.png".into(),
        ..Image::default()
    });
    fc_doc::fcf::write(&old, &font).unwrap();
    let session = Session::new();
    let id = session.open_font(font.clone(), Some(old.clone()));
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.apply(
                Command::SetWidth {
                    glyph: "base".into(),
                    layer,
                    width: 712.0,
                },
                "Width",
            )
            .unwrap();
            let before_save_generation = doc.generation;
            doc.save(&new).unwrap();
            assert!(doc.generation > before_save_generation);
            for step in 0..3 {
                let image = doc.font.glyph("base").unwrap().layers[0]
                    .background_image
                    .as_ref()
                    .unwrap();
                assert_eq!(image.file_name, "../source/scan.png");
                assert_eq!(
                    image_paths::resolve_image_path(doc.path.as_deref(), &image.file_name).unwrap(),
                    scratch.0.join("source/scan.png")
                );
                match step {
                    0 => {
                        doc.undo().unwrap();
                    }
                    1 => {
                        doc.redo().unwrap();
                    }
                    _ => {}
                }
            }
            assert!(!doc.dirty);
        })
        .unwrap();
    assert_eq!(fc_doc::fcf::read(&old).unwrap(), font);
    assert_eq!(
        fc_doc::fcf::read(&new)
            .unwrap()
            .glyph("base")
            .unwrap()
            .layers[0]
            .background_image
            .as_ref()
            .unwrap()
            .file_name,
        "../source/scan.png"
    );
}

#[test]
fn failed_save_as_leaves_links_path_and_history_unchanged() {
    let scratch = Scratch::new();
    let old = scratch.0.join("font.fcf");
    let mut font = common::test_font();
    font.glyph_mut("base").unwrap().layers[0].background_image = Some(Image {
        file_name: "scan.png".into(),
        ..Image::default()
    });
    let session = Session::new();
    let id = session.open_font(font, Some(old.clone()));
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.apply(
                Command::SetWidth {
                    glyph: "base".into(),
                    layer,
                    width: 712.0,
                },
                "Width",
            )
            .unwrap();
            let before = doc.font.clone();
            // A file as the destination's parent deterministically prevents writes.
            let blocked = scratch.0.join("blocked");
            std::fs::write(&blocked, b"occupied").unwrap();
            assert!(doc.save(&blocked.join("font.fcf")).is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.path, Some(old));
            assert!(doc.dirty);
            doc.undo().unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0]
                    .background_image
                    .as_ref()
                    .unwrap()
                    .file_name,
                "scan.png"
            );
        })
        .unwrap();
}
