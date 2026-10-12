//! MATH editing through the shared API: revisions, undo, save, preview and export.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use serde_json::{json, Value};

fn open(api: &Api, path: &std::path::Path) -> Value {
    api.call("session.open", json!({ "path": path })).unwrap()["doc"].clone()
}

fn state(api: &Api, doc: &Value) -> Value {
    api.call("math.state", json!({ "doc": doc })).unwrap()
}

fn exported_axis_height(path: &std::path::Path) -> f64 {
    let font = fc_sfnt::read_path(path, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    font.math.unwrap().constants.unwrap().axis_height.value
}

#[test]
fn edits_are_revision_checked_undoable_saved_and_exported() {
    let dir = tempfile::tempdir().unwrap();
    let api = Api::new(Arc::new(Session::new()));
    let doc = open(&api, &fc_fixtures::path("DejaVuSans.ttf"));
    let initial = state(&api, &doc);
    assert_eq!(initial["status"], "Editable");
    let mut constants = initial["math"]["constants"].clone();
    let original_axis = constants["axis_height"]["value"].as_f64().unwrap();
    constants["axis_height"]["value"] = json!(777.0);
    let apply = |generation: &Value| {
        api.call(
            "math.apply",
            json!({
                "doc": doc,
                "expected_generation": generation,
                "label": "MATH constants",
                "commands": [{ "SetGlobals": {
                    "constants": constants,
                    "min_connector_overlap": 21.0,
                } }],
            }),
        )
    };
    let change = apply(&initial["generation"]).unwrap();
    assert_eq!(change["math"], true);
    // The previous revision is now stale.
    let stale = apply(&initial["generation"]).unwrap_err();
    assert!(stale.message.contains("reload"), "{stale:?}");
    let edited = state(&api, &doc);
    assert_eq!(edited["math"]["min_connector_overlap"], 21.0);

    // Invalid references are rejected without changing the revision.
    let error = api
        .call(
            "math.apply",
            json!({
                "doc": doc,
                "expected_generation": edited["generation"],
                "label": "Bad",
                "commands": [{ "SetGlyph": { "glyph": "no-such-glyph", "info": {} } }],
            }),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
    assert_eq!(state(&api, &doc)["generation"], edited["generation"]);

    let ttf = dir.path().join("edited.ttf");
    api.call(
        "session.export",
        json!({ "doc": doc, "path": ttf, "options": { "format": "Ttf" } }),
    )
    .unwrap();
    assert_eq!(exported_axis_height(&ttf), 777.0);

    let fcf = dir.path().join("edited.fcf");
    api.call("session.save", json!({ "doc": doc, "path": fcf }))
        .unwrap();
    let reopened = open(&api, &fcf);
    assert_eq!(state(&api, &reopened)["math"], edited["math"]);

    api.call("doc.undo", json!({ "doc": doc })).unwrap();
    let undone = state(&api, &doc);
    assert_eq!(
        undone["math"]["constants"]["axis_height"]["value"],
        original_axis
    );
    assert_eq!(undone["math"], initial["math"]);

    // Explicit deletion is exported as absence.
    api.call(
        "math.apply",
        json!({
            "doc": doc,
            "expected_generation": undone["generation"],
            "label": "Delete MATH",
            "commands": [{ "Replace": { "math": null } }],
        }),
    )
    .unwrap();
    assert_eq!(state(&api, &doc)["status"], "Deleted");
    let deleted = dir.path().join("deleted.ttf");
    api.call(
        "session.export",
        json!({ "doc": doc, "path": deleted, "options": { "format": "Ttf" } }),
    )
    .unwrap();
    let bytes = std::fs::read(&deleted).unwrap();
    let face = read_fonts::FontRef::new(&bytes).unwrap();
    assert!(face
        .table_data(read_fonts::types::Tag::new(b"MATH"))
        .is_none());
}

#[test]
fn preview_selects_variants_assemblies_and_unapplied_drafts() {
    let api = Api::new(Arc::new(Session::new()));
    let doc = open(&api, &fc_fixtures::path("DejaVuSans.ttf"));
    let math = state(&api, &doc)["math"].clone();
    let (glyph, construction) = math["glyphs"]
        .as_object()
        .unwrap()
        .iter()
        .find_map(|(name, info)| {
            let vertical = &info["vertical"];
            (!vertical["assembly"].is_null()).then(|| (name.clone(), vertical.clone()))
        })
        .expect("DejaVu has a vertical assembly");
    let preview = |target: f64, construction: Value| {
        api.call(
            "math.preview",
            json!({
                "doc": doc, "glyph": glyph, "vertical": true,
                "target": target, "construction": construction,
            }),
        )
        .unwrap()
    };
    // A ready-made variant that reaches the size wins over the assembly.
    let mut sized = construction.clone();
    sized["variants"] = json!([{ "glyph": glyph, "advance": 100.0 }]);
    let small = preview(1.0, sized.clone());
    assert_eq!(small["variant"], json!(glyph), "{small}");
    assert!(small["assembly"].is_null());
    assert!(small["glyphs"][0]["path"]
        .as_str()
        .unwrap()
        .starts_with('M'));
    let tall = preview(20_000.0, Value::Null);
    assert!(tall["variant"].is_null());
    let layout = &tall["assembly"];
    assert!(layout["size"].as_f64().unwrap() >= 20_000.0, "{layout}");
    assert!(layout["repeats"].as_u64().unwrap() > 0);
    // Every placed part has an outline in the response.
    let names: Vec<_> = tall["glyphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["name"].as_str().unwrap().to_owned())
        .collect();
    for part in layout["parts"].as_array().unwrap() {
        assert!(names.contains(&part["glyph"].as_str().unwrap().to_owned()));
    }
    // An unapplied draft without an assembly falls back to the largest variant.
    let mut draft = sized;
    draft["assembly"] = Value::Null;
    let fallback = preview(20_000.0, draft);
    assert!(fallback["assembly"].is_null());
    assert_eq!(fallback["fallback"], json!(glyph));
    // Previews never change the document.
    assert_eq!(state(&api, &doc)["math"], math);
    let error = api
        .call(
            "math.preview",
            json!({"doc": doc, "glyph": glyph, "vertical": true, "target": f64::MAX, "construction": null}),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
}

#[test]
fn legacy_imported_bytes_can_be_made_editable_and_undone() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).unwrap();
    let mut font = fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let expected = font.math.take().unwrap();
    font.math_table_owned = false;
    let raw = read_fonts::FontRef::new(&bytes)
        .unwrap()
        .table_data(read_fonts::types::Tag::new(b"MATH"))
        .unwrap()
        .as_bytes()
        .to_vec();
    font.extra.raw_tables.insert(fc_model::Tag(*b"MATH"), raw);
    let legacy = dir.path().join("legacy.fcf");
    fc_doc::fcf::write(&legacy, &font).unwrap();

    let api = Api::new(Arc::new(Session::new()));
    let doc = open(&api, &legacy);
    let imported = state(&api, &doc);
    assert_eq!(imported["status"], "Imported");
    assert!(imported["import_error"].is_null());
    api.call(
        "math.decode_imported",
        json!({ "doc": doc, "expected_generation": imported["generation"], "label": "解码" }),
    )
    .unwrap();
    let editable = state(&api, &doc);
    assert_eq!(editable["status"], "Editable");
    assert_eq!(editable["math"], serde_json::to_value(&expected).unwrap());
    api.call("doc.undo", json!({ "doc": doc })).unwrap();
    assert_eq!(state(&api, &doc)["status"], "Imported");

    // Imported bytes whose glyphs were renamed cannot resolve and say why.
    let renamed_glyph = expected.glyphs.keys().next().unwrap().clone();
    font.rename_glyph(&renamed_glyph, "renamed.glyph").unwrap();
    let renamed = dir.path().join("renamed.fcf");
    fc_doc::fcf::write(&renamed, &font).unwrap();
    let doc = open(&api, &renamed);
    let imported = state(&api, &doc);
    assert_eq!(imported["status"], "Imported");
    assert!(imported["import_error"]
        .as_str()
        .unwrap()
        .contains(renamed_glyph.as_ref() as &str));
    let error = api
        .call(
            "math.decode_imported",
            json!({ "doc": doc, "expected_generation": imported["generation"], "label": "x" }),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
}

#[test]
fn formats_without_math_disclose_the_loss() {
    let dir = tempfile::tempdir().unwrap();
    let mut font = fc_model::Font::new_empty("Math Loss", 1000);
    let master = font.default_master().id.clone();
    for name in [".notdef", "A"] {
        let mut glyph = fc_model::Glyph::new(name);
        let mut layer = fc_model::Layer::new_master(master.clone());
        layer.width = 500.0;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.math = Some(fc_model::MathTable::default());
    font.math_table_owned = true;
    let session = Arc::new(Session::new());
    let doc = session.open_font(font.clone(), None);
    // Bitmap formats need a strike, which source formats refuse.
    font.bitmap_strikes.push(fc_model::BitmapStrike {
        ppem: 12,
        depth: 1,
        ..Default::default()
    });
    let bitmap_doc = session.open_font(font, None);
    let api = Api::new(session);
    for (format, extension) in [
        ("Glyphs", "glyphs"),
        ("Sfd", "sfd"),
        ("Pfa", "pfa"),
        ("Bdf", "bdf"),
    ] {
        let doc = if format == "Bdf" { bitmap_doc } else { doc };
        let path = dir.path().join(format!("loss.{extension}"));
        let report = api
            .call(
                "session.export",
                json!({ "doc": doc, "path": path, "options": { "format": format } }),
            )
            .unwrap_or_else(|error| panic!("{format}: {error:?}"));
        assert!(
            report["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["code"] == "math-not-exported"),
            "{format}: {report}"
        );
    }
}
