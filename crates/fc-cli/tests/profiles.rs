//! The command line builds from the same stored profiles the editor saves.

use std::{path::Path, sync::Arc};

use assert_cmd::Command;
use fc_api::Api;
use fc_doc::Session;
use fc_model::{Axis, Contour, Font, Glyph, Instance, Layer, LayerId, LayerKind, MasterId, Tag};
use kurbo::BezPath;
use predicates::prelude::*;
use serde_json::json;

fn fontcarver() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
}

/// A two-master weight axis with two named instances and real outlines.
fn source() -> Font {
    let mut font = Font::new_empty("Profile Family", 1000);
    // Without a stored creation date both exports stamp `head` with the current
    // second, and two exports straddling a second boundary differ by that alone.
    font.info.open_type_head_created = Some("2020/01/02 03:04:05".to_owned());
    let regular = font.default_master().id.clone();
    font.masters[0].name = "Regular".into();
    font.masters[0].metrics.ascender = 750.0;
    font.masters[0].metrics.descender = -250.0;
    for (name, unicode) in [(".notdef", None), ("A", Some(65u32)), ("B", Some(66u32))] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(regular.clone());
        layer.width = 600.0;
        let mut path = BezPath::new();
        path.move_to((50.0, 0.0));
        path.line_to((50.0, 700.0));
        path.line_to((500.0, 700.0));
        path.line_to((500.0, 0.0));
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert glyph");
    }
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
        let mut layer = glyph.master_layer(&regular).expect("regular").clone();
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

/// Saves a project whose stored profiles the command line will read back.
fn project(path: &Path) -> Api {
    let session = Arc::new(Session::new());
    let doc = session.open_font(source(), None);
    let api = Api::new(Arc::clone(&session));
    api.call(
        "export.profiles_set",
        json!({
            "doc": doc,
            "profiles": [
                {
                    "name": "Web",
                    "subfolder": "web",
                    "file_name": "",
                    "options": {
                        "format": "Woff2", "master": null, "variable": false,
                        "remove_overlap": false, "autohint": false,
                        "keep_tt_instructions": false,
                        "preserve_untouched_tables": true,
                        "production_names": false, "table_subset": null,
                    },
                },
                {
                    "name": "VF",
                    "subfolder": "",
                    "file_name": "ProfileFamily-VF",
                    "options": {
                        "format": "Ttf", "master": null, "variable": true,
                        "remove_overlap": false, "autohint": false,
                        "keep_tt_instructions": false,
                        "preserve_untouched_tables": true,
                        "production_names": false, "table_subset": null,
                    },
                },
            ],
            "variable_configs": [{
                "name": "Bold only",
                "file_name": "ProfileFamily-Bold-VF",
                "masters": [],
                "axes": ["wght"],
                "instances": ["Bold"],
            }],
        }),
    )
    .expect("store export profiles");
    api.call("session.save", json!({"doc": doc, "path": path}))
        .expect("save the project");
    api
}

#[test]
fn profiles_are_listed_from_the_project_file() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Profiles.fcf");
    let _api = project(&path);
    fontcarver()
        .arg("profiles")
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("profile Web: format=Woff2"))
        .stdout(predicate::str::contains("subfolder=web"))
        .stdout(predicate::str::contains("profile VF:"))
        .stdout(predicate::str::contains(
            "variable-config Bold only: file=ProfileFamily-Bold-VF",
        ))
        .stdout(predicate::str::contains("axes=wght"))
        .stdout(predicate::str::contains("instances=Bold"));
}

#[test]
fn a_stored_profile_names_the_destination_under_the_output_directory() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Profiles.fcf");
    let _api = project(&path);
    let out = directory.path().join("dist");
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("Web")
        .arg("--out-dir")
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::contains("web"));
    let written = out.join("web").join("Profile-Family-Web.woff2");
    assert!(written.is_file(), "{} was written", written.display());
    assert_eq!(&std::fs::read(&written).unwrap()[..4], b"wOF2");
}

