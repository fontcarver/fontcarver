use fc_layout::{conversion::lifting::lift_candidate, FontKey, LayoutImport, ShapeRequest, Shaper};
use fc_model::{Anchor, FeatureBlock, Features, Font, Glyph, KernSide};

fn font() -> Font {
    let mut font = Font::new_empty("Lossless lifting", 1000);
    for (name, unicode) in [
        (".notdef", None),
        ("A", Some(65)),
        ("V", Some(86)),
        ("acutecomb", Some(0x301)),
        ("gravecomb", Some(0x300)),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        font.insert_glyph(glyph).unwrap();
    }
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].width = if glyph.name.ends_with("comb") {
            0.0
        } else {
            600.0
        };
    }
    font
}

fn imported(font: &Font) -> (Font, Features) {
    let bytes = fc_compile::export(font, &|_| 0, &Default::default())
        .unwrap()
        .bytes;
    let font = fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let code = LayoutImport::from_font(&font).decompile().unwrap();
    (
        font,
        Features {
            prefixes: vec![FeatureBlock {
                tag: "Imported".into(),
                code,
                ..Default::default()
            }],
            ..Default::default()
        },
    )
}

#[test]
fn compiler_produced_glyph_kerning_lifts_atomically() {
    let mut source = font();
    source.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
        -80.0,
    );
    let (original, features) = imported(&source);
    let saved = original.clone();
    let result = lift_candidate(&original, &features);
    assert!(result.kerning_lifted, "{:?}", result.report);
    assert!(result.original_kerning.is_some());
    assert!(result.new_kerning.is_some());
    assert!(result.candidate.feature_settings.writer_policy.kerning);
    assert_eq!(original, saved);
    assert_eq!(
        result.candidate.masters[0].kerning_h,
        source.masters[0].kerning_h
    );
}

#[test]
fn combined_kern_mark_and_mkmk_transfer_preserves_compiler_lookup_order() {
    let mut source = font();
    source.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
        -80.0,
    );
    source.glyph_mut("A").unwrap().layers[0].anchors = vec![Anchor {
        name: "top".into(),
        x: 300.0,
        y: 700.0,
    }];
    for name in ["acutecomb", "gravecomb"] {
        source.glyph_mut(name).unwrap().layers[0].anchors = vec![
            Anchor {
                name: "_top".into(),
                x: 0.0,
                y: 0.0,
            },
            Anchor {
                name: "top".into(),
                x: 50.0,
                y: 200.0,
            },
        ];
    }
    let (original, features) = imported(&source);
    let saved = original.clone();
    let result = lift_candidate(&original, &features);
    assert!(
        result.kerning_lifted && result.anchors_lifted,
        "{:?}",
        result.report
    );
    assert!(result.original_kerning.is_some() && result.new_kerning.is_some());
    assert_eq!(result.original_anchors.len(), 3);
    assert_eq!(result.new_anchors.len(), 3);
    assert_eq!(
        result.candidate.masters[0].kerning_h,
        source.masters[0].kerning_h
    );
    assert_eq!(original, saved);
}

#[test]
fn rejected_joint_trial_keeps_independently_proven_anchor_transfer() {
    let mut source = font();
    source.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "lookupflag IgnoreBaseGlyphs; pos A V -80;".into(),
        ..Default::default()
    });
    source.glyph_mut("A").unwrap().layers[0].anchors = vec![Anchor {
        name: "top".into(),
        x: 300.0,
        y: 700.0,
    }];
    source.glyph_mut("acutecomb").unwrap().layers[0].anchors = vec![Anchor {
        name: "_top".into(),
        x: 0.0,
        y: 0.0,
    }];
    let (original, features) = imported(&source);
    let result = lift_candidate(&original, &features);
    assert!(!result.kerning_lifted, "{:?}", result.report);
    assert!(result.anchors_lifted, "{:?}", result.report);
    assert!(result.original_kerning.is_none() && result.new_kerning.is_none());
    assert_eq!(result.original_anchors.len(), 2);
    assert_eq!(result.new_anchors.len(), 2);
    assert_eq!(
        result.candidate.masters[0].kerning_h,
        original.masters[0].kerning_h
    );
}

#[test]
fn different_abvm_rules_cannot_borrow_a_mark_lookup_lifting_proof() {
    let mut source = font();
    source.features.prefixes.push(FeatureBlock {
        code: "markClass acutecomb <anchor 0 0> @TOP; feature abvm { pos mark gravecomb <anchor 70 250> mark @TOP; } abvm;".into(),
        ..Default::default()
    });
    source.glyph_mut("A").unwrap().layers[0].anchors = vec![Anchor {
        name: "top".into(),
        x: 300.0,
        y: 700.0,
    }];
    for name in ["acutecomb", "gravecomb"] {
        source.glyph_mut(name).unwrap().layers[0].anchors = vec![
            Anchor {
                name: "_top".into(),
                x: 0.0,
                y: 0.0,
            },
            Anchor {
                name: "top".into(),
                x: 50.0,
                y: 200.0,
            },
        ];
    }
    let (original, features) = imported(&source);
    let result = lift_candidate(&original, &features);
    assert!(!result.anchors_lifted, "{:?}", result.report);
    assert_eq!(result.candidate.features, features);
    assert!(result.original_anchors.is_empty() && result.new_anchors.is_empty());
}

