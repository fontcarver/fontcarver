use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use fc_api::{
    Api, Event, ExportReport, RecoveryEntry, COMPILE_STAGE, FAILED_STAGE, PRESERVE_STAGE,
    WRITE_STAGE,
};
use fc_doc::{
    autosave::{spawn, AutosaveConfig},
    Session,
};
use fc_model::{Contour, Font, Glyph, Layer};
use kurbo::BezPath;
use serde_json::json;
use sha2::{Digest, Sha256};

fn export_font() -> Font {
    let mut font = Font::new_empty("Export Test", 1_000);
    // A fixed creation stamp keeps two exports of the same font byte-identical.
    font.info.open_type_head_created = Some("2020/01/02 03:04:05".to_owned());
    let master = font.default_master().id.clone();
    for (name, unicode, width) in [
        (".notdef", None, 600.0),
        ("space", Some(0x20), 250.0),
        ("A", Some(0x41), 600.0),
        ("B", Some(0x42), 620.0),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        let mut path = BezPath::new();
        path.move_to((50.0, 0.0));
        path.line_to((50.0, 700.0));
        path.line_to((width - 50.0, 700.0));
        path.line_to((width - 50.0, 0.0));
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert export glyph");
    }
    font
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn new_formats_export_reopen_and_shape_through_shared_api() {
    let directory = tempfile::tempdir().unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(export_font(), None);
    let api = Api::new(session);
    api.configure_config_dir(directory.path().join("config"))
        .unwrap();
    for (format, extension) in [
        ("Otf", "otf"),
        ("Glyphs", "glyphs"),
        ("Sfd", "sfd"),
        ("Pfa", "pfa"),
        ("Pfb", "pfb"),
    ] {
        let path = directory.path().join(format!("export.{extension}"));
        let report = api
            .call(
                "session.export",
                json!({"doc":doc,"path":path,
            "options":{"format":format}}),
            )
            .unwrap_or_else(|error| panic!("{format}: {error}"));
        assert!(report["bytes_written"].as_u64().unwrap() > 0);
        let reopened = api.call("session.open", json!({"path":path})).unwrap();
        assert_eq!(reopened["glyph_count"], 4, "{format}: {reopened}");
        let run = api
            .call("text.shape", json!({"doc":reopened["doc"],"text":"AB"}))
            .unwrap();
        assert_eq!(run["names"], json!(["A", "B"]), "{format}: {run}");
        api.call("session.close", json!({"doc":reopened["doc"]}))
            .unwrap();
    }
}

#[test]
fn feature_drafts_save_natively_but_block_all_external_exports_without_output_changes() {
    let directory = tempfile::tempdir().unwrap();
    let mut font = export_font();
    let mut source = font.features.clone();
    source.prefixes.push(fc_model::FeatureBlock {
        code: "invalid source {".into(),
        ..fc_model::FeatureBlock::default()
    });
    font.feature_draft = Some(fc_model::FeatureDraft {
        source,
        settings: font.feature_settings.clone(),
        revision: 1,
    });
    let session = Arc::new(Session::new());
    let doc = session.open_font(font.clone(), None);
    let api = Api::new(session);
    for format in [
        "Ttf",
        "Otf",
        "Woff",
        "Woff2",
        "Ufo",
        "Designspace",
        "Glyphs",
        "Sfd",
        "Pfa",
        "Pfb",
    ] {
        let path = directory.path().join(format);
        std::fs::write(&path, b"previous output").unwrap();
        let error = api
            .call(
                "session.export",
                json!({"doc":doc,"path":path,
            "options":{"format":format}}),
            )
            .unwrap_err();
        assert_eq!(error.code, "unapplied_feature_draft", "{format}: {error}");
        assert_eq!(std::fs::read(&path).unwrap(), b"previous output");
    }
    let path = directory.path().join("draft.fcf");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Fcf"}}),
    )
    .unwrap();
    let reopened = fc_doc::fcf::read(&path).unwrap();
    assert_eq!(reopened.feature_draft, font.feature_draft);
    assert_eq!(reopened.features, font.features);
}

