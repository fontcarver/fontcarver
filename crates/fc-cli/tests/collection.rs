use assert_cmd::Command;

#[test]
fn collection_preserves_input_order_and_reopens_each_face() {
    let directory = tempfile::tempdir().unwrap();
    let collection = directory.path().join("fonts.ttc");
    Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
        .args(["collection", "--output"])
        .arg(&collection)
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .assert()
        .success();
    let bytes = std::fs::read(&collection).unwrap();
    assert_eq!(fc_sfnt::face_count(&bytes).unwrap(), 2);
    for index in 0..2 {
        Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
            .args(["inspect", "--json", "--face", &index.to_string()])
            .arg(&collection)
            .assert()
            .success();
    }
    let expanded = directory.path().join("expanded.ttc");
    Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
        .args(["collection", "--output"])
        .arg(&expanded)
        .arg(&collection)
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .assert()
        .success();
    assert_eq!(
        fc_sfnt::face_count(&std::fs::read(expanded).unwrap()).unwrap(),
        3
    );
}

#[test]
fn failed_collection_input_does_not_replace_destination() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("existing.ttc");
    std::fs::write(&output, b"keep destination").unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
        .args(["collection", "--output"])
        .arg(&output)
        .arg(fc_fixtures::ufo_path("Minimal.ufo"))
        .arg(directory.path().join("missing.ttf"))
        .assert()
        .failure();
    assert_eq!(std::fs::read(output).unwrap(), b"keep destination");
}
