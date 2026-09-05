use std::path::{Path, PathBuf};

use fc_formats::{
    ufo::{read_designspace, read_ufo, write_designspace},
    Error,
};
use fc_model::{Axis, Font, Instance, LayerKind, MasterId, Tag};
use norad::designspace::{
    Axis as NoradAxis, DesignSpaceDocument, Dimension, Instance as NoradInstance, LocalizedString,
    Source,
};

#[test]
fn axisless_fonts_are_written_as_ufo_instead_of_an_invalid_designspace() {
    let font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let temp = tempfile::tempdir().unwrap();

    let error = write_designspace(&font, temp.path()).unwrap_err();

    assert!(matches!(error, Error::NoAxes));
    assert!(!temp.path().join("Minimal.designspace").exists());
    let ufo_path = temp.path().join("Minimal-Regular.ufo");
    assert!(ufo_path.is_dir());
    let round_trip = read_ufo(&ufo_path).unwrap();
    assert_eq!(round_trip.info.family_name, "Minimal");
    assert_eq!(round_trip.default_master().name, "Regular");
}

#[test]
fn two_master_designspace_round_trip() {
    let mut font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".to_owned(),
        min: 100.0,
        default: 400.0,
        max: 900.0,
        map: vec![(100.0, 0.0), (400.0, 400.0), (900.0, 1_000.0)],
        ..Axis::default()
    });
    font.masters[0].name = "Regular".to_owned();
    font.masters[0].location.insert(Tag(*b"wght"), 400.0);
    let first_id = font.masters[0].id.clone();
    let mut bold = font.masters[0].clone();
    bold.id = MasterId::new();
    bold.name = "Bold".to_owned();
    bold.location.insert(Tag(*b"wght"), 1_000.0);
    for glyph in font.glyphs.values_mut() {
        let mut layer = glyph.master_layer(&first_id).unwrap().clone();
        layer.kind = LayerKind::Master(bold.id.clone());
        layer.width += 40.0;
        glyph.layers.push(layer);
    }
    let mut brace = font
        .glyph("A")
        .unwrap()
        .master_layer(&first_id)
        .unwrap()
        .clone();
    brace.name = "intermediate".to_owned();
    brace.kind = LayerKind::Brace {
        master: first_id.clone(),
        location: [(Tag(*b"wght"), 650.0)].into_iter().collect(),
    };
    brace.width += 20.0;
    font.glyph_mut("A").unwrap().layers.push(brace);
    font.masters.push(bold);
    font.instances.push(Instance {
        name: "Semibold".to_owned(),
        style_name: "Semibold".to_owned(),
        location: [(Tag(*b"wght"), 700.0)].into_iter().collect(),
        active: true,
        is_variable_named_instance: true,
        ..Instance::default()
    });

    let temp = tempfile::tempdir().unwrap();
    let designspace = write_designspace(&font, temp.path()).unwrap();
    assert_eq!(designspace.file_name().unwrap(), "Minimal.designspace");
    let round_trip = read_designspace(&designspace).unwrap();
    assert_eq!(round_trip.masters.len(), 2);
    assert_eq!(round_trip.masters[0].name, "Regular");
    assert_eq!(round_trip.masters[1].name, "Bold");
    assert_eq!(round_trip.axes, font.axes);
    assert_eq!(round_trip.instances.len(), 1);
    assert_eq!(round_trip.instances[0].name, "Semibold");
    let regular_width = round_trip
        .glyph("A")
        .unwrap()
        .master_layer(&round_trip.masters[0].id)
        .unwrap()
        .width;
    let bold_width = round_trip
        .glyph("A")
        .unwrap()
        .master_layer(&round_trip.masters[1].id)
        .unwrap()
        .width;
    assert_eq!(bold_width - regular_width, 40.0);
    assert!(round_trip.glyph("A").unwrap().layers.iter().any(|layer| {
        matches!(
            &layer.kind,
            LayerKind::Brace { location, .. }
                if location.get(&Tag(*b"wght")) == Some(&650.0)
        )
    }));
}

