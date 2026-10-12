use fc_doc::{kerning::resolve_stored_horizontal, Error};
use fc_model::{Font, Glyph, GlyphName, KernPair, KernSide, Master, MasterId};

fn font_with_groups() -> Font {
    let mut font = Font::new_empty("Kerning test", 1000);
    for name in ["A", "V", "X"] {
        font.insert_glyph(Glyph::new(name)).unwrap();
    }
    font.kerning_groups
        .side1
        .insert("public.kern1.A".into(), [GlyphName::from("A")].into());
    font.kerning_groups
        .side2
        .insert("public.kern2.V".into(), [GlyphName::from("V")].into());
    font
}

fn glyph(name: &str) -> KernSide {
    KernSide::Glyph(name.into())
}

fn group(name: &str) -> KernSide {
    KernSide::Group(name.into())
}

fn resolve(font: &Font, left: &str, right: &str) -> Option<KernPair> {
    resolve_stored_horizontal(font, &left.into(), &right.into(), &font.masters[0].id).unwrap()
}

#[test]
fn zero_exception_and_removal_follow_ufo_priority_and_report_the_winning_pair() {
    let mut font = font_with_groups();
    let candidates = [
        (glyph("A"), glyph("V"), 0.0),
        (glyph("A"), group("public.kern2.V"), -60.0),
        (group("public.kern1.A"), glyph("V"), -40.0),
        (group("public.kern1.A"), group("public.kern2.V"), -100.0),
    ];
    for (left, right, value) in &candidates {
        font.masters[0]
            .kerning_h
            .0
            .insert((left.clone(), right.clone()), *value);
    }
    // Removing each winner must expose the next exception, not add values.
    for (left, right, value) in candidates {
        assert_eq!(
            resolve(&font, "A", "V"),
            Some(KernPair {
                left: left.clone(),
                right: right.clone(),
                value,
            })
        );
        font.masters[0].kerning_h.0.remove(&(left, right));
    }
    assert_eq!(resolve(&font, "A", "V"), None);
}

#[test]
fn mixed_pairs_work_when_the_other_glyph_has_no_group() {
    let mut font = font_with_groups();
    font.masters[0]
        .kerning_h
        .0
        .insert((glyph("X"), group("public.kern2.V")), 12.5);
    font.masters[0]
        .kerning_h
        .0
        .insert((group("public.kern1.A"), glyph("X")), -25.0);
    assert_eq!(resolve(&font, "X", "V").unwrap().value, 12.5);
    assert_eq!(resolve(&font, "A", "X").unwrap().value, -25.0);
    assert_eq!(resolve(&font, "X", "X"), None);
}

#[test]
fn values_are_master_specific_horizontal_and_ordered() {
    let mut font = font_with_groups();
    let pair = (glyph("A"), glyph("V"));
    font.masters[0].kerning_h.0.insert(pair.clone(), -80.0);
    font.masters[0].kerning_v.0.insert(pair.clone(), -300.0);
    let mut other = Master::default();
    other.kerning_h.0.insert(pair, -120.0);
    let other_id = other.id.clone();
    font.masters.push(other);
    assert_eq!(resolve(&font, "A", "V").unwrap().value, -80.0);
    assert_eq!(resolve(&font, "V", "A"), None);
    assert_eq!(
        resolve_stored_horizontal(&font, &"A".into(), &"V".into(), &other_id)
            .unwrap()
            .unwrap()
            .value,
        -120.0
    );
    font.masters[0].kerning_h.0.clear();
    assert_eq!(resolve(&font, "A", "V"), None);
}

