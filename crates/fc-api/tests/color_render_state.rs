//! Read-only color payloads and selected-master COLRv1 preview geometry.
use fc_api::Api;
use fc_doc::Session;
use fc_model::{ColorTable, Contour, Font, Glyph, Layer, Master, SvgDocument, SvgTable, Tag};
use kurbo::{BezPath, Shape};
use serde_json::{json, Value};
use std::sync::Arc;

fn source(graph: bool) -> Font {
    let mut font = Font::new_empty("Color rendering", 1000);
    let master = font.default_master().id.clone();
    for name in ["A", "V"] {
        let mut glyph = Glyph::new(name);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 600.;
        if name == "V" {
            layer.contours =
                Contour::from_bezpath(&BezPath::from_svg("M100 0L500 0L500 700L100 700Z").unwrap());
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    let mut color = json!({
        "palettes":[{"colors":[{"red":255,"green":30,"blue":40,"alpha":127}]}],
        "glyphs":{"A":[{"glyph":"V","palette_index":0}]}
    });
    if graph {
        color["graph"] = json!({"glyphs":{"A":0},"paints":[
            {"kind":"Glyph","glyph":"V","paint":1},
            {"kind":"Solid","palette_index":0,"alpha":0.5}
        ]});
    }
    font.color = Some(serde_json::from_value::<ColorTable>(color).unwrap());
    font.color_tables_owned = true;
    font
}

fn imported(graph: bool) -> Font {
    let report = fc_compile::export(&source(graph), &|_| 0, &Default::default()).unwrap();
    let mut font = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let order: Vec<_> = font.glyph_order().cloned().collect();
    let (colr, cpal) = fc_sfnt::color::write_tables(&font.color.take().unwrap(), &order).unwrap();
    font.color_tables_owned = false;
    font.extra.raw_tables.insert(Tag(*b"COLR"), colr.unwrap());
    font.extra.raw_tables.insert(Tag(*b"CPAL"), cpal);
    let svg = SvgTable {
        documents: vec![SvgDocument {
            source: r#"<svg xmlns="http://www.w3.org/2000/svg" id="art"><rect width="50" height="50" fill="red"/></svg>"#.into(),
            glyphs: [("A".into(), "art".into())].into(),
        }],
    };
    font.extra.raw_tables.insert(
        Tag(*b"SVG "),
        fc_sfnt::svg::write_table(&svg, &order).unwrap(),
    );
    font
}

#[test]
fn imported_render_payloads_preserve_ownership_generation_raw_bytes_and_undo() {
    for graph in [false, true] {
        let source = imported(graph);
        let order: Vec<_> = source.glyph_order().cloned().collect();
        let expected_color = fc_sfnt::color::read_tables(
            Some(&source.extra.raw_tables[&Tag(*b"COLR")]),
            &source.extra.raw_tables[&Tag(*b"CPAL")],
            &order,
        )
        .unwrap();
        let expected_svg =
            fc_sfnt::svg::read_table(&source.extra.raw_tables[&Tag(*b"SVG ")], &order).unwrap();
        let session = Arc::new(Session::new());
        let doc = session.open_font(source, None);
        let original = session
            .with_doc(doc, |document| document.font.clone())
            .unwrap();
        let api = Api::new(session.clone());
        for _ in 0..2 {
            let color = api.call("color.state", json!({"doc":doc})).unwrap();
            assert_eq!(color["status"], "Imported");
            assert!(color["color"].is_null());
            assert_eq!(color["preview_color"], json!(expected_color));
            assert!(color["import_error"].is_null());
            let svg = api.call("svg.state", json!({"doc":doc})).unwrap();
            assert_eq!(svg["status"], "Imported");
            assert!(svg["svg"].is_null());
            assert_eq!(svg["preview_svg"], json!(expected_svg));
            assert!(svg["import_error"].is_null());
            if graph {
                api.call(
                    "color.graph_preview",
                    json!({"doc":doc,"glyph":"A","color":color["preview_color"],"palette_index":0}),
                )
                .unwrap();
            } else {
                let preview = api.call("color.preview", json!({"doc":doc,"glyph":"A","layers":color["preview_color"]["glyphs"]["A"],"palette":color["preview_color"]["palettes"][0]["colors"]})).unwrap();
                assert_eq!(preview["layers"][0]["color"]["alpha"], 127);
            }
        }
        session
            .with_doc(doc, |document| {
                assert_eq!(document.font, original);
                assert_eq!(document.generation, 0);
                assert_eq!(document.undo_len(), 0);
            })
            .unwrap();
    }
}

#[test]
fn invalid_imports_expose_errors_without_render_payloads_or_mutation() {
    for malformed in [false, true] {
        let mut source = imported(false);
        if malformed {
            source
                .extra
                .raw_tables
                .get_mut(&Tag(*b"COLR"))
                .unwrap()
                .truncate(3);
            source
                .extra
                .raw_tables
                .get_mut(&Tag(*b"SVG "))
                .unwrap()
                .truncate(3);
        } else {
            source.rename_glyph("V", "renamed.V").unwrap();
            source.rename_glyph("A", "renamed.A").unwrap();
        }
        let session = Arc::new(Session::new());
        let doc = session.open_font(source, None);
        let original = session
            .with_doc(doc, |document| document.font.clone())
            .unwrap();
        let api = Api::new(session.clone());
        for (method, field, preview) in [
            ("color.state", "color", "preview_color"),
            ("svg.state", "svg", "preview_svg"),
        ] {
            let state = api.call(method, json!({"doc":doc})).unwrap();
            assert_eq!(state["status"], "Imported");
            assert!(state[field].is_null());
            assert!(state[preview].is_null());
            assert!(state["import_error"].is_string());
        }
        session
            .with_doc(doc, |document| {
                assert_eq!(document.font, original);
                assert_eq!(document.generation, 0);
                assert_eq!(document.undo_len(), 0);
            })
            .unwrap();
    }
}

#[test]
fn graph_preview_uses_selected_master_outlines_and_advance() {
    let mut source = source(true);
    let alternate = Master {
        name: "Wide".into(),
        ..Default::default()
    };
    for name in ["A", "V"] {
        let mut layer = Layer::new_master(alternate.id.clone());
        layer.width = 900.;
        if name == "V" {
            layer.contours =
                Contour::from_bezpath(&BezPath::from_svg("M200 0L800 0L800 750L200 750Z").unwrap());
        }
        source.glyph_mut(name).unwrap().layers.push(layer);
    }
    source.masters.push(alternate.clone());
    let color = source.color.clone().unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(source, None);
    let api = Api::new(session.clone());
    for (master, width, bounds) in [
        (Value::Null, 600., kurbo::Rect::new(100., 0., 500., 700.)),
        (
            json!(alternate.id),
            900.,
            kurbo::Rect::new(200., 0., 800., 750.),
        ),
    ] {
        let result = api
            .call(
                "color.graph_preview",
                json!({"doc":doc,"glyph":"A","color":color,"palette_index":0,"master":master}),
            )
            .unwrap();
        let bytes: Vec<u8> = serde_json::from_value(result["bytes"].clone()).unwrap();
        let output = fc_sfnt::read_bytes(&bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let selected = &output.default_master().id;
        assert_eq!(
            output
                .glyph("A")
                .unwrap()
                .master_layer(selected)
                .unwrap()
                .width,
            width
        );
        assert_eq!(
            fc_geom::flatten_layer(&output, "V", selected)
                .unwrap()
                .bounding_box(),
            bounds
        );
    }
    let invalid = api.call("color.graph_preview", json!({"doc":doc,"glyph":"A","color":color,"palette_index":0,"master":fc_model::MasterId::new()})).unwrap_err();
    assert!(invalid.message.contains("master"));
    session
        .with_doc(doc, |document| {
            assert_eq!(document.generation, 0);
            assert_eq!(document.undo_len(), 0);
        })
        .unwrap();
}

#[test]
fn graph_preview_does_not_fall_back_when_selected_master_has_no_layer() {
    let mut source = source(true);
    let alternate = Master::default();
    source.masters.push(alternate.clone());
    let color = source.color.clone().unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(source, None);
    let api = Api::new(session.clone());
    let before = api.call("color.state", json!({"doc":doc})).unwrap();
    assert_eq!(before["status"], "Editable");
    assert!(before.get("preview_color").is_none());
    let error = api
        .call(
            "color.graph_preview",
            json!({"doc":doc,"glyph":"A","color":color,"palette_index":0,"master":alternate.id}),
        )
        .unwrap_err();
    assert!(error.message.contains("selected-master layer"));
    assert_eq!(api.call("color.state", json!({"doc":doc})).unwrap(), before);
    assert_eq!(
        session
            .with_doc(doc, |document| document.undo_len())
            .unwrap(),
        0
    );
}
