//! Shape-order data, segment status and drag-to-swap repair.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Axis, Component, Contour, Font, Glyph, Layer, LayerId, LayerKind, MasterId, Tag};
use kurbo::BezPath;
use serde_json::{json, Value};

fn rect(x: f64, y: f64, width: f64, height: f64) -> Vec<Contour> {
    let mut path = BezPath::new();
    path.move_to((x, y));
    path.line_to((x, y + height));
    path.line_to((x + width, y + height));
    path.line_to((x + width, y));
    path.close_path();
    Contour::from_bezpath(&path)
}

/// Two masters whose `A` holds two rectangles in a different shape order.
fn swapped_order_font() -> Font {
    let mut font = Font::new_empty("Compatibility", 1000);
    let regular = font.default_master().id.clone();
    font.masters[0].name = "Regular".into();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 400.0,
        default: 400.0,
        max: 700.0,
        map: Vec::new(),
        ..Default::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 400.0);
    let mut bold = font.masters[0].clone();
    bold.id = MasterId::new();
    bold.name = "Bold".into();
    bold.location.insert(Tag(*b"wght"), 700.0);
    let bold_id = bold.id.clone();
    font.masters.push(bold);

    let mut glyph = Glyph::new("A");
    glyph.unicodes.push(65);
    let mut light = Layer::new_master(regular);
    light.width = 600.0;
    light.contours = rect(0.0, 0.0, 200.0, 200.0);
    light.contours.extend(rect(300.0, 0.0, 200.0, 600.0));
    glyph.layers.push(light);
    let mut heavy = Layer::new_master(bold_id);
    heavy.id = LayerId::new();
    heavy.width = 620.0;
    // The same two rectangles, stored in the opposite order: a shape shifter.
    heavy.contours = rect(300.0, 0.0, 200.0, 600.0);
    heavy.contours.extend(rect(0.0, 0.0, 200.0, 200.0));
    glyph.layers.push(heavy);
    font.insert_glyph(glyph).expect("insert A");
    font
}

fn api_with(font: Font) -> (Api, fc_doc::DocId, Arc<Session>) {
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(Arc::clone(&session)), doc, session)
}

fn preview(api: &Api, doc: fc_doc::DocId) -> Value {
    api.call(
        "variation.preview",
        json!({"doc": doc, "glyph": "A", "location": {}}),
    )
    .expect("preview A")
}

#[test]
fn every_source_reports_its_shapes_in_shape_order() {
    let (api, doc, _session) = api_with(swapped_order_font());
    let preview = preview(&api, doc);
    let sources = preview["sources"].as_array().expect("sources");
    assert_eq!(sources.len(), 2);
    for source in sources {
        let shapes = source["shapes"].as_array().expect("shapes");
        assert_eq!(shapes.len(), 2);
        assert_eq!(shapes[0]["index"], 0);
        assert_eq!(shapes[1]["index"], 1);
        assert_eq!(shapes[0]["closed"], true);
        assert!(shapes[0]["path"]
            .as_str()
            .is_some_and(|p| p.starts_with('M')));
        assert_eq!(shapes[0]["points"].as_array().expect("points").len(), 4);
        assert_eq!(shapes[0]["on_curve"], json!([true, true, true, true]));
    }
    // Shape 0 of the two sources sits in a different place: the connectors cross.
    let first = &sources[0]["shapes"][0]["center"];
    let second = &sources[1]["shapes"][0]["center"];
    assert_ne!(first, second);
}

#[test]
fn a_component_is_a_shape_with_its_base_name_and_resolved_outline() {
    let mut font = swapped_order_font();
    let masters: Vec<MasterId> = font.masters.iter().map(|m| m.id.clone()).collect();
    let mut composite = Glyph::new("Aacute");
    for master in &masters {
        let mut layer = Layer::new_master(master.clone());
        layer.id = LayerId::new();
        layer.width = 600.0;
        layer.contours = rect(0.0, 700.0, 100.0, 60.0);
        layer.components.push(Component {
            base: "A".into(),
            ..Default::default()
        });
        composite.layers.push(layer);
    }
    font.insert_glyph(composite).expect("insert Aacute");
    let (api, doc, _session) = api_with(font);
    let result = api
        .call(
            "variation.preview",
            json!({"doc": doc, "glyph": "Aacute", "location": {}}),
        )
        .expect("preview Aacute");
    let shapes = result["sources"][0]["shapes"]
        .as_array()
        .expect("shapes")
        .clone();
    assert_eq!(shapes.len(), 2, "one contour and one component");
    assert!(shapes[0]["component"].is_null());
    assert_eq!(shapes[1]["component"], "A");
    assert_eq!(
        shapes[1]["index"], 0,
        "components index within their own kind"
    );
    assert!(shapes[1]["points"].as_array().expect("points").is_empty());
    assert!(shapes[1]["path"].as_str().is_some_and(|p| !p.is_empty()));
}

