use fc_model::{
    dependents, flatten_layer, glyph_for_unicode, layer_bbox, masters_compatible, validate,
    Component, Contour, Error, Glyph, Incompatibility, Issue, KernSide, Master, MasterId, Point,
    PointKind, TtIndex,
};
use kurbo::{Affine, PathEl, Shape};

fn point(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        x,
        y,
        kind,
        smooth: false,
        name: None,
        tt_index: TtIndex::Unnumbered,
    }
}

fn triangle() -> Contour {
    Contour {
        closed: true,
        points: vec![
            point(0.0, 0.0, PointKind::Line),
            point(100.0, 0.0, PointKind::Line),
            point(0.0, 100.0, PointKind::Line),
        ],
        ..Contour::default()
    }
}

#[test]
fn flatten_layer_recursively_applies_component_transforms() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    let master = font.default_master().id.clone();
    let mut base = Glyph::new("base");
    base.layers
        .push(fc_model::Layer::new_master(master.clone()));
    base.master_layer_mut(&master)
        .unwrap()
        .contours
        .push(triangle());
    font.insert_glyph(base).unwrap();

    let mut middle = Glyph::new("middle");
    middle
        .layers
        .push(fc_model::Layer::new_master(master.clone()));
    middle
        .master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "base".into(),
            transform: Affine::translate((10.0, 20.0)),
            ..Component::default()
        });
    font.insert_glyph(middle).unwrap();

    let mut composite = Glyph::new("composite");
    composite
        .layers
        .push(fc_model::Layer::new_master(master.clone()));
    composite
        .master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "middle".into(),
            transform: Affine::scale(2.0),
            ..Component::default()
        });
    font.insert_glyph(composite).unwrap();

    let flattened = flatten_layer(&font, "composite", &master).unwrap();
    assert_eq!(flattened.elements()[0], PathEl::MoveTo((20.0, 40.0).into()));
    assert_eq!(flattened.bounding_box().x1, 220.0);
    assert_eq!(flattened.bounding_box().y1, 240.0);
    assert_eq!(
        layer_bbox(&font, "composite", &master).unwrap(),
        Some(flattened.bounding_box())
    );
}

#[test]
fn flatten_layer_detects_component_cycles() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    let master = font.default_master().id.clone();
    for name in ["A", "B"] {
        font.insert_glyph(Glyph::new(name)).unwrap();
    }
    for (name, base) in [("A", "B"), ("B", "A")] {
        font.glyph_mut(name)
            .unwrap()
            .master_layer_mut(&master)
            .unwrap()
            .components
            .push(Component {
                base: base.into(),
                ..Component::default()
            });
    }

    assert!(matches!(
        flatten_layer(&font, "A", &master),
        Err(Error::ComponentCycle { glyph }) if glyph == "A".into() || glyph == "B".into()
    ));
}

#[test]
fn validate_reports_dangling_components() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("A");
    glyph
        .layers
        .push(fc_model::Layer::new_master(master.clone()));
    glyph
        .master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "missing".into(),
            ..Component::default()
        });
    font.insert_glyph(glyph).unwrap();

    assert!(validate(&font).iter().any(|issue| matches!(
        issue,
        Issue::DanglingComponent { glyph, base }
            if glyph == &"A".into() && base == &"missing".into()
    )));
    assert!(matches!(
        flatten_layer(&font, "A", &master),
        Err(Error::MissingGlyph { glyph }) if glyph == "missing".into()
    ));
}

