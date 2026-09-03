use std::{fs, sync::Arc};

use fw_api::{
    Api, ApplyParams, DocParams, EditApplyParams, EditBeginParams, EditEndParams, EditTokenId,
    GlyphCell, GlyphCellParams,
};
use fw_doc::Session;
use fw_model::{Font, Glyph, Layer};
use serde_json::json;

#[test]
fn dispatches_session_open_and_reports_boundary_errors() {
    let config_dir = test_config_dir("dispatch");
    std::env::set_var("FW_CONFIG_DIR", &config_dir);
    let api = Api::new(Arc::new(Session::new()));
    let fixture = fw_fixtures::path("DejaVuSans.ttf");

    let summary = api
        .call(
            "session.open",
            json!({ "path": fixture, "face_index": null }),
        )
        .expect("open DejaVuSans through dispatch");
    assert_eq!(summary["family_name"], "DejaVu Sans");
    assert_eq!(summary["glyph_count"], 6_253);

    let faces = api
        .call("session.faces", json!({ "path": fixture }))
        .expect("inspect font faces");
    assert_eq!(faces.as_array().map(Vec::len), Some(1));

    let defaults = api.call("prefs.get", json!({})).expect("default prefs");
    assert_eq!(
        defaults,
        json!({
            "theme": "system",
            "language": "en",
            "recent_limit": 20,
            "autosave_seconds": 60
        })
    );
    let updated = api
        .call(
            "prefs.set",
            json!({
                "prefs": {
                    "theme": "dark",
                    "language": "en",
                    "recent_limit": 5,
                    "autosave_seconds": 30
                }
            }),
        )
        .expect("set prefs");
    assert_eq!(api.call("prefs.get", json!({})).unwrap(), updated);

    api.call(
        "session.open",
        json!({ "path": fixture, "face_index": null }),
    )
    .expect("reopen fixture");
    let recent = api.call("session.recent", json!({})).expect("recent paths");
    assert_eq!(recent, json!([fixture]));
    assert_eq!(
        api.call("session.list", json!({}))
            .expect("list documents")
            .as_array()
            .map(Vec::len),
        Some(2)
    );

    let unknown = api.call("nope", json!({})).unwrap_err();
    assert_eq!(unknown.code, "not_found");

    let invalid = api
        .call("session.open", json!({ "face_index": null }))
        .unwrap_err();
    assert_eq!(invalid.code, "invalid_params");
    assert!(invalid.message.contains("path"));

    fs::remove_dir_all(config_dir).ok();
}

fn test_config_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fontworkshop-fw-api-{label}-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&dir).ok();
    dir
}

#[test]
fn dispatches_queries_commands_edits_undo_and_events() {
    let mut font = Font::new_empty("Test Family", 1_000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![0x41];
    let mut layer = Layer::new_master(master.clone());
    layer.width = 500.0;
    let layer_id = layer.id.clone();
    glyph.layers.push(layer);
    font.insert_glyph(glyph).expect("insert test glyph");

    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    let mut events = api.subscribe();

    let sidebar = api
        .call("doc.sidebar", json!({ "doc": doc }))
        .expect("initial sidebar");
    assert_eq!(sidebar["unencoded"], 0);
    assert_eq!(sidebar["total"], 1);
    assert_eq!(
        api.call("doc.font_info", json!({ "doc": doc }))
            .expect("font info")["info"]["family_name"],
        "Test Family"
    );

    let change = api
        .call(
            "doc.apply",
            serde_json::to_value(ApplyParams {
                doc,
                command: fw_doc::Command::SetUnicodes {
                    glyph: "A".into(),
                    unicodes: Vec::new(),
                },
                label: "Clear Unicode".to_owned(),
            })
            .unwrap(),
        )
        .expect("apply command");
    assert_eq!(change["glyphs"], json!([0]));
    assert_eq!(events.try_recv().expect("document event").doc, doc);
    assert_eq!(
        api.call("doc.sidebar", json!({ "doc": doc }))
            .expect("invalidated sidebar")["unencoded"],
        1
    );

    assert!(api
        .call("doc.undo", serde_json::to_value(DocParams { doc }).unwrap())
        .expect("undo")
        .is_object());
    assert!(api
        .call("doc.redo", serde_json::to_value(DocParams { doc }).unwrap())
        .expect("redo")
        .is_object());

    let token: EditTokenId = serde_json::from_value(
        api.call(
            "doc.edit_begin",
            serde_json::to_value(EditBeginParams {
                doc,
                label: "Set width".to_owned(),
            })
            .unwrap(),
        )
        .expect("begin edit"),
    )
    .unwrap();
    api.call(
        "doc.edit_apply",
        serde_json::to_value(EditApplyParams {
            doc,
            token,
            command: fw_doc::Command::SetWidth {
                glyph: "A".into(),
                layer: layer_id,
                width: 640.0,
            },
        })
        .unwrap(),
    )
    .expect("apply edit");
    assert_eq!(
        api.call(
            "doc.edit_end",
            serde_json::to_value(EditEndParams { doc, token }).unwrap()
        )
        .expect("end edit"),
        serde_json::Value::Null
    );

    let cell: GlyphCell = serde_json::from_value(
        api.call(
            "glyphs.cell",
            serde_json::to_value(GlyphCellParams {
                doc,
                gid: fw_model::GlyphId(0),
                master,
            })
            .unwrap(),
        )
        .expect("glyph cell"),
    )
    .unwrap();
    assert_eq!(cell.width, 640.0);
    assert!(cell.path.is_some());

    let detail = api
        .call("glyphs.detail", json!({ "doc": doc, "gid": 0 }))
        .expect("glyph detail");
    assert_eq!(detail["glyph"]["name"], "A");
    assert_eq!(detail["paths"].as_array().map(Vec::len), Some(1));
}
