use std::collections::{BTreeMap, BTreeSet};

use fc_model::{
    Component, Contour, Error, Glyph, GlyphName, KernSide, Layer, LayerKind, Master, MasterId,
    Point, PointKind, TtIndex,
};
use kurbo::{BezPath, PathEl};

fn point(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        corner_radius: 0.,
        x,
        y,
        kind,
        smooth: false,
        name: None,
        id: None,
        super_smooth: false,
        tt_index: TtIndex::Unnumbered,
    }
}

#[test]
fn insert_glyph_rejects_duplicates_and_adds_every_master_layer() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    font.add_master(Master {
        id: MasterId::new(),
        name: "Bold".into(),
        ..Master::default()
    });

    font.insert_glyph(Glyph::new("A")).unwrap();
    let glyph = font.glyph("A").unwrap();
    assert_eq!(glyph.layers.len(), 2);
    for master in &font.masters {
        assert!(glyph.master_layer(&master.id).is_some());
    }

    assert!(matches!(
        font.insert_glyph(Glyph::new("A")),
        Err(Error::DuplicateGlyph { .. })
    ));
}

#[test]
fn default_and_deserialized_fonts_always_have_a_regular_master() {
    let default = fc_model::Font::default();
    assert_eq!(default.default_master().name, "Regular");

    let missing: fc_model::Font = serde_json::from_str("{}").unwrap();
    assert_eq!(missing.default_master().name, "Regular");

    let empty: fc_model::Font = serde_json::from_str(r#"{"masters": []}"#).unwrap();
    assert_eq!(empty.default_master().name, "Regular");
}

#[test]
fn default_master_is_safe_after_callers_clear_the_public_master_list() {
    let mut font = fc_model::Font::default();
    font.masters.clear();

    assert_eq!(font.default_master().name, "Regular");
}

#[test]
fn inserting_simple_glyphs_scales_linearly() {
    let mut font = fc_model::Font::default();
    let started = std::time::Instant::now();

    for index in 0..20_000 {
        font.insert_glyph(Glyph::new(format!("glyph{index}")))
            .unwrap();
    }

    if cfg!(debug_assertions) {
        return;
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "20,000 simple insertions took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_preexisting_cycle_elsewhere_does_not_block_insertion() {
    let mut font = fc_model::Font::default();
    let master = font.default_master().id.clone();
    font.insert_glyph(Glyph::new("A")).unwrap();
    font.insert_glyph(Glyph::new("B")).unwrap();
    font.insert_glyph(Glyph::new("base")).unwrap();
    font.glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "B".into(),
            ..Component::default()
        });
    font.glyph_mut("B")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "A".into(),
            ..Component::default()
        });

    let mut unrelated = Glyph::new("unrelated");
    unrelated.layers.push(Layer::new_master(master.clone()));
    unrelated
        .master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "base".into(),
            ..Component::default()
        });
    font.insert_glyph(unrelated).unwrap();

    assert!(font.glyph("unrelated").is_some());
}

#[test]
fn rename_glyph_preserves_order_and_updates_all_references() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    font.insert_glyph(Glyph::new("A")).unwrap();
    font.insert_glyph(Glyph::new("B")).unwrap();
    font.insert_glyph(Glyph::new("C")).unwrap();
    font.glyph_mut("B").unwrap().layers[0]
        .components
        .push(Component {
            base: "A".into(),
            ..Component::default()
        });
    font.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("B".into())),
        -80.0,
    );
    font.kerning_groups
        .side1
        .insert("public.kern1.A".into(), BTreeSet::from(["A".into()]));
    font.kerning_groups
        .side2
        .insert("public.kern2.A".into(), BTreeSet::from(["A".into()]));
    font.bitmap_strikes.push(fc_model::BitmapStrike {
        glyphs: BTreeMap::from([(
            GlyphName::from("B"),
            fc_model::BitmapGlyph {
                refs: vec![fc_model::BitmapRef {
                    base: "A".into(),
                    dx: 1,
                    dy: 2,
                }],
                ..fc_model::BitmapGlyph::default()
            },
        )]),
        ..fc_model::BitmapStrike::default()
    });
    font.cid = Some(fc_model::CidInfo {
        cid_map: BTreeMap::from([(65, "A".into())]),
        ..fc_model::CidInfo::default()
    });

    font.rename_glyph("A", "A.alt").unwrap();

    assert_eq!(
        font.glyph_order().map(String::from).collect::<Vec<_>>(),
        ["A.alt", "B", "C"]
    );
    assert_eq!(
        font.glyph("B").unwrap().layers[0].components[0].base,
        GlyphName::from("A.alt")
    );
    assert_eq!(
        font.masters[0].kerning_h.0.keys().next().unwrap().0,
        KernSide::Glyph("A.alt".into())
    );
    assert!(font.kerning_groups.side1["public.kern1.A"].contains("A.alt"));
    assert!(font.kerning_groups.side2["public.kern2.A"].contains("A.alt"));
    assert_eq!(
        font.bitmap_strikes[0].glyphs["B"].refs[0].base,
        GlyphName::from("A.alt")
    );
    assert_eq!(font.cid.as_ref().unwrap().cid_map[&65], "A.alt".into());
}

