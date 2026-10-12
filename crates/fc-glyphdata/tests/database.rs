use fc_glyphdata::{
    parse_overlay, resolve_info, validate_overlay, GlyphDataOverlay, BUNDLED_DATABASE_JSON,
};

#[test]
fn bundled_file_is_valid_versioned_and_contains_composites_and_marks() {
    let database = parse_overlay(BUNDLED_DATABASE_JSON).unwrap();
    assert_eq!(database.version, 1);
    assert!(database.glyphs.len() >= 30);
    let composite = resolve_info("Aacute", &[65], &GlyphDataOverlay::default());
    assert_eq!(composite.name, "Aacute");
    assert_eq!(composite.unicodes, [0xC1]);
    assert_eq!(composite.category.as_deref(), Some("Letter"));
    assert_eq!(composite.sub_category.as_deref(), Some("Uppercase"));
    assert_eq!(composite.script.as_deref(), Some("Latn"));
    assert_eq!(composite.anchors, ["top", "bottom"]);
    assert_eq!(composite.composition, ["A", "acutecomb"]);
    let mark = resolve_info("acutecomb", &[], &GlyphDataOverlay::default());
    assert_eq!(mark.unicodes, [0x301]);
    assert_eq!(mark.category.as_deref(), Some("Mark"));
    assert_eq!(mark.anchors, ["_top", "top"]);
}

#[test]
fn generic_ucd_and_aglfn_fallbacks_work_without_curated_entries() {
    let overlay = GlyphDataOverlay::default();
    let han = resolve_info("uni4E00", &[], &overlay);
    assert_eq!(han.unicodes, [0x4E00]);
    assert_eq!(han.script.as_deref(), Some("Hani"));
    assert_eq!(han.sub_category.as_deref(), Some("Ideograph"));
    let fallback = resolve_info("customName", &[0x1F600], &overlay);
    assert_eq!(fallback.unicodes, [0x1F600]);
    assert_eq!(fallback.category.as_deref(), Some("Symbol"));
    assert_eq!(fallback.production_name.as_deref(), Some("u1F600"));
    let unknown = resolve_info("unencodedCustom", &[], &overlay);
    assert!(unknown.unicodes.is_empty());
    assert!(unknown.category.is_none());
}

#[test]
fn partial_overlay_inherits_independent_fields_and_empty_arrays_clear() {
    let overlay = parse_overlay(
        r#"{"version":1,"glyphs":{"Aacute":{"anchors":[],"production_name":"customAcute"},"privateMark":{"unicodes":[57344],"category":"Mark","sub_category":"Nonspacing","script":"Latn","anchors":["_top"],"composition":["acutecomb"]}}}"#,
    )
    .unwrap();
    let inherited = resolve_info("Aacute", &[], &overlay);
    assert_eq!(inherited.unicodes, [0xC1]);
    assert_eq!(inherited.category.as_deref(), Some("Letter"));
    assert_eq!(inherited.composition, ["A", "acutecomb"]);
    assert!(inherited.anchors.is_empty());
    assert_eq!(inherited.production_name.as_deref(), Some("customAcute"));
    let custom = resolve_info("privateMark", &[], &overlay);
    assert_eq!(custom.unicodes, [0xE000]);
    assert_eq!(custom.category.as_deref(), Some("Mark"));
    assert_eq!(custom.anchors, ["_top"]);
    let reparsed = parse_overlay(&serde_json::to_string(&overlay).unwrap()).unwrap();
    assert_eq!(reparsed, overlay);
}

#[test]
fn changing_overlay_encoding_recomputes_ucd_fallback_fields() {
    let overlay = parse_overlay(
        r#"{"version":1,"glyphs":{"someName":{"unicodes":[769]},"A":{"unicodes":[]}}}"#,
    )
    .unwrap();
    let custom = resolve_info("someName", &[65], &overlay);
    assert_eq!(custom.category.as_deref(), Some("Mark"));
    assert_eq!(custom.sub_category.as_deref(), Some("Nonspacing"));
    assert!(resolve_info("A", &[65], &overlay).unicodes.is_empty());
}

