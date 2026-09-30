//! Changing units per em, with the drawing scaled in one undo entry.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Anchor, Component, Contour, Font, Glyph, KernSide, Layer, LayerId, Stem, Zone};
use kurbo::{Affine, BezPath};
use serde_json::json;

fn source() -> Font {
    let mut font = Font::new_empty("Rescale", 1000);
    let master = font.default_master().id.clone();
    font.masters[0].metrics.ascender = 800.0;
    font.masters[0].metrics.descender = -200.0;
    font.masters[0].metrics.x_height = 500.0;
    font.masters[0].metrics.cap_height = 700.0;
    font.masters[0].metrics.typo_ascender = Some(750.0);
    font.masters[0].alignment_zones.push(Zone {
        position: 700.0,
        size: 12.0,
    });
    font.masters[0].stems.push(Stem {
        name: "stem".into(),
        horizontal: false,
        width: 80.0,
    });
    font.masters[0].private_dict.blue_values = vec![-12.0, 0.0, 700.0, 712.0];
    font.masters[0].private_dict.std_vw = Some(80.0);
    font.masters[0].guides.push(fc_model::Guide {
        x: 100.0,
        y: 200.0,
        ..Default::default()
    });
    font.guides.push(fc_model::Guide {
        x: 10.0,
        y: 20.0,
        ..Default::default()
    });
    for name in [".notdef", "A", "Aacute"] {
        let mut glyph = Glyph::new(name);
        let mut layer = Layer::new_master(master.clone());
        layer.id = LayerId::new();
        layer.width = 600.0;
        layer.vertical_width = Some(1000.0);
        layer.vertical_origin = Some(800.0);
        if name == "Aacute" {
            layer.components.push(Component {
                base: "A".into(),
                transform: Affine::new([1.0, 0.0, 0.0, 1.0, 40.0, 60.0]),
                ..Default::default()
            });
        } else {
            let mut path = BezPath::new();
            path.move_to((50.0, 0.0));
            path.line_to((50.0, 700.0));
            path.line_to((500.0, 700.0));
            path.line_to((500.0, 0.0));
            path.close_path();
            layer.contours = Contour::from_bezpath(&path);
            layer.contours[0].points[0].corner_radius = 20.0;
        }
        layer.anchors.push(Anchor {
            name: "top".into(),
            x: 275.0,
            y: 700.0,
        });
        layer.hints.v_stems.push(fc_model::StemHint {
            start: 50.0,
            width: 80.0,
            ranges: vec![(0.0, 700.0)],
        });
        layer.hints.h_stems.push(fc_model::StemHint {
            start: 700.0,
            width: -20.0,
            ranges: Vec::new(),
        });
        layer.tt_instructions = Some(vec![0x00, 0x01]);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert glyph");
    }
    font.masters[0].kerning_h.0.insert(
        (
            KernSide::Glyph("A".into()),
            KernSide::Glyph("Aacute".into()),
        ),
        -40.0,
    );
    font
}

fn api_with(font: Font) -> (Api, fc_doc::DocId, Arc<Session>) {
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(Arc::clone(&session)), doc, session)
}

fn rescale(
    api: &Api,
    doc: fc_doc::DocId,
    generation: u64,
    extra: serde_json::Value,
) -> serde_json::Value {
    let mut params = json!({
        "doc": doc,
        "expected_generation": generation,
        "units_per_em": 2000,
        "label": "Change units per em",
    });
    for (key, value) in extra.as_object().expect("object") {
        params[key] = value.clone();
    }
    api.call("variation.rescale_upm", params)
        .expect("rescale units per em")
}

