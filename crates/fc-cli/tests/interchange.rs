use assert_cmd::Command;
use fc_model::{Contour, Font, Glyph, Point};

fn supported_source(path: &std::path::Path) {
    // Minimal.ufo includes components, active FEA and class kerning. Build an
    // outline-only fixture within all these explicitly bounded format subsets.
    let mut font = Font::new_empty("Interchange", 1000);
    font.info.postscript_font_name = Some("Interchange-Regular".into());
    for (name, code, width) in [("A", 65, 600.0), ("V", 86, 650.0), ("space", 32, 300.0)] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = vec![code];
        font.insert_glyph(glyph).unwrap();
        let layer = &mut font.glyph_mut(name).unwrap().layers[0];
        layer.width = width;
        if name != "space" {
            layer.contours.push(Contour {
                closed: true,
                points: [(50.0, 0.0), (300.0, 700.0), (550.0, 0.0)]
                    .into_iter()
                    .map(|(x, y)| Point {
                        x,
                        y,
                        ..Point::default()
                    })
                    .collect(),
                ..Contour::default()
            });
        }
    }
    fc_doc::fcf::write(path, &font).unwrap();
}

#[test]
fn new_interchange_outputs_reopen_through_cli_reader() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("Interchange.fcf");
    supported_source(&input);
    for extension in ["svg", "eps", "afm", "pfm", "ttx", "cff", "t42"] {
        let output = directory.path().join(format!("Minimal.{extension}"));
        let mut command = Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"));
        command.arg("convert").arg(&input).arg(&output);
        if extension == "eps" {
            command.args(["--glyph", "A"]);
        }
        command.assert().success();
        let inspected = Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
            .arg("inspect")
            .arg(&output)
            .arg("--json")
            .output()
            .unwrap();
        assert!(
            inspected.status.success(),
            "{extension}: {}",
            String::from_utf8_lossy(&inspected.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
        assert!(report["summary"]["glyph_count"].as_u64().unwrap() > 0);
        if matches!(extension, "afm" | "pfm") {
            assert!(report["summary"]["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|warning| warning["code"] == "metrics-only-source"));
        }
    }
}

#[test]
fn unsupported_source_semantics_keep_the_destination_intact() {
    let directory = tempfile::tempdir().unwrap();
    for extension in ["svg", "afm", "pfm"] {
        let output = directory.path().join(format!("existing.{extension}"));
        std::fs::write(&output, b"existing destination").unwrap();
        Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
            .arg("convert")
            .arg(fc_fixtures::ufo_path("Minimal.ufo"))
            .arg(&output)
            .assert()
            .failure();
        assert_eq!(std::fs::read(&output).unwrap(), b"existing destination");
    }
}
