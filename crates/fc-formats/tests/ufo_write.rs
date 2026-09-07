use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use fc_formats::ufo::{read_ufo, write_ufo, UfoWriteOptions};
use fc_model::{BitmapStrike, Guide, StemHint, Tag, TtIndex};

#[test]
fn minimal_round_trip_preserves_ufo_semantics() {
    let source_path = fc_fixtures::ufo_path("Minimal.ufo");
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
    let mut source = norad::Font::load(fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    source.font_info.year = Some(2026);
    source.font_info.woff_major_version = Some(2);
    source.font_info.postscript_weight_name = Some("Book".to_owned());
    let mut global_guide = norad::Guideline::new(
        norad::Line::Angle {
            x: 0.0,
            y: 500.0,
            degrees: 0.0,
        },
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
    glyph_a.note = Some("hello note".to_owned());
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
        matches!(&layer.kind, fc_model::LayerKind::Custom(name) if name == "Sketch")
    }));
    assert!(model
        .glyph("A")
        .unwrap()
        .layers
        .iter()
        .any(|layer| { matches!(&layer.kind, fc_model::LayerKind::Background(_)) }));
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
    let mut expected_sketch = original
        .layers
        .get("Sketch")
        .unwrap()
        .get_glyph("A")
        .unwrap()
        .clone();
    expected_sketch.codepoints = Default::default();
    expected_sketch.note = None;
    assert_eq!(
        written.layers.get("Sketch").unwrap().get_glyph("A"),
        Some(&expected_sketch)
    );
    let background = written
        .layers
        .get("public.background")
        .unwrap()
        .get_glyph("A")
        .unwrap();
    assert!(background.codepoints.is_empty());
    assert!(background.note.is_none());
    assert_eq!(
        fs::read(output.join("images/reference.png")).unwrap(),
        [137, 80, 78, 71, 13, 10, 26, 10]
    );
}

