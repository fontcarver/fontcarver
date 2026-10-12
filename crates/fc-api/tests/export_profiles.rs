//! Export profiles and variable-font configurations are project data.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Axis, Contour, Font, Glyph, Instance, Layer, LayerId, LayerKind, MasterId, Tag};
use kurbo::BezPath;
use serde_json::{json, Value};

fn source() -> Font {
    let mut font = Font::new_empty("Profile Test", 1000);
    let master = font.default_master().id.clone();
    for (name, unicode) in [(".notdef", None), ("A", Some(65u32))] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 600.0;
        let mut path = BezPath::new();
        path.move_to((10.0, 0.0));
        path.line_to((10.0, 700.0));
        path.line_to((590.0, 700.0));
        path.line_to((590.0, 0.0));
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font
}

fn variable_source() -> Font {
    let mut font = source();
    let regular = font.default_master().id.clone();
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
    for glyph in font.glyphs.values_mut() {
        let mut layer = glyph.master_layer(&regular).unwrap().clone();
        layer.id = LayerId::new();
        layer.kind = LayerKind::Master(bold.id.clone());
        for contour in &mut layer.contours {
            for point in &mut contour.points {
                point.x *= 1.2;
            }
        }
        glyph.layers.push(layer);
    }
    font.masters.push(bold);
    for (name, wght) in [("Regular", 400.0), ("Bold", 700.0)] {
        font.instances.push(Instance {
            name: name.into(),
            style_name: name.into(),
            location: [(Tag(*b"wght"), wght)].into_iter().collect(),
            active: true,
            is_variable_named_instance: true,
            ..Default::default()
        });
    }
    font
}

fn api_with(font: Font) -> (Api, fc_doc::DocId, Arc<Session>) {
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(Arc::clone(&session)), doc, session)
}

fn web_profile() -> Value {
    json!({
        "name": "Web",
        "subfolder": "web",
        "file_name": "",
        "options": {
            "format": "Woff2",
            "master": null,
            "variable": false,
            "remove_overlap": false,
            "autohint": false,
            "keep_tt_instructions": true,
            "preserve_untouched_tables": true,
            "production_names": true,
            "table_subset": null,
        }
    })
}

#[test]
fn profiles_round_trip_through_the_document_and_its_undo_history() {
    let (api, doc, session) = api_with(source());
    let empty = api
        .call("export.profiles_get", json!({"doc": doc}))
        .unwrap();
    assert_eq!(empty["profiles"].as_array().unwrap().len(), 0);
    assert_eq!(empty["variable_configs"].as_array().unwrap().len(), 0);

    let saved = api
        .call(
            "export.profiles_set",
            json!({"doc": doc, "profiles": [web_profile()], "variable_configs": []}),
        )
        .unwrap();
    assert_eq!(saved["profiles"][0]["name"], "Web");
    assert_eq!(saved["profiles"][0]["options"]["production_names"], true);

    // The collection lives in the document, so it survives a save/reopen.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("profiles.fcf");
    api.call("session.save", json!({"doc": doc, "path": path}))
        .unwrap();
    let reopened = Arc::new(Session::new());
    let reopened_doc = reopened
        .open_path(&path, None)
        .expect("reopen the saved project");
    let reopened_api = Api::new(reopened);
    let stored = reopened_api
        .call("export.profiles_get", json!({"doc": reopened_doc}))
        .unwrap();
    assert_eq!(stored["profiles"][0]["subfolder"], "web");

    // Saving a profile is an ordinary undo entry.
    let label = session
        .with_doc(doc, |document| document.undo_label().map(str::to_owned))
        .unwrap();
    assert_eq!(label.as_deref(), Some("Edit export profiles"));
    api.call("doc.undo", json!({"doc": doc})).unwrap();
    api.call("doc.undo", json!({"doc": doc})).unwrap();
    let after_undo = api
        .call("export.profiles_get", json!({"doc": doc}))
        .unwrap();
    assert_eq!(after_undo["profiles"].as_array().unwrap().len(), 0);
}

