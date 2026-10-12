use fc_compile::{export, shaping_fingerprint, ExportOptions};
use fc_formats::ufo::{read_ufo, write_ufo, UfoWriteOptions};
use fc_model::{FeatureBlock, FeatureClass, Features, Font};

const WHOLE_SOURCE: &str = concat!(
    "languagesystem DFLT dflt;\r\n",
    "languagesystem latn dflt;\r\n",
    "# 保留中文与 emoji 🪵\r\n",
    "@First = [ A ];\r\n",
    "lookup Ligature {\r\n",
    "  sub @First A by B; # preserve spacing\r\n",
    "} Ligature;\r\n",
    "feature liga { lookup Ligature; } liga;\r\n",
);

#[test]
fn whole_file_ufo_features_compile_and_round_trip() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("Source.ufo");
    let mut ufo = norad::Font::load(fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    ufo.features = WHOLE_SOURCE.to_owned();
    ufo.save(&input).unwrap();
    // Norad normalizes CRLF when saving, so write the intended source fixture directly.
    std::fs::write(input.join("features.fea"), WHOLE_SOURCE).unwrap();

    let font = read_ufo(&input).unwrap();
    let bytes = compile(&font);
    assert_ligature(&bytes, "AA");
    assert!(font.features.features.is_empty());
    assert_eq!(font.features.prefixes.len(), 1);
    assert_eq!(font.features.prefixes[0].code, WHOLE_SOURCE);
    assert_eq!(font.features.to_fea(), WHOLE_SOURCE);

    let output = temp.path().join("RoundTrip.ufo");
    write_ufo(
        &font,
        &font.default_master().id,
        &output,
        &UfoWriteOptions::default(),
    )
    .unwrap();
    assert_eq!(norad::Font::load(&output).unwrap().features, WHOLE_SOURCE);
    let reopened = read_ufo(&output).unwrap();
    assert_eq!(reopened.features.to_fea(), WHOLE_SOURCE);
    let reopened_bytes = compile(&reopened);
    assert_eq!(
        shaping_fingerprint(&reopened_bytes, "AAAA").unwrap(),
        shaping_fingerprint(&bytes, "AAAA").unwrap()
    );
    assert_ligature(&reopened_bytes, "AA");
}

#[test]
fn structured_source_ufo_export_uses_the_compiled_assembly() {
    for class_code in [" A # 首字 🪵\n", "[ A # 首字 🪵\n]"] {
        let mut font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
        font.features = Features {
            prefixes: vec![
                FeatureBlock {
                    tag: "Languages".into(),
                    code: "languagesystem DFLT dflt;\r\nlanguagesystem latn dflt;".into(),
                    ..FeatureBlock::default()
                },
                FeatureBlock {
                    tag: "Disabled prefix".into(),
                    code: "invalid {".into(),
                    disabled: true,
                    ..FeatureBlock::default()
                },
            ],
            classes: vec![FeatureClass {
                name: "@First".into(),
                code: class_code.into(),
                ..FeatureClass::default()
            }],
            features: vec![
                FeatureBlock {
                    tag: "liga".into(),
                    code: "sub @First A by B; # 合字\n".into(),
                    ..FeatureBlock::default()
                },
                FeatureBlock {
                    tag: "liga".into(),
                    code: "feature liga { sub A B by B; } liga;".into(),
                    ..FeatureBlock::default()
                },
                FeatureBlock {
                    tag: "calt".into(),
                    code: "invalid {".into(),
                    disabled: true,
                    ..FeatureBlock::default()
                },
            ],
            ..Features::default()
        };
        let bytes = compile(&font);
        assert_ligature(&bytes, "AA");
        assert_ligature(&bytes, "AB");

        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("Structured.ufo");
        write_ufo(
            &font,
            &font.default_master().id,
            &output,
            &UfoWriteOptions::default(),
        )
        .unwrap();
        let written_source = norad::Font::load(&output).unwrap().features;
        assert_eq!(written_source, font.features.to_fea());
        assert!(!written_source.contains("invalid"));
        let reopened = read_ufo(&output).unwrap();
        let reopened_bytes = compile(&reopened);
        for text in ["AA", "AB", "ABA", "AAB", "AAAA"] {
            assert_eq!(
                shaping_fingerprint(&reopened_bytes, text).unwrap(),
                shaping_fingerprint(&bytes, text).unwrap(),
                "{class_code:?}: {text}"
            );
        }
    }
}

#[test]
fn legacy_whole_file_records_keep_enabled_and_disabled_ufo_behavior() {
    for disabled in [false, true] {
        let mut font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
        font.features = serde_json::from_value(serde_json::json!({
            "features": [{
                "tag": "fea",
                "code": WHOLE_SOURCE,
                "disabled": disabled
            }]
        }))
        .unwrap();
        assert!(font.features.features.is_empty());
        assert_eq!(font.features.prefixes.len(), 1);
        assert_eq!(font.features.prefixes[0].code, WHOLE_SOURCE);
        assert_eq!(font.features.prefixes[0].disabled, disabled);
        let bytes = compile(&font);
        let expected_source = if disabled { "" } else { WHOLE_SOURCE };
        assert_eq!(font.features.to_fea(), expected_source);
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("Legacy.ufo");
        write_ufo(
            &font,
            &font.default_master().id,
            &output,
            &UfoWriteOptions::default(),
        )
        .unwrap();
        assert_eq!(
            norad::Font::load(&output).unwrap().features,
            expected_source
        );
        let reopened = read_ufo(&output).unwrap();
        let reopened_bytes = compile(&reopened);
        assert_eq!(
            shaping_fingerprint(&reopened_bytes, "AA").unwrap(),
            shaping_fingerprint(&bytes, "AA").unwrap()
        );
        assert_eq!(
            shaping_fingerprint(&bytes, "AA").unwrap().len(),
            if disabled { 2 } else { 1 }
        );
    }
}

#[test]
fn empty_feature_source_removes_the_previous_ufo_source_file() {
    let mut font = read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("Cleared.ufo");
    let master = font.default_master().id.clone();
    write_ufo(&font, &master, &output, &UfoWriteOptions::default()).unwrap();
    assert!(output.join("features.fea").is_file());

    font.features = Features::default();
    write_ufo(&font, &master, &output, &UfoWriteOptions::default()).unwrap();
    assert!(!output.join("features.fea").exists());
    assert!(read_ufo(&output).unwrap().features.prefixes.is_empty());
}

fn compile(font: &Font) -> Vec<u8> {
    export(font, &|_| 0, &ExportOptions::default())
        .unwrap()
        .bytes
}

fn assert_ligature(bytes: &[u8], text: &str) {
    let expected = shaping_fingerprint(bytes, "B").unwrap();
    assert_eq!(expected.len(), 1);
    assert_eq!(shaping_fingerprint(bytes, text).unwrap(), expected);
}
