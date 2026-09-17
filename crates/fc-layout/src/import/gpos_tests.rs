use super::*;

fn words(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

fn names() -> Vec<GlyphName> {
    [".notdef", "A", "V", "acute", "ligature", "grave"]
        .into_iter()
        .map(GlyphName::from)
        .collect()
}

#[test]
fn single_formats_preserve_all_four_dimensions_and_explicit_zero() {
    let data = words(&[1, 14, 15, 12, (-7_i16) as u16, 24, 31, 1, 1, 1]);
    let rules = parse(Reader::new(&data), 1, &names(), "L0S0").unwrap();
    assert_eq!(rules.statements, ["pos \\A <12 -7 24 31>;"]);
    let data = words(&[2, 12, 4, 2, 0, (-80_i16) as u16, 1, 2, 1, 2]);
    let rules = parse(Reader::new(&data), 1, &names(), "L0S0").unwrap();
    assert_eq!(
        rules.statements,
        ["pos \\A <0 0 0 0>;", "pos \\V <0 0 -80 0>;"]
    );
}

#[test]
fn pair_one_keeps_encoded_zero_second_value() {
    let data = words(&[1, 20, 4, 4, 1, 12, 1, 2, (-80_i16) as u16, 0, 1, 1, 1]);
    let rules = parse(Reader::new(&data), 2, &names(), "L0S0").unwrap();
    assert_eq!(rules.statements, ["pos \\A <0 0 -80 0> \\V <0 0 0 0>;"]);
    let data = words(&[1, 18, 4, 0, 1, 12, 1, 2, (-80_i16) as u16, 1, 1, 1]);
    let rules = parse(Reader::new(&data), 2, &names(), "L0S0").unwrap();
    assert_eq!(rules.statements, ["pos \\A <0 0 -80 0> \\V;"]);
}

#[test]
fn pair_two_class_zero_is_the_complement_and_zero_cells_are_retained() {
    // Coverage A,V; class 1 A, class 2 V; all remaining glyphs are class zero.
    let data = words(&[
        2,
        24,
        4,
        0,
        32,
        40,
        2,
        2,
        0,
        0,
        (-20_i16) as u16,
        (-80_i16) as u16,
        1,
        2,
        1,
        2,
        1,
        1,
        1,
        1,
        1,
        2,
        1,
        1,
    ]);
    let rules = parse(Reader::new(&data), 2, &names(), "L0S0").unwrap();
    assert_eq!(rules.statements.len(), 4);
    assert!(rules
        .statements
        .contains(&"pos [\\V] <0 0 0 0> [\\.notdef \\A \\acute \\ligature \\grave];".into()));
    assert!(rules
        .statements
        .contains(&"pos [\\A] <0 0 -80 0> [\\V];".into()));
    assert!(rules
        .statements
        .iter()
        .all(|rule| !rule.starts_with("pos [\\.notdef")));
}

#[test]
fn class_pairs_also_preserve_encoded_zero_second_records() {
    let data = words(&[
        2,
        32,
        4,
        4,
        40,
        48,
        2,
        2,
        0,
        0,
        0,
        0,
        (-20_i16) as u16,
        0,
        (-80_i16) as u16,
        0,
        1,
        2,
        1,
        2,
        1,
        1,
        1,
        1,
        1,
        2,
        1,
        1,
    ]);
    let rules = parse(Reader::new(&data), 2, &names(), "L").unwrap();
    assert_eq!(rules.statements.len(), 4);
    assert!(rules
        .statements
        .iter()
        .all(|rule| rule.ends_with(" <0 0 0 0>;")));
}

fn pair_class_padding_fixtures() -> [(Vec<u8>, [usize; 2]); 2] {
    [
        // Coverage A is wholly in first class 1, leaving mandatory class 0
        // empty. The first two cells are zero padding, including record two.
        (
            words(&[
                2,
                32,
                4,
                4,
                38,
                46,
                2,
                2,
                0,
                0,
                0,
                0,
                (-20_i16) as u16,
                0,
                (-80_i16) as u16,
                0,
                1,
                1,
                1,
                1,
                1,
                1,
                1,
                1,
                2,
                1,
                1,
            ]),
            [16, 18],
        ),
        // One first class, three second classes; second class 2 has no glyphs.
        (
            words(&[
                2,
                28,
                4,
                4,
                34,
                40,
                1,
                3,
                (-20_i16) as u16,
                0,
                (-80_i16) as u16,
                0,
                0,
                0,
                1,
                1,
                1,
                1,
                0,
                0,
                1,
                2,
                1,
                1,
            ]),
            [24, 26],
        ),
    ]
}

#[test]
fn unreachable_class_pair_zero_padding_is_accepted() {
    for (data, _) in pair_class_padding_fixtures() {
        let rules = parse(Reader::new(&data), 2, &names(), "L").unwrap();
        assert_eq!(rules.statements.len(), 2);
        assert!(rules
            .statements
            .iter()
            .all(|rule| rule.starts_with("pos [\\A]")));
        assert!(rules
            .statements
            .iter()
            .all(|rule| rule.ends_with(" <0 0 0 0>;")));
    }
}

#[test]
fn unreachable_class_pair_nonzero_values_are_rejected_in_either_record() {
    for (data, fields) in pair_class_padding_fixtures() {
        for field in fields {
            let mut nonzero = data.clone();
            nonzero[field..field + 2].copy_from_slice(&(-7_i16).to_be_bytes());
            let error = parse(Reader::new(&nonzero), 2, &names(), "L").unwrap_err();
            assert!(
                error.contains("unreachable nonzero pair class value"),
                "{error}"
            );
        }
    }
}

#[test]
fn invalid_value_formats_and_bad_counts_are_rejected() {
    for format in [0x10, 0x80, 0x100, 0x8000] {
        let data = words(&[1, 8, format, 0, 1, 1, 1]);
        assert!(parse(Reader::new(&data), 1, &names(), "L").is_err());
    }
    let data = words(&[2, 10, 4, 2, 0, 1, 1, 1]);
    assert!(parse(Reader::new(&data), 1, &names(), "L").is_err());
    let data = words(&[
        2, 16, 0, 0, 22, 30, 65535, 65535, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0,
    ]);
    assert!(parse(Reader::new(&data), 2, &names(), "L").is_err());
}

#[test]
fn cursive_preserves_null_entry_and_rejects_point_anchors() {
    let data = words(&[1, 14, 1, 0, 10, 1, 80, 9, 1, 1, 1]);
    // Anchor extends through byte 15, so coverage starts at 16.
    let mut data = data;
    data[2..4].copy_from_slice(&16_u16.to_be_bytes());
    let rules = parse(Reader::new(&data), 3, &names(), "L").unwrap();
    assert_eq!(
        rules.statements,
        ["pos cursive \\A <anchor NULL> <anchor 80 9>;"]
    );
    data[10..12].copy_from_slice(&2_u16.to_be_bytes());
    assert!(parse(Reader::new(&data), 3, &names(), "L").is_err());
}

#[test]
fn mark_attachments_use_scoped_classes_and_checked_relative_offsets() {
    let data = words(&[
        1, 12, 18, 1, 24, 36, 1, 1, 3, 1, 1, 1, 1, 0, 6, 1, 10, 20, 1, 4, 1, 200, 700,
    ]);
    for (kind, command) in [(4, "base"), (6, "mark")] {
        let rules = parse(Reader::new(&data), kind, &names(), "L2S1").unwrap();
        assert_eq!(
            rules.preamble,
            ["markClass \\acute <anchor 10 20> @fc_L2S1_mark_00000;"]
        );
        assert_eq!(
            rules.statements,
            [format!(
                "pos {command} \\A <anchor 200 700> mark @fc_L2S1_mark_00000;"
            )]
        );
        assert_eq!(rules.glyphs, [vec![1, 3]]);
    }
    for field in [24, 28, 30, 36, 38] {
        let mut invalid = data.clone();
        invalid[field..field + 2].copy_from_slice(&65535_u16.to_be_bytes());
        assert!(parse(Reader::new(&invalid), 4, &names(), "L").is_err());
    }
    let mut all_null = data;
    all_null[38..40].copy_from_slice(&0_u16.to_be_bytes());
    assert!(parse(Reader::new(&all_null), 4, &names(), "L").is_err());
    assert!(parse(Reader::new(&all_null), 6, &names(), "L").is_err());
}

#[test]
fn anchor_format_three_accepts_only_null_device_offsets() {
    let mut data = words(&[1, 20, 1, 0, 10, 3, 80, 9, 0, 0, 1, 1, 1]);
    assert!(parse(Reader::new(&data), 3, &names(), "L").is_ok());
    data[16..18].copy_from_slice(&4_u16.to_be_bytes());
    assert!(parse(Reader::new(&data), 3, &names(), "L").is_err());
}

fn fixture(source: &str) -> fc_model::Font {
    use fc_model::{FeatureBlock, Font, Glyph, Layer};
    let mut font = Font::new_empty("GPOS roundtrip", 1000);
    let master = font.default_master().id.clone();
    for (name, unicode) in [
        (".notdef", None),
        ("A", Some(0x41)),
        ("V", Some(0x56)),
        ("acute", Some(0x301)),
        ("ligature", Some(0xe000)),
        ("grave", Some(0x300)),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = if name == "acute" || name == "grave" {
            0.0
        } else {
            600.0
        };
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.features.prefixes.push(FeatureBlock {
        code: source.into(),
        ..Default::default()
    });
    font
}

fn compile(font: &fc_model::Font) -> fc_compile::StaticCompile {
    fc_compile::compile_static_with_metadata(font, &font.default_master().id, &Default::default())
        .unwrap()
}

fn first_lookup(data: &[u8]) -> Reader<'_> {
    let table = Reader::new(data);
    let list = table.offset16(8).unwrap();
    assert_eq!(list.u16(0).unwrap(), 1);
    list.offset16(2).unwrap()
}

fn roundtrip_source(declarations: &str, statements: &str, kind: u16) {
    use read_fonts::{types::Tag, FontRef};
    let source = format!("{declarations}\nlookup Original {{ {statements} }} Original; feature kern {{ lookup Original; }} kern;");
    let before = fixture(&source);
    let compiled = compile(&before);
    let imported = crate::LayoutImport::from_bytes(&compiled.bytes, &compiled.glyph_order).unwrap();
    assert!(
        imported.can_convert(),
        "kind {kind}: {:?}",
        imported.blockers
    );
    let complete_source = imported.decompile().unwrap();
    let complete_candidate = compile(&fixture(&complete_source));
    crate::equivalent_layout(
        &imported,
        &complete_candidate.bytes,
        &complete_candidate.glyph_order,
    )
    .unwrap_or_else(|error| panic!("kind {kind}: {error}\n{complete_source}"));
    let face = FontRef::new(&compiled.bytes).unwrap();
    let data = face.table_data(Tag::new(b"GPOS")).unwrap();
    let lookup = first_lookup(data.as_bytes());
    assert_eq!(lookup.u16(0).unwrap(), kind);
    let mut preamble = Vec::new();
    let mut body = Vec::new();
    for index in 0..lookup.u16(4).unwrap() as usize {
        let subtable = lookup.offset16(6 + index * 2).unwrap();
        let rules = parse(
            subtable,
            kind,
            &compiled.glyph_order,
            &format!("L0S{index}"),
        )
        .unwrap();
        preamble.extend(rules.preamble);
        if index != 0 {
            body.push("subtable;".into());
        }
        body.extend(rules.statements);
    }
    let converted_source = format!(
        "{}\nlookup Converted {{ {} }} Converted; feature kern {{ lookup Converted; }} kern;",
        preamble.join("\n"),
        body.join("\n")
    );
    let after = fixture(&converted_source);
    let converted = compile(&after);
    let face = FontRef::new(&converted.bytes).unwrap();
    let data = face.table_data(Tag::new(b"GPOS")).unwrap();
    assert_eq!(first_lookup(data.as_bytes()).u16(0).unwrap(), kind);
    let shaper = crate::Shaper::new();
    for direction in ["ltr", "rtl", "ttb"] {
        for text in ["AVA", "AVAV", "A\u{301}\u{300}", "\u{e000}\u{301}\u{300}"] {
            let request = crate::ShapeRequest {
                text: text.into(),
                direction: Some(direction.into()),
                features: vec![("kern".into(), true)],
                ..Default::default()
            };
            let original = shaper
                .shape(
                    &before,
                    &crate::FontKey {
                        doc: 901,
                        master: before.default_master().id.clone(),
                    },
                    1,
                    &request,
                )
                .unwrap();
            let result = shaper
                .shape(
                    &after,
                    &crate::FontKey {
                        doc: 902,
                        master: after.default_master().id.clone(),
                    },
                    1,
                    &request,
                )
                .unwrap();
            assert_eq!(
                original.names, result.names,
                "kind {kind}, {direction}, {text}"
            );
            assert_eq!(
                original.glyphs, result.glyphs,
                "kind {kind}, {direction}, {text}\n{converted_source}"
            );
        }
    }
}

#[test]
fn compiler_roundtrip_single_and_pair_preserves_full_shaping() {
    roundtrip_source("", "pos [A V] <-3 7 40 -2>;", 1);
    roundtrip_source("", "pos A <-3 7 40 -2>; pos V <0 0 0 0>;", 1);
    roundtrip_source("", "pos A <0 0 -80 0> V <0 0 0 0>; pos V A -40;", 2);
    roundtrip_source("", "pos A <7 -3 -80 0> V <-4 11 20 6>;", 2);
    roundtrip_source(
        "",
        "pos [A V] [.notdef acute grave ligature] -20; pos [A V] [A V] -60;",
        2,
    );
    roundtrip_source(
        "",
        "pos [A V] <0 0 -20 0> [.notdef acute grave ligature] <0 0 0 0>; pos [A V] <0 0 -60 0> [A V] <0 0 0 0>;",
        2,
    );
    roundtrip_source("", "pos [A V] <0 0 -40 0> [A V] <0 0 0 0>;", 2);
}

#[test]
fn compiler_roundtrip_cursive_and_all_mark_attachments_preserves_full_shaping() {
    roundtrip_source(
        "",
        "pos cursive A <anchor NULL> <anchor 500 20>; pos cursive V <anchor 30 40> <anchor NULL>;",
        3,
    );
    let declarations =
        "markClass acute <anchor 10 50> @top; markClass grave <anchor 30 60> @bottom;";
    roundtrip_source(
        declarations,
        "pos base A <anchor 200 700> mark @top; pos base V <anchor 300 -200> mark @bottom;",
        4,
    );
    roundtrip_source(
        declarations,
        "pos ligature ligature <anchor NULL> ligComponent <anchor 400 700> mark @top <anchor 100 -200> mark @bottom;",
        5,
    );
    roundtrip_source(
        "markClass acute <anchor 10 50> @top;",
        "pos mark grave <anchor 80 200> mark @top;",
        6,
    );
}

#[test]
fn generated_mark_class_names_preserve_numeric_order_beyond_ten_classes() {
    use fc_model::{Glyph, Layer};
    let mut font = fixture("");
    let mut source = String::new();
    let mut anchors = String::new();
    for index in 0..12 {
        let name = format!("mark_{index}");
        let mut glyph = Glyph::new(name.clone());
        glyph
            .layers
            .push(Layer::new_master(font.default_master().id.clone()));
        font.insert_glyph(glyph).unwrap();
        source.push_str(&format!(
            "markClass {name} <anchor {index} 100> @class_{index};\n"
        ));
        anchors.push_str(&format!(
            "<anchor {} 700> mark @class_{index} ",
            200 + index
        ));
    }
    source.push_str(&format!("feature mark {{ pos base A {anchors}; }} mark;"));
    font.features.prefixes[0].code = source;
    let original = compile(&font);
    let imported = crate::LayoutImport::from_bytes(&original.bytes, &original.glyph_order).unwrap();
    font.features.prefixes[0].code = imported.decompile().unwrap();
    let candidate = compile(&font);
    crate::equivalent_layout(&imported, &candidate.bytes, &candidate.glyph_order).unwrap();
}
