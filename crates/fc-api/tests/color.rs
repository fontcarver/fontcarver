//! Color authoring through the same API used by the browser and desktop shell.
use fc_api::Api;
use fc_doc::Session;
use fc_model::{ColorTable, Font, Tag};
use serde_json::{json, Value};
use std::sync::Arc;

fn font() -> Font {
    let mut font = fc_formats::ufo::read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let mut overlay = font.glyph("A").unwrap().clone();
    overlay.name = "V".into();
    overlay.unicodes = vec![0x56];
    for layer in &mut overlay.layers {
        layer.id = fc_model::LayerId::new();
        layer.width += 50.;
    }
    font.insert_glyph(overlay).unwrap();
    font.color = Some(serde_json::from_value(json!({
        "palette_version": 1,
        "palettes": [
            {"colors": [{"red": 240, "green": 20, "blue": 10, "alpha": 128}, {"red": 10, "green": 50, "blue": 200, "alpha": 255}], "flags": 1, "label": null},
            {"colors": [{"red": 1, "green": 2, "blue": 3, "alpha": 0}, {"red": 200, "green": 0, "blue": 150, "alpha": 255}], "flags": 2, "label": null}
        ],
        "entry_labels": [null, null],
        "glyphs": {"A": [{"glyph": "A", "palette_index": 0}, {"glyph": "V", "palette_index": null}]}
    })).unwrap());
    font.color_tables_owned = true;
    font
}
fn setup(font: Font) -> (Api, Value) {
    let session = Arc::new(Session::new());
    let doc = serde_json::to_value(session.open_font(font, None)).unwrap();
    (Api::new(session), doc)
}
fn state(api: &Api, doc: &Value) -> Value {
    api.call("color.state", json!({"doc": doc})).unwrap()
}
fn apply(
    api: &Api,
    doc: &Value,
    generation: &Value,
    color: Value,
) -> Result<Value, fc_api::ApiError> {
    api.call(
        "color.apply",
        json!({"doc": doc, "expected_generation": generation, "color": color, "label": "Colors"}),
    )
}