#[test]
fn scaling_everything_doubles_the_drawing_metrics_kerning_and_guides() {
    let (api, doc, session) = api_with(source());
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    let result = rescale(
        &api,
        doc,
        generation,
        json!({
            "scale_outlines": true,
            "scale_metrics": true,
            "scale_kerning": true,
            "scale_guides": true,
        }),
    );
    assert_eq!(result["factor"], 2.0);
    assert_eq!(result["glyphs"], 3);
    assert_eq!(result["cleared_programs"], 3);

    session
        .with_doc(doc, |document| {
            let font = &document.font;
            assert_eq!(font.info.units_per_em, 2000);
            let master = &font.masters[0];
            assert_eq!(master.metrics.ascender, 1600.0);
            assert_eq!(master.metrics.descender, -400.0);
            assert_eq!(master.metrics.typo_ascender, Some(1500.0));
            assert_eq!(master.alignment_zones[0].position, 1400.0);
            assert_eq!(master.alignment_zones[0].size, 24.0);
            assert_eq!(master.stems[0].width, 160.0);
            assert_eq!(
                master.private_dict.blue_values,
                vec![-24.0, 0.0, 1400.0, 1424.0]
            );
            assert_eq!(master.private_dict.std_vw, Some(160.0));
            assert_eq!(master.guides[0].x, 200.0);
            assert_eq!(font.guides[0].y, 40.0);
            assert_eq!(
                master.kerning_h.0.values().copied().collect::<Vec<_>>(),
                vec![-80.0]
            );
            let a = font.glyph("A").expect("A");
            let layer = &a.layers[0];
            assert_eq!(layer.width, 1200.0);
            assert_eq!(layer.vertical_width, Some(2000.0));
            assert_eq!(layer.vertical_origin, Some(1600.0));
            assert_eq!(layer.contours[0].points[0].x, 100.0);
            assert_eq!(layer.contours[0].points[0].corner_radius, 40.0);
            assert_eq!(layer.anchors[0].y, 1400.0);
            assert_eq!(layer.hints.v_stems[0].start, 100.0);
            assert_eq!(layer.hints.v_stems[0].width, 160.0);
            assert_eq!(layer.hints.v_stems[0].ranges[0], (0.0, 1400.0));
            // A ghost hint keeps its reserved width.
            assert_eq!(layer.hints.h_stems[0].width, -20.0);
            assert!(layer.tt_instructions.is_none());
            // A component keeps its linear part and scales only its offset.
            let composite = &font.glyph("Aacute").expect("Aacute").layers[0];
            let coefficients = composite.components[0].transform.as_coeffs();
            assert_eq!(coefficients, [1.0, 0.0, 0.0, 1.0, 80.0, 120.0]);
        })
        .expect("read the rescaled font");
}

#[test]
fn the_whole_change_is_one_undo_entry() {
    let (api, doc, session) = api_with(source());
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    rescale(
        &api,
        doc,
        generation,
        json!({"scale_outlines": true, "scale_metrics": true, "scale_kerning": true, "scale_guides": true}),
    );
    let label = session
        .with_doc(doc, |d| d.undo_label().map(str::to_owned))
        .expect("undo label");
    assert_eq!(label.as_deref(), Some("Change units per em"));
    api.call("doc.undo", json!({"doc": doc})).expect("undo");
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.info.units_per_em, 1000);
            assert_eq!(document.font.masters[0].metrics.ascender, 800.0);
            assert_eq!(
                document.font.glyph("A").expect("A").layers[0].contours[0].points[0].x,
                50.0
            );
            assert_eq!(
                document.font.glyph("A").expect("A").layers[0]
                    .tt_instructions
                    .as_deref(),
                Some([0x00, 0x01].as_slice())
            );
        })
        .expect("read the restored font");
    assert!(session
        .with_doc(doc, |d| d.undo_label().map(str::to_owned))
        .expect("label")
        .is_none());
}

#[test]
fn changing_only_the_number_leaves_the_drawing_alone() {
    let (api, doc, session) = api_with(source());
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    let result = rescale(&api, doc, generation, json!({}));
    assert_eq!(result["glyphs"], 0);
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.info.units_per_em, 2000);
            assert_eq!(document.font.masters[0].metrics.ascender, 800.0);
            assert_eq!(
                document.font.glyph("A").expect("A").layers[0].contours[0].points[0].x,
                50.0
            );
        })
        .expect("read the font");
}

