use assert_cmd::Command;
use fc_model::{Font, Glyph, Layer};

#[test]
fn check_reports_errors_as_json_and_preserves_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("problems.fcf");
    let mut font = Font::new_empty("Check", 1000);
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![0xd800, 0x110000];
    glyph
        .layers
        .push(Layer::new_master(font.default_master().id.clone()));
    font.insert_glyph(glyph).unwrap();
    fc_doc::fcf::write(&path, &font).unwrap();
    let before = std::fs::read(&path).unwrap();
    let output = Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
        .arg("check")
        .arg(&path)
        .args(["--json", "--limit", "1"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["issues"][0]["code"], "invalid-unicode");
    assert_eq!(report["truncated"], true);
    assert_eq!(report["has_errors"], true);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn check_clean_font_succeeds_and_rejects_unbounded_limit() {
    Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
        .arg("check")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .arg("--json")
        .assert()
        .success();
    Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
        .arg("check")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .args(["--limit", "2001"])
        .assert()
        .code(2);
}