#[test]
fn segment_status_warns_when_a_direction_differs_by_more_than_the_threshold() {
    let mut font = swapped_order_font();
    // Put the shapes back in the same order, then skew one segment of Bold.
    {
        let glyph = font.glyph_mut("A").expect("A");
        glyph.layers[1].contours = rect(0.0, 0.0, 200.0, 200.0);
        glyph.layers[1]
            .contours
            .extend(rect(300.0, 0.0, 200.0, 600.0));
    }
    let (api, doc, _session) = api_with(font.clone());
    let clean = preview(&api, doc);
    let status = clean["shape_status"].as_array().expect("shape status");
    assert_eq!(status.len(), 2);
    assert!(
        status.iter().all(|shape| shape["status"] == "compatible"),
        "{status:?}"
    );

    {
        let glyph = font.glyph_mut("A").expect("A");
        let point = &mut glyph.layers[1].contours[0].points[1];
        point.x += 150.0;
    }
    let (api, doc, _session) = api_with(font);
    let skewed = preview(&api, doc);
    let status = skewed["shape_status"].as_array().expect("shape status");
    assert_eq!(status[0]["status"], "angle");
    assert!(
        status[0]["max_angle"].as_f64().expect("angle") > 20.0,
        "{status:?}"
    );
    assert!(status[0]["segments"]
        .as_array()
        .expect("segments")
        .iter()
        .any(|segment| segment == "angle"));
    assert_eq!(status[1]["status"], "compatible");
}

#[test]
fn a_different_point_count_is_reported_as_incompatible_not_warned() {
    let mut font = swapped_order_font();
    {
        let glyph = font.glyph_mut("A").expect("A");
        glyph.layers[1].contours = rect(0.0, 0.0, 200.0, 200.0);
        glyph.layers[1]
            .contours
            .extend(rect(300.0, 0.0, 200.0, 600.0));
        let extra = glyph.layers[1].contours[0].points[0].clone();
        glyph.layers[1].contours[0].points.push(extra);
    }
    let (api, doc, _session) = api_with(font);
    let result = preview(&api, doc);
    assert_eq!(result["shape_status"][0]["status"], "incompatible");
    assert!(
        result["issues"]
            .as_array()
            .expect("issues")
            .iter()
            .any(|issue| issue
                .as_str()
                .is_some_and(|text| text.contains("point count"))),
        "{:?}",
        result["issues"]
    );
}

#[test]
fn swapping_shape_order_is_one_undo_entry_across_every_source() {
    let (api, doc, session) = api_with(swapped_order_font());
    let generation = preview(&api, doc)["generation"]
        .as_u64()
        .expect("generation");
    let before: Vec<Vec<f64>> = session
        .with_doc(doc, |document| {
            document
                .font
                .glyph("A")
                .expect("A")
                .layers
                .iter()
                .map(|layer| layer.contours.iter().map(|c| c.points[0].x).collect())
                .collect()
        })
        .expect("read A");

    // Repairing a shape shifter reorders the mismatched source alone.
    let heavy = session
        .with_doc(doc, |document| {
            document.font.glyph("A").expect("A").layers[1]
                .id
                .to_string()
        })
        .expect("heavy layer id");
    api.call(
        "variation.apply",
        json!({
            "doc": doc,
            "expected_generation": generation,
            "label": "Swap shape order",
            "command": {"ReorderShapes": {"glyph": "A", "layer": heavy, "order": [1, 0]}},
        }),
    )
    .expect("reorder shapes");

    let after: Vec<Vec<f64>> = session
        .with_doc(doc, |document| {
            document
                .font
                .glyph("A")
                .expect("A")
                .layers
                .iter()
                .map(|layer| layer.contours.iter().map(|c| c.points[0].x).collect())
                .collect()
        })
        .expect("read A");
    assert_eq!(after[0], before[0], "the other source is untouched");
    assert_eq!(after[1], vec![before[1][1], before[1][0]]);
    // The heavy master now matches the light master's shape order.
    assert_eq!(after[0][0], after[1][0]);
    assert_eq!(after[0][1], after[1][1]);

    // One gesture, one undo entry.
    let label = session
        .with_doc(doc, |document| document.undo_label().map(str::to_owned))
        .expect("undo label");
    assert_eq!(label.as_deref(), Some("Swap shape order"));
    api.call("doc.undo", json!({"doc": doc})).expect("undo");
    let restored: Vec<Vec<f64>> = session
        .with_doc(doc, |document| {
            document
                .font
                .glyph("A")
                .expect("A")
                .layers
                .iter()
                .map(|layer| layer.contours.iter().map(|c| c.points[0].x).collect())
                .collect()
        })
        .expect("read A");
    assert_eq!(restored, before);
}

