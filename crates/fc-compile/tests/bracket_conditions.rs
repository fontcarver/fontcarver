//! Compound alternate-layer conditions: several axes in one condition set.

mod common;

use fc_model::{BracketCondition, Font, Glyph, Instance, Layer, LayerId, LayerKind, MasterId, Tag};
use read_fonts::{FontRef, TableProvider};

/// Two axes with four corner masters, so a compound region is meaningful.
fn two_axis_font() -> Font {
    let mut font = common::scratch_font();
    for (tag, name, min, default, max) in [
        (*b"wght", "Weight", 100.0, 100.0, 900.0),
        (*b"wdth", "Width", 50.0, 100.0, 100.0),
    ] {
        font.axes.push(fc_model::Axis {
            tag: Tag(tag),
            name: name.into(),
            min,
            default,
            max,
            map: Vec::new(),
            ..Default::default()
        });
    }
    let regular = font.masters[0].id.clone();
    font.masters[0].name = "Regular".into();
    font.masters[0].location.insert(Tag(*b"wght"), 100.0);
    font.masters[0].location.insert(Tag(*b"wdth"), 100.0);
    for (name, wght, wdth) in [
        ("Bold", 900.0, 100.0),
        ("Condensed", 100.0, 50.0),
        ("Bold Condensed", 900.0, 50.0),
    ] {
        let mut master = font.masters[0].clone();
        master.id = MasterId::new();
        master.name = name.into();
        master.location.insert(Tag(*b"wght"), wght);
        master.location.insert(Tag(*b"wdth"), wdth);
        for glyph in font.glyphs.values_mut() {
            let mut layer = glyph.master_layer(&regular).expect("regular").clone();
            layer.id = LayerId::new();
            layer.kind = LayerKind::Master(master.id.clone());
            glyph.layers.push(layer);
        }
        font.masters.push(master);
    }
    font
}

/// Add an alternate of `A` active only for heavy *and* narrow positions.
fn with_compound_alternate(font: &mut Font) {
    let conditions = [
        BracketCondition {
            axis: Tag(*b"wght"),
            min: Some(500.0),
            max: None,
        },
        BracketCondition {
            axis: Tag(*b"wdth"),
            min: Some(50.0),
            max: Some(75.0),
        },
    ];
    let masters: Vec<MasterId> = font
        .masters
        .iter()
        .map(|master| master.id.clone())
        .collect();
    let glyph = font.glyph_mut("A").expect("A");
    let mut alternates = Vec::new();
    for master in &masters {
        let mut layer = glyph.master_layer(master).expect("master layer").clone();
        layer.id = LayerId::new();
        layer.kind = LayerKind::from_bracket_conditions(master.clone(), &conditions)
            .expect("compound condition set");
        layer.width += 111.0;
        alternates.push(layer);
    }
    glyph.layers.extend(alternates);
}

#[test]
fn a_compound_condition_set_is_rejected_when_it_repeats_an_axis() {
    let repeated = [
        BracketCondition {
            axis: Tag(*b"wght"),
            min: Some(100.0),
            max: None,
        },
        BracketCondition {
            axis: Tag(*b"wght"),
            min: None,
            max: Some(900.0),
        },
    ];
    let error = LayerKind::from_bracket_conditions(MasterId::new(), &repeated).unwrap_err();
    assert!(error.to_string().contains("twice"), "{error}");
    let unbounded = [BracketCondition {
        axis: Tag(*b"wght"),
        min: None,
        max: None,
    }];
    let error = LayerKind::from_bracket_conditions(MasterId::new(), &unbounded).unwrap_err();
    assert!(error.to_string().contains("bound"), "{error}");
    assert!(LayerKind::from_bracket_conditions(MasterId::new(), &[]).is_err());
}

#[test]
fn the_condition_set_is_the_canonical_identity_of_a_region() {
    let master = MasterId::new();
    let low = BracketCondition {
        axis: Tag(*b"wdth"),
        min: Some(50.0),
        max: Some(75.0),
    };
    let high = BracketCondition {
        axis: Tag(*b"wght"),
        min: Some(500.0),
        max: None,
    };
    let one = LayerKind::from_bracket_conditions(master.clone(), &[low, high]).unwrap();
    let other = LayerKind::from_bracket_conditions(master, &[high, low]).unwrap();
    assert_eq!(one.bracket_conditions(), other.bracket_conditions());
    assert_eq!(
        one.bracket_conditions().unwrap()[0].axis,
        Tag(*b"wdth"),
        "conditions sort by axis tag"
    );
}

