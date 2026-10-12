use super::*;
use fc_model::{Anchor, Component, Contour, GlyphName, Point, PointKind, StemHint};
use serde_json::json;

/// The bundled database alone: these tests carry no project overlay.
fn overlay() -> fc_glyphdata::GlyphDataOverlay {
    fc_glyphdata::GlyphDataOverlay::default()
}

fn predicate(field: &str, operator: Op, value: &str) -> SmartFilterNode {
    SmartFilterNode::Predicate {
        field: field.into(),
        operator,
        value: value.into(),
    }
}

fn fixture() -> Font {
    let mut font = Font::default();
    let mut glyph = Glyph::new("A.alt");
    glyph.unicodes = vec![0x41];
    glyph.category = Some("Letter".into());
    glyph.sub_category = Some("Uppercase".into());
    glyph.script = Some("Latn".into());
    glyph.color_label = Some(3);
    glyph.tags.insert("Review".into());
    glyph.note = "needs polish".into();
    glyph.kern_group_side1 = Some("A".into());
    glyph.user_data = json!({"is_hangul_key":true});
    let layer = Layer {
        kind: LayerKind::Master(font.default_master().id.clone()),
        width: 600.,
        contours: vec![Contour {
            closed: true,
            points: vec![Point {
                x: 10.,
                y: 20.,
                kind: PointKind::Line,
                smooth: false,
                name: None,
                corner_radius: 5.,
                ..Point::default()
            }],
            ..Contour::default()
        }],
        components: vec![Component {
            base: GlyphName::from("acute"),
            auto_align: true,
            ..Component::default()
        }],
        anchors: vec![Anchor::default()],
        tt_instructions: Some(vec![0x00]),
        user_data: json!({"annotations":[{"text":"check"}],"color_label":4}),
        ..Layer::default()
    };
    glyph.layers.push(layer);
    glyph.layers[0].hints.h_stems.push(StemHint::default());
    glyph.layers.push(Layer {
        kind: LayerKind::Brace {
            master: font.default_master().id.clone(),
            location: Default::default(),
        },
        ..Layer::default()
    });
    font.insert_glyph(glyph).unwrap();
    font
}

fn test_match(font: &Font, field: &str, op: Op, value: &str) -> bool {
    let glyph = font.glyph("A.alt").unwrap();
    CompiledSmartFilter::new(&predicate(field, op, value))
        .unwrap()
        .matches(font, &overlay(), glyph, glyph.layers.first(), 1)
}

#[test]
fn every_predicate_family_matches_model_data() {
    let font = fixture();
    for (field, value) in [
        ("name", "A.alt"),
        ("paths", "1"),
        ("components", "1"),
        ("tags", "Review"),
        ("script", "Latn"),
        ("category", "Letter"),
        ("subcategory", "Uppercase"),
        ("case", "Uppercase"),
        ("master_compatible", "true"),
        ("export", "true"),
        ("has_unicode", "true"),
        ("has_components", "true"),
        ("contains_component", "acute"),
        ("has_hints", "true"),
        ("has_ps_hints", "true"),
        ("has_tt_hints", "true"),
        ("has_corners", "true"),
        ("has_anchors", "true"),
        ("has_special_layers", "true"),
        ("has_custom_glyph_info", "true"),
        ("has_annotations", "true"),
        ("is_auto_aligned", "true"),
        ("has_metrics_keys", "false"),
        ("metrics_out_of_sync", "false"),
        ("has_kerning_groups", "true"),
        ("color_label", "3"),
        ("layer_color_label", "4"),
        ("is_hangul_key", "true"),
        ("note", "needs polish"),
        ("unicode", "U+0041"),
        ("changed", "true"),
        ("locked", "false"),
        ("width", "600"),
        ("block", "Basic Latin"),
    ] {
        assert!(test_match(&font, field, Op::Eq, value), "{field}");
        assert!(!test_match(&font, field, Op::Ne, value), "{field} !=");
    }
    assert!(test_match(&font, "width", Op::Gte, "600"));
    assert!(test_match(&font, "width", Op::Gt, "599"));
    assert!(test_match(&font, "width", Op::Lte, "600"));
    assert!(test_match(&font, "width", Op::Lt, "601"));
    assert!(test_match(&font, "tags", Op::Contains, "view"));
    assert!(test_match(&font, "name", Op::Regex, r"^A\.alt$"));
}