#[test]
fn an_unmodelled_layer_feature_refuses_before_any_change() {
    let mut font = source();
    font.glyph_mut("A").expect("A").layers[0]
        .effects
        .push(fc_model::LayerEffect {
            enabled: true,
            effect: fc_model::LayerEffectKind::RemoveOverlap,
        });
    let (api, doc, session) = api_with(font);
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    let error = api
        .call(
            "variation.rescale_upm",
            json!({
                "doc": doc,
                "expected_generation": generation,
                "units_per_em": 2000,
                "scale_outlines": true,
                "label": "Change units per em",
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
    assert!(error.message.contains("A carries layer effects"), "{error}");
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.info.units_per_em, 1000);
            assert!(document.undo_label().is_none());
        })
        .expect("read the untouched font");
}

#[test]
fn invalid_values_and_stale_revisions_are_refused() {
    let (api, doc, session) = api_with(source());
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    for upm in [0, 15, 16_385] {
        let error = api
            .call(
                "variation.rescale_upm",
                json!({
                    "doc": doc,
                    "expected_generation": generation,
                    "units_per_em": upm,
                    "label": "Change units per em",
                }),
            )
            .unwrap_err();
        assert!(error.message.contains("16"), "{upm}: {error}");
    }
    let error = api
        .call(
            "variation.rescale_upm",
            json!({
                "doc": doc,
                "expected_generation": generation + 5,
                "units_per_em": 2000,
                "label": "Change units per em",
            }),
        )
        .unwrap_err();
    assert!(error.message.contains("reload"), "{error}");
}

#[test]
fn stale_preserved_tables_are_reported_rather_than_left_silent() {
    let mut font = source();
    font.extra
        .raw_tables
        .insert(fc_model::Tag(*b"kern"), vec![0, 0, 0, 0]);
    let (api, doc, session) = api_with(font);
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    let result = rescale(
        &api,
        doc,
        generation,
        json!({"scale_outlines": true, "scale_metrics": true}),
    );
    let warnings = result["warnings"].as_array().expect("warnings");
    assert!(
        warnings.iter().any(|warning| warning
            .as_str()
            .is_some_and(|text| text.contains("table preservation"))),
        "{warnings:?}"
    );
}

#[test]
fn per_font_editor_settings_override_the_preferences_and_survive_a_reopen() {
    let (api, doc, session) = api_with(source());
    let defaults = api
        .call("fontinfo.editor_settings_get", json!({"doc": doc}))
        .expect("read editor settings");
    assert_eq!(defaults["stored"], json!({}));
    let preference_grid = defaults["grid_size"].as_f64().expect("grid size");

    let saved = api
        .call(
            "fontinfo.editor_settings_set",
            json!({
                "doc": doc,
                "settings": {"grid_size": 4.0, "grid_subdivision": 2, "nudge": [2.0, 20.0, 200.0]},
            }),
        )
        .expect("store editor settings");
    assert_eq!(saved["grid_size"], 4.0);
    assert_eq!(saved["grid_subdivision"], 2);
    assert_eq!(saved["nudge"], json!([2.0, 20.0, 200.0]));
    assert_ne!(
        saved["grid_size"].as_f64(),
        Some(preference_grid).filter(|value| *value != 4.0)
    );

    // Storing them is an ordinary undo entry in the document.
    let label = session
        .with_doc(doc, |d| d.undo_label().map(str::to_owned))
        .expect("undo label");
    assert_eq!(label.as_deref(), Some("Edit font editor settings"));

    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("settings.fcf");
    api.call("session.save", json!({"doc": doc, "path": path}))
        .expect("save the project");
    let reopened = Arc::new(Session::new());
    let reopened_doc = reopened.open_path(&path, None).expect("reopen");
    let reopened_api = Api::new(reopened);
    let stored = reopened_api
        .call("fontinfo.editor_settings_get", json!({"doc": reopened_doc}))
        .expect("read stored settings");
    assert_eq!(stored["stored"]["grid_size"], 4.0);
    assert_eq!(stored["nudge"], json!([2.0, 20.0, 200.0]));

    // Clearing one field falls back to the preference again.
    let cleared = api
        .call(
            "fontinfo.editor_settings_set",
            json!({"doc": doc, "settings": {"grid_subdivision": 2}}),
        )
        .expect("clear the grid size");
    assert!(cleared["stored"]["grid_size"].is_null());
    assert_eq!(cleared["grid_size"].as_f64(), Some(preference_grid));
}

