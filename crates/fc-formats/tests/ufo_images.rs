use fc_formats::ufo::{read_ufo, write_ufo, UfoWriteOptions};
use fc_model::{Font, Glyph, Image, Layer};

#[test]
fn embedded_image_export_uses_portable_unique_names_and_preserves_source() {
    let mut font = Font::new_empty("Image export", 1000);
    let master = font.default_master().id.clone();
    for (index, file_name) in ["../scans/reference.png", "../other/reference.png"]
        .into_iter()
        .enumerate()
    {
        let mut glyph = Glyph::new(format!("image{index}"));
        let mut layer = Layer::new_master(master.clone());
        layer.background_image = Some(Image {
            file_name: file_name.into(),
            data: Some(if index == 0 {
                PNG.to_vec()
            } else {
                WHITE_PNG.to_vec()
            }),
            transform: kurbo::Affine::new([1.5, 0.0, 0.0, 2.0, 17.0, -24.0]),
            locked: index == 0,
            ..Image::default()
        });
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    let before = font.clone();
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("Images.ufo");
    write_ufo(&font, &master, &output, &UfoWriteOptions::default()).unwrap();
    assert_eq!(font, before);
    let reopened = read_ufo(&output).unwrap();
    let mut names = std::collections::HashSet::new();
    for index in 0..2 {
        let name = format!("image{index}");
        let original = font.glyph(&name).unwrap().layers[0]
            .background_image
            .as_ref()
            .unwrap();
        let imported = reopened.glyph(&name).unwrap().layers[0]
            .background_image
            .as_ref()
            .unwrap();
        assert!(names.insert(imported.file_name.clone()));
        assert_eq!(
            std::path::Path::new(&imported.file_name)
                .components()
                .count(),
            1
        );
        assert_eq!(
            std::fs::read(output.join("images").join(&imported.file_name)).unwrap(),
            original.data.as_ref().unwrap().as_slice()
        );
        assert_eq!(imported.transform, original.transform);
        assert_eq!(imported.locked, original.locked);
        assert!(imported.data.is_none());
    }
}

#[test]
fn missing_linked_image_reports_export_error_without_creating_output() {
    let mut font = Font::new_empty("Missing image", 1000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("A");
    let mut layer = Layer::new_master(master.clone());
    layer.background_image = Some(Image {
        file_name: "missing.png".into(),
        ..Image::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("Missing.ufo");
    assert!(write_ufo(&font, &master, &output, &UfoWriteOptions::default()).is_err());
    assert!(!output.exists());
}

const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0,
    0, 0, 114, 182, 13, 36, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 96, 96, 96, 248, 15, 2, 16,
    18, 8, 0, 95, 178, 12, 244, 216, 82, 206, 121, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
const WHITE_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0,
    0, 0, 114, 182, 13, 36, 0, 0, 0, 14, 73, 68, 65, 84, 120, 156, 99, 248, 15, 5, 12, 48, 6, 0,
    143, 130, 15, 241, 60, 165, 86, 81, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