#[test]
fn collection_not_equal_means_no_member_matches() {
    let mut font = fixture();
    let glyph = font.glyphs.get_mut("A.alt").unwrap();
    glyph.tags.insert("Other".into());
    glyph.unicodes.push(0x42);
    assert!(!test_match(&font, "tags", Op::Ne, "Review"));
    assert!(!test_match(&font, "unicode", Op::Ne, "65"));
    assert!(test_match(&font, "unicode", Op::Ne, "67"));
}

#[test]
fn nesting_and_missing_selected_layer_are_deterministic() {
    let font = fixture();
    let glyph = font.glyph("A.alt").unwrap();
    let root = SmartFilterNode::All {
        children: vec![
            SmartFilterNode::Any {
                children: vec![
                    predicate("name", Op::Eq, "missing"),
                    predicate("name", Op::Eq, "A.alt"),
                ],
            },
            SmartFilterNode::None {
                children: vec![predicate("export", Op::Eq, "false")],
            },
            predicate("paths", Op::Eq, "0"),
            predicate("layer_color_label", Op::Eq, "-1"),
            predicate("is_auto_aligned", Op::Eq, "false"),
        ],
    };
    assert!(CompiledSmartFilter::new(&root)
        .unwrap()
        .matches(&font, &overlay(), glyph, None, 0));
}

#[test]
fn validation_rejects_malformed_and_expensive_predicates() {
    for (field, op, value) in [
        ("unknown", Op::Eq, "x"),
        ("export", Op::Eq, "yes"),
        ("paths", Op::Contains, "1"),
        ("paths", Op::Eq, "-1"),
        ("paths", Op::Eq, "1.5"),
        ("width", Op::Eq, "NaN"),
        ("name", Op::Gt, "A"),
        ("export", Op::Regex, "true"),
        ("name", Op::Regex, "["),
        ("name", Op::Regex, "(a{1000}){1000}"),
        ("unicode", Op::Eq, "U+D800"),
        ("color_label", Op::Eq, "256"),
    ] {
        assert!(
            validate_predicate(field, op, value).is_err(),
            "{field}: {value}"
        );
    }
    assert!(validate_predicate("name", Op::Regex, &"a".repeat(1025)).is_err());
    assert!(validate_predicate("note", Op::Eq, &"x".repeat(4097)).is_err());
    let mut root = predicate("export", Op::Eq, "true");
    for _ in 0..20 {
        root = SmartFilterNode::All {
            children: vec![root],
        };
    }
    assert!(CompiledSmartFilter::new(&root).is_err());
    let root = SmartFilterNode::All {
        children: vec![predicate("export", Op::Eq, "true"); 257],
    };
    assert!(CompiledSmartFilter::new(&root).is_err());
}

#[test]
fn metrics_keys_compare_resolved_values_and_report_broken_references() {
    let mut font = Font::default();
    let mut glyph = Glyph::new("A.alt");
    glyph.layers.push(Layer {
        kind: LayerKind::Master(font.default_master().id.clone()),
        width: 600.,
        vertical_width: Some(1000.),
        ..Layer::default()
    });
    glyph.metrics_keys.width = Some("=600".into());
    glyph.metrics_keys.vertical_width = Some("=1000".into());
    font.insert_glyph(glyph).unwrap();
    assert!(test_match(&font, "has_metrics_keys", Op::Eq, "true"));
    assert!(test_match(&font, "metrics_out_of_sync", Op::Eq, "false"));
    font.glyphs.get_mut("A.alt").unwrap().metrics_keys.width = Some("=601".into());
    assert!(test_match(&font, "metrics_out_of_sync", Op::Eq, "true"));
    font.glyphs.get_mut("A.alt").unwrap().metrics_keys.width = Some("=A.alt".into());
    assert!(test_match(&font, "metrics_out_of_sync", Op::Eq, "true"));
}

