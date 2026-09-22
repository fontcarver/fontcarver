use fc_model::{
    Font, Glyph, MathConstants, MathDevice, MathGlyphInfo, MathKern, MathTable, MathValue,
};

#[test]
fn validates_devices_ranges_and_kern_intervals() {
    let mut math = MathTable {
        constants: Some(MathConstants::default()),
        ..Default::default()
    };
    math.constants.as_mut().unwrap().axis_height = MathValue {
        value: 250.25,
        device: Some(MathDevice {
            start_size: 12,
            deltas: vec![-2, 1, 0],
        }),
    };
    math.validate(&[]).unwrap();
    math.constants
        .as_mut()
        .unwrap()
        .axis_height
        .device
        .as_mut()
        .unwrap()
        .start_size = u16::MAX;
    assert!(math.validate(&[]).is_err());
    math.constants.as_mut().unwrap().axis_height.device = None;
    let mut glyph = MathGlyphInfo::default();
    glyph.kern.top_right = Some(MathKern {
        heights: vec![MathValue::from(10.), MathValue::from(10.1)],
        values: vec![MathValue::default(); 3],
    });
    math.glyphs.insert("A".into(), glyph);
    assert!(
        math.validate(&["A".into()]).is_err(),
        "rounded duplicate heights must fail"
    );
    math.glyphs
        .get_mut("A")
        .unwrap()
        .kern
        .top_right
        .as_mut()
        .unwrap()
        .heights[1]
        .value = 20.;
    math.validate(&["A".into()]).unwrap();
    assert!(math.validate(&[]).is_err(), "dangling glyph reference");
}

#[test]
fn legacy_defaults_and_glyph_rename_preserve_math() {
    let mut font: Font = serde_json::from_str("{}").unwrap();
    assert!(font.math.is_none());
    assert!(!font.math_table_owned);
    font.insert_glyph(Glyph::new("A")).unwrap();
    let mut math = MathTable::default();
    math.glyphs.insert(
        "A".into(),
        MathGlyphInfo {
            extended_shape: true,
            ..Default::default()
        },
    );
    font.math = Some(math);
    font.math_table_owned = true;
    font.rename_glyph("A", "B").unwrap();
    assert!(font.math.as_ref().unwrap().glyphs.contains_key("B"));
    assert!(!font.math.as_ref().unwrap().glyphs.contains_key("A"));
    let encoded = serde_json::to_string(&font).unwrap();
    assert_eq!(font, serde_json::from_str(&encoded).unwrap());
}
