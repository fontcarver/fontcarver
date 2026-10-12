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

#[test]
fn retaining_glyphs_drops_dangling_references_without_partial_assemblies() {
    use fc_model::{MathAssembly, MathConstruction, MathPart, MathVariant};
    let part = |glyph: &str| MathPart {
        glyph: glyph.into(),
        full_advance: 100.,
        ..Default::default()
    };
    let variant = |glyph: &str| MathVariant {
        glyph: glyph.into(),
        advance: 100.,
    };
    let mut math = MathTable::default();
    math.glyphs.insert(
        "paren".into(),
        MathGlyphInfo {
            vertical: Some(MathConstruction {
                variants: vec![variant("paren"), variant("gone")],
                assembly: Some(MathAssembly {
                    parts: vec![part("top"), part("gone")],
                    ..Default::default()
                }),
            }),
            horizontal: Some(MathConstruction {
                variants: vec![variant("gone")],
                assembly: None,
            }),
            ..Default::default()
        },
    );
    math.glyphs.insert("gone".into(), MathGlyphInfo::default());
    let dropped = math.retain_glyphs(|name| name.as_ref() != "gone");
    // One record, two variants and one assembly.
    assert_eq!(dropped, 4);
    assert!(!math.glyphs.contains_key("gone"));
    let paren = &math.glyphs["paren"];
    let vertical = paren.vertical.as_ref().unwrap();
    assert_eq!(vertical.variants, vec![variant("paren")]);
    assert!(vertical.assembly.is_none());
    assert!(paren.horizontal.is_none(), "empty construction is removed");
    assert_eq!(math.retain_glyphs(|_| true), 0);
}

#[test]
fn assembly_layout_repeats_extenders_and_interpolates_overlaps() {
    use fc_model::{MathAssembly, MathPart};
    let part = |glyph: &str, extender| MathPart {
        glyph: glyph.into(),
        start_connector: 50.,
        end_connector: 50.,
        full_advance: 200.,
        extender,
    };
    let assembly = MathAssembly {
        parts: vec![part("bottom", false), part("ext", true), part("top", false)],
        ..Default::default()
    };
    // Without extenders: 400 advance minus one overlap between 10 and 50.
    let small = assembly.layout(360., 10., 100).unwrap();
    assert_eq!(small.repeats, 0);
    assert_eq!(small.parts.len(), 2);
    assert_eq!(small.size, 360.);
    assert_eq!(small.parts[1].offset, 160.);
    assert!(!small.short_connectors && !small.limited);
    // Below the minimum size, the fewest parts use the largest overlap.
    let tiny = assembly.layout(1., 10., 100).unwrap();
    assert_eq!(tiny.size, 350.);
    // One extender: 600 advance and two overlaps, between 580 and 500.
    let medium = assembly.layout(560., 10., 100).unwrap();
    assert_eq!(medium.repeats, 1);
    assert_eq!(medium.size, 560.);
    let names: Vec<_> = medium.parts.iter().map(|p| p.glyph.as_ref()).collect();
    assert_eq!(names, ["bottom", "ext", "top"]);
    // The part limit stops growth and reports it.
    let limited = assembly.layout(10_000., 10., 4).unwrap();
    assert_eq!(limited.repeats, 2);
    assert!(limited.limited);
    assert!(assembly.layout(10., 10., 1).is_none());
    // Fixed parts alone report a shortfall too.
    let fixed = MathAssembly {
        parts: vec![part("bottom", false), part("top", false)],
        ..Default::default()
    };
    let layout = fixed.layout(1_000., 10., 10).unwrap();
    assert_eq!(layout.size, 390.);
    assert!(layout.limited);
    assert!(!fixed.layout(300., 10., 10).unwrap().limited);
    assert!(MathAssembly::default().layout(10., 0., 10).is_none());
    // Connectors shorter than the minimum overlap are reported and used fully.
    let mut short = assembly.clone();
    short.parts[0].end_connector = 5.;
    let layout = short.layout(0., 10., 10).unwrap();
    assert!(layout.short_connectors);
    assert_eq!(layout.size, 395.);
    // An assembly of only extenders starts from one repeat.
    let only = MathAssembly {
        parts: vec![part("ext", true)],
        ..Default::default()
    };
    assert_eq!(only.layout(500., 0., 10).unwrap().repeats, 3);
}