#[test]
fn unicode_fallbacks_and_group_membership_are_available() {
    let mut font = fixture();
    let glyph = font.glyphs.get_mut("A.alt").unwrap();
    glyph.category = None;
    glyph.sub_category = None;
    glyph.script = None;
    glyph.user_data = Value::Null;
    glyph.color_label = None;
    font.extra.lib.insert(
        "hangul_composition_groups".into(),
        json!({"initial":["A.alt"]}),
    );
    assert!(test_match(&font, "category", Op::Eq, "Letter"));
    assert!(test_match(&font, "case", Op::Eq, "Uppercase"));
    assert!(test_match(&font, "script", Op::Eq, "Latn"));
    assert!(test_match(&font, "color_label", Op::Eq, "-1"));
    assert!(test_match(&font, "is_hangul_key", Op::Eq, "true"));
}

#[test]
fn compiled_regex_filters_twenty_thousand_cells_within_budget() {
    let font = fixture();
    let glyph = font.glyph("A.alt").unwrap();
    let filter = CompiledSmartFilter::new(&predicate("name", Op::Regex, r"^A\..*$")).unwrap();
    let start = std::time::Instant::now();
    for _ in 0..20_000 {
        assert!(filter.matches(&font, &overlay(), glyph, glyph.layers.first(), 0));
    }
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn annotations_and_geometry_follow_selected_layer() {
    let mut font = fixture();
    let glyph = font.glyphs.get_mut("A.alt").unwrap();
    glyph.layers[0].user_data = json!({"com.fontcarver.editor":{"annotations":[{"kind":"text"}]}});
    assert!(test_match(&font, "has_annotations", Op::Eq, "true"));
    let glyph = font.glyphs.get_mut("A.alt").unwrap();
    glyph.layers[1].user_data = glyph.layers[0].user_data.clone();
    glyph.layers[0].user_data = Value::Null;
    glyph.layers[0].components.clear();
    assert!(test_match(&font, "has_annotations", Op::Eq, "false"));
    assert!(test_match(&font, "is_auto_aligned", Op::Eq, "false"));
    assert!(test_match(&font, "has_components", Op::Eq, "false"));
}

#[test]
fn locked_and_custom_glyph_info_follow_the_cell_badges() {
    let mut font = fixture();
    assert!(test_match(&font, "locked", Op::Eq, "false"));
    let glyph = font.glyphs.get_mut("A.alt").unwrap();
    glyph.locked = true;
    assert!(test_match(&font, "locked", Op::Eq, "true"));
    assert!(!test_match(&font, "locked", Op::Ne, "true"));
    // The predicate and the badge share one rule, so database-supplied values
    // are not custom while a divergent one is.
    let glyph = font.glyphs.get_mut("A.alt").unwrap();
    glyph.user_data = Value::Null;
    assert!(test_match(&font, "has_custom_glyph_info", Op::Eq, "false"));
    let glyph = font.glyphs.get_mut("A.alt").unwrap();
    glyph.production_name = Some("uni0041.alt".into());
    assert!(test_match(&font, "has_custom_glyph_info", Op::Eq, "true"));
}

#[test]
fn master_compatibility_rejects_missing_and_different_master_layers() {
    let mut font = fixture();
    let second = fc_model::Master::default();
    let second_id = second.id.clone();
    font.masters.push(second);
    assert!(test_match(&font, "master_compatible", Op::Eq, "false"));
    let glyph = font.glyphs.get_mut("A.alt").unwrap();
    let mut layer = glyph.layers[0].clone();
    layer.id = fc_model::LayerId::new();
    layer.kind = LayerKind::Master(second_id);
    glyph.layers.push(layer);
    assert!(test_match(&font, "master_compatible", Op::Eq, "true"));
    font.glyphs
        .get_mut("A.alt")
        .unwrap()
        .layers
        .last_mut()
        .unwrap()
        .components
        .clear();
    assert!(test_match(&font, "master_compatible", Op::Eq, "false"));
}
