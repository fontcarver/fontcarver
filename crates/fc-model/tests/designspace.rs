use fc_model::{Axis, Font, Tag};

fn mapped_font() -> Font {
    let mut font = Font::new_empty("Mapped", 1000);
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.0,
        default: 400.0,
        max: 900.0,
        map: vec![(100.0, 0.0), (400.0, 50.0), (900.0, 100.0)],
        ..Default::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 50.0);
    font
}

#[test]
fn maps_user_coordinates_and_finds_default_independent_of_order() {
    let mut font = mapped_font();
    let default = font.masters[0].id.clone();
    let mut heavy = font.masters[0].clone();
    heavy.id = fc_model::MasterId("heavy".into());
    heavy.name = "Heavy".into();
    heavy.location.insert(Tag(*b"wght"), 100.0);
    font.masters.insert(0, heavy);
    font.validate_designspace().unwrap();
    assert_eq!(font.design_default_master().unwrap().id, default);
    let axis = &font.axes[0];
    assert_eq!(axis.user_to_design(650.0), 75.0);
    assert_eq!(axis.design_to_user(75.0), 650.0);
    assert_eq!(axis.normalize_design(75.0), 0.5);
    assert_eq!(
        font.complete_location(&Default::default()).unwrap()[&axis.tag],
        50.0
    );
}

#[test]
fn rejects_duplicate_sources_unknown_coordinates_and_nonmonotonic_mapping() {
    let mut font = mapped_font();
    let mut duplicate = font.masters[0].clone();
    duplicate.id = fc_model::MasterId("duplicate".into());
    font.masters.push(duplicate);
    assert!(font.validate_designspace().is_err());
    font.masters.pop();
    assert!(font
        .complete_location(&[(Tag(*b"wdth"), 1.0)].into())
        .is_err());
    assert!(font
        .complete_location(&[(Tag(*b"wght"), 101.0)].into())
        .is_err());
    font.axes[0].map[1].1 = 101.0;
    assert!(font.validate_designspace().is_err());
}

#[test]
fn intermediate_sources_inherit_other_master_axes() {
    let mut font = mapped_font();
    font.axes.push(Axis {
        tag: Tag(*b"wdth"),
        name: "Width".into(),
        min: 50.0,
        default: 100.0,
        max: 200.0,
        ..Default::default()
    });
    font.masters[0].location.insert(Tag(*b"wdth"), 180.0);
    let location = font
        .intermediate_location(&font.masters[0].id, &[(Tag(*b"wght"), 75.0)].into())
        .unwrap();
    assert_eq!(location[&Tag(*b"wdth")], 180.0);
    assert_eq!(location[&Tag(*b"wght")], 75.0);
}

#[test]
fn rejects_intermediates_that_duplicate_a_master_or_another_source() {
    let mut font = mapped_font();
    let master = font.masters[0].id.clone();
    let mut glyph = fc_model::Glyph::new("A");
    glyph
        .layers
        .push(fc_model::Layer::new_master(master.clone()));
    let mut brace = fc_model::Layer::new_master(master.clone());
    brace.kind = fc_model::LayerKind::Brace {
        master: master.clone(),
        location: Default::default(),
    };
    glyph.layers.push(brace);
    font.insert_glyph(glyph).unwrap();
    assert!(font.validate_designspace().is_err());
    font.glyph_mut("A").unwrap().layers[1].kind = fc_model::LayerKind::Brace {
        master,
        location: [(Tag(*b"wght"), 75.)].into(),
    };
    font.validate_designspace().unwrap();
    let mut duplicate = font.glyph("A").unwrap().layers[1].clone();
    duplicate.id = fc_model::LayerId::new();
    font.glyph_mut("A").unwrap().layers.push(duplicate);
    assert!(font.validate_designspace().is_err());
}
