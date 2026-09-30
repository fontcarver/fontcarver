mod common;

use fc_doc::{metrics, Command, Error, MetricsSource, Session};
use fc_model::{
    Anchor, Component, Contour, Glyph, GlyphName, Layer, LayerKind, MetricsKeys, Point, PointKind,
};
use kurbo::Affine;

use common::test_font;

#[test]
fn set_metrics_moves_all_positioned_content_and_applies_width_lsb_rsb_order() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let target = &mut doc.font.glyph_mut(&glyph).unwrap().layers[0];
            target.components.push(Component {
                base: "missing-until-query".into(),
                transform: Affine::translate((5.0, 7.0)),
                ..Component::default()
            });
            target.anchors.push(Anchor {
                name: "top".to_owned(),
                x: 50.0,
                y: 100.0,
            });
            // Keep bounds flattenable while still observing component translation.
            doc.font
                .insert_glyph(Glyph::new("missing-until-query"))
                .unwrap();
            let before = doc.font.glyph(&glyph).unwrap().clone();
            let change = doc
                .apply(
                    Command::SetMetrics {
                        glyph: glyph.clone(),
                        layer,
                        width: Some(600.0),
                        lsb: Some(20.0),
                        rsb: Some(30.0),
                    },
                    "Metrics",
                )
                .unwrap();
            assert!(change.metrics);
            let target = &doc.font.glyph(&glyph).unwrap().layers[0];
            assert_eq!(target.contours[0].points[0].x, 20.0);
            assert_eq!(target.components[0].transform.as_coeffs()[4], 25.0);
            assert_eq!(target.anchors[0].x, 70.0);
            assert_eq!(target.width, 150.0);
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
        })
        .unwrap();
}

#[test]
fn metrics_keys_resolve_mirroring_offsets_literals_and_cycles() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    font.insert_glyph(Glyph::new("keyed")).unwrap();
    font.glyph_mut("keyed").unwrap().metrics_keys = MetricsKeys {
        left: Some("=|base+10".to_owned()),
        right: Some("=base-5".to_owned()),
        width: Some("=700".to_owned()),
        ..MetricsKeys::default()
    };
    let resolved = metrics::resolve(&font, &GlyphName::from("keyed"), &master).unwrap();
    assert_eq!(resolved.lsb, 410.0);
    assert_eq!(resolved.rsb, 395.0);
    assert_eq!(resolved.width, 700.0);
    assert_eq!(resolved.source, MetricsSource::Key);

    font.glyph_mut("base").unwrap().metrics_keys.left = Some("=keyed".to_owned());
    font.glyph_mut("keyed").unwrap().metrics_keys.left = Some("=base".to_owned());
    assert!(matches!(
        metrics::resolve(&font, &GlyphName::from("keyed"), &master),
        Err(Error::MetricsKeyCycle)
    ));
}

#[test]
fn set_metrics_keys_is_undoable() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.glyph("base").unwrap().clone();
            let change = doc
                .apply(
                    Command::SetMetricsKeys {
                        glyph: "base".into(),
                        keys: MetricsKeys {
                            width: Some("=600".to_owned()),
                            ..MetricsKeys::default()
                        },
                    },
                    "Keys",
                )
                .unwrap();
            assert!(change.metrics);
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap(), &before);
        })
        .unwrap();
}

#[test]
fn set_metrics_uses_the_commands_own_layer_bounds() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let master = doc.font.default_master().id.clone();
            let mut background = Layer {
                kind: LayerKind::Background(master),
                width: 500.0,
                ..Layer::default()
            };
            background.contours.push(Contour {
                closed: true,
                points: vec![
                    Point {
                        x: 200.0,
                        y: 0.0,
                        kind: PointKind::Line,
                        ..Point::default()
                    },
                    Point {
                        x: 300.0,
                        y: 0.0,
                        kind: PointKind::Line,
                        ..Point::default()
                    },
                    Point {
                        x: 300.0,
                        y: 100.0,
                        kind: PointKind::Line,
                        ..Point::default()
                    },
                ],
                ..Contour::default()
            });
            let background_id = background.id.clone();
            doc.apply(
                Command::AddLayer {
                    glyph: glyph.clone(),
                    layer: background,
                },
                "Background",
            )
            .unwrap();

            doc.apply(
                Command::SetMetrics {
                    glyph: glyph.clone(),
                    layer: background_id.clone(),
                    width: None,
                    lsb: Some(50.0),
                    rsb: Some(10.0),
                },
                "Background metrics",
            )
            .unwrap();

            let stored = doc.font.glyph(&glyph).unwrap();
            let background = stored.layer(&background_id).unwrap();
            // The background layer's own bounds are 200..300, not the master's 0..100.
            assert_eq!(background.contours[0].points[0].x, 50.0);
            assert_eq!(background.width, 160.0);
            // The master layer is untouched.
            assert_eq!(stored.layers[0].contours[0].points[0].x, 0.0);
            assert_eq!(stored.layers[0].width, 500.0);
        })
        .unwrap();
}