#[test]
fn invalid_profiles_are_rejected_without_changing_the_document() {
    let (api, doc, _session) = api_with(source());
    api.call(
        "export.profiles_set",
        json!({"doc": doc, "profiles": [web_profile()], "variable_configs": []}),
    )
    .unwrap();
    let mut duplicate = web_profile();
    duplicate["name"] = json!(" web ");
    for (profile, expected) in [
        (json!({"name": "  ", "options": {"format": "Ttf"}}), "name"),
        (
            json!({"name": "Escape", "subfolder": "../secrets", "options": {"format": "Ttf"}}),
            "subfolder",
        ),
        (
            json!({"name": "Absolute", "subfolder": "/etc", "options": {"format": "Ttf"}}),
            "subfolder",
        ),
        (
            json!({"name": "Pathy", "file_name": "a/b", "options": {"format": "Ttf"}}),
            "file name",
        ),
        (
            json!({"name": "Bad", "options": {"format": "Ttf", "subroutinize": true}}),
            "subroutinize",
        ),
    ] {
        let error = api
            .call(
                "export.profiles_set",
                json!({"doc": doc, "profiles": [profile], "variable_configs": []}),
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_params", "{error}");
        assert!(error.message.contains(expected), "{error}");
    }
    let error = api
        .call(
            "export.profiles_set",
            json!({"doc": doc, "profiles": [web_profile(), duplicate], "variable_configs": []}),
        )
        .unwrap_err();
    assert!(error.message.contains("named"), "{error}");
    let kept = api
        .call("export.profiles_get", json!({"doc": doc}))
        .unwrap();
    assert_eq!(kept["profiles"].as_array().unwrap().len(), 1);
}

#[test]
fn a_resolved_profile_names_its_destination_and_its_options() {
    let (api, doc, _session) = api_with(variable_source());
    api.call(
        "export.profiles_set",
        json!({
            "doc": doc,
            "profiles": [web_profile(), {
                "name": "Desktop",
                "subfolder": "",
                "file_name": "MyFont",
                "options": {"format": "Otf", "master": null, "variable": false,
                    "remove_overlap": false, "autohint": false,
                    "keep_tt_instructions": true, "preserve_untouched_tables": true,
                    "production_names": false, "table_subset": null},
            }],
            "variable_configs": [{
                "name": "Text VF",
                "file_name": "MyFont-Text",
                "masters": [],
                "axes": ["wght"],
                "instances": ["Bold"],
            }],
        }),
    )
    .unwrap();

    let static_instance = api
        .call(
            "export.resolve_profile",
            json!({"doc": doc, "profile": "web", "instance": "Bold"}),
        )
        .unwrap();
    assert_eq!(
        static_instance["relative_path"],
        "web/Profile-Test-Bold.woff2"
    );
    assert_eq!(static_instance["options"]["instance"], 1);
    assert_eq!(static_instance["options"]["variable"], false);

    // A stored file name still names the file, with the row appended so that
    // running the profile over several rows cannot write one path twice.
    let explicit_name = api
        .call(
            "export.resolve_profile",
            json!({"doc": doc, "profile": "Desktop", "instance": "Regular"}),
        )
        .unwrap();
    assert_eq!(explicit_name["relative_path"], "MyFont-Regular.otf");
    let without_row = api
        .call(
            "export.resolve_profile",
            json!({"doc": doc, "profile": "Desktop"}),
        )
        .unwrap();
    assert_eq!(without_row["relative_path"], "MyFont.otf");

    let variable = api
        .call(
            "export.resolve_profile",
            json!({"doc": doc, "profile": "Web", "variable_config": "Text VF"}),
        )
        .unwrap();
    assert_eq!(variable["relative_path"], "web/MyFont-Text.woff2");
    assert_eq!(variable["options"]["variable"], true);
    assert_eq!(variable["options"]["axis_subset"], json!(["wght"]));
    assert_eq!(variable["options"]["instance_subset"], json!(["Bold"]));
    assert!(variable["options"]["master_subset"].is_null());

    for (params, expected) in [
        (
            json!({"doc": doc, "profile": "Nope", "instance": "Bold"}),
            "Nope",
        ),
        (
            json!({"doc": doc, "profile": "Web", "instance": "Nope"}),
            "Nope",
        ),
        (
            json!({"doc": doc, "profile": "Web", "variable_config": "Nope"}),
            "Nope",
        ),
    ] {
        let error = api.call("export.resolve_profile", params).unwrap_err();
        assert_eq!(error.code, "not_found", "{error}");
        assert!(error.message.contains(expected), "{error}");
    }
    let error = api
        .call(
            "export.resolve_profile",
            json!({"doc": doc, "profile": "Web", "variable_config": "Text VF", "instance": "Bold"}),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params", "{error}");
}

#[test]
fn a_configuration_naming_a_removed_source_is_reported_not_ignored() {
    let (api, doc, session) = api_with(variable_source());
    let bold = session
        .with_doc(doc, |document| {
            document
                .font
                .masters
                .iter()
                .find(|master| master.name == "Bold")
                .map(|master| master.id.to_string())
                .unwrap()
        })
        .unwrap();
    api.call(
        "export.profiles_set",
        json!({
            "doc": doc,
            "profiles": [web_profile()],
            "variable_configs": [{
                "name": "Two masters",
                "file_name": "",
                "masters": [bold, "missing-master"],
                "axes": [],
                "instances": [],
            }],
        }),
    )
    .unwrap();
    let error = api
        .call(
            "export.resolve_profile",
            json!({"doc": doc, "profile": "Web", "variable_config": "Two masters"}),
        )
        .unwrap_err();
    assert_eq!(error.code, "not_found", "{error}");
    assert!(error.message.contains("missing-master"), "{error}");
}

#[test]
fn a_resolved_variable_configuration_exports_the_subset_it_names() {
    let (api, doc, _session) = api_with(variable_source());
    api.call(
        "export.profiles_set",
        json!({
            "doc": doc,
            "profiles": [{
                "name": "VF",
                "subfolder": "",
                "file_name": "Subset",
                "options": {"format": "Ttf", "master": null, "variable": true,
                    "remove_overlap": false, "autohint": false,
                    "keep_tt_instructions": true, "preserve_untouched_tables": true,
                    "production_names": false, "table_subset": null},
            }],
            "variable_configs": [{
                "name": "Bold only instance list",
                "file_name": "",
                "masters": [],
                "axes": [],
                "instances": ["Bold"],
            }],
        }),
    )
    .unwrap();
    let resolved = api
        .call(
            "export.resolve_profile",
            json!({"doc": doc, "profile": "VF", "variable_config": "Bold only instance list"}),
        )
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join(resolved["relative_path"].as_str().unwrap());
    api.call(
        "session.export",
        json!({"doc": doc, "path": path, "options": resolved["options"]}),
    )
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let face = read_fonts::FontRef::new(&bytes).unwrap();
    let fvar = read_fonts::TableProvider::fvar(&face).unwrap();
    assert_eq!(fvar.instances().unwrap().iter().count(), 1);
}

#[test]
fn a_named_profile_gives_every_matrix_row_its_own_file() {
    let (api, doc, _session) = api_with(variable_source());
    api.call(
        "export.profiles_set",
        json!({
            "doc": doc,
            "profiles": [{
                "name": "Desktop",
                "subfolder": "desktop",
                // One stored name for a run of several rows.
                "file_name": "MyFont",
                "options": {"format": "Otf", "master": null, "variable": false,
                    "remove_overlap": false, "autohint": false,
                    "keep_tt_instructions": true, "preserve_untouched_tables": true,
                    "production_names": false, "table_subset": null},
            }],
            "variable_configs": [
                {"name": "Text VF", "file_name": "", "masters": [], "axes": ["wght"], "instances": []},
                {"name": "Display VF", "file_name": "", "masters": [], "axes": ["wght"], "instances": ["Bold"]},
            ],
        }),
    )
    .unwrap();

    let path = |row: Value| -> String {
        let mut params = json!({"doc": doc, "profile": "Desktop"});
        for (key, value) in row.as_object().unwrap() {
            params[key] = value.clone();
        }
        api.call("export.resolve_profile", params).unwrap()["relative_path"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let instances = [
        path(json!({"instance": "Regular"})),
        path(json!({"instance": "Bold"})),
    ];
    assert_eq!(
        instances,
        [
            "desktop/MyFont-Regular.otf".to_owned(),
            "desktop/MyFont-Bold.otf".to_owned()
        ]
    );
    let configs = [
        path(json!({"variable_config": "Text VF"})),
        path(json!({"variable_config": "Display VF"})),
    ];
    assert_eq!(
        configs,
        [
            "desktop/MyFont-Text-VF.otf".to_owned(),
            "desktop/MyFont-Display-VF.otf".to_owned()
        ]
    );
    // Every row of the matrix is a different destination.
    let mut all: Vec<&String> = instances.iter().chain(configs.iter()).collect();
    all.sort();
    let count = all.len();
    all.dedup();
    assert_eq!(all.len(), count, "two rows resolved to the same file");
}