#[test]
fn colors_are_atomic_revision_checked_undoable_and_saved() {
    let dir = tempfile::tempdir().unwrap();
    let (api, doc) = setup(font());
    let initial = state(&api, &doc);
    let mut draft = initial["color"].clone();
    draft["palettes"][0]["colors"][0]["alpha"] = json!(42);
    assert_eq!(
        apply(&api, &doc, &initial["generation"], draft.clone()).unwrap()["color"],
        true
    );
    assert!(apply(&api, &doc, &initial["generation"], Value::Null)
        .unwrap_err()
        .message
        .contains("reload"));
    let edited = state(&api, &doc);
    let mut invalid = draft.clone();
    invalid["glyphs"]["A"][0]["glyph"] = json!("missing");
    assert_eq!(
        apply(&api, &doc, &edited["generation"], invalid)
            .unwrap_err()
            .code,
        "invalid_params"
    );
    assert_eq!(state(&api, &doc), edited);
    for extension in ["fcf", "fcf.gz"] {
        let path = dir.path().join(format!("edited.{extension}"));
        api.call("session.save", json!({"doc": doc, "path": path}))
            .unwrap();
        let opened = api.call("session.open", json!({"path": path})).unwrap();
        assert_eq!(state(&api, &opened["doc"])["color"], draft);
    }
    api.call("doc.undo", json!({"doc": doc})).unwrap();
    assert_eq!(state(&api, &doc)["color"], initial["color"]);
    api.call("doc.redo", json!({"doc": doc})).unwrap();
    assert_eq!(state(&api, &doc)["color"], draft);
    let path = dir.path().join("color.ttf");
    api.call(
        "session.export",
        json!({"doc": doc, "path": path, "options": {"format": "Ttf"}}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(
            fc_sfnt::read_path(&path, &Default::default())
                .unwrap()
                .fonts
                .remove(0)
                .color
        )
        .unwrap(),
        draft
    );
    apply(&api, &doc, &state(&api, &doc)["generation"], Value::Null).unwrap();
    assert_eq!(state(&api, &doc)["status"], "Deleted");
    let path = dir.path().join("deleted.ttf");
    api.call(
        "session.export",
        json!({"doc": doc, "path": path, "options": {"format": "Ttf"}}),
    )
    .unwrap();
    let bytes = std::fs::read(path).unwrap();
    let face = read_fonts::FontRef::new(&bytes).unwrap();
    for tag in [b"COLR", b"CPAL"] {
        assert!(face.table_data(read_fonts::types::Tag::new(tag)).is_none());
    }
}

#[test]
fn previews_keep_paint_order_alpha_foreground_and_monochrome_self_references() {
    let (api, doc) = setup(font());
    let original = state(&api, &doc);
    let params =
        json!({"doc": doc, "glyph": "A", "layers": null, "palette": null, "palette_index": 0});
    let preview = api.call("color.preview", params.clone()).unwrap();
    assert_eq!(preview["layers"][0]["glyph"], "A");
    assert_eq!(preview["layers"][0]["color"]["alpha"], 128);
    assert_eq!(preview["layers"][1]["glyph"], "V");
    assert!(preview["layers"][1]["color"].is_null());
    assert!(preview["layers"][0]["path"]
        .as_str()
        .unwrap()
        .starts_with('M'));
    let mut alternate = params.clone();
    alternate["palette_index"] = json!(1);
    assert_eq!(
        api.call("color.preview", alternate).unwrap()["layers"][0]["color"]["alpha"],
        0
    );
    let mut draft = params.clone();
    draft["layers"] =
        json!([{"glyph":"V", "palette_index":0}, {"glyph":"A", "palette_index":null}]);
    draft["palette"] = json!([{"red":1,"green":2,"blue":3,"alpha":4}]);
    let preview = api.call("color.preview", draft).unwrap();
    assert_eq!(preview["layers"][0]["glyph"], "V");
    assert_eq!(preview["layers"][0]["color"]["alpha"], 4);
    for (field, value) in [
        ("palette_index", json!(999)),
        ("glyph", json!("missing")),
        (
            "layers",
            json!(vec![json!({"glyph":"A","palette_index":0}); 1025]),
        ),
    ] {
        let mut bad = params.clone();
        bad[field] = value;
        assert!(api.call("color.preview", bad).is_err());
    }
    assert_eq!(state(&api, &doc), original);
}

fn legacy() -> Font {
    let source = font();
    let report = fc_compile::export(&source, &|_| 0, &Default::default()).unwrap();
    let mut decoded = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let order = decoded.glyph_order().cloned().collect::<Vec<_>>();
    let (colr, cpal) =
        fc_sfnt::color::write_tables(&decoded.color.take().unwrap(), &order).unwrap();
    decoded.color_tables_owned = false;
    decoded
        .extra
        .raw_tables
        .insert(Tag(*b"COLR"), colr.unwrap());
    decoded.extra.raw_tables.insert(Tag(*b"CPAL"), cpal);
    decoded
}

#[test]
fn legacy_conversion_uses_provenance_and_is_undoable() {
    let source = legacy();
    let expected = serde_json::to_value(font().color).unwrap();
    let (api, doc) = setup(source.clone());
    let initial = state(&api, &doc);
    assert_eq!(initial["status"], "Imported");
    assert!(initial["import_error"].is_null());
    let params =
        json!({"doc": doc, "expected_generation": initial["generation"], "label": "Decode colors"});
    api.call("color.decode_imported", params.clone()).unwrap();
    assert_eq!(state(&api, &doc)["color"], expected);
    assert!(api.call("color.decode_imported", params).is_err());
    api.call("doc.undo", json!({"doc": doc})).unwrap();
    assert_eq!(state(&api, &doc)["status"], "Imported");
    let mut missing = source.clone();
    missing.extra.source = None;
    let (api, doc) = setup(missing);
    assert!(state(&api, &doc)["import_error"]
        .as_str()
        .unwrap()
        .contains("order"));
    let mut renamed = source;
    renamed.rename_glyph("V", "different").unwrap();
    let (api, doc) = setup(renamed);
    assert!(state(&api, &doc)["import_error"]
        .as_str()
        .unwrap()
        .contains("V"));
}

#[test]
fn unsupported_and_malformed_imports_keep_both_raw_tables() {
    for unsupported in [false, true] {
        let mut source = legacy();
        let bytes = source.extra.raw_tables.get_mut(&Tag(*b"COLR")).unwrap();
        if unsupported {
            bytes[1] = 1;
        } else {
            bytes.truncate(3);
        }
        let original = source.extra.raw_tables.clone();
        let report = fc_compile::export(&source, &|_| 0, &Default::default()).unwrap();
        let result = fc_sfnt::read_bytes(&report.bytes, &Default::default()).unwrap();
        let font = &result.fonts[0];
        assert!(font.color.is_none() && !font.color_tables_owned);
        for tag in [Tag(*b"COLR"), Tag(*b"CPAL")] {
            assert_eq!(font.extra.raw_tables[&tag], original[&tag]);
        }
        let (api, doc) = setup(font.clone());
        let before = state(&api, &doc);
        assert_eq!(before["status"], "Imported");
        assert!(before["import_error"].is_string());
        assert!(api
            .call(
                "color.decode_imported",
                json!({"doc":doc,"expected_generation":before["generation"],"label":"Decode"})
            )
            .is_err());
        assert_eq!(state(&api, &doc), before);
    }
}

#[test]
fn ufo_and_designspace_keep_palette_metadata_and_deleted_ownership() {
    let temp = tempfile::tempdir().unwrap();
    let mut source = font();
    source.axes.push(fc_model::Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.,
        default: 400.,
        max: 900.,
        ..Default::default()
    });
    source.masters[0].location.insert(Tag(*b"wght"), 400.);
    for deleted in [false, true] {
        if deleted {
            source.color = None;
        }
        let ufo = temp.path().join(format!("color-{deleted}.ufo"));
        fc_formats::ufo::write_ufo(
            &source,
            &source.default_master().id,
            &ufo,
            &Default::default(),
        )
        .unwrap();
        let roundtrip = fc_formats::ufo::read_ufo(&ufo).unwrap();
        assert_eq!(roundtrip.color, source.color);
        assert!(roundtrip.color_tables_owned);
        let designspace =
            fc_formats::ufo::write_designspace(&source, &temp.path().join(format!("ds-{deleted}")))
                .unwrap();
        let roundtrip = fc_formats::ufo::read_designspace(&designspace).unwrap();
        assert_eq!(roundtrip.color, source.color);
        assert!(roundtrip.color_tables_owned);
    }
}

#[test]
fn unsupported_formats_report_color_loss() {
    let dir = tempfile::tempdir().unwrap();
    let (api, doc) = setup(font());
    for format in ["Glyphs", "Sfd", "Pfa"] {
        let path = dir.path().join(format!("colors.{format}"));
        let report = api
            .call(
                "session.export",
                json!({"doc":doc,"path":path,"options":{"format":format}}),
            )
            .unwrap();
        assert!(report["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "color-not-exported"));
    }
}

#[test]
fn preview_ignores_nested_color_compositions() {
    let mut source = font();
    let table: &mut ColorTable = source.color.as_mut().unwrap();
    table.glyphs.insert(
        "V".into(),
        vec![fc_model::ColrLayer {
            glyph: "A".into(),
            palette_index: Some(1),
        }],
    );
    let (api, doc) = setup(source);
    let preview = api
        .call(
            "color.preview",
            json!({"doc":doc,"glyph":"A","layers":null,"palette":null,"palette_index":0}),
        )
        .unwrap();
    assert_eq!(preview["layers"].as_array().unwrap().len(), 2);
    assert_eq!(preview["layers"][1]["glyph"], "V");
    assert!(preview["layers"][1]["color"].is_null());
}