#[test]
fn height_keys_measure_the_outline_cross_section() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    font.glyph_mut("base").unwrap().layers[0].contours[0].points = vec![
        common::point(0.0, 0.0),
        common::point(100.0, 0.0),
        common::point(60.0, 100.0),
        common::point(40.0, 100.0),
    ];
    font.glyph_mut("composite").unwrap().metrics_keys.left = Some("=base@50".into());
    let answer = metrics::resolve(&font, &"composite".into(), &master).unwrap();
    assert_eq!(answer.lsb, 20.0);
    assert_eq!(answer.width, 620.0);
}

#[test]
fn local_metrics_inherit_unset_fields_and_stay_in_their_layer() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    let original = font.glyph("base").unwrap().layers[0].id.clone();
    let mut local = font.glyph("base").unwrap().layers[0].clone();
    local.id = fc_model::LayerId::new();
    local.kind = LayerKind::Background(master.clone());
    local.metrics_keys = Some(MetricsKeys {
        left: Some("==40".into()),
        ..Default::default()
    });
    let local_id = local.id.clone();
    font.glyph_mut("base").unwrap().layers.push(local);
    font.glyph_mut("base").unwrap().metrics_keys.right = Some("=30".into());
    let answer = metrics::resolve_layer(&font, &"base".into(), &master, &local_id).unwrap();
    assert_eq!((answer.lsb, answer.rsb, answer.width), (40.0, 30.0, 170.0));
    let answer = metrics::resolve_layer(&font, &"base".into(), &master, &original).unwrap();
    assert_eq!((answer.lsb, answer.rsb, answer.width), (0.0, 30.0, 130.0));
}

#[test]
fn auto_alignment_offsets_use_the_base_metrics_without_drifting() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    let glyph = font.glyph_mut("composite").unwrap();
    glyph.layers[0].components[0].auto_align = true;
    glyph.metrics_keys.left = Some("=+25".into());
    let answer = metrics::resolve(&font, &"composite".into(), &master).unwrap();
    assert_eq!(answer.lsb, 25.0);
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let command = Command::UpdateMetrics {
                glyph: "composite".into(),
                master: Some(master.clone()),
            };
            doc.apply(command.clone(), "Update metrics").unwrap();
            let once = doc.font.glyph("composite").unwrap().clone();
            doc.apply(command, "Update metrics again").unwrap();
            assert_eq!(doc.font.glyph("composite").unwrap(), &once);
        })
        .unwrap();
}

