use fc_model::{Font, Glyph, Layer, TtInstructionBinding};

#[test]
fn bindings_round_trip_and_older_documents_keep_imported_semantics() {
    let mut font = Font::default();
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("A");
    let mut layer = Layer::new_master(master);
    layer.tt_instructions = Some(vec![0x00]);
    let legacy = serde_json::to_value(&layer).unwrap();
    assert!(legacy.get("tt_binding").is_none());
    let restored: Layer = serde_json::from_value(legacy).unwrap();
    assert!(restored.tt_binding.is_none());
    layer.tt_binding = Some(TtInstructionBinding {
        fingerprint: format!("tt-points-v1:{}", "a".repeat(64)),
        authored: true,
    });
    glyph.layers.push(layer);
    font.glyphs.insert(glyph.name.clone(), glyph);
    font.masters[0].tt.authored_units_per_em = Some(font.info.units_per_em);
    let encoded = serde_json::to_string(&font).unwrap();
    let restored: Font = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored, font);
}