#[test]
fn rename_rejects_a_missing_source_or_duplicate_destination() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    font.insert_glyph(Glyph::new("A")).unwrap();
    font.insert_glyph(Glyph::new("B")).unwrap();

    assert!(matches!(
        font.rename_glyph("missing", "C"),
        Err(Error::MissingGlyph { .. })
    ));
    assert!(matches!(
        font.rename_glyph("A", "B"),
        Err(Error::DuplicateGlyph { .. })
    ));
}

#[test]
fn add_master_adds_a_layer_to_every_glyph() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    font.insert_glyph(Glyph::new("A")).unwrap();
    font.insert_glyph(Glyph::new("B")).unwrap();
    let master = Master {
        id: MasterId::new(),
        name: "Bold".into(),
        ..Master::default()
    };

    font.add_master(master.clone());

    assert_eq!(font.master(&master.id), Some(&master));
    assert!(font
        .glyphs
        .values()
        .all(|glyph| glyph.master_layer(&master.id).is_some()));
}

#[test]
fn add_master_does_not_duplicate_an_existing_glyph_layer() {
    let mut font = fc_model::Font::default();
    let master = Master {
        id: MasterId::new(),
        name: "Bold".into(),
        ..Master::default()
    };
    let mut glyph = Glyph::new("A");
    glyph.layers.push(Layer::new_master(master.id.clone()));
    font.insert_glyph(glyph).unwrap();

    font.add_master(master.clone());

    assert_eq!(
        font.glyph("A")
            .unwrap()
            .layers
            .iter()
            .filter(|layer| matches!(&layer.kind, LayerKind::Master(id) if id == &master.id))
            .count(),
        1
    );
}

#[test]
fn quadratic_contour_inserts_implied_on_curve_points() {
    let contour = Contour {
        closed: false,
        points: vec![
            point(0.0, 0.0, PointKind::Line),
            point(50.0, 100.0, PointKind::OffCurve),
            point(150.0, 100.0, PointKind::OffCurve),
            point(200.0, 0.0, PointKind::QCurve),
        ],
        name: None,
        id: None,
    };

    assert_eq!(
        contour.to_bezpath().elements(),
        &[
            PathEl::MoveTo((0.0, 0.0).into()),
            PathEl::QuadTo((50.0, 100.0).into(), (100.0, 100.0).into()),
            PathEl::QuadTo((150.0, 100.0).into(), (200.0, 0.0).into()),
        ]
    );
}

#[test]
fn malformed_cubic_is_rejected_by_checked_conversion_and_has_quadratic_fallback() {
    let contour = Contour {
        closed: false,
        points: vec![
            point(0.0, 0.0, PointKind::Move),
            point(40.0, 100.0, PointKind::OffCurve),
            point(100.0, 0.0, PointKind::Curve),
        ],
        ..Contour::default()
    };

    assert!(matches!(
        contour.try_to_bezpath(),
        Err(Error::MalformedContour)
    ));
    assert!(matches!(
        contour.to_bezpath().elements()[1],
        PathEl::QuadTo(_, _)
    ));
}

#[test]
fn cubic_path_round_trips_through_point_contours() {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.0));
    path.curve_to((25.0, 100.0), (75.0, 100.0), (100.0, 0.0));
    path.close_path();

    let contours = Contour::from_bezpath(&path);

    assert_eq!(contours.len(), 1);
    assert!(contours[0].closed);
    assert_eq!(contours[0].to_bezpath(), path);
    assert_eq!(
        contours[0]
            .points
            .iter()
            .map(|point| point.kind)
            .collect::<Vec<_>>(),
        [
            PointKind::Line,
            PointKind::OffCurve,
            PointKind::OffCurve,
            PointKind::Curve,
        ]
    );
}

