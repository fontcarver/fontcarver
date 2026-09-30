use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use fc_api::{
    Api, ApplyParams, DocParams, EditApplyParams, EditBeginParams, EditEndParams, EditTokenId,
    GlyphCell, GlyphCellParams,
};
use fc_doc::Session;
use fc_model::{Font, Glyph, Instance, Layer};
use serde_json::json;
use sha2::{Digest, Sha256};

static CONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

/// Waits for the next document event on the API's forwarded event stream.
fn next_document_event(
    events: &mut tokio::sync::broadcast::Receiver<fc_api::Event>,
) -> fc_api::DocumentEvent {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match events.try_recv() {
            Ok(fc_api::Event::Document(event)) => return event,
            Ok(fc_api::Event::Progress(_)) => continue,
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(error) => panic!("no document event arrived: {error}"),
        }
    }
}

#[test]
fn font_info_includes_named_instances() {
    let mut font = Font::new_empty("Test Family", 1_000);
    let mut instance = Instance {
        name: "Display Semibold".to_owned(),
        style_name: "Semibold".to_owned(),
        family_name: Some("Test Display".to_owned()),
        active: true,
        is_variable_named_instance: true,
        ..Instance::default()
    };
    instance
        .location
        .insert("wght".parse().expect("valid axis tag"), 650.0);
    font.instances.push(instance);

    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);

    let info = api
        .call("doc.font_info", json!({ "doc": doc }))
        .expect("font info");
    assert_eq!(
        info["instances"],
        json!([{
            "name": "Display Semibold",
            "location": { "wght": 650.0 },
            "style_name": "Semibold",
            "family_name": "Test Display",
            "active": true,
            "is_variable_named_instance": true,
            "user_data": null
        }])
    );
}

#[test]
fn explicit_config_directory_is_used_without_an_environment_override() {
    let config_dir = test_config_dir("explicit-config");
    let api = Api::with_config_dir(&config_dir);

    api.call(
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
    .expect("write preferences to explicit config directory");

    assert!(config_dir.join("prefs.json").is_file());
    fs::remove_dir_all(config_dir).ok();
}

#[test]
fn explicit_null_clears_glyph_color_label_through_json_dispatch() {
    let mut font = Font::new_empty("Test Family", 1_000);
    let mut glyph = Glyph::new("A");
    glyph.color_label = Some(3);
    font.insert_glyph(glyph).expect("insert test glyph");
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(Arc::clone(&session));

    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": {
                "SetGlyphMeta": {
                    "glyph": "A",
                    "color_label": null
                }
            },
            "label": "Clear color label"
        }),
    )
    .expect("clear color label through raw JSON");
    assert_eq!(
        session
            .with_doc(doc, |document| document
                .font
                .glyph("A")
                .unwrap()
                .color_label)
            .unwrap(),
        None
    );

    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "SetGlyphMeta": { "glyph": "A", "color_label": 7 } },
            "label": "Set color label"
        }),
    )
    .expect("set color label through raw JSON");
    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "SetGlyphMeta": { "glyph": "A", "export": false } },
            "label": "Disable export"
        }),
    )
    .expect("leave omitted color label unchanged");
    assert_eq!(
        session
            .with_doc(doc, |document| document
                .font
                .glyph("A")
                .unwrap()
                .color_label)
            .unwrap(),
        Some(7)
    );
}

#[test]
fn dispatches_session_open_and_reports_boundary_errors() {
    let _env_guard = config_env_guard();
    let config_dir = test_config_dir("dispatch");
    std::env::set_var("FC_CONFIG_DIR", &config_dir);
    let api = Api::new(Arc::new(Session::new()));
    let fixture = fc_fixtures::path("DejaVuSans.ttf");

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
            "autosave_seconds": 60,
            "grid_size": 1.0,
            "grid_subdivision": 1,
            "snap_distance": 6.0,
            "hit_distance": 5.0,
            "handle_size": 3.5,
            "node_size": 5.0,
            "snap": {
                "nodes": true,
                "handles": true,
                "anchors": true,
                "zones": true,
                "background": true,
                "guides": true,
                "metrics": true,
                "grid": true,
                "components": true
            },
            "nudge": [1.0, 10.0, 100.0]
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
    std::env::set_var("FC_CONFIG_DIR", &config_dir);
    fs::create_dir_all(&config_dir).expect("create test directory");
    let imported_path = config_dir.join("imported.ttf");
    fs::copy(fc_fixtures::path("DejaVuSans.ttf"), &imported_path)
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
        .expect_err("imported font needs an explicit FCF path");

    assert_eq!(error.code, "invalid_params");
    assert_eq!(
        error.message,
        "imported fonts need an explicit .fcf or .fcfp path"
    );
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
        "save target must end with .fcf, .fcf.gz, or .fcfp; use export for binary font formats"
    );
    assert_eq!(sha256(&binary_target), binary_target_before);

    let compressed_target = config_dir.join("saved.fcf.gz");
    let saved = api
        .call(
            "session.save",
            json!({ "doc": opened["doc"], "path": compressed_target }),
        )
        .expect("explicit .fcf.gz target succeeds");
    assert_eq!(saved["path"], compressed_target.to_string_lossy().as_ref());
    assert!(compressed_target.is_file());
    fs::remove_dir_all(config_dir).ok();
}