#[test]
fn a_compound_alternate_applies_only_where_every_range_holds() {
    let mut font = two_axis_font();
    with_compound_alternate(&mut font);
    font.validate_designspace().expect("valid design space");
    let base = font.glyph("A").unwrap().layers[0].width;
    for (wght, wdth, expected) in [
        (900.0, 50.0, base + 111.0),
        (600.0, 75.0, base + 111.0),
        (900.0, 100.0, base),
        (100.0, 50.0, base),
    ] {
        let location = [(Tag(*b"wght"), wght), (Tag(*b"wdth"), wdth)]
            .into_iter()
            .collect();
        let instance = fc_compile::instantiate(&font, &location, None)
            .unwrap_or_else(|error| panic!("{wght}/{wdth}: {error}"));
        assert_eq!(
            instance.glyph("A").unwrap().layers[0].width,
            expected,
            "{wght}/{wdth}"
        );
    }
}

#[test]
fn a_compound_alternate_compiles_one_two_axis_condition_set() {
    let mut font = two_axis_font();
    with_compound_alternate(&mut font);
    font.instances.push(Instance {
        name: "Bold Condensed".into(),
        style_name: "Bold Condensed".into(),
        location: [(Tag(*b"wght"), 900.0), (Tag(*b"wdth"), 50.0)]
            .into_iter()
            .collect(),
        active: true,
        is_variable_named_instance: true,
        ..Default::default()
    });
    let report = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            variable: true,
            format: fc_compile::ExportFormat::Ttf,
            ..Default::default()
        },
    )
    .expect("variable export");
    let face = FontRef::new(&report.bytes).expect("compiled font");
    let gsub = face.gsub().expect("GSUB");
    let variations = gsub
        .feature_variations()
        .expect("feature variations are present")
        .expect("feature variations table");
    let records = variations
        .feature_variation_records()
        .iter()
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 1, "one conditional region");
    let set = records[0]
        .condition_set(variations.offset_data())
        .expect("condition set offset")
        .expect("condition set table");
    assert_eq!(
        set.condition_count(),
        2,
        "the compound region constrains two axes"
    );
    let axes: Vec<u16> = set
        .conditions()
        .iter()
        .map(|condition| match condition.expect("condition table") {
            read_fonts::tables::layout::Condition::Format1AxisRange(table) => table.axis_index(),
            other => panic!("unexpected condition format: {other:?}"),
        })
        .collect();
    assert_eq!(axes.len(), 2, "one axis range per constrained axis");
    assert_ne!(axes[0], axes[1], "the two ranges constrain different axes");
    // The alternate glyph exists and is not encoded.
    assert!(
        (0..face.maxp().unwrap().num_glyphs()).any(|gid| {
            face.post()
                .ok()
                .and_then(|post| post.glyph_name(read_fonts::types::GlyphId16::new(gid)))
                .is_some_and(|name| name.contains("fcBracket"))
        }),
        "the generated alternate is present in post"
    );
}

#[test]
fn overlapping_compound_regions_are_refused_with_the_glyph_name() {
    let mut font = two_axis_font();
    with_compound_alternate(&mut font);
    // A second region that intersects the first on both axes.
    let overlapping = [
        BracketCondition {
            axis: Tag(*b"wght"),
            min: Some(700.0),
            max: None,
        },
        BracketCondition {
            axis: Tag(*b"wdth"),
            min: Some(60.0),
            max: Some(90.0),
        },
    ];
    let masters: Vec<MasterId> = font
        .masters
        .iter()
        .map(|master| master.id.clone())
        .collect();
    {
        let glyph: &mut Glyph = font.glyph_mut("A").unwrap();
        let mut layer: Layer = glyph.master_layer(&masters[0]).unwrap().clone();
        layer.id = LayerId::new();
        layer.kind = LayerKind::from_bracket_conditions(masters[0].clone(), &overlapping).unwrap();
        glyph.layers.push(layer);
    }
    let error = fc_model::query::expand_brackets(&font).expect_err("overlapping regions");
    assert!(error.to_string().contains("A"), "{error}");
}