#[test]
fn type1_reports_requested_but_unsupported_export_options() {
    let directory = tempfile::tempdir().unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(export_font(), None);
    let api = Api::new(session);
    for format in ["Pfa", "Pfb"] {
        let result = api
            .call(
                "session.export",
                json!({"doc": doc,
                "path": directory.path().join(format), "options": {
                    "format": format, "remove_overlap": true, "autohint": true,
                    "production_names": true
                }}),
            )
            .unwrap();
        let codes: Vec<_> = result["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|warning| warning["code"].as_str().unwrap())
            .collect();
        for expected in [
            "remove-overlap-unavailable",
            "autohint-unavailable",
            "production-names-unavailable",
        ] {
            assert!(codes.contains(&expected), "{format}: {codes:?}");
        }
    }
}

#[test]
fn type1_hint_import_diagnostics_survive_native_save_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let session = Arc::new(Session::new());
    let mut font = export_font();
    font.user_data = json!({"org.fontcarver.type1": {
        "unretained_glyph_hints": ["A"],
        "unretained_private_hints": true,
        "original_private": {"blue_values": [-10.0, 0.0]},
    }});
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    api.configure_config_dir(directory.path().join("config"))
        .unwrap();
    let path = directory.path().join("imported.fcf");
    let saved = api
        .call("session.save", json!({"doc": doc, "path": path}))
        .unwrap();
    let reopened = api.call("session.open", json!({"path": path})).unwrap();
    for result in [saved, reopened] {
        let warnings = result["warnings"].as_array().unwrap();
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0]["code"], "type1-import-hints-partial");
        assert_eq!(warnings[1]["code"], "type1-import-private-transform");
    }
    let font = fc_doc::fcf::read(&path).unwrap();
    assert_eq!(
        font.user_data["org.fontcarver.type1"]["original_private"]["blue_values"],
        json!([-10.0, 0.0])
    );
}

#[test]
fn imported_source_export_requires_conversion_before_touching_destination() {
    let directory = tempfile::tempdir().unwrap();
    let mut font = export_font();
    font.features.source_of_truth = fc_model::FeaturesSource::ImportedTables;
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    for format in ["Ufo", "Designspace", "Glyphs", "Sfd"] {
        let path = directory.path().join(format);
        let error = api
            .call(
                "session.export",
                json!({"doc":doc,"path":path,
            "options":{"format":format}}),
            )
            .unwrap_err();
        assert_eq!(error.code, "layout_conversion_required");
        assert!(!path.exists());
    }
}

#[test]
fn source_export_revalidates_active_features_against_current_glyphs() {
    let directory = tempfile::tempdir().unwrap();
    let mut font = export_font();
    font.features.features.push(fc_model::FeatureBlock {
        tag: "liga".into(),
        code: "sub A A by Missing;".into(),
        ..fc_model::FeatureBlock::default()
    });
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    for format in ["Ufo", "Designspace", "Glyphs", "Sfd"] {
        let path = directory.path().join(format);
        let error = api
            .call(
                "session.export",
                json!({"doc":doc,"path":path,
            "options":{"format":format}}),
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_feature_source");
        assert!(!path.exists());
    }
}

#[test]
fn source_export_rejects_exponential_component_expansion_before_writing() {
    let directory = tempfile::tempdir().unwrap();
    let mut font = export_font();
    let master = font.default_master().id.clone();
    let contours = font.glyphs["A"].layers[0].contours.clone();
    for index in 0..24 {
        let mut glyph = Glyph::new(format!("branch{index}"));
        let mut layer = Layer::new_master(master.clone());
        layer.contours = contours.clone();
        if index > 0 {
            layer.components = vec![
                fc_model::Component {
                    base: format!("branch{}", index - 1).into(),
                    ..fc_model::Component::default()
                };
                2
            ];
        }
        glyph.layers.push(layer);
        font.glyphs.insert(glyph.name.clone(), glyph);
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    for format in ["Glyphs", "Sfd"] {
        let path = directory.path().join(format);
        std::fs::write(&path, b"previous output").unwrap();
        let started = Instant::now();
        let error = api
            .call(
                "session.export",
                json!({"doc":doc,"path":path,"options":{"format":format}}),
            )
            .unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(error.code, "invalid_font_geometry");
        assert!(error.message.contains("component-expansion-limit"));
        assert_eq!(std::fs::read(path).unwrap(), b"previous output");
    }
}

#[test]
fn static_sfd_validates_only_the_selected_master() {
    let directory = tempfile::tempdir().unwrap();
    let mut font = export_font();
    let regular = font.default_master().id.clone();
    let mut bold = font.default_master().clone();
    bold.id = fc_model::MasterId::new();
    bold.name = "Unfinished Bold".into();
    // No glyph has a Bold layer yet. Regular is complete and independently
    // exportable; Glyphs still requires validation of every master.
    font.masters.push(bold);
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    let path = directory.path().join("Regular.sfd");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Sfd","master":regular}}),
    )
    .unwrap();
    let reopened = fc_formats::sfd::read_sfd(&path).unwrap();
    assert_eq!(reopened.masters.len(), 1);
    assert_eq!(reopened.glyphs["A"].layers[0].width, 600.0);
    let all_masters_path = directory.path().join("All.glyphs");
    let error = api
        .call(
            "session.export",
            json!({"doc":doc,"path":all_masters_path,"options":{"format":"Glyphs"}}),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_feature_source");
    assert!(!all_masters_path.exists());
}

#[test]
fn exported_ttf_matches_the_compiler_output_byte_for_byte() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("ExportTest.ttf");
    let font = export_font();
    let expected = fc_compile::export(&font, &|_| 0, &fc_compile::ExportOptions::default())
        .expect("reference export");
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);

    let report: ExportReport = serde_json::from_value(
        api.call(
            "session.export",
            json!({ "doc": doc, "path": path, "options": { "format": "Ttf" } }),
        )
        .expect("export ttf"),
    )
    .expect("decode export report");

    let written = std::fs::read(&path).expect("read exported font");
    assert_eq!(digest(&written), digest(&expected.bytes));
    assert_eq!(report.bytes_written, written.len() as u64);
    assert_eq!(report.touched_glyphs, 0);
    assert!(report
        .tables
        .iter()
        .any(|(tag, length)| tag == "glyf" && *length > 0));
}