#[test]
fn update_current_and_all_masters_is_undoable_and_idempotent() {
    let mut font = test_font();
    let first = font.default_master().id.clone();
    let mut second = font.default_master().clone();
    second.id = fc_model::MasterId::new();
    second.name = "Second".into();
    let second_id = second.id.clone();
    font.masters.push(second);
    let glyph = font.glyph_mut("base").unwrap();
    let mut layer = glyph.layers[0].clone();
    layer.id = fc_model::LayerId::new();
    layer.kind = LayerKind::Master(second_id.clone());
    layer.metrics_keys = Some(MetricsKeys {
        left: Some("==40".into()),
        ..Default::default()
    });
    glyph.layers.push(layer);
    glyph.metrics_keys.left = Some("=20".into());
    glyph.metrics_keys.right = Some("=30".into());
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.glyph("base").unwrap().clone();
            doc.apply(
                Command::UpdateMetrics {
                    glyph: "base".into(),
                    master: Some(first.clone()),
                },
                "Update current",
            )
            .unwrap();
            let current = doc.font.glyph("base").unwrap();
            assert_eq!(current.master_layer(&first).unwrap().width, 150.0);
            assert_eq!(current.master_layer(&second_id).unwrap().width, 500.0);
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap(), &before);
            doc.apply(
                Command::UpdateMetrics {
                    glyph: "base".into(),
                    master: None,
                },
                "Update all",
            )
            .unwrap();
            assert_eq!(
                doc.font
                    .glyph("base")
                    .unwrap()
                    .master_layer(&second_id)
                    .unwrap()
                    .width,
                170.0
            );
            let once = doc.font.glyph("base").unwrap().clone();
            doc.apply(
                Command::UpdateMetrics {
                    glyph: "base".into(),
                    master: None,
                },
                "Update all again",
            )
            .unwrap();
            assert_eq!(doc.font.glyph("base").unwrap(), &once);
            let scratch = common::temp_dir("metrics-local-roundtrip");
            let path = scratch.join("metrics.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            let reloaded = fc_doc::fcf::read(&path).unwrap();
            assert_eq!(reloaded.glyph("base").unwrap(), &once);
            std::fs::remove_dir_all(scratch).unwrap();
        })
        .unwrap();
}

#[test]
fn conflicting_metrics_update_fails_without_mutation() {
    let mut font = test_font();
    font.glyph_mut("base").unwrap().metrics_keys = MetricsKeys {
        left: Some("=20".into()),
        right: Some("=30".into()),
        width: Some("=999".into()),
        ..Default::default()
    };
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.glyph("base").unwrap().clone();
            assert!(doc
                .apply(
                    Command::UpdateMetrics {
                        glyph: "base".into(),
                        master: None
                    },
                    "Conflicting"
                )
                .is_err());
            assert_eq!(doc.font.glyph("base").unwrap(), &before);
        })
        .unwrap();
}

#[test]
fn width_key_preserves_lsb_and_updates_the_opposite_sidebearing() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    font.glyph_mut("base").unwrap().metrics_keys.width = Some("=700".into());
    let answer = metrics::resolve(&font, &"base".into(), &master).unwrap();
    assert_eq!((answer.lsb, answer.rsb, answer.width), (0.0, 600.0, 700.0));
}

#[test]
fn malformed_keys_and_unaligned_offsets_are_rejected() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    for key in [
        "=NaN",
        "=inf",
        "=base/0",
        "=base@NaN",
        "=base@200",
        "=+20",
        "===base",
    ] {
        font.glyph_mut("base").unwrap().metrics_keys.left = Some(key.into());
        assert!(
            metrics::resolve(&font, &"base".into(), &master).is_err(),
            "{key}"
        );
    }
}

#[test]
fn local_formula_commands_are_undoable_and_reject_global_double_equals() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let keys = MetricsKeys {
                left: Some("==42".into()),
                ..Default::default()
            };
            assert!(doc
                .apply(
                    Command::SetMetricsKeys {
                        glyph: "base".into(),
                        keys: keys.clone()
                    },
                    "Bad scope"
                )
                .is_err());
            doc.apply(
                Command::SetLayerMetricsKeys {
                    glyph: "base".into(),
                    layer,
                    keys: Some(keys.clone()),
                },
                "Local key",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0]
                    .metrics_keys
                    .as_ref(),
                Some(&keys)
            );
            doc.undo().unwrap();
            assert!(doc.font.glyph("base").unwrap().layers[0]
                .metrics_keys
                .is_none());
            doc.redo().unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0]
                    .metrics_keys
                    .as_ref(),
                Some(&keys)
            );
        })
        .unwrap();
}

/// Formulas the grammar accepts, mirrored by `metricsKeyCommands.test.ts`.
///
/// The inspector validates a formula before sending a command, so the two
/// grammars have to agree. Both sides assert this exact table; a change on one
/// side that is not mirrored fails here or there.
const VALID_KEYS: &[&str] = &[
    "=H",
    "=H+10",
    "=H-10",
    "=H*2",
    "=H/2",
    "=|H",
    "=|",
    "=50",
    "=-25",
    "=+25",
    "==H",
    "==50",
    "=H@200",
    "=|H@200",
    "=H+10@200",
    "=0.5",
    "=.5",
    // A trailing hyphen belongs to the glyph name, not to an arithmetic tail.
    "=H-",
];