#[test]
fn authoritative_groups_override_stale_glyph_membership_fields() {
    let mut font = font_with_groups();
    font.glyph_mut("A").unwrap().kern_group_side1 = Some("public.kern1.stale".into());
    font.glyph_mut("V").unwrap().kern_group_side2 = Some("public.kern2.stale".into());
    font.masters[0].kerning_h.0.insert(
        (group("public.kern1.stale"), group("public.kern2.stale")),
        -999.0,
    );
    font.masters[0]
        .kerning_h
        .0
        .insert((group("public.kern1.A"), group("public.kern2.V")), -100.0);
    assert_eq!(resolve(&font, "A", "V").unwrap().value, -100.0);
    font.kerning_groups.side1.clear();
    font.kerning_groups.side2.clear();
    assert_eq!(resolve(&font, "A", "V"), None);
}

#[test]
fn missing_glyphs_and_master_are_errors_even_with_stored_pairs() {
    let mut font = font_with_groups();
    font.masters[0]
        .kerning_h
        .0
        .insert((glyph("missing"), glyph("V")), -80.0);
    for (left, right) in [("missing", "V"), ("A", "missing")] {
        assert!(matches!(
            resolve_stored_horizontal(&font, &left.into(), &right.into(), &font.masters[0].id),
            Err(Error::MissingGlyph(name)) if name.as_ref() == "missing"
        ));
    }
    let missing = MasterId::new();
    assert!(matches!(
        resolve_stored_horizontal(&font, &"A".into(), &"V".into(), &missing),
        Err(Error::MissingMaster(id)) if id == missing
    ));
}

#[test]
fn ambiguous_membership_is_rejected_on_either_side() {
    for side1 in [true, false] {
        let mut font = font_with_groups();
        if side1 {
            font.kerning_groups.side1.insert(
                "public.kern1.duplicate".into(),
                [GlyphName::from("A")].into(),
            );
        } else {
            font.kerning_groups.side2.insert(
                "public.kern2.duplicate".into(),
                [GlyphName::from("V")].into(),
            );
        }
        assert!(matches!(
            resolve_stored_horizontal(&font, &"A".into(), &"V".into(), &font.masters[0].id),
            Err(Error::InvalidCommand(_))
        ));
    }
}

#[test]
fn relevant_groups_must_have_the_correct_side_prefix_and_nonempty_name() {
    for (side1, name) in [
        (true, "public.kern2.wrong"),
        (false, "public.kern1.wrong"),
        (true, "unprefixed"),
        (false, "public.kern2."),
        (true, "public.kern1.   "),
        (false, "public.kern2.V\n"),
    ] {
        let mut font = font_with_groups();
        let (groups, member) = if side1 {
            (&mut font.kerning_groups.side1, "A")
        } else {
            (&mut font.kerning_groups.side2, "V")
        };
        groups.clear();
        groups.insert(name.into(), [GlyphName::from(member)].into());
        assert!(matches!(
            resolve_stored_horizontal(&font, &"A".into(), &"V".into(), &font.masters[0].id),
            Err(Error::InvalidCommand(_))
        ));
    }
}

#[test]
fn unresolved_members_in_winning_group_are_reported_but_explicit_pairs_still_work() {
    let mut font = font_with_groups();
    font.kerning_groups
        .side1
        .get_mut("public.kern1.A")
        .unwrap()
        .insert("missing".into());
    font.masters[0]
        .kerning_h
        .0
        .insert((group("public.kern1.A"), glyph("V")), -100.0);
    assert!(
        resolve_stored_horizontal(&font, &"A".into(), &"V".into(), &font.masters[0].id).is_err()
    );
    font.masters[0]
        .kerning_h
        .0
        .insert((glyph("A"), glyph("V")), -80.0);
    assert_eq!(resolve(&font, "A", "V").unwrap().value, -80.0);
}

#[test]
fn nonfinite_winning_values_are_errors_instead_of_falling_back() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut font = font_with_groups();
        font.masters[0]
            .kerning_h
            .0
            .insert((glyph("A"), glyph("V")), value);
        font.masters[0]
            .kerning_h
            .0
            .insert((group("public.kern1.A"), group("public.kern2.V")), -100.0);
        assert!(matches!(
            resolve_stored_horizontal(&font, &"A".into(), &"V".into(), &font.masters[0].id),
            Err(Error::InvalidCommand(_))
        ));
    }
}
