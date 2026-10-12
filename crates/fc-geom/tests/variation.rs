use fc_geom::variation::interpolate_sources;
use fc_model::{Axis, Layer, Tag};

#[test]
fn sparse_two_axis_interpolation_respects_intermediate_peak_and_metrics() {
    let axes: Vec<_> = [*b"wght", *b"wdth"]
        .into_iter()
        .map(|tag| Axis {
            tag: Tag(tag),
            name: String::from_utf8(tag.to_vec()).unwrap(),
            min: 0.0,
            default: 0.0,
            max: 1.0,
            ..Default::default()
        })
        .collect();
    let mut layers: Vec<_> = [500.0, 700.0, 600.0, 900.0, 650.0]
        .into_iter()
        .map(|width| {
            let mut layer = Layer::new_master(fc_model::MasterId("regular".into()));
            layer.width = width;
            layer.vertical_width = Some(width * 2.0);
            layer
        })
        .collect();
    let locations: Vec<_> = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.5, 0.0)]
        .into_iter()
        .map(|(w, d)| [(Tag(*b"wght"), w), (Tag(*b"wdth"), d)].into())
        .collect();
    let sources: Vec<_> = locations.iter().cloned().zip(layers.iter()).collect();
    let at = [(Tag(*b"wght"), 0.5), (Tag(*b"wdth"), 0.5)].into();
    let middle = interpolate_sources(&axes, &sources, &at).unwrap();
    assert_eq!(middle.width, 725.0);
    assert_eq!(middle.vertical_width, Some(1450.0));
    for (location, layer) in &sources {
        assert_eq!(
            interpolate_sources(&axes, &sources, location)
                .unwrap()
                .width,
            layer.width
        );
    }
    let default = layers.remove(0);
    assert!(interpolate_sources(&axes, &[(locations[1].clone(), &default)], &at).is_err());
}

#[test]
fn rejects_duplicate_source_and_nonfinite_outline_values() {
    let axis = Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 0.0,
        default: 0.0,
        max: 1.0,
        ..Default::default()
    };
    let mut layer = Layer::new_master(fc_model::MasterId("regular".into()));
    let location = Default::default();
    assert!(interpolate_sources(
        std::slice::from_ref(&axis),
        &[(location, &layer), (Default::default(), &layer)],
        &Default::default()
    )
    .is_err());
    layer.width = f64::NAN;
    assert!(interpolate_sources(
        &[axis],
        &[(Default::default(), &layer)],
        &Default::default()
    )
    .is_err());
}

#[test]
fn preview_resolves_mixed_vertical_overrides_against_master_defaults() {
    let mut font = fc_model::Font::new_empty("Vertical preview", 1000);
    font.masters[0].metrics.ascender = 800.;
    font.masters[0].metrics.descender = -200.;
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 0.,
        default: 0.,
        max: 1.,
        ..Default::default()
    });
    let mut bold = font.masters[0].clone();
    bold.id = fc_model::MasterId::new();
    bold.name = "Bold".into();
    bold.location.insert(Tag(*b"wght"), 1.);
    bold.metrics.default_vertical_origin = Some(900.);
    let mut glyph = fc_model::Glyph::new("A");
    let mut regular = Layer::new_master(font.masters[0].id.clone());
    regular.vertical_width = Some(1200.);
    glyph.layers.push(regular);
    glyph.layers.push(Layer::new_master(bold.id.clone()));
    font.masters.push(bold);
    font.insert_glyph(glyph).unwrap();
    let layer = fc_geom::variation::interpolate_glyph(
        &font,
        font.glyph("A").unwrap(),
        &[(Tag(*b"wght"), 0.5)].into(),
    )
    .unwrap();
    assert_eq!(layer.vertical_width, Some(1100.));
    assert_eq!(layer.vertical_origin, Some(850.));
    assert_eq!(font.glyph("A").unwrap().layers[1].vertical_width, None);
}
