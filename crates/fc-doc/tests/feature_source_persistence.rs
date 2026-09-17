mod common;

use std::{collections::BTreeSet, fs, path::Path};

use common::{temp_dir, test_font};
use fc_doc::{
    fcf,
    kerning::{check_horizontal_editing, HorizontalKerningEditBlock as Block},
};
use fc_model::{FeaturesSource, Font, Glyph};
use serde_json::{json, Value};

const SOURCE: &str = "# 保留原文 🪶\r\nlanguagesystem DFLT dflt;\r\n@Pair = [A V];\r\nlookup Spacing { pos A V -80; } Spacing;\r\nfeature kern { lookup Spacing; } kern;\r\n";
const EXISTING_ID: &str = "5e8bcba0-819c-45a0-8dc9-9163c59c10bf";

fn read_source_document(path: &Path, features: Value) -> Font {
    let mut fixture = test_font();
    for name in ["A", "V"] {
        fixture.insert_glyph(Glyph::new(name)).unwrap();
    }
    let mut font = serde_json::to_value(fixture).unwrap();
    font["features"] = features;
    fs::write(
        path,
        serde_json::to_vec(&json!({
            "format": "fontcarver",
            "version": 1,
            "font": font
        }))
        .unwrap(),
    )
    .unwrap();
    fcf::read(path).unwrap()
}

fn legacy_features(disabled: bool) -> Value {
    json!({
        "prefixes": [],
        "classes": [],
        "features": [{
            "id": EXISTING_ID,
            "tag": "fea",
            "code": SOURCE,
            "automatic": false,
            "disabled": disabled,
            "notes": ""
        }],
        "source_of_truth": "Fea"
    })
}

fn source_ids(font: &Font) -> Vec<String> {
    let features = serde_json::to_value(&font.features).unwrap();
    ["prefixes", "classes", "features"]
        .into_iter()
        .flat_map(|kind| features[kind].as_array().unwrap())
        .map(|item| {
            item["id"]
                .as_str()
                .expect("source item has an ID")
                .to_owned()
        })
        .collect()
}

#[test]
fn old_native_whole_file_fea_becomes_a_prefix_without_losing_source_or_id() {
    let dir = temp_dir("feature-source-legacy");
    let path = dir.join("legacy.fcf");
    let font = read_source_document(&path, legacy_features(false));

    assert!(font.features.features.is_empty());
    assert_eq!(font.features.prefixes.len(), 1);
    let prefix = &font.features.prefixes[0];
    assert_eq!(prefix.tag, "fea");
    assert_eq!(prefix.code, SOURCE);
    assert!(!prefix.disabled);
    assert_eq!(source_ids(&font), [EXISTING_ID]);
    assert_eq!(font.features.to_fea(), SOURCE);
    assert_eq!(
        check_horizontal_editing(&font),
        Err(Block::SourceOwnedKerning)
    );

    fcf::write(&path, &font).unwrap();
    assert_eq!(fcf::read(&path).unwrap(), font);
}

#[test]
fn idless_legacy_source_gets_a_stable_identity_when_migrated() {
    let dir = temp_dir("feature-source-idless-legacy");
    let path = dir.join("idless.fcf");
    let font = read_source_document(&path, json!({"features": [{"tag": "fea", "code": SOURCE}]}));

    assert!(font.features.features.is_empty());
    assert_eq!(font.features.prefixes.len(), 1);
    assert_eq!(font.features.prefixes[0].code, SOURCE);
    let ids = source_ids(&font);
    assert_eq!(ids.len(), 1);
    assert!(!ids[0].trim().is_empty());
    assert_eq!(
        check_horizontal_editing(&font),
        Err(Block::SourceOwnedKerning)
    );

    fcf::write(&path, &font).unwrap();
    assert_eq!(fcf::read(&path).unwrap(), font);
}

#[test]
fn legacy_disabled_source_stays_disabled_after_native_reopen() {
    let dir = temp_dir("feature-source-disabled");
    let path = dir.join("disabled.fcf");
    let font = read_source_document(&path, legacy_features(true));

    assert!(font.features.features.is_empty());
    assert_eq!(font.features.prefixes.len(), 1);
    assert!(font.features.prefixes[0].disabled);
    assert_eq!(font.features.prefixes[0].code, SOURCE);
    assert_eq!(source_ids(&font), [EXISTING_ID]);
    assert!(font.features.to_fea().is_empty());
    assert_eq!(check_horizontal_editing(&font), Ok(()));

    fcf::write(&path, &font).unwrap();
    assert_eq!(fcf::read(&path).unwrap(), font);
}