#[test]
fn copies_data_and_unreferenced_images_to_new_ufo_and_preserves_them_in_place() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("Source.ufo");
    let mut source = norad::Font::load(fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    source
        .data
        .insert(
            "com.example/state.json".into(),
            br#"{"state":true}"#.to_vec(),
        )
        .unwrap();
    source
        .images
        .insert(
            "unreferenced.png".into(),
            vec![137, 80, 78, 71, 13, 10, 26, 10, 1, 2, 3],
        )
        .unwrap();
    source.save(&source_path).unwrap();

    let model = read_ufo(&source_path).unwrap();
    let output = temp.path().join("Output.ufo");
    write_ufo(
        &model,
        &model.default_master().id,
        &output,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    assert_eq!(
        fs::read(output.join("data/com.example/state.json")).unwrap(),
        br#"{"state":true}"#
    );
    assert_eq!(
        fs::read(output.join("images/unreferenced.png")).unwrap(),
        [137, 80, 78, 71, 13, 10, 26, 10, 1, 2, 3]
    );

    write_ufo(
        &model,
        &model.default_master().id,
        &source_path,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    assert_eq!(
        fs::read(source_path.join("data/com.example/state.json")).unwrap(),
        br#"{"state":true}"#
    );
    assert_eq!(
        fs::read(source_path.join("images/unreferenced.png")).unwrap(),
        [137, 80, 78, 71, 13, 10, 26, 10, 1, 2, 3]
    );
}

#[test]
fn extras_are_compact_first_class_and_not_duplicated_in_ufo_lib() {
    let mut font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let master_id = font.default_master().id.clone();
    font.extra
        .raw_tables
        .insert(Tag(*b"TEST"), vec![1, 2, 3, 4]);
    font.bitmap_strikes.push(BitmapStrike {
        ppem: 12,
        depth: 1,
        ..BitmapStrike::default()
    });
    {
        let layer = font
            .glyph_mut("A")
            .unwrap()
            .master_layer_mut(&master_id)
            .unwrap();
        layer.tt_instructions = Some(vec![0xB0, 0x01]);
        layer.hints.h_stems.push(StemHint {
            start: 10.0,
            width: 20.0,
            ranges: vec![(0.0, 100.0)],
        });
        layer.contours[0].points[0].tt_index = TtIndex::Index(u16::MAX);
        layer.contours[0].points[1].tt_index = TtIndex::Implied;
    }
    {
        let component = &mut font
            .glyph_mut("B")
            .unwrap()
            .master_layer_mut(&master_id)
            .unwrap()
            .components[0];
        component.anchor = Some("top".to_owned());
        component.auto_align = true;
        component.use_my_metrics = true;
        component.round_to_grid = true;
        component.smart = Some([("Width".to_owned(), 42.0)].into_iter().collect());
        component.point_match = Some((3, 7));
    }

    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("Extras.ufo");
    write_ufo(&font, &master_id, &output, &UfoWriteOptions::default()).unwrap();
    let written = norad::Font::load(&output).unwrap();
    let extra = written.lib["com.fontcutter.extra"].as_dictionary().unwrap();
    let packed = extra["tt_indices"].as_string();
    assert!(packed.is_some());

    let round_trip = read_ufo(&output).unwrap();
    assert_eq!(round_trip.extra.raw_tables, font.extra.raw_tables);
    assert_eq!(round_trip.bitmap_strikes, font.bitmap_strikes);
    let round_layer = round_trip
        .glyph("A")
        .unwrap()
        .master_layer(&round_trip.default_master().id)
        .unwrap();
    assert_eq!(
        round_layer.tt_instructions.as_deref(),
        Some([0xB0, 0x01].as_slice())
    );
    assert_eq!(
        round_layer.hints,
        font.glyph("A")
            .unwrap()
            .master_layer(&master_id)
            .unwrap()
            .hints
    );
    assert_eq!(
        round_layer.contours[0].points[0].tt_index,
        TtIndex::Index(u16::MAX)
    );
    assert_eq!(round_layer.contours[0].points[1].tt_index, TtIndex::Implied);
    assert_eq!(
        round_trip
            .glyph("B")
            .unwrap()
            .master_layer(&round_trip.default_master().id)
            .unwrap()
            .components,
        font.glyph("B")
            .unwrap()
            .master_layer(&master_id)
            .unwrap()
            .components
    );
    assert!(!round_trip
        .extra
        .lib
        .get("ufo.lib")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|lib| lib.contains_key("com.fontcutter.extra")));
}

#[test]
fn pristine_ufo_does_not_gain_an_empty_fontcutter_extra() {
    let font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("Pristine.ufo");
    write_ufo(
        &font,
        &font.default_master().id,
        &output,
        &UfoWriteOptions::default(),
    )
    .unwrap();

    let written = norad::Font::load(output).unwrap();
    assert!(!written.lib.contains_key("com.fontcutter.extra"));
}

#[test]
fn orthogonal_angle_guides_keep_both_coordinates() {
    let mut font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    font.guides = vec![
        Guide {
            x: 123.0,
            y: 456.0,
            angle: 0.0,
            ..Guide::default()
        },
        Guide {
            x: 321.0,
            y: 654.0,
            angle: 90.0,
            ..Guide::default()
        },
    ];
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("Guides.ufo");
    write_ufo(
        &font,
        &font.default_master().id,
        &output,
        &UfoWriteOptions::default(),
    )
    .unwrap();

    let written = norad::Font::load(output).unwrap();
    let guides = written.font_info.guidelines.unwrap();
    assert_eq!(
        guides[0].line,
        norad::Line::Angle {
            x: 123.0,
            y: 456.0,
            degrees: 0.0
        }
    );
    assert_eq!(
        guides[1].line,
        norad::Line::Angle {
            x: 321.0,
            y: 654.0,
            degrees: 90.0
        }
    );
}

#[test]
fn incremental_write_touches_only_changed_glif_and_removes_stale_glifs() {
    let mut model = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
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
    let outcome = fc_sfnt::read_path(
        &fc_fixtures::path("DejaVuSans.ttf"),
        &fc_sfnt::ReadOptions {
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
    assert!(started.elapsed() < Duration::from_secs(10));
    let lib_size = fs::metadata(output.join("lib.plist")).unwrap().len();
    assert!(lib_size < 1_000_000, "lib.plist is {lib_size} bytes");
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
