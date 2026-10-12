use fc_compile::{export, horizontal_kerning_warnings, shaping_fingerprint};
use fc_model::{Font, Glyph, KernSide};

fn font() -> Font {
    let mut font = Font::new_empty("Kerning validation", 1000);
    for name in [".notdef", "A", "V", "a", "v", "hidden"] {
        let mut glyph = Glyph::new(name);
        if name.len() == 1 {
            glyph.unicodes.push(name.chars().next().unwrap() as u32);
        }
        glyph.export = name != "hidden";
        font.insert_glyph(glyph).unwrap();
    }
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].width = 600.0;
    }
    font
}

fn advance(font: &Font, text: &str) -> i32 {
    let output = export(font, &|_| 0, &Default::default()).unwrap();
    shaping_fingerprint(&output.bytes, text)
        .unwrap()
        .iter()
        .map(|(_, advance, _)| *advance)
        .sum()
}

fn glyph(name: &str) -> KernSide {
    KernSide::Glyph(name.into())
}

fn group(name: &str) -> KernSide {
    KernSide::Group(name.into())
}

#[test]
fn missing_group_members_exclude_class_rules_but_keep_concrete_exceptions() {
    let mut font = font();
    font.kerning_groups.side1.insert(
        "public.kern1.A".into(),
        ["A".into(), "a".into(), "missing".into()].into(),
    );
    font.masters[0]
        .kerning_h
        .0
        .insert((group("public.kern1.A"), glyph("V")), -100.0);
    font.masters[0]
        .kerning_h
        .0
        .insert((glyph("A"), glyph("V")), -80.0);
    let original = font.clone();
    assert_eq!(advance(&font, "AV"), 1120);
    assert_eq!(advance(&font, "aV"), 1200);
    assert_eq!(
        horizontal_kerning_warnings(&font, &font.masters[0].id).len(),
        1
    );
    assert_eq!(font, original);
}

#[test]
fn ambiguous_or_malformed_membership_does_not_compile_arbitrary_spacing() {
    for malformed in [
        None,
        Some("public.kern1.   "),
        Some("public.kern1.A\n"),
        Some("public.kern2.A"),
    ] {
        let mut font = font();
        font.kerning_groups
            .side1
            .insert("public.kern1.A".into(), ["A".into()].into());
        font.kerning_groups.side1.insert(
            malformed.unwrap_or("public.kern1.Other").into(),
            ["A".into()].into(),
        );
        font.masters[0]
            .kerning_h
            .0
            .insert((group("public.kern1.A"), glyph("V")), -100.0);
        font.masters[0]
            .kerning_h
            .0
            .insert((glyph("A"), glyph("V")), -80.0);
        assert_eq!(advance(&font, "AV"), 1200);
        assert_eq!(
            horizontal_kerning_warnings(&font, &font.masters[0].id).len(),
            1
        );
    }
}

#[test]
fn non_exporting_group_members_are_filtered_without_losing_valid_pairs() {
    let mut font = font();
    font.kerning_groups.side1.insert(
        "public.kern1.A".into(),
        ["A".into(), "hidden".into()].into(),
    );
    font.kerning_groups
        .side2
        .insert("public.kern2.V".into(), ["V".into()].into());
    font.masters[0]
        .kerning_h
        .0
        .insert((group("public.kern1.A"), group("public.kern2.V")), -100.0);
    assert_eq!(advance(&font, "AV"), 1100);
    assert!(horizontal_kerning_warnings(&font, &font.masters[0].id).is_empty());
    font.masters[0]
        .kerning_h
        .0
        .insert((glyph("hidden"), glyph("V")), -80.0);
    assert_eq!(advance(&font, "AV"), 1100);
    assert_eq!(
        horizontal_kerning_warnings(&font, &font.masters[0].id).len(),
        1
    );
}

#[test]
fn empty_and_all_non_exporting_groups_do_not_break_compilation() {
    let mut font = font();
    font.kerning_groups
        .side1
        .insert("public.kern1.Empty".into(), Default::default());
    font.kerning_groups
        .side1
        .insert("public.kern1.Hidden".into(), ["hidden".into()].into());
    for name in ["public.kern1.Empty", "public.kern1.Hidden"] {
        font.masters[0]
            .kerning_h
            .0
            .insert((group(name), glyph("V")), -100.0);
    }
    assert_eq!(advance(&font, "AV"), 1200);
}
