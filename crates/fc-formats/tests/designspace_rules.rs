use fc_formats::ufo::{read_designspace, write_designspace};
use fc_model::{Axis, Component, Font, Glyph, Layer, LayerKind, MasterId, Tag};

fn fixture() -> Font {
    let mut font = Font::new_empty("Native Brackets", 1000);
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.,
        default: 100.,
        max: 900.,
        map: vec![(100., 0.), (900., 1.)],
        ..Default::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 0.);
    let mut heavy = font.masters[0].clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(Tag(*b"wght"), 1.);
    font.masters.push(heavy);
    let mut glyph = Glyph::new("A");
    glyph.unicodes.push(0x41);
    for (index, master) in font.masters.iter().enumerate() {
        let mut normal = Layer::new_master(master.id.clone());
        normal.width = 500. + index as f64 * 100.;
        glyph.layers.push(normal);
        let mut alternate = Layer::new_master(master.id.clone());
        alternate.kind = LayerKind::Bracket {
            master: master.id.clone(),
            axis: Tag(*b"wght"),
            min: Some(0.5),
            max: None,
            extra: Vec::new(),
        };
        alternate.width = 900. + index as f64 * 100.;
        glyph.layers.push(alternate);
    }
    font.insert_glyph(glyph).unwrap();
    let mut composite = Glyph::new("Aacute");
    for master in &font.masters {
        let mut layer = Layer::new_master(master.id.clone());
        layer.width = 600.;
        layer.components.push(Component {
            base: "A".into(),
            ..Default::default()
        });
        composite.layers.push(layer);
    }
    font.insert_glyph(composite).unwrap();
    font
}

#[test]
fn native_rules_roundtrip_without_private_layer_roles_or_accumulating_aliases() {
    let font = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = write_designspace(&font, temp.path()).unwrap();
    let document = norad::designspace::DesignSpaceDocument::load(&path).unwrap();
    assert_eq!(document.rules.rules.len(), 1);
    let rule = &document.rules.rules[0];
    assert_eq!(rule.condition_sets[0].conditions[0].minimum, Some(0.5));
    assert_eq!(rule.substitutions.len(), 2);
    let ufo = norad::Font::load(temp.path().join(&document.sources[0].filename)).unwrap();
    let alias = &rule
        .substitutions
        .iter()
        .find(|s| s.name.as_str() == "A")
        .unwrap()
        .with;
    assert_eq!(ufo.default_layer().get_glyph(alias).unwrap().width, 900.);
    let imported = read_designspace(&path).unwrap();
    assert!(!imported.extra.lib.contains_key("ufo.designspace.rules"));
    assert_eq!(imported.glyphs.len(), font.glyphs.len());
    assert_eq!(
        imported
            .glyph("A")
            .unwrap()
            .layers
            .iter()
            .filter(|l| matches!(l.kind, LayerKind::Bracket { .. }))
            .count(),
        2
    );
    let next = tempfile::tempdir().unwrap();
    let path = write_designspace(&imported, next.path()).unwrap();
    let again = read_designspace(&path).unwrap();
    assert_eq!(again.glyphs.len(), font.glyphs.len());
    again.validate_designspace().unwrap();
    let instance = fc_compile::instantiate(&again, &[(Tag(*b"wght"), 0.75)].into(), None).unwrap();
    assert_eq!(instance.glyph("A").unwrap().layers[0].width, 975.);
}

#[test]
fn external_rule_edits_take_precedence_and_unsupported_order_remains_lossless() {
    let font = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = write_designspace(&font, temp.path()).unwrap();
    let mut document = norad::designspace::DesignSpaceDocument::load(&path).unwrap();
    document.rules.rules[0].condition_sets[0].conditions[0].minimum = Some(0.8);
    document.save(&path).unwrap();
    let changed = read_designspace(&path).unwrap();
    let instance =
        fc_compile::instantiate(&changed, &[(Tag(*b"wght"), 0.75)].into(), None).unwrap();
    assert_eq!(instance.glyph("A").unwrap().layers[0].width, 575.);
    document.rules.processing = norad::designspace::RuleProcessing::Last;
    document.save(&path).unwrap();
    let preserved = read_designspace(&path).unwrap();
    assert!(preserved.extra.lib.contains_key("ufo.designspace.rules"));
    assert!(fc_compile::instantiate(&preserved, &[(Tag(*b"wght"), 0.75)].into(), None).is_err());
    let output = tempfile::tempdir().unwrap();
    let next = write_designspace(&preserved, output.path()).unwrap();
    assert_eq!(
        norad::designspace::DesignSpaceDocument::load(next)
            .unwrap()
            .rules,
        document.rules
    );
}