#[test]
fn suffixes_inherit_base_metadata_without_duplicate_encodings() {
    let overlay = parse_overlay(
        r#"{"version":1,"glyphs":{"Aacute":{"anchors":["top"],"production_name":"uni00C1"},"Aacute.ss01":{"composition":[],"anchors":["top.alt"]},"Aacute.ss02":{"unicodes":[57344]}}}"#,
    )
    .unwrap();
    let suffix = resolve_info("Aacute.ss01", &[0xC1], &overlay);
    assert!(suffix.unicodes.is_empty());
    assert_eq!(suffix.category.as_deref(), Some("Letter"));
    assert_eq!(suffix.production_name.as_deref(), Some("uni00C1.ss01"));
    assert_eq!(suffix.anchors, ["top.alt"]);
    assert!(suffix.composition.is_empty());
    assert_eq!(
        resolve_info("Aacute.ss02", &[], &overlay).unicodes,
        [0xE000]
    );
    let suffix = resolve_info("uni4E00.alt.case", &[], &overlay);
    assert!(suffix.unicodes.is_empty());
    assert_eq!(suffix.script.as_deref(), Some("Hani"));
    assert_eq!(suffix.production_name.as_deref(), Some("uni4E00.alt.case"));
    assert_eq!(resolve_info(".notdef", &[], &overlay).name, ".notdef");
}

#[test]
fn validation_rejects_invalid_untrusted_documents() {
    for input in [
        "{}",
        r#"{"version":2,"glyphs":{}}"#,
        r#"{"version":1,"glyphs":{"bad name":{}}}"#,
        r#"{"version":1,"glyphs":{"A":{"unicodes":[55296]}}}"#,
        r#"{"version":1,"glyphs":{"A":{"unicodes":[1114112]}}}"#,
        r#"{"version":1,"glyphs":{"A":{"unicodes":[65,65]}}}"#,
        r#"{"version":1,"glyphs":{"A":{"category":"Bogus"}}}"#,
        r#"{"version":1,"glyphs":{"A":{"script":"Latin"}}}"#,
        r#"{"version":1,"glyphs":{"A":{"anchors":["top","top"]}}}"#,
        r#"{"version":1,"glyphs":{"A":{"composition":["bad/name"]}}}"#,
        r#"{"version":1,"glyphs":{"A":{"production_name":"bad name"}}}"#,
        r#"{"version":1,"glyphs":{"A":{},"A":{}}}"#,
    ] {
        assert!(parse_overlay(input).is_err(), "accepted {input}");
    }
}

