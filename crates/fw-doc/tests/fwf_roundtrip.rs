mod common;

use std::{fs, sync::Arc};

use common::{temp_dir, test_font};
use fw_doc::fwf;

#[test]
fn fwf_plain_and_gzip_round_trip_semantically() {
    let dir = temp_dir("roundtrip");
    let plain = dir.join("font.fwf");
    let gzip = dir.join("font.fwf.gz");
    let disguised_gzip = dir.join("font.data");
    let mut font = test_font();
    font.extra.source = Some(fw_model::SourceProvenance {
        original_bytes: Some(Arc::new(vec![1, 2, 3, 4])),
        ..fw_model::SourceProvenance::default()
    });

    fwf::write(&plain, &font).unwrap();
    fwf::write(&gzip, &font).unwrap();
    fs::copy(&gzip, &disguised_gzip).unwrap();

    let mut expected = font.clone();
    expected.extra.source.as_mut().unwrap().original_bytes = None;
    assert_eq!(fwf::read(&plain).unwrap(), expected);
    assert_eq!(fwf::read(&gzip).unwrap(), expected);
    assert_eq!(fwf::read(&disguised_gzip).unwrap(), expected);

    let json: serde_json::Value = serde_json::from_slice(&fs::read(plain).unwrap()).unwrap();
    assert_eq!(json["format"], "fontworkshop");
    assert_eq!(json["version"], 1);
    assert!(json.get("font").is_some());
}

#[test]
fn reader_accepts_unknown_fields_and_rejects_newer_versions() {
    let dir = temp_dir("versions");
    let compatible = dir.join("compatible.fwf");
    let future = dir.join("future.fwf");
    let expected = test_font();
    let font = serde_json::to_value(&expected).unwrap();
    fs::write(
        &compatible,
        serde_json::to_vec(&serde_json::json!({
            "format": "fontworkshop",
            "version": 1,
            "font": font,
            "futureField": true
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fwf::read(&compatible).unwrap(), expected);

    fs::write(
        &future,
        serde_json::to_vec(&serde_json::json!({
            "format": "fontworkshop",
            "version": 2,
            "font": test_font()
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(fwf::read(&future).is_err());
}