#[test]
fn damaged_preferences_fall_back_and_do_not_block_opening() {
    let _env_guard = config_env_guard();
    let config_dir = test_config_dir("damaged-prefs");
    std::env::set_var("FC_CONFIG_DIR", &config_dir);
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
            "autosave_seconds": 60,
            "grid_size": 1.0,
            "grid_subdivision": 1,
            "snap_distance": 6.0,
            "hit_distance": 5.0,
            "handle_size": 3.5,
            "node_size": 5.0,
            "snap": {
                "nodes": true,
                "handles": true,
                "anchors": true,
                "zones": true,
                "background": true,
                "guides": true,
                "metrics": true,
                "grid": true,
                "components": true
            },
            "nudge": [1.0, 10.0, 100.0]
        })
    );
    let opened = api
        .call(
            "session.open",
            json!({
                "path": fc_fixtures::path("DejaVuSans.ttf"),
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
    std::env::set_var("FC_CONFIG_DIR", &config_dir);
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
        fs::copy(fc_fixtures::path("DejaVuSans.ttf"), &path).expect("copy font fixture");
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
    std::env::set_var("FC_CONFIG_DIR", &config_dir);
    fs::create_dir_all(&config_dir).expect("create test directory");
    let path = config_dir.join("collection-disguised-as-font.ttf");
    fs::write(&path, two_face_ttc(&fc_fixtures::path("DejaVuSans.ttf")))
        .expect("write temporary collection");
    let api = Api::new(Arc::new(Session::new()));

    let opened = api
        .call("session.open", json!({ "path": path, "face_index": null }))
        .expect("open collection with ttf extension");

    assert_eq!(
        opened["warnings"],
        json!([{
            "code": "collection-face-opened",
            "message": "collection has 2 faces; opened face 0"
        }])
    );
    fs::remove_dir_all(config_dir).ok();
}

#[test]
fn representable_cff_hints_import_without_a_loss_warning() {
    let _env_guard = config_env_guard();
    let config_dir = test_config_dir("cff-warning");
    std::env::set_var("FC_CONFIG_DIR", &config_dir);
    let api = Api::new(Arc::new(Session::new()));

    let opened = api
        .call(
            "session.open",
            json!({
                "path": fc_fixtures::path("SourceSans3Min.otf"),
                "face_index": null
            }),
        )
        .expect("open CFF fixture");

    assert!(!opened["warnings"]
        .as_array()
        .expect("warning array")
        .iter()
        .any(|warning| warning["code"] == "cff-hints-not-imported"));
    fs::remove_dir_all(config_dir).ok();
}

#[test]
fn font_info_reports_preserved_and_consumed_table_lengths() {
    let _env_guard = config_env_guard();
    let config_dir = test_config_dir("font-info-tables");
    std::env::set_var("FC_CONFIG_DIR", &config_dir);
    let fixture = fc_fixtures::path("DejaVuSans.ttf");
    let bytes = fs::read(&fixture).expect("read font fixture");
    let api = Api::new(Arc::new(Session::new()));
    let opened = api
        .call(
            "session.open",
            json!({ "path": fixture, "face_index": null }),
        )
        .expect("open fixture");

    let font_info = api
        .call("doc.font_info", json!({ "doc": opened["doc"] }))
        .expect("font info");
    let tables = font_info["tables"]
        .as_array()
        .expect("font info table inventory");
    assert!(tables.windows(2).all(|pair| {
        pair[0]["tag"].as_str().expect("table tag") < pair[1]["tag"].as_str().expect("table tag")
    }));

    let gsub = tables
        .iter()
        .find(|table| table["tag"] == "GSUB")
        .expect("preserved GSUB table");
    assert_eq!(gsub["preserved"], true);
    assert_eq!(gsub["len"], sfnt_table_len(&bytes, b"GSUB"));

    let glyf = tables
        .iter()
        .find(|table| table["tag"] == "glyf")
        .expect("consumed glyf table");
    assert_eq!(glyf["preserved"], false);
    assert_eq!(glyf["len"], sfnt_table_len(&bytes, b"glyf"));
    assert_eq!(
        tables.iter().filter(|table| table["tag"] == "GSUB").count(),
        1
    );

    fs::remove_dir_all(config_dir).ok();
}

fn test_config_dir(label: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("fontcarver-fc-api-{label}-{}", std::process::id()));
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

fn sfnt_table_len(bytes: &[u8], wanted: &[u8; 4]) -> u32 {
    let table_count = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    (0..table_count)
        .find_map(|index| {
            let record = &bytes[12 + index * 16..12 + (index + 1) * 16];
            (&record[..4] == wanted)
                .then(|| u32::from_be_bytes(record[12..16].try_into().expect("table length bytes")))
        })
        .expect("fixture table")
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
                command: fc_doc::Command::SetUnicodes {
                    glyph: "A".into(),
                    unicodes: Vec::new(),
                },
                label: "Clear Unicode".to_owned(),
            })
            .unwrap(),
        )
        .expect("apply command");
    assert_eq!(change["glyphs"], json!([0]));
    assert_eq!(next_document_event(&mut events).doc, doc);
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
            command: fc_doc::Command::SetWidth {
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
                gid: fc_model::GlyphId(0),
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