#[test]
fn export_emits_compile_and_write_progress() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Progress.ttf");
    let session = Arc::new(Session::new());
    let doc = session.open_font(export_font(), None);
    let api = Api::new(session);
    let mut events = api.subscribe();

    api.call(
        "session.export",
        json!({ "doc": doc, "path": path, "options": { "format": "Ttf" } }),
    )
    .expect("export ttf");

    let mut stages = Vec::new();
    while let Ok(event) = events.try_recv() {
        if let Event::Progress(progress) = event {
            assert_eq!(progress.doc, doc);
            stages.push((progress.stage, progress.percent));
        }
    }
    assert_eq!(
        stages,
        vec![
            (COMPILE_STAGE.to_owned(), 0),
            (PRESERVE_STAGE.to_owned(), 60),
            (WRITE_STAGE.to_owned(), 90),
            (WRITE_STAGE.to_owned(), 100),
        ]
    );
    // The exported names are part of the wire contract with the frontend.
    assert_eq!(
        [COMPILE_STAGE, PRESERVE_STAGE, WRITE_STAGE, FAILED_STAGE],
        ["compile", "preserve", "write", "failed"]
    );
}

#[test]
fn a_failing_export_always_ends_with_a_terminal_progress_event() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let session = Arc::new(Session::new());
    let doc = session.open_font(export_font(), None);
    let api = Api::new(session);
    let mut events = api.subscribe();

    // A master that does not exist fails before any stage starts.
    api.call(
        "session.export",
        json!({
            "doc": doc,
            "path": directory.path().join("Unknown.ttf"),
            "options": { "format": "Ttf", "master": "nope" }
        }),
    )
    .expect_err("unknown master");
    let unknown = progress_stages(&mut events);
    assert_eq!(
        unknown
            .last()
            .map(|(stage, percent, _)| (stage.as_str(), *percent)),
        Some((FAILED_STAGE, 100)),
        "{unknown:?}"
    );
    assert!(
        unknown
            .last()
            .and_then(|(_, _, message)| message.clone())
            .is_some_and(|message| message.contains("nope")),
        "{unknown:?}"
    );

    // A path that cannot be created fails after the compile stages ran.
    let blocker = directory.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").expect("write the blocking file");
    api.call(
        "session.export",
        json!({
            "doc": doc,
            "path": blocker.join("Blocked.ttf"),
            "options": { "format": "Ttf" }
        }),
    )
    .expect_err("unwritable path");
    let blocked = progress_stages(&mut events);
    assert_eq!(
        blocked
            .iter()
            .map(|(stage, percent, _)| (stage.as_str(), *percent))
            .collect::<Vec<_>>(),
        vec![
            (COMPILE_STAGE, 0),
            (PRESERVE_STAGE, 60),
            (WRITE_STAGE, 90),
            (FAILED_STAGE, 100)
        ],
        "{blocked:?}"
    );
}

