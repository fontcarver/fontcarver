mod common;

use std::{fs, sync::Arc};

use common::{temp_dir, test_font};
use fc_doc::fcf;

#[test]
fn fcf_plain_and_gzip_round_trip_semantically() {
    let dir = temp_dir("roundtrip");
    let plain = dir.join("font.fcf");
    let gzip = dir.join("font.fcf.gz");
    let disguised_gzip = dir.join("font.data");
    let mut font = test_font();
    font.extra.source = Some(fc_model::SourceProvenance {
        original_bytes: Some(Arc::new(vec![1, 2, 3, 4])),
        ..fc_model::SourceProvenance::default()
    });

    fcf::write(&plain, &font).unwrap();
    fcf::write(&gzip, &font).unwrap();
    fs::copy(&gzip, &disguised_gzip).unwrap();

    let mut expected = font.clone();
    expected.extra.source.as_mut().unwrap().original_bytes = None;
    assert_eq!(fcf::read(&plain).unwrap(), expected);
    assert_eq!(fcf::read(&gzip).unwrap(), expected);
    assert_eq!(fcf::read(&disguised_gzip).unwrap(), expected);

    let json: serde_json::Value = serde_json::from_slice(&fs::read(plain).unwrap()).unwrap();
    assert_eq!(json["format"], "fontcutter");
    assert_eq!(json["version"], 1);
    assert!(json.get("font").is_some());
}

#[test]
fn writer_keeps_each_glyph_on_one_compact_line() {
    let dir = temp_dir("glyph-lines");
    let path = dir.join("font.fcf");
    let font = test_font();

    fcf::write(&path, &font).unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    let glyph_lines = contents
        .lines()
        .filter(|line| line.contains("\"contours\""))
        .collect::<Vec<_>>();
    assert_eq!(glyph_lines.len(), 2);
    assert!(glyph_lines[0].contains("\"base\": {"));
    assert!(glyph_lines[1].contains("\"composite\": {"));
    assert!(glyph_lines.iter().all(|line| {
        let line = line.trim();
        line.starts_with('"') && (line.ends_with('}') || line.ends_with(','))
    }));
    assert_eq!(fcf::read(&path).unwrap(), font);
}

#[test]
fn reader_accepts_unknown_fields_and_rejects_newer_versions() {
    let dir = temp_dir("versions");
    let compatible = dir.join("compatible.fcf");
    let future = dir.join("future.fcf");
    let expected = test_font();
    let font = serde_json::to_value(&expected).unwrap();
    fs::write(
        &compatible,
        serde_json::to_vec(&serde_json::json!({
            "format": "fontcutter",
            "version": 1,
            "font": font,
            "futureField": true
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fcf::read(&compatible).unwrap(), expected);

    fs::write(
        &future,
        serde_json::to_vec(&serde_json::json!({
            "format": "fontcutter",
            "version": 2,
            "font": test_font()
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(fcf::read(&future).is_err());
}

#[test]
fn reader_accepts_the_pre_rename_format_tag() {
    let dir = temp_dir("legacy-format-tag");
    let legacy = dir.join("legacy.fcf");
    let expected = test_font();
    fs::write(
        &legacy,
        serde_json::to_vec(&serde_json::json!({
            "format": "fontworkshop",
            "version": 1,
            "font": serde_json::to_value(&expected).unwrap()
        }))
        .unwrap(),
    )
    .unwrap();

    assert_eq!(fcf::read(&legacy).unwrap(), expected);

    // Saving the document again re-stamps it with the current tag.
    fcf::write(&legacy, &expected).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&fs::read(&legacy).unwrap()).unwrap();
    assert_eq!(json["format"], "fontcutter");
}

#[test]
fn repeated_save_is_atomic_and_leaves_no_temporary_file() {
    let dir = temp_dir("atomic-write");
    let path = dir.join("font.fcf");
    let temporary_path = dir.join("font.fcf.tmp");
    let first = test_font();
    fcf::write(&path, &first).unwrap();

    let mut second = first;
    second.info.family_name = "Second save".to_owned();
    fcf::write(&path, &second).unwrap();

    assert!(!temporary_path.exists());
    assert_eq!(fcf::read(&path).unwrap(), second);
}
