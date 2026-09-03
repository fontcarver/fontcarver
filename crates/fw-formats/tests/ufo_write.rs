use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use fw_formats::ufo::{read_ufo, write_ufo, UfoWriteOptions};

#[test]
fn minimal_round_trip_preserves_ufo_semantics() {
    let source_path = fw_fixtures::ufo_path("Minimal.ufo");
    let source = norad::Font::load(&source_path).unwrap();
    let model = read_ufo(&source_path).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("RoundTrip.ufo");
    write_ufo(
        &model,
        &model.default_master().id,
        &output,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    let written = norad::Font::load(output).unwrap();

    assert_eq!(written.font_info, source.font_info);
    assert_eq!(written.groups, source.groups);
    assert_eq!(written.kerning, source.kerning);
    assert_eq!(written.features, source.features);
    assert_eq!(written.glyph_count(), source.glyph_count());
    for name in ["A", "B", "space"] {
        assert_eq!(written.get_glyph(name), source.get_glyph(name));
    }
    for (key, value) in &source.lib {
        assert_eq!(written.lib.get(key), Some(value));
    }
}

#[test]
fn preserves_layers_images_object_identifiers_and_unmapped_fontinfo() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("Source.ufo");
    let mut source = norad::Font::load(fw_fixtures::ufo_path("Minimal.ufo")).unwrap();
    source.font_info.year = Some(2026);
    source.font_info.woff_major_version = Some(2);
    source.font_info.postscript_weight_name = Some("Book".to_owned());
    let mut global_guide = norad::Guideline::new(
        norad::Line::Horizontal(500.0),
        Some(norad::Name::new("cap").unwrap()),
        None,
        Some(norad::Identifier::new("global-guide").unwrap()),
    );
    global_guide.replace_lib(
        [("role".to_owned(), plist::Value::String("cap".to_owned()))]
            .into_iter()
            .collect(),
    );
    source.font_info.guidelines = Some(vec![global_guide]);
    source.groups.insert(
        norad::Name::new("com.example.uppercase").unwrap(),
        vec![
            norad::Name::new("A").unwrap(),
            norad::Name::new("B").unwrap(),
        ],
    );
    let glyph_a = source.get_glyph_mut("A").unwrap();
    glyph_a.contours[0].replace_identifier(norad::Identifier::new("outer-contour").unwrap());
    glyph_a.contours[0].points[0].name = Some(norad::Name::new("start").unwrap());
    glyph_a.contours[0].points[0]
        .replace_identifier(norad::Identifier::new("start-point").unwrap());
    glyph_a.image = Some(
        norad::Image::new(
            "reference.png".into(),
            Some(norad::Color::new(1.0, 0.0, 0.0, 0.5).unwrap()),
            norad::AffineTransform {
                x_offset: 12.0,
                y_offset: 34.0,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    source
        .images
        .insert(
            "reference.png".into(),
            vec![137, 80, 78, 71, 13, 10, 26, 10],
        )
        .unwrap();
    let glyph_b = source.get_glyph_mut("B").unwrap();
    glyph_b.components[0].replace_identifier(norad::Identifier::new("component-a").unwrap());
    glyph_b.anchors[0].replace_identifier(norad::Identifier::new("anchor-top").unwrap());
    let mut sketch_a = source.get_glyph("A").unwrap().clone();
    sketch_a.width = 601.0;
    let sketch = source.layers.get_or_create_layer("Sketch").unwrap();
    sketch.insert_glyph(sketch_a);
    let background = source
        .layers
        .get_or_create_layer("public.background")
        .unwrap();
    background.insert_glyph(norad::Glyph::new("A"));
    source.save(&source_path).unwrap();

    let model = read_ufo(&source_path).unwrap();
    assert!(model.glyph("A").unwrap().layers.iter().any(|layer| {
        matches!(&layer.kind, fw_model::LayerKind::Custom(name) if name == "Sketch")
    }));
    assert!(model
        .glyph("A")
        .unwrap()
        .layers
        .iter()
        .any(|layer| { matches!(&layer.kind, fw_model::LayerKind::Background(_)) }));
    let output = temp.path().join("Output.ufo");
    write_ufo(
        &model,
        &model.default_master().id,
        &output,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    let original = norad::Font::load(&source_path).unwrap();
    let written = norad::Font::load(&output).unwrap();
    assert_eq!(written.font_info, original.font_info);
    assert_eq!(written.groups, original.groups);
    assert_eq!(written.get_glyph("A"), original.get_glyph("A"));
    assert_eq!(written.get_glyph("B"), original.get_glyph("B"));
    assert_eq!(
        written.layers.get("Sketch").unwrap().get_glyph("A"),
        original.layers.get("Sketch").unwrap().get_glyph("A")
    );
    assert_eq!(
        fs::read(output.join("images/reference.png")).unwrap(),
        [137, 80, 78, 71, 13, 10, 26, 10]
    );
}

#[test]
fn incremental_write_touches_only_changed_glif_and_removes_stale_glifs() {
    let mut model = read_ufo(&fw_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let master_id = model.default_master().id.clone();
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("Incremental.ufo");
    write_ufo(&model, &master_id, &output, &UfoWriteOptions::default()).unwrap();
    fs::write(output.join("keep.me"), b"untouched").unwrap();
    let before = file_times(&output);
    thread::sleep(Duration::from_millis(30));
    model
        .glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master_id)
        .unwrap()
        .width += 1.0;
    write_ufo(&model, &master_id, &output, &UfoWriteOptions::default()).unwrap();
    let after = file_times(&output);
    let a_path = glyph_path(&output, "A");
    assert!(after[&a_path] > before[&a_path]);
    for (path, modified) in &before {
        if path != &a_path {
            assert_eq!(
                after[path],
                *modified,
                "unexpected rewrite of {}",
                path.display()
            );
        }
    }
    assert_eq!(fs::read(output.join("keep.me")).unwrap(), b"untouched");

    let space_path = glyph_path(&output, "space");
    model.remove_glyph("space");
    write_ufo(&model, &master_id, &output, &UfoWriteOptions::default()).unwrap();
    assert!(!output.join(space_path).exists());
}

#[test]
fn sfnt_round_trip_preserves_outlines_and_raw_tables() {
    let outcome = fw_sfnt::read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &fw_sfnt::ReadOptions {
            keep_original_bytes: false,
            ..Default::default()
        },
    )
    .unwrap();
    let source = &outcome.fonts[0];
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("DejaVuSans.ufo");
    let started = Instant::now();
    write_ufo(
        source,
        &source.default_master().id,
        &output,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    assert!(started.elapsed() < Duration::from_secs(60));
    let round_trip = read_ufo(&output).unwrap();
    assert_eq!(round_trip.extra.raw_tables, source.extra.raw_tables);
    for (name, glyph) in &source.glyphs {
        let source_layer = glyph.master_layer(&source.default_master().id).unwrap();
        let round_layer = round_trip
            .glyph(name.as_ref())
            .unwrap()
            .master_layer(&round_trip.default_master().id)
            .unwrap();
        assert_eq!(round_layer.contours, source_layer.contours, "glyph {name}");
        assert_eq!(
            round_layer.components, source_layer.components,
            "glyph {name}"
        );
        assert_eq!(round_layer.anchors, source_layer.anchors, "glyph {name}");
    }
}

fn glyph_path(ufo: &Path, name: &str) -> std::path::PathBuf {
    let contents = plist::Value::from_file(ufo.join("glyphs/contents.plist"))
        .unwrap()
        .into_dictionary()
        .unwrap();
    Path::new("glyphs").join(contents[name].as_string().unwrap())
}

fn file_times(root: &Path) -> BTreeMap<std::path::PathBuf, std::time::SystemTime> {
    fn visit(
        root: &Path,
        path: &Path,
        result: &mut BTreeMap<std::path::PathBuf, std::time::SystemTime>,
    ) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    entry.metadata().unwrap().modified().unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
