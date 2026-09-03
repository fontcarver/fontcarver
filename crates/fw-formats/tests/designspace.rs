use fw_formats::ufo::{read_designspace, read_ufo, write_designspace};
use fw_model::{Axis, Instance, LayerKind, MasterId, Tag};

#[test]
fn two_master_designspace_round_trip() {
    let mut font = read_ufo(&fw_fixtures::ufo_path("Minimal.ufo")).unwrap();
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