#[test]
fn only_the_exact_historical_synthetic_feature_shape_is_migrated() {
    let dir = temp_dir("feature-source-migration-guards");
    let mut cases = Vec::new();
    let mut with_prefix = legacy_features(false);
    with_prefix["prefixes"] = json!([{"tag": "existing", "code": "# prefix"}]);
    cases.push(with_prefix);
    let mut with_class = legacy_features(false);
    with_class["classes"] = json!([{"name": "Pair", "code": "base composite"}]);
    cases.push(with_class);
    let mut with_other_feature = legacy_features(false);
    with_other_feature["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"tag": "liga", "code": "sub base base by composite;"}));
    cases.push(with_other_feature);
    let mut automatic = legacy_features(false);
    automatic["features"][0]["automatic"] = json!(true);
    cases.push(automatic);
    let mut notes = legacy_features(false);
    notes["features"][0]["notes"] = json!("User-authored block");
    cases.push(notes);
    let mut ordinary_tag = legacy_features(false);
    ordinary_tag["features"][0]["tag"] = json!("kern");
    cases.push(ordinary_tag);

    for (index, source) in cases.into_iter().enumerate() {
        let font = read_source_document(&dir.join(format!("guard-{index}.fcf")), source.clone());
        assert_eq!(
            font.features.features.len(),
            source["features"].as_array().unwrap().len()
        );
        assert_eq!(
            font.features.prefixes.len(),
            source["prefixes"].as_array().unwrap().len()
        );
        let stored = serde_json::to_value(&font.features).unwrap();
        assert_eq!(stored["features"][0], source["features"][0]);
    }
}

#[test]
fn imported_table_authority_and_legacy_source_are_unchanged_on_reopen() {
    let dir = temp_dir("feature-source-imported");
    let path = dir.join("imported.fcf");
    let mut source = legacy_features(false);
    source["source_of_truth"] = json!("ImportedTables");
    let font = read_source_document(&path, source.clone());

    assert_eq!(
        font.features.source_of_truth,
        FeaturesSource::ImportedTables
    );
    assert!(font.features.prefixes.is_empty());
    assert_eq!(font.features.features.len(), 1);
    assert_eq!(serde_json::to_value(&font.features).unwrap(), source);
    assert!(font.features.to_fea().is_empty());
    assert_eq!(check_horizontal_editing(&font), Err(Block::ImportedLayout));

    fcf::write(&path, &font).unwrap();
    assert_eq!(fcf::read(&path).unwrap(), font);
}

#[test]
fn native_load_normalizes_missing_blank_and_duplicate_ids_and_keeps_them_on_save() {
    let dir = temp_dir("feature-source-identities");
    let path = dir.join("identities.fcf");
    let source = json!({
        "prefixes": [
            {"id": EXISTING_ID, "tag": "languages", "code": "languagesystem DFLT dflt;"},
            {"tag": "comment", "code": "# 中文 🪶\r\n"}
        ],
        "classes": [
            {"id": "", "name": "Pair", "code": "base composite"},
            {"id": EXISTING_ID, "name": "DuplicateId", "code": "base"}
        ],
        "features": [
            {"tag": "liga", "code": "sub base base by composite;"},
            {"tag": "liga", "code": "sub composite composite by base;", "disabled": true}
        ]
    });
    let mut font = read_source_document(&path, source);
    let ids = source_ids(&font);
    assert_eq!(ids.len(), 6);
    assert!(ids.iter().all(|id| !id.trim().is_empty()));
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    assert!(ids.iter().any(|id| id == EXISTING_ID));

    fcf::write(&path, &font).unwrap();
    assert_eq!(fcf::read(&path).unwrap(), font);

    // A repeated feature tag identifies neither a source item nor its position.
    // Reordering must carry each existing source ID with its corresponding code.
    font.features.prefixes.reverse();
    font.features.classes.reverse();
    font.features.features.reverse();
    let reordered_ids = source_ids(&font);
    assert_ne!(reordered_ids, ids);
    assert!(font.features.features[0].disabled);
    fcf::write(&path, &font).unwrap();
    let reopened = fcf::read(&path).unwrap();
    assert_eq!(reopened, font);
    assert_eq!(source_ids(&reopened), reordered_ids);
}