/// Drains the buffered progress events of one operation.
fn progress_stages(
    events: &mut tokio::sync::broadcast::Receiver<Event>,
) -> Vec<(String, u8, Option<String>)> {
    let mut stages = Vec::new();
    while let Ok(event) = events.try_recv() {
        if let Event::Progress(progress) = event {
            stages.push((progress.stage, progress.percent, progress.message));
        }
    }
    stages
}

#[test]
fn edited_glyphs_are_reported_as_touched_since_import() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Touched.ttf");
    let session = Arc::new(Session::new());
    let doc = session.open_font(export_font(), None);
    let api = Api::new(Arc::clone(&session));
    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": { "SetWidth": { "glyph": "A", "layer": layer_of(&session, doc, "A"), "width": 640.0 } },
            "label": "Set width"
        }),
    )
    .expect("edit a glyph");

    let report: ExportReport = serde_json::from_value(
        api.call(
            "session.export",
            json!({ "doc": doc, "path": path, "options": { "format": "Ttf" } }),
        )
        .expect("export ttf"),
    )
    .expect("decode export report");

    assert_eq!(report.touched_glyphs, 1);
}

fn layer_of(session: &Session, doc: fc_doc::DocId, glyph: &str) -> String {
    session
        .with_doc(doc, |document| {
            let master = document.font.default_master().id.clone();
            document
                .font
                .glyph(glyph)
                .and_then(|glyph| glyph.master_layer(&master))
                .map(|layer| layer.id.to_string())
                .expect("master layer")
        })
        .expect("open document")
}

#[test]
fn exporting_an_untouched_import_preserves_its_raw_tables() {
    use read_fonts::{FontRef, TableProvider as _};

    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Preserved.ttf");
    let source = fc_fixtures::path("DejaVuSansMin.woff2");
    let api = Api::new(Arc::new(Session::new()));
    api.configure_config_dir(directory.path().join("config"))
        .expect("configure the config directory");
    let summary = api
        .call("session.open", json!({ "path": source }))
        .expect("open the imported font");
    let doc = summary["doc"].clone();

    let report: ExportReport = serde_json::from_value(
        api.call(
            "session.export",
            json!({ "doc": doc, "path": path, "options": { "format": "Ttf" } }),
        )
        .expect("export ttf"),
    )
    .expect("decode export report");

    assert_eq!(report.touched_glyphs, 0);
    assert!(
        !report
            .warnings
            .iter()
            .any(|warning| warning.code.starts_with("hinting-dropped")),
        "an untouched import must keep its hinting: {:?}",
        report.warnings
    );

    let imported = fc_sfnt::read_path(&source, &fc_sfnt::ReadOptions::default())
        .expect("read the fixture")
        .fonts
        .remove(0);
    let original = imported
        .extra
        .source
        .as_ref()
        .and_then(|source| source.original_bytes.clone())
        .expect("imported source bytes");
    let written = std::fs::read(&path).expect("read the exported font");
    let input = FontRef::new(&original).expect("parse the imported sfnt");
    let output = FontRef::new(&written).expect("parse the exported font");
    for tag in [*b"glyf", *b"loca", *b"maxp", *b"fpgm", *b"prep", *b"cvt "] {
        let tag = read_fonts::types::Tag::new(&tag);
        assert_eq!(
            output.table_data(tag).map(|data| data.as_bytes().to_vec()),
            input.table_data(tag).map(|data| data.as_bytes().to_vec()),
            "{tag} was not preserved verbatim"
        );
    }
    for (tag, expected) in &imported.extra.raw_tables {
        let actual = output
            .table_data(read_fonts::types::Tag::new(&tag.0))
            .unwrap_or_else(|| panic!("missing preserved {tag}"));
        assert_eq!(actual.as_bytes(), expected, "raw table {tag} changed");
    }
    assert!(output.glyf().is_ok());
}

