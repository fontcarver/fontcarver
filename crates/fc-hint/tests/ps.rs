use fc_hint::ps;
use fc_model::{Contour, Font, Glyph};
use kurbo::{Rect, Shape};

#[test]
fn detects_ink_stems_without_hinting_counter_space() {
    let mut font = Font::new_empty("Hints", 1000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("H");
    font.insert_glyph(glyph.clone()).unwrap();
    glyph = font.glyph("H").unwrap().clone();
    let layer = glyph.master_layer_mut(&master).unwrap();
    for rectangle in [
        Rect::new(50.0, 0.0, 130.0, 700.0),
        Rect::new(430.0, 0.0, 510.0, 700.0),
        Rect::new(130.0, 300.0, 430.0, 380.0),
    ] {
        layer
            .contours
            .extend(Contour::from_bezpath(&rectangle.to_path(0.1)));
    }
    font.glyphs.insert(glyph.name.clone(), glyph);
    let hints = ps::detect(&font, &"H".into(), &master).unwrap();
    assert_eq!(
        hints
            .v_stems
            .iter()
            .map(|s| (s.start, s.width))
            .collect::<Vec<_>>(),
        [(50.0, 80.0), (430.0, 80.0)]
    );
    assert!(hints
        .h_stems
        .iter()
        .any(|s| s.start == 300.0 && s.width == 80.0));
    let private = ps::infer_private(&font, &master, &[hints]).unwrap();
    assert_eq!(private.std_vw, Some(80.0));
    assert!(!private.blue_values.is_empty());
    private.validate().unwrap();
}

#[test]
fn rejects_invalid_and_extreme_curve_coordinates_before_subdivision() {
    let mut font = Font::new_empty("Invalid hints", 1000);
    let master = font.default_master().id.clone();
    font.insert_glyph(Glyph::new("A")).unwrap();
    for value in [f64::NAN, f64::INFINITY, 1e100] {
        let mut path = kurbo::BezPath::new();
        path.move_to((0.0, 0.0));
        path.curve_to((value, 10.0), (50.0, value), (100.0, 0.0));
        path.close_path();
        font.glyph_mut("A").unwrap().layers[0].contours = Contour::from_bezpath(&path);
        assert!(ps::detect(&font, &"A".into(), &master).is_err());
    }
}

#[test]
fn detects_round_bowl_stems_at_exact_curve_extrema() {
    let mut font = Font::new_empty("Round hints", 1000);
    let master = font.default_master().id.clone();
    font.insert_glyph(Glyph::new("O")).unwrap();
    let mut path = kurbo::Circle::new((300.0, 300.0), 250.0).to_path(0.1);
    path.extend(
        kurbo::Circle::new((300.0, 300.0), 170.0)
            .to_path(0.1)
            .reverse_subpaths(),
    );
    font.glyph_mut("O").unwrap().layers[0].contours = Contour::from_bezpath(&path);
    let hints = ps::detect(&font, &"O".into(), &master).unwrap();
    for stems in [&hints.h_stems, &hints.v_stems] {
        assert_eq!(stems.len(), 2);
        for (stem, start) in stems.iter().zip([50.0, 470.0]) {
            assert!((stem.start - start).abs() < 0.01);
            assert!((stem.width - 80.0).abs() < 0.01);
        }
    }
}

#[test]
fn a_crossbar_does_not_turn_counter_space_into_a_stem() {
    for horizontal in [false, true] {
        for reversed in [false, true] {
            let mut font = Font::new_empty("Crossed counters", 1000);
            let master = font.default_master().id.clone();
            font.insert_glyph(Glyph::new("test")).unwrap();
            let mut path = Rect::new(0.0, 0.0, 200.0, 700.0).to_path(0.1);
            path.extend(
                Rect::new(40.0, 40.0, 160.0, 660.0)
                    .to_path(0.1)
                    .reverse_subpaths(),
            );
            path.extend(Rect::new(40.0, 330.0, 160.0, 370.0).to_path(0.1));
            if horizontal {
                path.apply_affine(kurbo::Affine::new([0.0, 1.0, 1.0, 0.0, 0.0, 0.0]));
            }
            if reversed {
                path = path.reverse_subpaths();
            }
            font.glyph_mut("test").unwrap().layers[0].contours = Contour::from_bezpath(&path);
            let hints = ps::detect(&font, &"test".into(), &master).unwrap();
            let stems = if horizontal {
                hints.h_stems
            } else {
                hints.v_stems
            };
            assert_eq!(
                stems
                    .iter()
                    .map(|stem| (stem.start, stem.width))
                    .collect::<Vec<_>>(),
                [(0.0, 40.0), (160.0, 40.0)],
                "horizontal={horizontal}, reversed={reversed}",
            );
        }
    }
}

#[test]
fn automatic_analysis_infers_each_private_dictionary_it_uses() {
    let mut font = Font::new_empty("CID analysis", 1000);
    let master = font.default_master().id.clone();
    font.masters[0].cff_private_dicts.push(Default::default());
    for (name, left, right) in [("first", 0.0, 60.0), ("second", 0.0, 90.0)] {
        font.insert_glyph(Glyph::new(name)).unwrap();
        let layer = &mut font.glyph_mut(name).unwrap().layers[0];
        layer.contours = Contour::from_bezpath(&Rect::new(left, 0.0, right, 700.0).to_path(0.1));
    }
    font.glyph_mut("second").unwrap().layers[0].cff_private_dict_index = Some(1);
    let before = font.clone();
    let analysis = ps::analyze(&font, &master, &["first".into(), "second".into()]).unwrap();
    assert_eq!(font, before, "analysis must not mutate the document");
    assert_eq!(analysis.private.std_vw, Some(60.0));
    assert_eq!(
        analysis
            .cid_private
            .iter()
            .map(|(index, private)| (*index, private.std_vw))
            .collect::<Vec<_>>(),
        [(1, Some(90.0))]
    );
    font.glyph_mut("second").unwrap().layers[0].cff_private_dict_index = Some(4);
    let error = ps::analyze(&font, &master, &["second".into()]).unwrap_err();
    assert!(error.to_string().contains("does not define"), "{error}");
}

#[test]
fn conflicting_stems_become_point_bound_replacement_hints() {
    let mut font = Font::new_empty("Replacement", 1000);
    let master = font.default_master().id.clone();
    // Keep every alignment zone away from this outline so the assertion below is
    // about replacement alone; ghost hints have their own test.
    font.masters[0].alignment_zones = vec![fc_model::Zone {
        position: 950.0,
        size: 10.0,
    }];
    font.insert_glyph(Glyph::new("s")).unwrap();
    let layer = &mut font.glyph_mut("s").unwrap().layers[0];
    for rectangle in [
        Rect::new(0.0, 0.0, 80.0, 400.0),
        Rect::new(40.0, 500.0, 120.0, 900.0),
    ] {
        layer
            .contours
            .extend(Contour::from_bezpath(&rectangle.to_path(0.1)));
    }
    let hints = ps::detect(&font, &"s".into(), &master).unwrap();
    assert_eq!(
        hints
            .v_stems
            .iter()
            .map(|stem| (stem.start, stem.width))
            .collect::<Vec<_>>(),
        [(0.0, 80.0), (40.0, 80.0)],
        "the conflicting stem is retained, not dropped"
    );
    assert!(hints.h_stems.is_empty());
    let masks = hints
        .hint_masks
        .iter()
        .map(|mask| (mask.contour, mask.counter, mask.mask.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        masks,
        [(0, false, vec![0b1000_0000]), (1, false, vec![0b0100_0000])]
    );
    // Every mask must select a nonoverlapping active set, including the first.
    fc_model::query::path_hint_masks(&hints, &font.glyph("s").unwrap().layers[0].contours).unwrap();
}

#[test]
fn alignment_edges_without_a_facing_edge_become_ghost_hints() {
    let mut font = Font::new_empty("Ghosts", 1000);
    let master = font.default_master().id.clone();
    font.masters[0].alignment_zones = vec![
        fc_model::Zone {
            position: 0.0,
            size: -10.0,
        },
        fc_model::Zone {
            position: 700.0,
            size: 10.0,
        },
    ];
    font.insert_glyph(Glyph::new("A")).unwrap();
    let mut path = kurbo::BezPath::new();
    path.move_to((0.0, 0.0));
    path.line_to((300.0, 0.0));
    path.line_to((200.0, 700.0));
    path.line_to((100.0, 700.0));
    path.close_path();
    font.glyph_mut("A").unwrap().layers[0].contours = Contour::from_bezpath(&path);
    let hints = ps::detect(&font, &"A".into(), &master).unwrap();
    assert_eq!(
        hints
            .h_stems
            .iter()
            .map(|stem| (stem.start, stem.width))
            .collect::<Vec<_>>(),
        [(21.0, -21.0), (700.0, -20.0)],
        "a bottom ghost stores edge + 21; a top ghost stores the edge itself"
    );
    assert!(hints.v_stems.is_empty());
    assert!(hints.hint_masks.is_empty());
}

#[test]
fn three_evenly_spaced_stems_propose_a_counter_group() {
    let mut font = Font::new_empty("Counters", 1000);
    let master = font.default_master().id.clone();
    font.masters[0].alignment_zones = vec![fc_model::Zone {
        position: 900.0,
        size: 10.0,
    }];
    font.insert_glyph(Glyph::new("m")).unwrap();
    let layer = &mut font.glyph_mut("m").unwrap().layers[0];
    for left in [0.0, 150.0, 300.0] {
        layer.contours.extend(Contour::from_bezpath(
            &Rect::new(left, 0.0, left + 50.0, 700.0).to_path(0.1),
        ));
    }
    let hints = ps::detect(&font, &"m".into(), &master).unwrap();
    assert_eq!(
        hints
            .v_stems
            .iter()
            .map(|stem| (stem.start, stem.width))
            .collect::<Vec<_>>(),
        [(0.0, 50.0), (150.0, 50.0), (300.0, 50.0)]
    );
    assert!(hints.h_stems.is_empty());
    assert_eq!(
        hints
            .hint_masks
            .iter()
            .map(|mask| (mask.counter, mask.mask.clone()))
            .collect::<Vec<_>>(),
        [(true, vec![0b1110_0000])]
    );
}

#[test]
fn analysis_keeps_flex_hints_and_dot_sections_it_did_not_propose() {
    use fc_model::{PointKind, PsHints};
    let mut font = Font::new_empty("Flex survives", 1000);
    let master = font.default_master().id.clone();
    font.masters[0].alignment_zones = vec![fc_model::Zone {
        position: 950.0,
        size: 10.0,
    }];
    font.insert_glyph(Glyph::new("i")).unwrap();
    let point = |x: f64, y: f64, kind| fc_model::Point {
        x,
        y,
        kind,
        ..fc_model::Point::default()
    };
    let layer = &mut font.glyph_mut("i").unwrap().layers[0];
    layer.contours = vec![
        // Two conflicting stems force replacement masks, and the cupped serif
        // of Type 1 figure 8e carries the flex hint.
        fc_model::Contour {
            closed: true,
            points: vec![
                point(100.0, 690.0, PointKind::Line),
                point(115.0, 690.0, PointKind::OffCurve),
                point(125.0, 700.0, PointKind::OffCurve),
                point(150.0, 700.0, PointKind::Curve),
                point(175.0, 700.0, PointKind::OffCurve),
                point(185.0, 690.0, PointKind::OffCurve),
                point(200.0, 690.0, PointKind::Curve),
                point(200.0, 0.0, PointKind::Line),
                point(100.0, 0.0, PointKind::Line),
            ],
            name: None,
            id: None,
        },
        Contour::from_bezpath(&Rect::new(140.0, 800.0, 220.0, 1200.0).to_path(0.1)).remove(0),
    ];
    layer.hints = PsHints {
        flex_hints: vec![fc_model::FlexHint {
            contour: 0,
            point: 0,
            depth: 50.0,
        }],
        dot_sections: vec![1],
        ..PsHints::default()
    };
    layer.hints.validate(&layer.contours).unwrap();
    let before = layer.hints.clone();
    let hints = ps::detect(&font, &"i".into(), &master).unwrap();
    assert_eq!(hints.flex_hints, before.flex_hints);
    assert_eq!(hints.dot_sections, before.dot_sections);
    assert!(
        !hints.v_stems.is_empty(),
        "the analysis still proposes stems"
    );
    hints
        .validate(&font.glyph("i").unwrap().layers[0].contours)
        .unwrap();
    // The same values reach the selection analysis both callers use.
    let analysis = ps::analyze(&font, &master, &["i".into()]).unwrap();
    assert_eq!(analysis.hints[0].1.flex_hints, before.flex_hints);
    assert_eq!(analysis.hints[0].1.dot_sections, before.dot_sections);
    // An unresolvable imported flex marker is kept too, so export still refuses it.
    font.glyph_mut("i").unwrap().layers[0].hints.flex = true;
    assert!(ps::detect(&font, &"i".into(), &master).unwrap().flex);
}