#[test]
fn validation_bounds_documents_and_typed_overlays_and_allows_unknown_fields() {
    let overlay = GlyphDataOverlay {
        version: 2,
        ..GlyphDataOverlay::default()
    };
    assert!(validate_overlay(&overlay).is_err());
    let oversized = " ".repeat(4 * 1024 * 1024 + 1);
    assert!(parse_overlay(&oversized).is_err());
    let long_name = "x".repeat(256);
    assert!(parse_overlay(&format!(
        r#"{{"version":1,"glyphs":{{"{long_name}":{{}}}}}}"#
    ))
    .is_err());
    assert!(parse_overlay(r#"{"version":1,"future":true,"glyphs":{"A":{"future":42}}}"#).is_ok());
}

#[test]
fn collection_limits_apply_to_json_and_typed_documents() {
    for (field, entries) in [
        ("unicodes", serde_json::json!((0..65).collect::<Vec<_>>())),
        ("composition", serde_json::json!(vec!["A"; 65])),
        (
            "anchors",
            serde_json::json!((0..129)
                .map(|index| format!("top{index}"))
                .collect::<Vec<_>>()),
        ),
    ] {
        let mut document = serde_json::json!({"version": 1, "glyphs": {"A": {}}});
        document["glyphs"]["A"][field] = entries;
        let json = document.to_string();
        assert!(parse_overlay(&json).is_err(), "accepted oversized {field}");
        let typed = serde_json::from_value(document).unwrap();
        assert!(validate_overlay(&typed).is_err());
    }
}

#[test]
fn null_inherits_repeated_components_and_leading_dot_suffixes_work() {
    let overlay = parse_overlay(
        r#"{"version":1,"glyphs":{"Aacute":{"anchors":null,"category":null},"f_f_i":{"composition":["f","f","i"]},".notdef":{"category":"Other","anchors":["origin"],"production_name":"missing"}}}"#,
    )
    .unwrap();
    assert_eq!(
        resolve_info("Aacute", &[], &overlay).anchors,
        ["top", "bottom"]
    );
    assert_eq!(
        resolve_info("f_f_i", &[], &overlay).composition,
        ["f", "f", "i"]
    );
    let variant = resolve_info(".notdef.alt", &[], &overlay);
    assert_eq!(variant.anchors, ["origin"]);
    assert_eq!(variant.production_name.as_deref(), Some("missing.alt"));
    assert!(variant.unicodes.is_empty());
}

#[test]
fn fallback_encodings_are_sanitized_without_truncating_existing_assignments() {
    let overlay = GlyphDataOverlay::default();
    let info = resolve_info("custom", &[0xD800, 0x42, 0x41, 0x42, 0x110000], &overlay);
    assert_eq!(info.unicodes, [0x42, 0x41]);
    assert_eq!(info.production_name.as_deref(), Some("B"));
    let many = (0..100).collect::<Vec<_>>();
    assert_eq!(resolve_info("custom", &many, &overlay).unicodes, many);
}

#[test]
fn bundled_compositions_reference_available_glyphs() {
    let database = parse_overlay(BUNDLED_DATABASE_JSON).unwrap();
    for (name, info) in &database.glyphs {
        for component in info.composition.as_deref().unwrap_or_default() {
            assert!(
                database.glyphs.contains_key(component),
                "missing component {component} in {name}"
            );
        }
    }
}

#[test]
fn inverse_lookup_prefers_explicit_overlay_names_and_falls_back_to_ucd() {
    let overlay = parse_overlay(
        r#"{"version":1,"glyphs":{"houseName":{"unicodes":[57344],"category":"Symbol"},"houseName.alt":{"anchors":["top"]},"zAlternative":{"unicodes":[57344]}}}"#,
    )
    .unwrap();
    let info = fc_glyphdata::resolve_unicode(0xE000, &overlay).unwrap();
    assert_eq!(info.name, "houseName");
    assert_eq!(info.category.as_deref(), Some("Symbol"));
    assert_eq!(
        fc_glyphdata::resolve_unicode(0xC1, &overlay).unwrap().name,
        "Aacute"
    );
    let han = fc_glyphdata::resolve_unicode(0x4E00, &overlay).unwrap();
    assert_eq!(han.name, "uni4E00");
    assert_eq!(han.script.as_deref(), Some("Hani"));
    assert!(fc_glyphdata::resolve_unicode(0xD800, &overlay).is_none());
    assert!(fc_glyphdata::resolve_unicode(0x110000, &overlay).is_none());
}

#[test]
fn inverse_lookup_respects_cleared_encoding_and_explicit_suffix_alias() {
    let overlay = parse_overlay(r#"{"version":1,"glyphs":{"A":{"unicodes":[]}}}"#).unwrap();
    assert!(fc_glyphdata::resolve_unicode(65, &overlay).is_none());
    let overlay =
        parse_overlay(r#"{"version":1,"glyphs":{"A":{"unicodes":[]},"A.alt":{"unicodes":[65]}}}"#)
            .unwrap();
    let alias = fc_glyphdata::resolve_unicode(65, &overlay).unwrap();
    assert_eq!(alias.name, "A.alt");
    assert_eq!(alias.unicodes, [65]);
}
