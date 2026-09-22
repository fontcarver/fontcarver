use assert_cmd::Command;
use predicates::prelude::*;

fn fontcarver() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
}

#[test]
fn inspect_ttf_prints_summary_and_preserved_tables() {
    fontcarver()
        .arg("inspect")
        .arg(fc_fixtures::path("DejaVuSans.ttf"))
        .assert()
        .success()
        .stdout(predicate::str::contains("family: DejaVu Sans"))
        .stdout(predicate::str::contains("glyphs: 6253"))
        .stdout(predicate::str::contains("tables:"))
        .stdout(predicate::str::contains("preserved"));
}

#[test]
fn convert_preserves_the_glyph_count_in_fcf() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let source = fc_fixtures::path("DejaVuSans.ttf");
    let output = directory.path().join("DejaVuSans.fcf");

    let original = fontcarver()
        .arg("inspect")
        .arg(&source)
        .arg("--json")
        .output()
        .expect("inspect source font");
    assert!(original.status.success());
    let original: serde_json::Value =
        serde_json::from_slice(&original.stdout).expect("parse source summary");

    fontcarver()
        .arg("convert")
        .arg(&source)
        .arg(&output)
        .assert()
        .success();

    let converted = fontcarver()
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

    fontcarver().arg("convert")
        .arg(fc_fixtures::path("DejaVuSans.ttf"))
        .arg(output)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "unsupported output format 'bogus': supported formats are bdf, pcf, psf, fnt, fon, otb, fcf, fcf.gz, ttf, otf, woff, woff2, ufo, designspace, glyphs, sfd, pfa, pfb",
        ));
}

#[test]
fn build_compiles_a_ufo_into_a_truetype_font() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let output = directory.path().join("Minimal.ttf");

    fontcarver()
        .arg("build")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .arg(&output)
        .args(["--master", "Regular"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote"));

    let bytes = std::fs::read(&output).expect("read the compiled font");
    assert_eq!(&bytes[..4], &[0x00, 0x01, 0x00, 0x00]);
}

#[test]
fn build_autohint_emits_cff_stems_and_rejects_variable_requests_before_writing() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("hinted.otf");
    fontcarver()
        .arg("build")
        .arg(fc_fixtures::path("SourceSans3Min.otf"))
        .arg(&output)
        .arg("--autohint")
        .assert()
        .success();
    let font = fc_sfnt::read_bytes(&std::fs::read(&output).unwrap(), &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert!(font
        .glyphs
        .values()
        .any(|glyph| !glyph.layers[0].hints.is_empty()));
    let rejected = directory.path().join("rejected.ttf");
    fontcarver()
        .arg("build")
        .arg(fc_fixtures::path("SourceSans3Min.otf"))
        .arg(&rejected)
        .args(["--autohint", "--variable"])
        .assert()
        .failure();
    assert!(!rejected.exists());
}

#[test]
fn build_rejects_a_source_output_format_with_exit_code_two() {
    let directory = tempfile::tempdir().expect("create temporary directory");

    fontcarver()
        .arg("build")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .arg(directory.path().join("Minimal.ufo"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "unsupported output format 'ufo': supported formats are ttf, otf, woff, woff2, pfa, pfb",
        ));
}

#[test]
fn build_reports_an_unknown_master() {
    let directory = tempfile::tempdir().expect("create temporary directory");

    fontcarver()
        .arg("build")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .arg(directory.path().join("Minimal.ttf"))
        .args(["--master", "Nope"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown master 'Nope'"));
}

#[test]
fn convert_writes_a_ufo_package_from_source_font() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let output = directory.path().join("Minimal.ufo");

    fontcarver()
        .arg("convert")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .arg(&output)
        .assert()
        .success();

    assert!(output.join("metainfo.plist").is_file());
}

#[test]
fn binary_to_source_export_requires_validated_layout_conversion() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let binary = directory.path().join("Minimal.ttf");
    fontcarver()
        .arg("build")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .arg(&binary)
        .assert()
        .success();
    let output = directory.path().join("Converted.ufo");
    fontcarver()
        .arg("convert")
        .arg(&binary)
        .arg(&output)
        .assert()
        .failure()
        .stderr(predicate::str::contains("conversion"));
    assert!(!output.exists());
}

#[test]
fn shape_prints_one_line_per_glyph_with_kerning_applied() {
    let output = fontcarver()
        .arg("shape")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
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

/// Runs `fontcarver shape` on the minimal UFO and returns the advance column.
fn shaped_advances(arguments: &[&str]) -> Vec<String> {
    let output = fontcarver()
        .arg("shape")
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
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
    fontcarver()
        .arg("glyphs")
        .arg(fc_fixtures::path("DejaVuSans.ttf"))
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
    fontcarver()
        .arg("glyphs")
        .arg(fc_fixtures::path("DejaVuSans.ttf"))
        .args(["--filter", "unencoded=true"])
        .args(["--limit", "1"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[^\t]+\t\t[0-9]+(?:\.[0-9]+)?\n$").unwrap());
}
