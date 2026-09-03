use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use fw_api::{
    Api, ApplyParams, DocParams, EditApplyParams, EditBeginParams, EditEndParams, EditTokenId,
    GlyphCell, GlyphCellParams,
};
use fw_doc::Session;
use fw_model::{Font, Glyph, Layer};
use serde_json::json;
use sha2::{Digest, Sha256};

static CONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn dispatches_session_open_and_reports_boundary_errors() {
    let _env_guard = config_env_guard();
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
        api.call("session.list", serde_json::Value::Null)
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

#[test]
fn save_without_path_never_overwrites_an_imported_font() {
    let _env_guard = config_env_guard();
    let config_dir = test_config_dir("safe-save");
    std::env::set_var("FW_CONFIG_DIR", &config_dir);
    fs::create_dir_all(&config_dir).expect("create test directory");
    let imported_path = config_dir.join("imported.ttf");
    fs::copy(fw_fixtures::path("DejaVuSans.ttf"), &imported_path)
        .expect("copy imported font fixture");
    let before = sha256(&imported_path);
    let api = Api::new(Arc::new(Session::new()));
    let opened = api
        .call(
            "session.open",
            json!({ "path": imported_path, "face_index": null }),
        )
        .expect("open temporary font copy");

    let error = api
        .call(
            "session.save",
            json!({ "doc": opened["doc"], "path": null }),
        )
        .expect_err("imported font needs an explicit FWF path");

    assert_eq!(error.code, "invalid_params");
    assert_eq!(error.message, "imported fonts need an explicit .fwf path");
    assert_eq!(sha256(&imported_path), before);

    let binary_target = config_dir.join("existing-target.ttf");
    fs::write(&binary_target, b"known existing bytes").expect("write existing binary target");
    let binary_target_before = sha256(&binary_target);
    let error = api
        .call(
            "session.save",
            json!({ "doc": opened["doc"], "path": binary_target }),
        )
        .expect_err("explicit binary font target must be rejected");
    assert_eq!(error.code, "invalid_params");
    assert_eq!(
        error.message,
        "save target must end with .fwf or .fwf.gz; use export for binary font formats"
    );
    assert_eq!(sha256(&binary_target), binary_target_before);

    let compressed_target = config_dir.join("saved.fwf.gz");
    let saved = api
        .call(
            "session.save",
            json!({ "doc": opened["doc"], "path": compressed_target }),
        )
        .expect("explicit .fwf.gz target succeeds");
    assert_eq!(saved["path"], compressed_target.to_string_lossy().as_ref());
    assert!(compressed_target.is_file());
    fs::remove_dir_all(config_dir).ok();
}

#[test]
fn damaged_preferences_fall_back_and_do_not_block_opening() {
    let _env_guard = config_env_guard();
    let config_dir = test_config_dir("damaged-prefs");
    std::env::set_var("FW_CONFIG_DIR", &config_dir);
    fs::create_dir_all(&config_dir).expect("create config directory");
    fs::write(config_dir.join("prefs.json"), br#"{"theme":"dark""#)
        .expect("write truncated preferences");
    let api = Api::new(Arc::new(Session::new()));

    assert_eq!(
        api.call("prefs.get", serde_json::Value::Null)
            .expect("damaged preferences use defaults"),
        json!({
            "theme": "system",
            "language": "en",
            "recent_limit": 20,
            "autosave_seconds": 60
        })
    );
    let opened = api
        .call(
            "session.open",
            json!({
                "path": fw_fixtures::path("DejaVuSans.ttf"),
                "face_index": null
            }),
        )
        .expect("damaged preferences do not block opening");
    assert_eq!(opened["family_name"], "DejaVu Sans");
    fs::remove_dir_all(config_dir).ok();
}

#[test]
fn recent_limit_only_limits_returned_history() {
    let _env_guard = config_env_guard();
    let config_dir = test_config_dir("recent-limit");
    std::env::set_var("FW_CONFIG_DIR", &config_dir);
    let api = Api::new(Arc::new(Session::new()));
    api.call(
        "prefs.set",
        json!({
            "prefs": {
                "theme": "system",
                "language": "en",
                "recent_limit": 2,
                "autosave_seconds": 60
            }
        }),
    )
    .expect("set short recent display limit");
    for name in ["one.ttf", "two.ttf", "three.ttf"] {
        let path = config_dir.join(name);
        fs::create_dir_all(&config_dir).expect("create config directory");
        fs::copy(fw_fixtures::path("DejaVuSans.ttf"), &path).expect("copy font fixture");
        api.call("session.open", json!({ "path": path, "face_index": null }))
            .expect("open recent font");
    }
    assert_eq!(
        api.call("session.recent", serde_json::Value::Null)
            .expect("limited recent list")
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    api.call(
        "prefs.set",
        json!({
            "prefs": {
                "theme": "system",
                "language": "en",
                "recent_limit": 20,
                "autosave_seconds": 60
            }
        }),
    )
    .expect("raise recent display limit");
    assert_eq!(
        api.call("session.recent", serde_json::Value::Null)
            .expect("full recent list")
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    fs::remove_dir_all(config_dir).ok();
}

#[test]
fn collection_warning_uses_file_contents_not_extension() {
    let _env_guard = config_env_guard();
    let config_dir = test_config_dir("collection-sniff");
    std::env::set_var("FW_CONFIG_DIR", &config_dir);
    fs::create_dir_all(&config_dir).expect("create test directory");
    let path = config_dir.join("collection-disguised-as-font.ttf");
    fs::write(&path, two_face_ttc(&fw_fixtures::path("DejaVuSans.ttf")))
        .expect("write temporary collection");
    let api = Api::new(Arc::new(Session::new()));

    let opened = api
        .call("session.open", json!({ "path": path, "face_index": null }))
        .expect("open collection with ttf extension");

    assert_eq!(
        opened["warnings"],
        json!(["collection has 2 faces; opened face 0"])
    );
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

fn config_env_guard() -> MutexGuard<'static, ()> {
    CONFIG_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

fn sha256(path: &Path) -> Vec<u8> {
    Sha256::digest(fs::read(path).expect("read file for SHA-256")).to_vec()
}

fn two_face_ttc(sfnt_path: &Path) -> Vec<u8> {
    let mut sfnt = fs::read(sfnt_path).expect("read sfnt fixture");
    let font_offset = 20_u32;
    let table_count = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
    for index in 0..table_count {
        let offset_position = 12 + index * 16 + 8;
        let old_offset = u32::from_be_bytes(
            sfnt[offset_position..offset_position + 4]
                .try_into()
                .expect("table offset bytes"),
        );
        sfnt[offset_position..offset_position + 4]
            .copy_from_slice(&(old_offset + font_offset).to_be_bytes());
    }
    let mut ttc = Vec::with_capacity(font_offset as usize + sfnt.len());
    ttc.extend_from_slice(b"ttcf");
    ttc.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    ttc.extend_from_slice(&2_u32.to_be_bytes());
    ttc.extend_from_slice(&font_offset.to_be_bytes());
    ttc.extend_from_slice(&font_offset.to_be_bytes());
    ttc.extend_from_slice(&sfnt);
    ttc
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