#[test]
fn closed_cubic_loop_deduplicates_its_explicit_return_to_start() {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.0));
    path.curve_to((25.0, 100.0), (75.0, 100.0), (100.0, 0.0));
    path.curve_to((75.0, -100.0), (25.0, -100.0), (0.0, 0.0));
    path.close_path();

    let contours = Contour::from_bezpath(&path);

    assert_eq!(contours.len(), 1);
    assert_eq!(contours[0].points.len(), 6);
    assert_eq!(contours[0].points[0].kind, PointKind::Curve);
    assert_eq!(contours[0].to_bezpath(), path);
}

#[test]
fn closed_contours_rotate_to_an_on_curve_start() {
    let contour = Contour {
        closed: true,
        points: vec![
            point(50.0, 100.0, PointKind::OffCurve),
            point(100.0, 0.0, PointKind::Line),
            point(0.0, 0.0, PointKind::Line),
        ],
        ..Contour::default()
    };

    assert_eq!(
        contour.to_bezpath().elements(),
        &[
            PathEl::MoveTo((100.0, 0.0).into()),
            PathEl::LineTo((0.0, 0.0).into()),
            PathEl::QuadTo((50.0, 100.0).into(), (100.0, 0.0).into()),
            PathEl::ClosePath,
        ]
    );
}

#[test]
fn closed_all_off_curve_contours_start_at_an_implied_midpoint() {
    let contour = Contour {
        closed: true,
        points: vec![
            point(0.0, 0.0, PointKind::OffCurve),
            point(100.0, 0.0, PointKind::OffCurve),
        ],
        ..Contour::default()
    };

    assert_eq!(
        contour.to_bezpath().elements(),
        &[
            PathEl::MoveTo((50.0, 0.0).into()),
            PathEl::QuadTo((0.0, 0.0).into(), (50.0, 0.0).into()),
            PathEl::QuadTo((100.0, 0.0).into(), (50.0, 0.0).into()),
            PathEl::ClosePath,
        ]
    );
}

#[test]
fn anchor_names_map_to_derived_roles() {
    use fc_model::AnchorKind;

    let anchor = |name: &str| fc_model::Anchor {
        name: name.into(),
        ..fc_model::Anchor::default()
    };
    assert_eq!(anchor("_top").kind(), AnchorKind::Mark("top".into()));
    assert_eq!(anchor("_top_2").kind(), AnchorKind::Mark("top_2".into()));
    assert_eq!(anchor("top").kind(), AnchorKind::Base("top".into()));
    assert_eq!(
        anchor("top_2").kind(),
        AnchorKind::Ligature {
            name: "top".into(),
            index: 2,
        }
    );
    assert_eq!(anchor("entry").kind(), AnchorKind::CursiveEntry);
    assert_eq!(anchor("exit").kind(), AnchorKind::CursiveExit);
    assert_eq!(anchor("caret_3").kind(), AnchorKind::Caret(3));
    assert_eq!(anchor("vcaret_4").kind(), AnchorKind::VCaret(4));
}

#[test]
fn inserting_a_component_cycle_is_rejected_without_inserting_the_glyph() {
    let mut font = fc_model::Font::new_empty("Test Family", 1_000);
    let master = font.default_master().id.clone();
    let mut a = Glyph::new("A");
    a.layers.push(Layer::new_master(master.clone()));
    a.master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "B".into(),
            ..Component::default()
        });
    font.insert_glyph(a).unwrap();
    let mut b = Glyph::new("B");
    b.layers.push(Layer::new_master(master.clone()));
    b.master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "A".into(),
            ..Component::default()
        });

    assert!(matches!(
        font.insert_glyph(b),
        Err(Error::ComponentCycle { .. })
    ));
    assert!(font.glyph("B").is_none());
}

#[test]
fn layer_empty_state_accounts_for_all_editable_content() {
    let mut layer = Layer::new_master(MasterId::new());
    assert!(layer.is_empty());
    layer.anchors.push(fc_model::Anchor {
        name: "top".into(),
        x: 0.0,
        y: 0.0,
    });
    assert!(!layer.is_empty());
}

#[test]
fn layer_kind_is_required_in_json_and_has_a_deterministic_rust_default() {
    assert!(serde_json::from_str::<Layer>("{}").is_err());
    assert_eq!(LayerKind::default(), LayerKind::Custom("default".into()));
}

#[test]
fn error_module_remains_public() {
    let root_error = Error::MalformedContour;
    let module_error: fc_model::error::Error = root_error.clone();
    assert_eq!(module_error, root_error);
}