#[test]
fn disjoint_compound_regions_on_different_axes_are_allowed() {
    let mut font = two_axis_font();
    with_compound_alternate(&mut font);
    // Same weight range, a width range that does not intersect the first.
    let disjoint = [
        BracketCondition {
            axis: Tag(*b"wght"),
            min: Some(500.0),
            max: None,
        },
        BracketCondition {
            axis: Tag(*b"wdth"),
            min: Some(90.0),
            max: Some(100.0),
        },
    ];
    let masters: Vec<MasterId> = font
        .masters
        .iter()
        .map(|master| master.id.clone())
        .collect();
    {
        let glyph: &mut Glyph = font.glyph_mut("A").unwrap();
        let mut alternates = Vec::new();
        for master in &masters {
            let mut layer: Layer = glyph.master_layer(master).unwrap().clone();
            layer.id = LayerId::new();
            layer.kind = LayerKind::from_bracket_conditions(master.clone(), &disjoint).unwrap();
            layer.width += 222.0;
            alternates.push(layer);
        }
        glyph.layers.extend(alternates);
    }
    let (_, rules) = fc_model::query::expand_brackets(&font).expect("disjoint regions");
    assert_eq!(rules.len(), 2);
    assert!(rules.iter().all(|rule| rule.conditions.len() == 2));
}

#[test]
fn an_axis_subset_trims_the_condition_ranges_it_drops() {
    let mut font = two_axis_font();
    with_compound_alternate(&mut font);
    let report = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            variable: true,
            format: fc_compile::ExportFormat::Ttf,
            axis_subset: Some(vec![Tag(*b"wght")]),
            ..Default::default()
        },
    )
    .expect("variable export with an axis subset");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "designspace-conditions-trimmed"),
        "{:?}",
        report.warnings
    );
    let face = FontRef::new(&report.bytes).expect("compiled font");
    assert_eq!(
        read_fonts::TableProvider::fvar(&face)
            .expect("fvar")
            .axis_count(),
        1
    );
    // The alternate survives on the retained axis and is still substituted.
    let gsub = face.gsub().expect("GSUB");
    let variations = gsub
        .feature_variations()
        .expect("feature variations present")
        .expect("feature variations table");
    let records = variations
        .feature_variation_records()
        .iter()
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 1);
    let set = records[0]
        .condition_set(variations.offset_data())
        .expect("condition set offset")
        .expect("condition set table");
    assert_eq!(
        set.condition_count(),
        1,
        "the dropped axis' range is gone, the retained one stays"
    );
}

#[test]
fn an_alternate_with_no_retained_axis_is_dropped_and_reported() {
    let mut font = two_axis_font();
    // A region bounded only on the width axis, which the subset removes.
    let conditions = [BracketCondition {
        axis: Tag(*b"wdth"),
        min: Some(50.0),
        max: Some(75.0),
    }];
    let masters: Vec<MasterId> = font.masters.iter().map(|m| m.id.clone()).collect();
    {
        let glyph = font.glyph_mut("A").expect("A");
        let mut alternates = Vec::new();
        for master in &masters {
            let mut layer = glyph.master_layer(master).expect("master layer").clone();
            layer.id = LayerId::new();
            layer.kind = LayerKind::from_bracket_conditions(master.clone(), &conditions).unwrap();
            alternates.push(layer);
        }
        glyph.layers.extend(alternates);
    }
    let report = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            variable: true,
            format: fc_compile::ExportFormat::Ttf,
            axis_subset: Some(vec![Tag(*b"wght")]),
            ..Default::default()
        },
    )
    .expect("variable export with an axis subset");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "designspace-conditional-layers-dropped"),
        "{:?}",
        report.warnings
    );
    let face = FontRef::new(&report.bytes).expect("compiled font");
    assert!(
        face.gsub()
            .ok()
            .and_then(|gsub| gsub.feature_variations().transpose().ok().flatten())
            .is_none(),
        "no conditional region remains"
    );
}