/// Formulas the grammar refuses, mirrored by `metricsKeyCommands.test.ts`.
const INVALID_KEYS: &[&str] = &[
    "=", "==", "=+", "=-x", "=H@", "=H@x", "=H/0", "=H+", "=H*", "=H V", "=H=V", "=@200",
];

#[test]
fn the_formula_grammar_matches_the_inspector_field_grammar() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            // Every accepted formula is stored by the layer command, which is
            // the only scope that may hold a local `==` formula.
            for key in VALID_KEYS {
                let keys = MetricsKeys {
                    left: Some((*key).into()),
                    ..Default::default()
                };
                doc.apply(
                    Command::SetLayerMetricsKeys {
                        glyph: "base".into(),
                        layer: layer.clone(),
                        keys: Some(keys),
                    },
                    "Valid",
                )
                .unwrap_or_else(|error| panic!("`{key}` should be accepted: {error}"));
            }
            for key in INVALID_KEYS {
                let keys = MetricsKeys {
                    left: Some((*key).into()),
                    ..Default::default()
                };
                let before = doc.font.clone();
                assert!(
                    doc.apply(
                        Command::SetLayerMetricsKeys {
                            glyph: "base".into(),
                            layer: layer.clone(),
                            keys: Some(keys),
                        },
                        "Invalid",
                    )
                    .is_err(),
                    "`{key}` should be refused"
                );
                assert_eq!(doc.font, before, "`{key}` must not change the document");
            }
        })
        .unwrap();
}

#[test]
fn vertical_formulas_are_refused_by_the_editing_commands_but_carried_through() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            for keys in [
                MetricsKeys {
                    vertical_origin: Some("=H".into()),
                    ..Default::default()
                },
                MetricsKeys {
                    vertical_width: Some("=1000".into()),
                    ..Default::default()
                },
            ] {
                let before = doc.font.clone();
                // Nothing resolves these fields, so storing one would keep a
                // formula that never takes effect.
                assert!(doc
                    .apply(
                        Command::SetMetricsKeys {
                            glyph: "base".into(),
                            keys: keys.clone(),
                        },
                        "Vertical glyph formula",
                    )
                    .is_err());
                assert!(doc
                    .apply(
                        Command::SetLayerMetricsKeys {
                            glyph: "base".into(),
                            layer: layer.clone(),
                            keys: Some(keys),
                        },
                        "Vertical layer formula",
                    )
                    .is_err());
                assert_eq!(doc.font, before);
            }
            // A value already stored on the glyph is carried through, so an
            // unrelated horizontal edit on such a glyph still works.
            doc.font
                .glyph_mut("base")
                .unwrap()
                .metrics_keys
                .vertical_origin = Some("=H".into());
            doc.apply(
                Command::SetMetricsKeys {
                    glyph: "base".into(),
                    keys: MetricsKeys {
                        left: Some("=40".into()),
                        vertical_origin: Some("=H".into()),
                        ..Default::default()
                    },
                },
                "Horizontal edit",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().metrics_keys.left.as_deref(),
                Some("=40")
            );
        })
        .unwrap();
}

#[test]
fn the_beam_measures_the_same_cross_section_a_height_formula_uses() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    let glyph = GlyphName::from("base");
    let width = font.glyph(&glyph).unwrap().layers[0].width;
    let bounds = {
        let layer = &font.glyph(&glyph).unwrap().layers[0];
        let path = fc_geom::flatten_layer_by_id(&font, &glyph, &layer.id).unwrap();
        kurbo::Shape::bounding_box(&path)
    };
    let height = (bounds.y0 + bounds.y1) / 2.0;
    let (left, right) = metrics::measure_beam(&font, &glyph, &master, None, height)
        .unwrap()
        .expect("the middle of the outline is crossed");
    assert!((left + right - (width - bounds.width())).abs() < 1e-6);

    // The same height in a formula resolves to the same left margin.
    font.glyph_mut("base").unwrap().metrics_keys.left = Some(format!("=base@{height}"));
    let resolved = metrics::resolve(&font, &glyph, &master).unwrap();
    assert!((resolved.lsb - left).abs() < 1e-6);

    // A height far above the outline is an ordinary "no measurement" answer.
    assert!(
        metrics::measure_beam(&font, &glyph, &master, None, bounds.y1 + 500.0)
            .unwrap()
            .is_none()
    );
}
