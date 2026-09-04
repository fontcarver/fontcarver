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
fn convert_rejects_an_unknown_output_format_with_exit_code_two() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let output = directory.path().join("DejaVuSans.bogus");

    fw().arg("convert")
        .arg(fw_fixtures::path("DejaVuSans.ttf"))
        .arg(output)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "unsupported output format 'bogus': supported formats are fwf, fwf.gz, ttf, woff, woff2, ufo, designspace",
        ));
}

#[test]
fn build_compiles_a_ufo_into_a_truetype_font() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let output = directory.path().join("Minimal.ttf");

    fw().arg("build")
        .arg(fw_fixtures::ufo_path("Minimal.ufo"))
        .arg(&output)
        .args(["--master", "Regular"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote"));

    let bytes = std::fs::read(&output).expect("read the compiled font");
    assert_eq!(&bytes[..4], &[0x00, 0x01, 0x00, 0x00]);
}

#[test]
fn build_rejects_a_source_output_format_with_exit_code_two() {
    let directory = tempfile::tempdir().expect("create temporary directory");

    fw().arg("build")
        .arg(fw_fixtures::ufo_path("Minimal.ufo"))
        .arg(directory.path().join("Minimal.ufo"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "unsupported output format 'ufo': supported formats are ttf, woff, woff2",
        ));
}

#[test]
fn build_reports_an_unknown_master() {
    let directory = tempfile::tempdir().expect("create temporary directory");

    fw().arg("build")
        .arg(fw_fixtures::ufo_path("Minimal.ufo"))
        .arg(directory.path().join("Minimal.ttf"))
        .args(["--master", "Nope"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown master 'Nope'"));
}

#[test]
fn convert_writes_a_ufo_package_from_a_binary_font() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let output = directory.path().join("Minimal.ufo");

    fw().arg("convert")
        .arg(fw_fixtures::ufo_path("Minimal.ufo"))
        .arg(&output)
        .assert()
        .success();

    assert!(output.join("metainfo.plist").is_file());
}

#[test]
fn shape_prints_one_line_per_glyph_with_kerning_applied() {
    let output = fw()
        .arg("shape")
        .arg(fw_fixtures::ufo_path("Minimal.ufo"))
        .arg("AB")
        .output()
        .expect("shape text");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf-8 output");
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    let first: Vec<_> = lines[0].split('\t').collect();
    assert_eq!(first.len(), 7, "{stdout}");
    assert_eq!(first[1], "A");
    assert_eq!(first[6], "0");
    assert_eq!(lines[1].split('\t').nth(1), Some("B"));
}

#[test]
fn shape_can_switch_a_feature_off() {
    let kerned = shaped_advances(&["AB"]);
    let unkerned = shaped_advances(&["AB", "--features", "-kern"]);

    assert_eq!(kerned, vec!["520", "620"]);
    assert_eq!(unkerned, vec!["600", "620"]);
}

/// Runs `fw shape` on the minimal UFO and returns the advance column.
fn shaped_advances(arguments: &[&str]) -> Vec<String> {
    let output = fw()
        .arg("shape")
        .arg(fw_fixtures::ufo_path("Minimal.ufo"))
        .args(arguments)
        .output()
        .expect("shape text");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8 output")
        .lines()
        .filter_map(|line| line.split('\t').nth(2).map(str::to_owned))
        .collect()
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