#[test]
fn brace_layers_do_not_duplicate_across_two_round_trips() {
    let temp = tempfile::tempdir().unwrap();
    let source = write_source_designspace(temp.path());
    let imported = read_designspace(&source).unwrap();
    let first_signatures = layer_signatures(&imported, "A");
    assert_eq!(
        first_signatures
            .iter()
            .filter(|signature| signature.starts_with("brace:"))
            .count(),
        1
    );
    assert!(!first_signatures
        .iter()
        .any(|signature| signature == "custom:{650}"));

    let first_dir = temp.path().join("first");
    let first_path = write_designspace(&imported, &first_dir).unwrap();
    let first = read_designspace(&first_path).unwrap();
    let second_dir = temp.path().join("second");
    let second_path = write_designspace(&first, &second_dir).unwrap();
    let second = read_designspace(&second_path).unwrap();

    assert_eq!(
        layer_signatures(&first, "A"),
        layer_signatures(&second, "A")
    );
    assert_eq!(
        layer_signatures(&imported, "A"),
        layer_signatures(&first, "A")
    );
}

#[test]
fn custom_layers_are_exported_only_with_their_owning_master() {
    let temp = tempfile::tempdir().unwrap();
    let source = write_source_designspace(temp.path());
    let mut font = read_designspace(&source).unwrap();
    let regular_id = font.masters[0].id.clone();
    let sketch = font
        .glyph("A")
        .unwrap()
        .layers
        .iter()
        .find(|layer| matches!(&layer.kind, LayerKind::Custom(name) if name == "Sketch"))
        .unwrap();
    assert_eq!(
        sketch
            .user_data
            .get("ufo.master")
            .and_then(serde_json::Value::as_str),
        Some(regular_id.to_string().as_str())
    );

    let mut ownerless = font
        .glyph("A")
        .unwrap()
        .master_layer(&regular_id)
        .unwrap()
        .clone();
    ownerless.kind = LayerKind::Custom("Ownerless".to_owned());
    ownerless.name = "Ownerless".to_owned();
    ownerless.user_data = serde_json::Value::Null;
    font.glyph_mut("A").unwrap().layers.push(ownerless);

    let output = temp.path().join("output");
    write_designspace(&font, &output).unwrap();
    let regular = norad::Font::load(output.join("Minimal-Regular.ufo")).unwrap();
    let bold = norad::Font::load(output.join("Minimal-Bold.ufo")).unwrap();
    assert!(regular.layers.get("Sketch").is_some());
    assert!(regular.layers.get("Ownerless").is_some());
    assert!(bold.layers.get("Sketch").is_none());
    assert!(bold.layers.get("Ownerless").is_none());
}

#[test]
fn each_master_preserves_and_exports_its_own_fontinfo() {
    let temp = tempfile::tempdir().unwrap();
    let source = write_source_designspace(temp.path());
    let font = read_designspace(&source).unwrap();

    assert_eq!(font.masters[0].metrics.ascender, 800.0);
    assert_eq!(font.masters[1].metrics.ascender, 900.0);
    assert_eq!(master_fontinfo_year(&font, 0), Some(2020));
    assert_eq!(master_fontinfo_year(&font, 1), Some(2021));

    let output = temp.path().join("fontinfo-output");
    write_designspace(&font, &output).unwrap();
    let regular = norad::Font::load(output.join("Minimal-Regular.ufo")).unwrap();
    let bold = norad::Font::load(output.join("Minimal-Bold.ufo")).unwrap();
    assert_eq!(regular.font_info.style_name.as_deref(), Some("Regular"));
    assert_eq!(bold.font_info.style_name.as_deref(), Some("Bold"));
    assert_eq!(regular.font_info.ascender, Some(800.0));
    assert_eq!(bold.font_info.ascender, Some(900.0));
    assert_eq!(regular.font_info.year, Some(2020));
    assert_eq!(bold.font_info.year, Some(2021));
}

