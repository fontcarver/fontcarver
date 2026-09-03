use assert_cmd::Command;
use predicates::prelude::*;

fn fw() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("fw"))
}

#[test]
fn inspect_ttf_prints_summary_and_preserved_tables() {
    fw().arg("inspect")
        .arg(fw_fixtures::path("DejaVuSans.ttf"))
        .assert()
        .success()
        .stdout(predicate::str::contains("family: DejaVu Sans"))
        .stdout(predicate::str::contains("glyphs: 6253"))
        .stdout(predicate::str::contains("tables:"))
        .stdout(predicate::str::contains("preserved"));
}

#[test]
fn convert_preserves_the_glyph_count_in_fwf() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let source = fw_fixtures::path("DejaVuSans.ttf");
    let output = directory.path().join("DejaVuSans.fwf");

    let original = fw()
        .arg("inspect")
        .arg(&source)
        .arg("--json")
        .output()
        .expect("inspect source font");
    assert!(original.status.success());
    let original: serde_json::Value =
        serde_json::from_slice(&original.stdout).expect("parse source summary");

    fw().arg("convert")
        .arg(&source)
        .arg(&output)
        .assert()
        .success();

    let converted = fw()
        .arg("inspect")
        .arg(&output)
        .arg("--json")
        .output()
        .expect("inspect converted font");
    assert!(converted.status.success());
    let converted: serde_json::Value =
        serde_json::from_slice(&converted.stdout).expect("parse converted summary");

    assert_eq!(
        original["summary"]["glyph_count"],
        converted["summary"]["glyph_count"]
    );
}

#[test]
fn convert_rejects_binary_output_with_exit_code_two() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let output = directory.path().join("DejaVuSans.ttf");

    fw().arg("convert")
        .arg(fw_fixtures::path("DejaVuSans.ttf"))
        .arg(output)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "unsupported output format 'ttf': only .fwf and .fwf.gz are supported in this version",
        ));
}

#[test]
fn glyphs_maps_filters_and_prints_tab_separated_columns() {
    fw().arg("glyphs")
        .arg(fw_fixtures::path("DejaVuSans.ttf"))
        .args(["--filter", "cat=Letter"])
        .args(["--filter", "script=Latn"])
        .args(["--filter", "block=Basic Latin"])
        .args(["--filter", "query=U+0041"])
        .args(["--limit", "1"])
        .assert()
        .success()
        .stdout("A\tU+0041\t1401\n");
}

#[test]
fn glyphs_accepts_the_unencoded_filter() {
    fw().arg("glyphs")
        .arg(fw_fixtures::path("DejaVuSans.ttf"))
        .args(["--filter", "unencoded=true"])
        .args(["--limit", "1"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[^\t]+\t\t[0-9]+(?:\.[0-9]+)?\n$").unwrap());
}
