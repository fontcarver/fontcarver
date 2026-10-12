use assert_cmd::Command;

#[test]
fn convert_and_inspect_all_new_formats_and_build_new_binary_targets() {
    let directory = tempfile::tempdir().unwrap();
    for extension in ["otf", "glyphs", "sfd", "pfa", "pfb"] {
        let output = directory.path().join(format!("Minimal.{extension}"));
        Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
            .arg("convert")
            .arg(fc_fixtures::ufo_path("Minimal.ufo"))
            .arg(&output)
            .assert()
            .success();
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
        let expected = if matches!(extension, "otf" | "pfa" | "pfb") {
            4
        } else {
            3
        };
        assert_eq!(
            report["summary"]["glyph_count"], expected,
            "{extension}: {report}"
        );
        Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
            .arg("shape")
            .arg(&output)
            .arg("AB")
            .assert()
            .success();
        if matches!(extension, "otf" | "pfa" | "pfb") {
            Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
                .arg("build")
                .arg(fc_fixtures::ufo_path("Minimal.ufo"))
                .arg(&output)
                .args(["--master", "Regular"])
                .assert()
                .success();
        }
    }
}