#[test]
fn an_invalid_permutation_leaves_the_glyph_untouched() {
    let (api, doc, session) = api_with(swapped_order_font());
    let generation = preview(&api, doc)["generation"]
        .as_u64()
        .expect("generation");
    for order in [json!([0]), json!([0, 0]), json!([0, 2]), json!([])] {
        let error = api
            .call(
                "variation.apply",
                json!({
                    "doc": doc,
                    "expected_generation": generation,
                    "label": "Swap shape order",
                    "command": {"ReorderShapes": {"glyph": "A", "order": order}},
                }),
            )
            .unwrap_err();
        assert!(error.message.contains("permutation"), "{order}: {error}");
    }
    let unchanged = session
        .with_doc(doc, |document| {
            document.font.glyph("A").expect("A").layers[0].contours[0].points[0].x
        })
        .expect("read A");
    assert_eq!(unchanged, 0.0);
}

#[test]
fn a_source_with_a_different_shape_count_refuses_the_reorder() {
    let mut font = swapped_order_font();
    {
        let glyph = font.glyph_mut("A").expect("A");
        glyph.layers[1].contours.truncate(1);
    }
    let (api, doc, _session) = api_with(font);
    let generation = preview(&api, doc)["generation"]
        .as_u64()
        .expect("generation");
    let error = api
        .call(
            "variation.apply",
            json!({
                "doc": doc,
                "expected_generation": generation,
                "label": "Swap shape order",
                "command": {"ReorderShapes": {"glyph": "A", "order": [1, 0]}},
            }),
        )
        .unwrap_err();
    assert!(error.message.contains("shape counts"), "{error}");
}

/// Two masters whose `A` carries one compound alternate region on both.
fn compound_region_font() -> Font {
    let mut font = swapped_order_font();
    let masters: Vec<MasterId> = font.masters.iter().map(|m| m.id.clone()).collect();
    // The fixture has one axis, so the region is one range.
    let region = [fc_model::BracketCondition {
        axis: Tag(*b"wght"),
        min: Some(500.0),
        max: None,
    }];
    let glyph = font.glyph_mut("A").expect("A");
    let mut alternates = Vec::new();
    for master in &masters {
        let mut layer = glyph.master_layer(master).expect("master layer").clone();
        layer.id = LayerId::new();
        layer.name = "[500<wght]".into();
        layer.kind = LayerKind::from_bracket_conditions(master.clone(), &region)
            .expect("single-axis region");
        alternates.push(layer);
    }
    glyph.layers.extend(alternates);
    font
}

#[test]
fn one_command_moves_every_source_of_an_alternate_region() {
    let (api, doc, session) = api_with(compound_region_font());
    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    api.call(
        "variation.apply",
        json!({
            "doc": doc,
            "expected_generation": generation,
            "label": "Edit condition",
            "command": {"Conditions": {
                "glyph": "A",
                "from": [{"axis": [119, 103, 104, 116], "min": 500.0}],
                "to": [{"axis": [119, 103, 104, 116], "min": 600.0, "max": 700.0}],
                "name": "[600<wght<700]",
            }},
        }),
    )
    .expect("edit the region");
    session
        .with_doc(doc, |document| {
            let layers = &document.font.glyph("A").expect("A").layers;
            let sets: Vec<_> = layers
                .iter()
                .filter_map(|layer| layer.kind.bracket_conditions())
                .collect();
            assert_eq!(sets.len(), 2, "both sources of the region moved");
            for set in &sets {
                assert_eq!(set.len(), 1);
                assert_eq!(set[0].min, Some(600.0));
                assert_eq!(set[0].max, Some(700.0));
            }
            for layer in layers {
                if layer.kind.bracket_conditions().is_some() {
                    assert_eq!(layer.name, "[600<wght<700]");
                }
            }
            // One gesture, one undo entry.
            assert_eq!(document.undo_label(), Some("Edit condition"));
        })
        .expect("read the edited glyph");

    let generation = session.with_doc(doc, |d| d.generation).expect("generation");
    let error = api
        .call(
            "variation.apply",
            json!({
                "doc": doc,
                "expected_generation": generation,
                "label": "Edit condition",
                "command": {"Conditions": {
                    "glyph": "A",
                    "from": [{"axis": [119, 103, 104, 116], "min": 500.0}],
                    "to": [{"axis": [119, 103, 104, 116], "min": 400.0}],
                    "name": "unused",
                }},
            }),
        )
        .unwrap_err();
    assert!(error.message.contains("condition set"), "{error}");
}
