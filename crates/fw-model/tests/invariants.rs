use std::collections::{BTreeMap, BTreeSet};

use fw_model::{
    Component, Contour, Error, Glyph, GlyphName, KernSide, Layer, Master, MasterId, Point,
    PointKind, TtIndex,
};
use kurbo::{BezPath, PathEl};

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

#[test]
fn insert_glyph_rejects_duplicates_and_adds_every_master_layer() {
    let mut font = fw_model::Font::new_empty("Test Family", 1_000);
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
fn rename_glyph_preserves_order_and_updates_all_references() {
    let mut font = fw_model::Font::new_empty("Test Family", 1_000);
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
    font.bitmap_strikes.push(fw_model::BitmapStrike {
        glyphs: BTreeMap::from([(
            GlyphName::from("B"),
            fw_model::BitmapGlyph {
                refs: vec![fw_model::BitmapRef {
                    base: "A".into(),
                    dx: 1,
                    dy: 2,
                }],
                ..fw_model::BitmapGlyph::default()
            },
        )]),
        ..fw_model::BitmapStrike::default()
    });
    font.cid = Some(fw_model::CidInfo {
        cid_map: BTreeMap::from([(65, "A".into())]),
        ..fw_model::CidInfo::default()
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
    let mut font = fw_model::Font::new_empty("Test Family", 1_000);
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
    let mut font = fw_model::Font::new_empty("Test Family", 1_000);
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
    path.line_to((0.0, 0.0));
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
            PointKind::Line,
        ]
    );
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
    use fw_model::AnchorKind;

    let anchor = |name: &str| fw_model::Anchor {
        name: name.into(),
        ..fw_model::Anchor::default()
    };
    assert_eq!(anchor("_top").kind(), AnchorKind::Mark("top".into()));
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
    let mut font = fw_model::Font::new_empty("Test Family", 1_000);
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
    layer.anchors.push(fw_model::Anchor {
        name: "top".into(),
        x: 0.0,
        y: 0.0,
    });
    assert!(!layer.is_empty());
}