#[test]
fn invalid_editor_settings_are_refused() {
    let (api, doc, _session) = api_with(source());
    for (settings, expected) in [
        (json!({"grid_size": -1.0}), "grid size"),
        (json!({"grid_size": 20000.0}), "grid size"),
        (json!({"grid_subdivision": 0}), "subdivision"),
        (json!({"grid_subdivision": 101}), "subdivision"),
        (json!({"nudge": [0.0, 10.0, 100.0]}), "increments"),
        (json!({"nudge": [10.0, 1.0, 100.0]}), "decrease"),
    ] {
        let error = api
            .call(
                "fontinfo.editor_settings_set",
                json!({"doc": doc, "settings": settings}),
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_params", "{settings}: {error}");
        assert!(error.message.contains(expected), "{settings}: {error}");
    }
}

/// A font whose sidebearings are formulas rather than stored numbers.
fn keyed() -> Font {
    let mut font = source();
    let glyph = font.glyph_mut("Aacute").expect("Aacute");
    glyph.metrics_keys = fc_model::MetricsKeys {
        left: Some("=A+25".into()),
        right: Some("=50".into()),
        // A ratio is dimensionless and must survive untouched.
        width: Some("=A*1.5".into()),
        vertical_origin: Some("=+30".into()),
        vertical_width: None,
    };
    glyph.layers[0].metrics_keys = Some(fc_model::MetricsKeys {
        left: Some("=A-10@200".into()),
        ..Default::default()
    });
    font
}

#[test]
fn metrics_formulas_are_rescaled_in_both_scopes() {
    let (api, doc, session) = api_with(keyed());
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    // Metrics only: the formulas belong to the metrics group, not the outlines.
    let result = rescale(&api, doc, generation, json!({ "scale_metrics": true }));
    assert_eq!(result["glyphs"], 1);
    assert_eq!(result["layers"], 0);
    session
        .with_doc(doc, |document| {
            let glyph = document.font.glyph("Aacute").expect("Aacute");
            assert_eq!(glyph.metrics_keys.left.as_deref(), Some("=A+50"));
            assert_eq!(glyph.metrics_keys.right.as_deref(), Some("=100"));
            assert_eq!(glyph.metrics_keys.width.as_deref(), Some("=A*1.5"));
            assert_eq!(glyph.metrics_keys.vertical_origin.as_deref(), Some("=+60"));
            assert_eq!(
                glyph.layers[0]
                    .metrics_keys
                    .as_ref()
                    .and_then(|keys| keys.left.as_deref()),
                Some("=A-20@400")
            );
            // The drawing was not part of the request.
            assert_eq!(glyph.layers[0].components[0].transform.as_coeffs()[4], 40.0);
            // Still one undo entry for the whole change.
            assert_eq!(document.undo_label(), Some("Change units per em"));
        })
        .expect("read the rescaled font");
    session
        .with_doc_mut(doc, |document| document.undo())
        .expect("undo")
        .expect("undo the rescale");
    session
        .with_doc(doc, |document| {
            let glyph = document.font.glyph("Aacute").expect("Aacute");
            assert_eq!(glyph.metrics_keys.left.as_deref(), Some("=A+25"));
            assert_eq!(document.font.info.units_per_em, 1000);
        })
        .expect("read the restored font");
}

#[test]
fn a_formula_this_operation_cannot_rescale_refuses_before_any_change() {
    let mut font = keyed();
    font.glyph_mut("Aacute").expect("Aacute").metrics_keys.right = Some("=A@nonsense".into());
    let (api, doc, session) = api_with(font);
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    let error = api
        .call(
            "variation.rescale_upm",
            json!({
                "doc": doc,
                "expected_generation": generation,
                "units_per_em": 2000,
                "scale_metrics": true,
                "label": "Change units per em",
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
    assert!(
        error.message.contains("Aacute") && error.message.contains("=A@nonsense"),
        "{error}"
    );
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font.info.units_per_em, 1000);
            assert_eq!(
                document.font.masters[0].metrics.ascender, 800.0,
                "the metrics must be untouched"
            );
            assert!(document.undo_label().is_none());
        })
        .expect("read the untouched font");
}
