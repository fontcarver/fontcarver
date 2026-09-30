//! Repertoire coverage, missing members and the sidebar badges built on them.

use std::sync::Arc;

use fc_api::{Api, InventoryCoverage, MissingMembers};
use fc_doc::Session;
use fc_model::{Font, Glyph, Layer};
use serde_json::json;

fn fixture() -> (Api, serde_json::Value) {
    let mut font = Font::new_empty("Coverage", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicodes) in [
        ("A", vec![0x0041_u32]),
        ("B", vec![0x0042]),
        ("alpha", vec![0x03B1]),
        // A second encoding on one glyph: the Greek block gains a character
        // without gaining a glyph of its own.
        ("Omega", vec![0x03A9, 0x2126]),
        ("a.sc", vec![]),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = unicodes;
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).expect("insert glyph");
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    (Api::new(session), json!(doc))
}

fn coverage(api: &Api, doc: &serde_json::Value) -> InventoryCoverage {
    serde_json::from_value(
        api.call("inventory.coverage", json!({ "doc": doc }))
            .expect("coverage"),
    )
    .expect("coverage shape")
}

fn missing(
    api: &Api,
    doc: &serde_json::Value,
    kind: &str,
    value: &str,
    limit: u32,
) -> MissingMembers {
    serde_json::from_value(
        api.call(
            "inventory.missing",
            json!({"doc": doc, "kind": kind, "value": value, "limit": limit}),
        )
        .expect("missing members"),
    )
    .expect("missing shape")
}

#[test]
fn coverage_reports_encoded_characters_glyph_counts_and_repertoire_sizes() {
    let (api, doc) = fixture();
    let coverage = coverage(&api, &doc);
    assert_eq!(coverage.total_glyphs, 5);

    let latin = coverage
        .blocks
        .iter()
        .find(|row| row.value == "Basic Latin")
        .expect("Basic Latin row");
    assert_eq!((latin.present, latin.total, latin.glyphs), (2, 128, 2));

    let greek = coverage
        .blocks
        .iter()
        .find(|row| row.value == "Greek and Coptic")
        .expect("Greek row");
    // Two glyphs, but three encoded characters — one of which is in another
    // block, so only the two Greek ones count towards this row.
    assert_eq!((greek.present, greek.glyphs), (2, 2));
    assert!(greek.total > greek.present);

    let letter = coverage
        .categories
        .iter()
        .find(|row| row.value == "Letter")
        .expect("Letter row");
    // The unencoded `a.sc` is classified by its first scalar, of which it has
    // none, so it is not in any repertoire row.
    assert_eq!(letter.glyphs, 4);
    assert_eq!(letter.present, 5);
    assert!(letter.total > 10_000);

    let uppercase = coverage
        .sub_categories
        .iter()
        .find(|row| row.value == "Letter/Uppercase")
        .expect("uppercase row");
    assert_eq!(uppercase.glyphs, 3);
    assert!(uppercase.total > uppercase.present);

    let latn = coverage
        .scripts
        .iter()
        .find(|row| row.value == "Latn")
        .expect("Latn row");
    assert_eq!((latn.present, latn.glyphs), (2, 2));
    assert!(coverage.scripts.iter().any(|row| row.value == "Grek"));
    // Nothing is reported for a repertoire the font does not touch at all.
    assert!(!coverage.blocks.iter().any(|row| row.value == "Cyrillic"));
}

#[test]
fn missing_members_are_ordered_bounded_and_expressed_as_a_recipe() {
    let (api, doc) = fixture();
    let answer = missing(&api, &doc, "Block", "Basic Latin", 8);
    assert_eq!(answer.total, 126);
    assert_eq!(answer.scalars, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(answer.recipe, "U+0000-U+0007");

    // Ranges, pairs and single values all read back as the Add Glyphs grammar.
    let sparse = missing(&api, &doc, "Block", "Basic Latin", 8_192);
    assert_eq!(sparse.scalars.len(), 126);
    assert!(sparse.recipe.contains("U+0000-U+0040"));
    assert!(sparse.recipe.contains("U+0043-U+007F"));
    assert!(!sparse.scalars.contains(&0x0041));

    // An unknown repertoire is empty rather than an error.
    assert_eq!(missing(&api, &doc, "Script", "Zxxx", 16).total, 0);
    assert!(missing(&api, &doc, "Category", "Nope", 16)
        .scalars
        .is_empty());
    assert!(api
        .call(
            "inventory.missing",
            json!({"doc": doc, "kind": "Block", "value": "x".repeat(129), "limit": 4})
        )
        .is_err());
}

#[test]
fn coverage_follows_an_edit_and_a_secondary_encoding() {
    let (api, doc) = fixture();
    let before = coverage(&api, &doc);
    let latin_before = before
        .blocks
        .iter()
        .find(|row| row.value == "Basic Latin")
        .expect("row")
        .present;

    api.call(
        "doc.apply",
        json!({
            "doc": doc,
            "command": {"AddGlyph": {"glyph": {"name": "C", "unicodes": [0x0043]}}},
            "label": "Add C"
        }),
    )
    .expect("add glyph");

    let after = coverage(&api, &doc);
    assert_eq!(
        after
            .blocks
            .iter()
            .find(|row| row.value == "Basic Latin")
            .expect("row")
            .present,
        latin_before + 1
    );
    assert_eq!(after.total_glyphs, 6);
    assert_eq!(
        missing(&api, &doc, "Block", "Basic Latin", 8_192).total,
        125
    );
}