#[test]
fn compound_condition_sets_round_trip_through_native_designspace_rules() {
    let mut font = fixture();
    font.axes.push(Axis {
        tag: Tag(*b"wdth"),
        name: "Width".into(),
        min: 50.,
        default: 100.,
        max: 100.,
        map: Vec::new(),
        ..Default::default()
    });
    for master in &mut font.masters {
        master.location.insert(Tag(*b"wdth"), 100.);
    }
    // A narrow pair of masters, so the width axis actually varies.
    let narrow: Vec<MasterId> = font
        .masters
        .iter()
        .map(|_| MasterId::new())
        .collect::<Vec<_>>();
    let wide: Vec<(MasterId, f64)> = font
        .masters
        .iter()
        .map(|master| (master.id.clone(), master.location[&Tag(*b"wght")]))
        .collect();
    for (index, (source, wght)) in wide.iter().enumerate() {
        let mut master = font.master(source).unwrap().clone();
        master.id = narrow[index].clone();
        master.name = format!("Narrow {index}");
        master.location.insert(Tag(*b"wdth"), 50.);
        master.location.insert(Tag(*b"wght"), *wght);
        for glyph in font.glyphs.values_mut() {
            let mut layer = glyph.master_layer(source).unwrap().clone();
            layer.id = fc_model::LayerId::new();
            layer.kind = LayerKind::Master(master.id.clone());
            glyph.layers.push(layer);
        }
        font.masters.push(master);
    }
    // Replace the single-axis alternates with a compound region.
    let conditions = [
        fc_model::BracketCondition {
            axis: Tag(*b"wght"),
            min: Some(0.5),
            max: None,
        },
        fc_model::BracketCondition {
            axis: Tag(*b"wdth"),
            min: Some(50.),
            max: Some(75.),
        },
    ];
    let masters: Vec<MasterId> = font.masters.iter().map(|m| m.id.clone()).collect();
    {
        let glyph = font.glyph_mut("A").unwrap();
        glyph
            .layers
            .retain(|layer| !matches!(layer.kind, LayerKind::Bracket { .. }));
        let mut alternates = Vec::new();
        for master in &masters {
            let mut layer = glyph.master_layer(master).unwrap().clone();
            layer.id = fc_model::LayerId::new();
            layer.kind = LayerKind::from_bracket_conditions(master.clone(), &conditions).unwrap();
            layer.width = 950.;
            alternates.push(layer);
        }
        glyph.layers.extend(alternates);
    }
    font.validate_designspace().unwrap();

    let temp = tempfile::tempdir().unwrap();
    let path = write_designspace(&font, temp.path()).unwrap();
    let document = norad::designspace::DesignSpaceDocument::load(&path).unwrap();
    assert_eq!(document.rules.rules.len(), 1);
    let set = &document.rules.rules[0].condition_sets[0];
    assert_eq!(set.conditions.len(), 2, "both axis ranges are written");
    assert!(set
        .conditions
        .iter()
        .any(|condition| condition.name == "Width" && condition.maximum == Some(75.)));

    let imported = read_designspace(&path).unwrap();
    assert!(!imported.extra.lib.contains_key("ufo.designspace.rules"));
    let restored = imported
        .glyph("A")
        .unwrap()
        .layers
        .iter()
        .filter_map(|layer| layer.kind.bracket_conditions())
        .next()
        .expect("a compound alternate survives the round trip");
    assert_eq!(restored.len(), 2);
    assert_eq!(restored[0].axis, Tag(*b"wdth"));
    assert_eq!(restored[0].max, Some(75.));
    assert_eq!(restored[1].axis, Tag(*b"wght"));
    assert_eq!(restored[1].min, Some(0.5));
}