#[test]
fn duplicate_master_names_get_distinct_ufo_filenames() {
    let mut font = two_master_font();
    font.masters[1].name = "Regular".to_owned();
    let temp = tempfile::tempdir().unwrap();

    let path = write_designspace(&font, temp.path()).unwrap();
    let document = DesignSpaceDocument::load(path).unwrap();

    assert_eq!(document.sources[0].filename, "Minimal-Regular.ufo");
    assert_eq!(document.sources[1].filename, "Minimal-Regular-2.ufo");
    let first = norad::Font::load(temp.path().join(&document.sources[0].filename)).unwrap();
    let second = norad::Font::load(temp.path().join(&document.sources[1].filename)).unwrap();
    assert_eq!(first.get_glyph("A").unwrap().width, 600.0);
    assert_eq!(second.get_glyph("A").unwrap().width, 640.0);
}

#[test]
fn discrete_axis_and_instance_metadata_survive_round_trip() {
    let temp = tempfile::tempdir().unwrap();
    let ufo_path = temp.path().join("Minimal-Regular.ufo");
    let ufo = norad::Font::load(fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    ufo.save(&ufo_path).unwrap();
    let axis = NoradAxis {
        name: "Weight".to_owned(),
        tag: "wght".to_owned(),
        default: 400.0,
        minimum: None,
        maximum: None,
        values: Some(vec![100.0, 400.0, 900.0]),
        label_names: vec![LocalizedString {
            language: "fr".to_owned(),
            string: "Graisse".to_owned(),
        }],
        ..NoradAxis::default()
    };
    let instance = NoradInstance {
        familyname: Some("Minimal Display".to_owned()),
        stylename: Some("Bold".to_owned()),
        name: Some("Minimal Display Bold".to_owned()),
        filename: Some("instances/MinimalDisplay-Bold.ufo".to_owned()),
        postscriptfontname: Some("MinimalDisplay-Bold".to_owned()),
        stylemapfamilyname: Some("Minimal Display".to_owned()),
        stylemapstylename: Some("bold".to_owned()),
        location: vec![dimension("Weight", 900.0)],
        lib: [("com.example.keep".to_owned(), plist::Value::Boolean(true))]
            .into_iter()
            .collect(),
    };
    let document = DesignSpaceDocument {
        format: 5.0,
        axes: vec![axis.clone()],
        sources: vec![source("Minimal-Regular.ufo", "Regular", 400.0)],
        instances: vec![instance.clone()],
        ..DesignSpaceDocument::default()
    };
    let input = temp.path().join("input.designspace");
    document.save(&input).unwrap();

    let font = read_designspace(&input).unwrap();
    let output_dir = temp.path().join("output");
    let output = write_designspace(&font, &output_dir).unwrap();
    let round_trip = DesignSpaceDocument::load(output).unwrap();

    assert_eq!(round_trip.axes[0].values, axis.values);
    assert_eq!(round_trip.axes[0].label_names, axis.label_names);
    assert_eq!(round_trip.instances[0].filename, instance.filename);
    assert_eq!(
        round_trip.instances[0].postscriptfontname,
        instance.postscriptfontname
    );
    assert_eq!(
        round_trip.instances[0].stylemapfamilyname,
        instance.stylemapfamilyname
    );
    assert_eq!(
        round_trip.instances[0].stylemapstylename,
        instance.stylemapstylename
    );
    assert_eq!(round_trip.instances[0].lib, instance.lib);
}

fn write_source_designspace(root: &Path) -> PathBuf {
    let regular_path = root.join("Minimal-Regular.ufo");
    let mut regular = norad::Font::load(fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    regular.font_info.style_name = Some("Regular".to_owned());
    regular.font_info.ascender = Some(800.0);
    regular.font_info.year = Some(2020);
    let mut brace_a = regular.get_glyph("A").unwrap().clone();
    brace_a.width = 620.0;
    regular
        .layers
        .get_or_create_layer("{650}")
        .unwrap()
        .insert_glyph(brace_a);
    let mut sketch_a = regular.get_glyph("A").unwrap().clone();
    sketch_a.width = 610.0;
    regular
        .layers
        .get_or_create_layer("Sketch")
        .unwrap()
        .insert_glyph(sketch_a);
    regular.save(&regular_path).unwrap();

    let bold_path = root.join("Minimal-Bold.ufo");
    let mut bold = norad::Font::load(fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    bold.font_info.style_name = Some("Bold".to_owned());
    bold.font_info.ascender = Some(900.0);
    bold.font_info.year = Some(2021);
    bold.get_glyph_mut("A").unwrap().width = 640.0;
    bold.save(&bold_path).unwrap();

    let document = DesignSpaceDocument {
        format: 5.0,
        axes: vec![NoradAxis {
            name: "Weight".to_owned(),
            tag: "wght".to_owned(),
            default: 400.0,
            minimum: Some(100.0),
            maximum: Some(900.0),
            ..NoradAxis::default()
        }],
        sources: vec![
            source("Minimal-Regular.ufo", "Regular", 400.0),
            source("Minimal-Bold.ufo", "Bold", 900.0),
            Source {
                familyname: Some("Minimal".to_owned()),
                name: Some("Regular {650}".to_owned()),
                filename: "Minimal-Regular.ufo".to_owned(),
                layer: Some("{650}".to_owned()),
                location: vec![dimension("Weight", 650.0)],
                ..Source::default()
            },
        ],
        ..DesignSpaceDocument::default()
    };
    let path = root.join("Minimal.designspace");
    document.save(&path).unwrap();
    path
}

fn two_master_font() -> Font {
    let mut font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".to_owned(),
        min: 100.0,
        default: 400.0,
        max: 900.0,
        ..Axis::default()
    });
    font.masters[0].name = "Regular".to_owned();
    font.masters[0].location.insert(Tag(*b"wght"), 400.0);
    let first_id = font.masters[0].id.clone();
    let mut second = font.masters[0].clone();
    second.id = MasterId::new();
    second.name = "Bold".to_owned();
    second.location.insert(Tag(*b"wght"), 900.0);
    for glyph in font.glyphs.values_mut() {
        let mut layer = glyph.master_layer(&first_id).unwrap().clone();
        layer.kind = LayerKind::Master(second.id.clone());
        layer.width += 40.0;
        glyph.layers.push(layer);
    }
    font.masters.push(second);
    font
}

fn source(filename: &str, name: &str, weight: f32) -> Source {
    Source {
        familyname: Some("Minimal".to_owned()),
        stylename: Some(name.to_owned()),
        name: Some(name.to_owned()),
        filename: filename.to_owned(),
        location: vec![dimension("Weight", weight)],
        ..Source::default()
    }
}

fn dimension(name: &str, value: f32) -> Dimension {
    Dimension {
        name: name.to_owned(),
        xvalue: Some(value),
        ..Dimension::default()
    }
}

fn layer_signatures(font: &Font, glyph_name: &str) -> Vec<String> {
    let glyph = font.glyph(glyph_name).unwrap();
    glyph
        .layers
        .iter()
        .map(|layer| match &layer.kind {
            LayerKind::Master(id) => format!(
                "master:{}",
                font.masters
                    .iter()
                    .position(|master| &master.id == id)
                    .unwrap()
            ),
            LayerKind::Brace { master, location } => format!(
                "brace:{}:{:?}",
                font.masters
                    .iter()
                    .position(|candidate| &candidate.id == master)
                    .unwrap(),
                location
            ),
            LayerKind::Custom(name) => format!("custom:{name}"),
            other => format!("{other:?}"),
        })
        .collect()
}

fn master_fontinfo_year(font: &Font, index: usize) -> Option<i64> {
    font.masters[index]
        .user_data
        .get("ufo.fontinfo")
        .and_then(|fontinfo| fontinfo.get("year"))
        .and_then(serde_json::Value::as_i64)
}