#[test]
fn complete_mark_and_mkmk_lookups_lift_to_editable_anchors() {
    let mut source = font();
    source.glyph_mut("A").unwrap().layers[0]
        .anchors
        .push(Anchor {
            name: "top".into(),
            x: 300.0,
            y: 700.0,
        });
    for name in ["acutecomb", "gravecomb"] {
        source.glyph_mut(name).unwrap().layers[0].anchors = vec![
            Anchor {
                name: "_top".into(),
                x: 0.0,
                y: 0.0,
            },
            Anchor {
                name: "top".into(),
                x: 50.0,
                y: 200.0,
            },
        ];
    }
    let (original, features) = imported(&source);
    let result = lift_candidate(&original, &features);
    assert!(result.anchors_lifted, "{:?}", result.report);
    assert_eq!(result.new_anchors.len(), 3);
    assert_eq!(result.original_anchors.len(), 3);
    assert!(result
        .original_anchors
        .iter()
        .all(|item| item.anchors.is_empty()));
    assert!(result.candidate.feature_settings.writer_policy.anchors);
    for name in ["A", "acutecomb", "gravecomb"] {
        let actual = &result.candidate.glyph(name).unwrap().layers[0].anchors;
        for expected in &source.glyph(name).unwrap().layers[0].anchors {
            assert!(actual.contains(expected), "{name}: {actual:?}");
        }
    }
    let shape = |font: &Font| {
        Shaper::new()
            .shape(
                font,
                &FontKey {
                    doc: 1,
                    master: font.masters[0].id.clone(),
                },
                1,
                &ShapeRequest {
                    text: "A\u{301}\u{300}".into(),
                    ..Default::default()
                },
            )
            .unwrap()
    };
    let before = shape(&result.candidate);
    let mut edited = result.candidate;
    edited.glyph_mut("A").unwrap().layers[0]
        .anchors
        .iter_mut()
        .find(|anchor| anchor.name == "top")
        .unwrap()
        .x += 50.0;
    edited.glyph_mut("acutecomb").unwrap().layers[0]
        .anchors
        .iter_mut()
        .find(|anchor| anchor.name == "top")
        .unwrap()
        .y += 200.0;
    let after = shape(&edited);
    assert_eq!(after.names, before.names);
    assert_eq!(after.glyphs[1].x_offset - before.glyphs[1].x_offset, 50.0);
    assert_eq!(after.glyphs[2].y_offset - before.glyphs[2].y_offset, 200.0);
}

#[test]
fn incompatible_lookup_flags_keep_kerning_in_source() {
    let mut source = font();
    source.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "lookupflag IgnoreBaseGlyphs; pos A V -80;".into(),
        ..Default::default()
    });
    let (original, features) = imported(&source);
    let result = lift_candidate(&original, &features);
    assert!(!result.kerning_lifted);
    assert!(result.original_kerning.is_none());
    assert_eq!(result.candidate.features, features);
    assert!(!result.candidate.feature_settings.writer_policy.kerning);
}

#[test]
fn script_assignment_mismatch_keeps_kerning_in_source() {
    let mut source = font();
    source.features.prefixes.push(FeatureBlock {
        code: "languagesystem cyrl dflt; feature kern { script cyrl; pos A V -80; } kern;".into(),
        ..Default::default()
    });
    let (original, features) = imported(&source);
    let result = lift_candidate(&original, &features);
    assert!(!result.kerning_lifted);
    assert_eq!(result.candidate.features, features);
}

#[test]
fn conflicting_existing_anchors_keep_complete_source_lookup() {
    let mut source = font();
    source.features.prefixes.push(FeatureBlock { code: "markClass acutecomb <anchor 0 0> @TOP; feature mark { pos base A <anchor 300 700> mark @TOP; } mark;".into(), ..Default::default() });
    let (mut original, features) = imported(&source);
    original.glyph_mut("A").unwrap().layers[0]
        .anchors
        .push(Anchor {
            name: "top".into(),
            x: 20.0,
            y: 50.0,
        });
    let result = lift_candidate(&original, &features);
    assert!(!result.anchors_lifted);
    assert_eq!(result.candidate.features, features);
    assert!(result.original_anchors.is_empty());
}

#[test]
fn zero_exception_and_later_class_subtable_remain_source_owned() {
    let mut source = font();
    source.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "pos A V 0; pos [A V] [A V] -30;".into(),
        ..Default::default()
    });
    let (original, features) = imported(&source);
    let result = lift_candidate(&original, &features);
    assert!(!result.kerning_lifted, "{:?}", result.report);
    assert_eq!(result.candidate.features, features);
    assert!(result.original_kerning.is_none());
}

#[test]
fn incompatible_mark_flags_leave_anchors_in_source() {
    let mut source = font();
    source.features.prefixes.push(FeatureBlock { code: "markClass acutecomb <anchor 0 0> @TOP; feature mark { lookupflag IgnoreMarks; pos base A <anchor 300 700> mark @TOP; } mark;".into(), ..Default::default() });
    let (original, features) = imported(&source);
    let result = lift_candidate(&original, &features);
    assert!(!result.anchors_lifted);
    assert_eq!(result.candidate.features, features);
    assert!(result.new_anchors.is_empty());
}