#[test]
fn relationship_queries_follow_glyph_order_and_include_color_components() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    let master = font.default_master().id.clone();
    let mut first = Glyph::new("first");
    first.unicodes.push(0x41);
    font.insert_glyph(first).unwrap();
    let mut second = Glyph::new("second");
    second.unicodes.push(0x41);
    second
        .layers
        .push(fc_model::Layer::new_master(master.clone()));
    second.master_layer_mut(&master).unwrap().color_layers = Some(vec![fc_model::ColorLayer {
        components: vec![Component {
            base: "first".into(),
            ..Component::default()
        }],
        ..fc_model::ColorLayer::default()
    }]);
    font.insert_glyph(second).unwrap();

    assert_eq!(glyph_for_unicode(&font, 0x41).unwrap().name, "first".into());
    assert_eq!(dependents(&font)["first"], ["second".into()]);
}

#[test]
fn compatibility_and_validation_report_topology_unicode_and_group_issues() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    let first_master = font.default_master().id.clone();
    let second_master = MasterId::new();
    font.add_master(Master {
        id: second_master.clone(),
        name: "Bold".into(),
        ..Master::default()
    });
    let mut a = Glyph::new("A");
    a.unicodes.push(0x41);
    font.insert_glyph(a).unwrap();
    font.glyph_mut("A")
        .unwrap()
        .master_layer_mut(&first_master)
        .unwrap()
        .contours
        .push(triangle());
    font.glyph_mut("A")
        .unwrap()
        .master_layer_mut(&second_master)
        .unwrap()
        .contours
        .push(Contour {
            closed: true,
            points: vec![point(0.0, 0.0, PointKind::Line)],
            ..Contour::default()
        });
    let mut duplicate = Glyph::new("A.alt");
    duplicate.unicodes.push(0x41);
    font.insert_glyph(duplicate).unwrap();
    font.masters[0].kerning_h.0.insert(
        (
            KernSide::Group("missing.side1".into()),
            KernSide::Group("missing.side2".into()),
        ),
        -20.0,
    );

    let incompatibilities = masters_compatible(&font, "A").unwrap_err();
    assert!(incompatibilities.iter().any(|issue| matches!(
        issue,
        Incompatibility::PointCount { master, .. } if master == &second_master
    )));
    let issues = validate(&font);
    assert!(issues
        .iter()
        .any(|issue| matches!(issue, Issue::DuplicateUnicode { unicode: 0x41, .. })));
    assert_eq!(
        issues
            .iter()
            .filter(|issue| matches!(issue, Issue::MissingKerningGroup { .. }))
            .count(),
        2
    );
}

#[test]
fn flatten_layer_by_id_uses_the_named_layer_and_its_master() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    let master = font.default_master().id.clone();

    let mut base = Glyph::new("base");
    base.layers
        .push(fc_model::Layer::new_master(master.clone()));
    base.master_layer_mut(&master)
        .unwrap()
        .contours
        .push(triangle());
    font.insert_glyph(base).unwrap();

    let mut composite = Glyph::new("composite");
    composite
        .layers
        .push(fc_model::Layer::new_master(master.clone()));
    composite
        .master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "base".into(),
            ..Component::default()
        });
    let mut background = fc_model::Layer {
        kind: fc_model::LayerKind::Background(master.clone()),
        ..fc_model::Layer::default()
    };
    background.contours.push(triangle());
    background.components.push(Component {
        base: "base".into(),
        transform: Affine::translate((500.0, 0.0)),
        ..Component::default()
    });
    let background_id = background.id.clone();
    composite.layers.push(background);
    font.insert_glyph(composite).unwrap();

    let master_layer_id = font
        .glyph("composite")
        .unwrap()
        .master_layer(&master)
        .unwrap()
        .id
        .clone();
    let master_path = fc_model::flatten_layer_by_id(&font, "composite", &master_layer_id).unwrap();
    assert_eq!(master_path.bounding_box().x1, 100.0);

    let background_path =
        fc_model::flatten_layer_by_id(&font, "composite", &background_id).unwrap();
    assert_eq!(background_path.bounding_box().x1, 600.0);

    let missing = fc_model::flatten_layer_by_id(&font, "composite", &fc_model::LayerId::new());
    assert!(matches!(missing, Err(Error::MissingLayer { .. })));
}