#[test]
fn exports_woff2_and_a_ufo_package() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let session = Arc::new(Session::new());
    let doc = session.open_font(export_font(), None);
    let api = Api::new(session);

    let woff2 = directory.path().join("ExportTest.woff2");
    api.call(
        "session.export",
        json!({ "doc": doc, "path": woff2, "options": { "format": "Woff2" } }),
    )
    .expect("export woff2");
    assert_eq!(&std::fs::read(&woff2).expect("read woff2")[..4], b"wOF2");

    let ufo = directory.path().join("ExportTest.ufo");
    let report: ExportReport = serde_json::from_value(
        api.call(
            "session.export",
            json!({ "doc": doc, "path": ufo, "options": { "format": "Ufo" } }),
        )
        .expect("export ufo"),
    )
    .expect("decode export report");
    assert!(ufo.join("metainfo.plist").is_file());
    assert!(report.bytes_written > 0);

    let fcf = directory.path().join("ExportTest.fcf");
    api.call(
        "session.export",
        json!({ "doc": doc, "path": fcf, "options": { "format": "Fcf" } }),
    )
    .expect("export fcf");
    assert!(fcf.is_file());
}

#[test]
fn recovery_lists_opens_and_discards_an_autosave() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let document_path = directory.path().join("Recovered.fcf");
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .expect("configure the config directory");
    let doc = session.open_font(export_font(), Some(document_path.clone()));
    session
        .with_doc_mut(doc, |document| document.dirty = true)
        .expect("mark the document dirty");
    let autosave_dir = directory.path().join("config/autosave");
    let handle = spawn(
        Arc::clone(&session),
        AutosaveConfig {
            interval: Duration::from_millis(20),
            dir: autosave_dir.clone(),
        },
    );

    let entries = wait_for_recoverable(&api);
    handle.stop();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].doc_path.as_deref(), document_path.to_str());
    assert!(entries[0].saved_at_ms > 0);

    let summary = api
        .call(
            "session.recover",
            json!({ "autosave_path": entries[0].autosave_path }),
        )
        .expect("recover the snapshot");
    assert_eq!(summary["family_name"], "Export Test");
    assert_eq!(summary["path"], json!(document_path.to_str()));
    assert_eq!(summary["dirty"], json!(true));
    assert_eq!(
        summary["recovered_from"],
        json!(document_path.to_str()),
        "a recovered document must be marked as such"
    );
    let opened = api
        .call("doc.summary", json!({ "doc": doc }))
        .expect("summary of the normally opened document");
    assert!(
        opened.get("recovered_from").is_none(),
        "a normally opened document must not be marked as recovered: {opened}"
    );

    api.call(
        "session.discard_recovery",
        json!({ "autosave_path": entries[0].autosave_path }),
    )
    .expect("discard the snapshot");
    assert!(!std::path::Path::new(&entries[0].autosave_path).exists());
    assert_eq!(
        api.call("session.recover_list", json!({}))
            .expect("list recoverable documents"),
        json!([])
    );

    // Saving the recovery over its original file ends the recovered state, so a
    // UI banner driven by the marker does not stay up for the rest of the session.
    let saved = api
        .call("session.save", json!({ "doc": summary["doc"] }))
        .expect("save the recovered document");
    assert!(
        saved.get("recovered_from").is_none(),
        "a saved document is no longer a recovery: {saved}"
    );
    assert_eq!(saved["dirty"], json!(false));
}

fn wait_for_recoverable(api: &Api) -> Vec<RecoveryEntry> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let entries: Vec<RecoveryEntry> = serde_json::from_value(
            api.call("session.recover_list", json!({}))
                .expect("list recoverable documents"),
        )
        .expect("decode recovery entries");
        if !entries.is_empty() || Instant::now() > deadline {
            return entries;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn autosave_starts_from_preferences() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .expect("configure the config directory");
    api.call(
        "prefs.set",
        json!({ "prefs": {
            "theme": "system",
            "language": "en",
            "recent_limit": 20,
            "autosave_seconds": 1
        }}),
    )
    .expect("store preferences");
    let doc = session.open_font(export_font(), Some(directory.path().join("Auto.fcf")));
    session
        .with_doc_mut(doc, |document| document.dirty = true)
        .expect("mark the document dirty");

    api.start_autosave().expect("start autosave");

    assert_eq!(wait_for_recoverable(&api).len(), 1);
}