#[test]
fn every_named_instance_builds_from_one_profile() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Profiles.fcf");
    let _api = project(&path);
    let out = directory.path().join("dist");
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("Web")
        .arg("--instances")
        .arg("all")
        .arg("--out-dir")
        .arg(&out)
        .assert()
        .success();
    for name in ["Regular", "Bold"] {
        let written = out.join("web").join(format!("Profile-Family-{name}.woff2"));
        assert!(written.is_file(), "{} was written", written.display());
    }
    // A named subset builds only what it names.
    let single = directory.path().join("single");
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("Web")
        .arg("--instances")
        .arg("Bold")
        .arg("--out-dir")
        .arg(&single)
        .assert()
        .success();
    assert!(single
        .join("web")
        .join("Profile-Family-Bold.woff2")
        .is_file());
    assert!(!single
        .join("web")
        .join("Profile-Family-Regular.woff2")
        .exists());
}

#[test]
fn a_stored_variable_configuration_subsets_the_design_space() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Profiles.fcf");
    let _api = project(&path);
    let out = directory.path().join("dist");
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("VF")
        .arg("--variable-config")
        .arg("Bold only")
        .arg("--out-dir")
        .arg(&out)
        .assert()
        .success();
    // A configuration's own file name is per-row, so it wins over the
    // profile's single stored name.
    let written = out.join("ProfileFamily-Bold-VF.ttf");
    assert!(written.is_file(), "{} was written", written.display());
    let bytes = std::fs::read(&written).expect("read the variable font");
    let face = read_fonts::FontRef::new(&bytes).expect("compiled variable font");
    let fvar = read_fonts::TableProvider::fvar(&face).expect("fvar");
    assert_eq!(fvar.axis_count(), 1);
    assert_eq!(
        fvar.instances().expect("instances").iter().count(),
        1,
        "only the instance the configuration names is written"
    );
}

#[test]
fn the_command_line_and_the_editor_produce_the_same_bytes_for_one_profile() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Profiles.fcf");
    let api = project(&path);
    let doc = fc_doc::DocId(1);
    let resolved = api
        .call(
            "export.resolve_profile",
            json!({"doc": doc, "profile": "VF", "variable_config": "Bold only"}),
        )
        .expect("resolve the stored configuration");
    let in_process = directory.path().join("in-process.ttf");
    api.call(
        "session.export",
        json!({"doc": doc, "path": in_process, "options": resolved["options"]}),
    )
    .expect("export in process");

    let out = directory.path().join("cli");
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("VF")
        .arg("--variable-config")
        .arg("Bold only")
        .arg("--out-dir")
        .arg(&out)
        .assert()
        .success();
    let from_cli = out.join(
        resolved["relative_path"]
            .as_str()
            .expect("the resolved destination"),
    );
    assert_eq!(resolved["relative_path"], "ProfileFamily-Bold-VF.ttf");
    assert_eq!(
        std::fs::read(&in_process).expect("in-process bytes"),
        std::fs::read(&from_cli).expect("command-line bytes"),
        "the same stored profile must produce the same file"
    );
}

#[test]
fn the_preflight_report_matches_the_check_command() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Profiles.fcf");
    let _api = project(&path);
    let out = directory.path().join("dist");
    let build = fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("Web")
        .arg("--out-dir")
        .arg(&out)
        .arg("--preflight")
        .output()
        .expect("run build with preflight");
    assert!(build.status.success());
    let build = String::from_utf8(build.stdout).expect("utf-8 build output");
    assert!(build.contains("preflight:"), "{build}");

    let check = fontcarver()
        .arg("check")
        .arg(&path)
        .output()
        .expect("run check");
    let check = String::from_utf8(check.stdout).expect("utf-8 check output");
    let summary = check
        .lines()
        .find(|line| line.contains("problems detected"))
        .expect("check prints a summary");
    assert!(build.contains(summary), "{build}\n---\n{check}");
}

#[test]
fn a_profile_cannot_be_silently_overridden_by_ad_hoc_flags() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Profiles.fcf");
    let _api = project(&path);
    let out = directory.path().join("dist");
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("Web")
        .arg("--out-dir")
        .arg(&out)
        .arg("--remove-overlap")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--remove-overlap"));
    assert!(!out.exists(), "nothing was written");
}

#[test]
fn unknown_profiles_instances_and_missing_destinations_are_refused() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Profiles.fcf");
    let _api = project(&path);
    let out = directory.path().join("dist");
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("Nope")
        .arg("--out-dir")
        .arg(&out)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Nope"));
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("Web")
        .arg("--instances")
        .arg("Nope")
        .arg("--out-dir")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown named instance 'Nope'"));
    fontcarver()
        .arg("build")
        .arg(&path)
        .arg("--profile")
        .arg("Web")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--out-dir"));
}
