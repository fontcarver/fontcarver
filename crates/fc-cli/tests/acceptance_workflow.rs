use std::sync::Arc;

use assert_cmd::Command as CliCommand;
use fc_api::{Api, ApplyParams, GlyphCell, LayerOutline};
use fc_doc::{Command, Session};
use fc_model::{Contour, Point, PointKind};
use serde_json::json;

fn fontcarver() -> CliCommand {
    CliCommand::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
}

fn triangle(offset: f64) -> Contour {
    Contour {
        closed: true,
        points: vec![
            Point {
                x: 50.0 + offset,
                y: 0.0,
                kind: PointKind::Line,
                ..Point::default()
            },
            Point {
                x: 300.0 + offset,
                y: 700.0,
                kind: PointKind::Line,
                ..Point::default()
            },
            Point {
                x: 550.0 + offset,
                y: 0.0,
                kind: PointKind::Line,
                ..Point::default()
            },
        ],
        ..Contour::default()
    }
}

#[test]
fn scratch_font_flows_from_api_drawing_through_cli_reopen() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let output_path = directory.path().join("ScratchLatin.ttf");
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .expect("configure API");
    let summary = api
        .call(
            "session.new",
            json!({ "family": "Scratch Latin", "units_per_em": 1000 }),
        )
        .expect("create font");
    let doc: fc_doc::DocId = serde_json::from_value(summary["doc"].clone()).expect("document id");
    let added: Vec<GlyphCell> = serde_json::from_value(
        api.call("glyphs.add", json!({ "doc": doc, "recipe": "A-C" }))
            .expect("add Latin glyphs")["added"]
            .clone(),
    )
    .expect("decode added glyphs");
    assert_eq!(
        added
            .iter()
            .map(|cell| cell.name.as_str())
            .collect::<Vec<_>>(),
        ["A", "B", "C"]
    );

    for (index, cell) in added.iter().enumerate() {
        let outline: LayerOutline = serde_json::from_value(
            api.call("glyphs.outline", json!({ "doc": doc, "gid": cell.gid }))
                .expect("read new glyph outline"),
        )
        .expect("decode new glyph outline");
        let width = 600.0 + index as f64 * 20.0;
        api.call(
            "doc.apply",
            serde_json::to_value(ApplyParams {
                doc,
                command: Command::Batch(vec![
                    Command::AddContour {
                        glyph: cell.name.as_str().into(),
                        layer: outline.layer.clone(),
                        contour: triangle(index as f64 * 10.0),
                    },
                    Command::SetWidth {
                        glyph: cell.name.as_str().into(),
                        layer: outline.layer,
                        width,
                    },
                ]),
                label: format!("Draw {}", cell.name),
            })
            .expect("serialize drawing"),
        )
        .expect("draw glyph");
    }
    api.call(
        "session.export",
        json!({
            "doc": doc,
            "path": output_path,
            "options": { "format": "Ttf" }
        }),
    )
    .expect("export scratch font");

    let inspected = fontcarver()
        .arg("inspect")
        .arg(&output_path)
        .arg("--json")
        .output()
        .expect("inspect exported font with current CLI");
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    let inspected: serde_json::Value =
        serde_json::from_slice(&inspected.stdout).expect("decode CLI inspection");
    assert_eq!(inspected["summary"]["family_name"], "Scratch Latin");
    assert_eq!(inspected["summary"]["glyph_count"], 4);

    let glyph = fontcarver()
        .arg("glyphs")
        .arg(&output_path)
        .args(["--filter", "query=U+0041", "--limit", "1"])
        .output()
        .expect("list A through current CLI");
    assert!(glyph.status.success());
    assert_eq!(
        String::from_utf8(glyph.stdout).expect("glyph output"),
        "A\tU+0041\t600\n"
    );

    let shaped = fontcarver()
        .arg("shape")
        .arg(&output_path)
        .arg("ABC")
        .output()
        .expect("shape scratch font through current CLI");
    assert!(shaped.status.success());
    let names = String::from_utf8(shaped.stdout)
        .expect("shape output")
        .lines()
        .filter_map(|line| line.split('\t').nth(1).map(str::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(names, ["A", "B", "C"]);

    let reopened = fc_sfnt::read_path(&output_path, &fc_sfnt::ReadOptions::default())
        .expect("reopen scratch export")
        .fonts
        .remove(0);
    let a = reopened.glyph("A").expect("reopened A");
    let layer = a
        .master_layer(&reopened.default_master().id)
        .expect("A master layer");
    assert_eq!(layer.width, 600.0);
    assert_eq!(layer.contours[0].points.len(), 3);
}
