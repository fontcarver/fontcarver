#[allow(dead_code)]
mod common;

use fc_doc::{
    kerning::{
        resolve_stored_horizontal, resolve_stored_vertical, KerningMaintenanceOperation as Op,
    },
    Command, Document, Session,
};
use fc_model::{FeatureBlock, Font, Glyph, KernPair, KernSide, Kerning, Master};

fn glyph(name: &str) -> KernSide {
    KernSide::Glyph(name.into())
}

fn group(name: &str) -> KernSide {
    KernSide::Group(name.into())
}

fn record(left: KernSide, right: KernSide, value: f64) -> KernPair {
    KernPair { left, right, value }
}

fn font() -> Font {
    let mut font = Font::new_empty("Maintenance", 1000);
    for name in ["A", "Aacute", "V", "W", "X"] {
        font.insert_glyph(Glyph::new(name)).unwrap();
    }
    font.add_master(Master::default());
    font.kerning_groups.side1.insert(
        "public.kern1.A".into(),
        ["A".into(), "Aacute".into()].into(),
    );
    font.kerning_groups
        .side2
        .insert("public.kern2.V".into(), ["V".into(), "W".into()].into());
    font
}

fn with_font(font: Font, test: impl FnOnce(&mut Document)) {
    let session = Session::new();
    let id = session.open_font(font, None);
    session.with_doc_mut(id, test).unwrap();
}

fn command(doc: &Document, vertical: bool, operation: Op) -> Command {
    Command::KerningMaintenance {
        master: doc.font.masters[0].id.clone(),
        vertical,
        expected_generation: doc.generation,
        operation,
    }
}

fn table(font: &Font, vertical: bool) -> &Kerning {
    if vertical {
        &font.masters[0].kerning_v
    } else {
        &font.masters[0].kerning_h
    }
}

fn resolved(font: &Font, vertical: bool) -> Vec<Option<f64>> {
    let resolve = if vertical {
        resolve_stored_vertical
    } else {
        resolve_stored_horizontal
    };
    ["A", "Aacute", "X"]
        .into_iter()
        .flat_map(|left| {
            ["V", "W", "X"].map(|right| {
                resolve(font, &left.into(), &right.into(), &font.masters[0].id)
                    .unwrap()
                    .map(|pair| pair.value)
            })
        })
        .collect()
}

#[test]
fn compression_preserves_resolved_values_zero_exceptions_and_other_tables() {
    for vertical in [false, true] {
        let mut initial = font();
        let records = Kerning(
            [
                ((group("public.kern1.A"), group("public.kern2.V")), -80.),
                ((glyph("A"), group("public.kern2.V")), -80.),
                ((group("public.kern1.A"), glyph("W")), -80.),
                ((glyph("A"), glyph("V")), -80.),
                ((glyph("Aacute"), glyph("V")), 0.),
                ((glyph("X"), glyph("X")), 0.),
            ]
            .into(),
        );
        initial.masters[0].kerning_h = records.clone();
        initial.masters[0].kerning_v = records.clone();
        initial.masters[1].kerning_h = records;
        with_font(initial, |doc| {
            let before = doc.font.clone();
            let values = resolved(&before, vertical);
            let change = doc
                .apply(command(doc, vertical, Op::Compress), "Compress")
                .unwrap();
            assert!(change.layout);
            assert!(change.glyphs.is_empty());
            assert_eq!(table(&doc.font, vertical).0.len(), 3);
            assert_eq!(resolved(&doc.font, vertical), values);
            assert_eq!(table(&doc.font, !vertical), table(&before, !vertical));
            assert_eq!(doc.font.masters[1], before.masters[1]);
            let after = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
        });
    }
}

#[test]
fn compression_checks_every_class_member_and_both_exception_axes() {
    let mut initial = font();
    initial.masters[0].kerning_h.0 = [
        ((group("public.kern1.A"), group("public.kern2.V")), -80.),
        ((glyph("A"), group("public.kern2.V")), -40.),
        ((group("public.kern1.A"), glyph("V")), -40.),
        ((glyph("Aacute"), glyph("V")), -40.),
    ]
    .into();
    with_font(initial, |doc| {
        let before = resolved(&doc.font, false);
        doc.apply(command(doc, false, Op::Compress), "Compress")
            .unwrap();
        assert_eq!(resolved(&doc.font, false), before);
        assert_eq!(table(&doc.font, false).0.len(), 3);
        assert!(table(&doc.font, false)
            .0
            .contains_key(&(glyph("A"), group("public.kern2.V"))));
    });
}

#[test]
fn compress_retains_empty_groups_and_invalid_records_until_explicit_cleanup() {
    let mut initial = font();
    initial
        .kerning_groups
        .side2
        .insert("public.kern2.Empty".into(), Default::default());
    initial.masters[0].kerning_h.0 = [
        ((glyph("A"), group("public.kern2.Empty")), -80.),
        ((glyph("missing"), glyph("V")), -80.),
        ((glyph("X"), glyph("X")), f64::NAN),
    ]
    .into();
    with_font(initial, |doc| {
        let change = doc
            .apply(command(doc, false, Op::Compress), "Compress")
            .unwrap();
        assert_eq!(change, Default::default());
        assert_eq!(doc.undo_len(), 0);
        assert_eq!(table(&doc.font, false).0.len(), 3);
        assert!(table(&doc.font, false).0[&(glyph("X"), glyph("X"))].is_nan());
    });
}

#[test]
fn compression_respects_cross_axis_fallback_and_zero_class_roots() {
    for value in [-80., 0.] {
        let mut initial = font();
        initial.masters[0].kerning_h.0 = [
            ((group("public.kern1.A"), group("public.kern2.V")), value),
            ((glyph("A"), group("public.kern2.V")), value),
            ((group("public.kern1.A"), glyph("V")), -40.),
        ]
        .into();
        with_font(initial, |doc| {
            let before = resolved(&doc.font, false);
            doc.apply(command(doc, false, Op::Compress), "Compress")
                .unwrap();
            assert_eq!(resolved(&doc.font, false), before);
            // The glyph/class exception matches its root but masks a different
            // class/glyph adjustment for one of the class's members.
            assert_eq!(table(&doc.font, false).0.len(), 3);
        });
    }
    let mut initial = font();
    initial.masters[0].kerning_h.0 = [
        ((group("public.kern1.A"), group("public.kern2.V")), 0.),
        ((glyph("A"), glyph("V")), 0.),
    ]
    .into();
    with_font(initial, |doc| {
        doc.apply(command(doc, false, Op::Compress), "Compress")
            .unwrap();
        assert_eq!(table(&doc.font, false).0.len(), 1);
        assert_eq!(resolved(&doc.font, false)[0], Some(0.));
    });
}

#[test]
fn finite_exception_masking_an_invalid_fallback_is_retained() {
    for invalid_value in [false, true] {
        let mut initial = font();
        if !invalid_value {
            initial
                .kerning_groups
                .side1
                .get_mut("public.kern1.A")
                .unwrap()
                .insert("missing".into());
        }
        initial.masters[0].kerning_h.0 = [
            (
                (group("public.kern1.A"), group("public.kern2.V")),
                if invalid_value { f64::NAN } else { -80. },
            ),
            ((glyph("A"), glyph("V")), -80.),
        ]
        .into();
        with_font(initial, |doc| {
            doc.apply(command(doc, false, Op::Compress), "Compress")
                .unwrap();
            assert_eq!(table(&doc.font, false).0.len(), 2);
            assert_eq!(doc.undo_len(), 0);
        });
    }
}

#[test]
fn compression_finishes_masked_cross_axis_exceptions_in_one_pass() {
    let mut initial = font();
    initial
        .kerning_groups
        .side1
        .insert("public.kern1.A".into(), ["A".into()].into());
    initial.masters[0].kerning_h.0 = [
        ((group("public.kern1.A"), group("public.kern2.V")), -80.),
        ((glyph("A"), group("public.kern2.V")), -80.),
        ((group("public.kern1.A"), glyph("V")), -40.),
    ]
    .into();
    with_font(initial, |doc| {
        let before = resolved(&doc.font, false);
        doc.apply(command(doc, false, Op::Compress), "Compress")
            .unwrap();
        assert_eq!(resolved(&doc.font, false), before);
        assert_eq!(table(&doc.font, false).0.len(), 1);
        let generation = doc.generation;
        assert_eq!(
            doc.apply(command(doc, false, Op::Compress), "Again")
                .unwrap(),
            Default::default()
        );
        assert_eq!(doc.generation, generation);
    });
}

#[test]
fn cleanup_removes_invalid_dangling_and_ambiguous_records_with_lossless_undo() {
    for vertical in [false, true] {
        let mut initial = font();
        initial
            .kerning_groups
            .side1
            .insert("public.kern1.Duplicate".into(), ["A".into()].into());
        let records = Kerning(
            [
                ((glyph("Aacute"), glyph("V")), 0.),
                ((glyph("missing"), glyph("V")), -80.),
                ((group("public.kern1.Missing"), glyph("V")), -80.),
                ((glyph("Aacute"), group("public.kern1.A")), -80.),
                ((glyph("A"), glyph("V")), -80.),
                ((glyph("X"), glyph("X")), f64::NAN),
                ((glyph("X"), glyph("V")), f64::INFINITY),
            ]
            .into(),
        );
        if vertical {
            initial.masters[0].kerning_v = records;
        } else {
            initial.masters[0].kerning_h = records;
        }
        with_font(initial, |doc| {
            doc.apply(command(doc, vertical, Op::CleanupInvalid), "Cleanup")
                .unwrap();
            assert_eq!(table(&doc.font, vertical).0.len(), 1);
            assert_eq!(
                table(&doc.font, vertical).0[&(glyph("Aacute"), glyph("V"))],
                0.
            );
            doc.undo().unwrap();
            assert_eq!(table(&doc.font, vertical).0.len(), 7);
            assert!(table(&doc.font, vertical).0[&(glyph("X"), glyph("X"))].is_nan());
            doc.redo().unwrap();
            assert_eq!(table(&doc.font, vertical).0.len(), 1);
        });
    }
}

#[test]
fn paste_merge_replace_and_native_persistence() {
    for vertical in [false, true] {
        with_font(font(), |doc| {
            let root = record(group("public.kern1.A"), group("public.kern2.V"), -90.);
            let zero = record(glyph("A"), glyph("V"), 0.);
            doc.apply(
                command(
                    doc,
                    vertical,
                    Op::Paste {
                        records: vec![root.clone(), zero.clone()],
                        replace: true,
                    },
                ),
                "Paste",
            )
            .unwrap();
            let before_merge = doc.font.clone();
            let mut replacement = root;
            replacement.value = -70.;
            doc.apply(
                command(
                    doc,
                    vertical,
                    Op::Paste {
                        records: vec![replacement],
                        replace: false,
                    },
                ),
                "Merge",
            )
            .unwrap();
            assert_eq!(table(&doc.font, vertical).0.len(), 2);
            assert_eq!(table(&doc.font, vertical).0[&(glyph("A"), glyph("V"))], 0.);
            assert_eq!(resolved(&doc.font, vertical)[0], Some(0.));
            let dir = common::temp_dir("kerning-maintenance");
            let path = dir.join("maintenance.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), doc.font);
            std::fs::remove_dir_all(dir).unwrap();
            doc.undo().unwrap();
            assert_eq!(doc.font, before_merge);
            doc.redo().unwrap();
            doc.apply(
                command(
                    doc,
                    vertical,
                    Op::Paste {
                        records: vec![zero],
                        replace: true,
                    },
                ),
                "Replace",
            )
            .unwrap();
            assert_eq!(table(&doc.font, vertical).0.len(), 1);
            doc.apply(
                command(
                    doc,
                    vertical,
                    Op::Paste {
                        records: vec![],
                        replace: true,
                    },
                ),
                "Clear",
            )
            .unwrap();
            assert!(table(&doc.font, vertical).0.is_empty());
            doc.undo().unwrap();
            assert_eq!(table(&doc.font, vertical).0.len(), 1);
        });
    }
}

#[test]
fn invalid_paste_is_atomic_for_merge_and_replace() {
    for replace in [false, true] {
        with_font(font(), |doc| {
            let valid = record(glyph("A"), glyph("V"), -80.);
            doc.apply(
                command(
                    doc,
                    false,
                    Op::Paste {
                        records: vec![valid.clone()],
                        replace,
                    },
                ),
                "Seed",
            )
            .unwrap();
            let before = doc.font.clone();
            let generation = doc.generation;
            for records in [
                vec![valid.clone(), valid.clone()],
                vec![valid.clone(), record(glyph("missing"), glyph("V"), -50.)],
                vec![valid.clone(), record(glyph("X"), glyph("V"), f64::INFINITY)],
                vec![
                    valid.clone(),
                    record(group("public.kern1.Missing"), glyph("V"), -50.),
                ],
                vec![
                    valid.clone(),
                    record(glyph("A"), group("public.kern1.A"), -50.),
                ],
            ] {
                assert!(doc
                    .apply(
                        command(doc, false, Op::Paste { records, replace }),
                        "Invalid"
                    )
                    .is_err());
                assert_eq!(doc.font, before);
                assert_eq!(doc.generation, generation);
                assert_eq!(doc.undo_len(), 1);
            }
        });
    }
}

#[test]
fn stale_generation_rejects_nested_batch_before_any_mutation() {
    with_font(font(), |doc| {
        let stale = command(doc, false, Op::CleanupInvalid);
        doc.apply(
            Command::SetHorizontalKerning {
                master: doc.font.masters[0].id.clone(),
                left: glyph("A"),
                right: glyph("V"),
                value: Some(-80.),
            },
            "Seed",
        )
        .unwrap();
        let before = doc.font.clone();
        let history = (doc.generation, doc.undo_len());
        let nested = Command::Batch(vec![
            Command::SetHorizontalKerning {
                master: doc.font.masters[0].id.clone(),
                left: glyph("X"),
                right: glyph("V"),
                value: Some(-30.),
            },
            Command::Batch(vec![stale.clone()]),
        ]);
        for attempted in [stale, nested] {
            let error = doc.apply(attempted, "Stale").unwrap_err();
            assert!(error.to_string().contains("generation"));
            assert_eq!(doc.font, before);
            assert_eq!((doc.generation, doc.undo_len()), history);
        }
    });
}

#[test]
fn current_generation_batch_is_atomic_and_missing_master_is_rejected() {
    with_font(font(), |doc| {
        let paste = command(
            doc,
            false,
            Op::Paste {
                records: vec![record(glyph("A"), glyph("V"), -80.)],
                replace: true,
            },
        );
        let vertical = command(
            doc,
            true,
            Op::Paste {
                records: vec![record(glyph("A"), glyph("V"), -20.)],
                replace: true,
            },
        );
        let before = doc.font.clone();
        doc.apply(Command::Batch(vec![paste, vertical]), "Both tables")
            .unwrap();
        assert_eq!(doc.undo_len(), 1);
        assert_eq!(doc.generation, 1);
        doc.undo().unwrap();
        assert_eq!(doc.font, before);
        let missing = Command::KerningMaintenance {
            master: fc_model::MasterId::new(),
            vertical: false,
            expected_generation: doc.generation,
            operation: Op::CleanupInvalid,
        };
        assert!(matches!(
            doc.apply(missing, "Missing"),
            Err(fc_doc::Error::MissingMaster(_))
        ));
        assert_eq!(doc.redo_label(), Some("Both tables"));
    });
}

#[test]
fn ownership_is_orientation_specific_and_noops_preserve_redo() {
    let mut initial = font();
    initial.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "pos A V -80;".into(),
        ..Default::default()
    });
    with_font(initial, |doc| {
        assert!(doc
            .apply(command(doc, false, Op::CleanupInvalid), "Unavailable")
            .is_err());
        let paste = Op::Paste {
            records: vec![record(glyph("A"), glyph("V"), -20.)],
            replace: true,
        };
        doc.apply(command(doc, true, paste), "Vertical").unwrap();
        doc.undo().unwrap();
        let generation = doc.generation;
        doc.apply(command(doc, true, Op::CleanupInvalid), "No-op")
            .unwrap();
        assert_eq!(doc.generation, generation);
        assert_eq!(doc.redo_label(), Some("Vertical"));
    });
}

#[test]
fn obsolete_cleanup_removes_redundant_zeros_and_keeps_real_ones() {
    let mut initial = font();
    initial.masters[0].kerning_h.0 = [
        // A real exception: it suppresses the class value for one pair.
        ((group("public.kern1.A"), group("public.kern2.V")), -80.),
        ((glyph("Aacute"), glyph("V")), 0.),
        // Redundant: nothing broader applies, so the pair is already zero.
        ((glyph("X"), glyph("X")), 0.),
        // A redundant zero class root with no exceptions under it.
        ((group("public.kern1.A"), glyph("X")), 0.),
        // Kept: it carries a value.
        ((glyph("X"), glyph("V")), -15.),
    ]
    .into();
    with_font(initial, |doc| {
        // Removing an obsolete zero leaves no rule at all, which is the same
        // effective kerning; compare effective values, not rule presence.
        let effective = |font: &Font| {
            resolved(font, false)
                .into_iter()
                .map(|value| value.unwrap_or(0.0))
                .collect::<Vec<_>>()
        };
        let values = effective(&doc.font);
        let before = doc.font.clone();
        doc.apply(command(doc, false, Op::CleanupObsolete), "Clean up")
            .unwrap();
        let keys: Vec<_> = table(&doc.font, false).0.keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                (glyph("Aacute"), glyph("V")),
                (glyph("X"), glyph("V")),
                (group("public.kern1.A"), group("public.kern2.V")),
            ]
        );
        assert_eq!(effective(&doc.font), values);
        doc.undo().unwrap();
        assert_eq!(doc.font, before);
    });
}

#[test]
fn obsolete_cleanup_also_removes_invalid_records() {
    let mut initial = font();
    initial.masters[0].kerning_h.0 = [
        ((glyph("A"), glyph("missing")), -30.),
        ((glyph("X"), glyph("V")), f64::NAN),
        ((glyph("X"), glyph("X")), -5.),
    ]
    .into();
    with_font(initial, |doc| {
        doc.apply(command(doc, false, Op::CleanupObsolete), "Clean up")
            .unwrap();
        let keys: Vec<_> = table(&doc.font, false).0.keys().cloned().collect();
        assert_eq!(keys, vec![(glyph("X"), glyph("X"))]);
    });
}

#[test]
fn compression_folds_agreeing_pairs_into_one_class_pair() {
    let mut initial = font();
    // Every member combination of the two groups carries the same value, with
    // no class record: A/V, A/W, Aacute/V and Aacute/W.
    initial.masters[0].kerning_h.0 = [
        ((glyph("A"), glyph("V")), -70.),
        ((glyph("A"), glyph("W")), -70.),
        ((glyph("Aacute"), glyph("V")), -70.),
        ((glyph("Aacute"), glyph("W")), -70.),
        ((glyph("X"), glyph("X")), -12.),
    ]
    .into();
    with_font(initial, |doc| {
        let values = resolved(&doc.font, false);
        let before = doc.font.clone();
        doc.apply(command(doc, false, Op::Compress), "Compress")
            .unwrap();
        let folded = table(&doc.font, false);
        assert_eq!(
            folded
                .0
                .get(&(group("public.kern1.A"), group("public.kern2.V"))),
            Some(&-70.)
        );
        // The four folded records are gone; the unrelated glyph pair stays.
        assert_eq!(folded.0.len(), 2);
        assert_eq!(folded.0.get(&(glyph("X"), glyph("X"))), Some(&-12.));
        assert_eq!(resolved(&doc.font, false), values);
        doc.undo().unwrap();
        assert_eq!(doc.font, before);
    });
}

#[test]
fn compression_keeps_disagreeing_pairs_as_exceptions() {
    let mut initial = font();
    initial.masters[0].kerning_h.0 = [
        ((glyph("A"), glyph("V")), -70.),
        ((glyph("A"), glyph("W")), -70.),
        ((glyph("Aacute"), glyph("V")), -70.),
        // Aacute/W disagrees, so no class record may be invented.
        ((glyph("Aacute"), glyph("W")), -20.),
    ]
    .into();
    with_font(initial, |doc| {
        let values = resolved(&doc.font, false);
        doc.apply(command(doc, false, Op::Compress), "Compress")
            .unwrap();
        let kept = table(&doc.font, false);
        assert!(!kept
            .0
            .contains_key(&(group("public.kern1.A"), group("public.kern2.V"))));
        assert_eq!(kept.0.len(), 4);
        assert_eq!(resolved(&doc.font, false), values);
    });
}

#[test]
fn compression_never_invents_a_zero_class_record() {
    let mut initial = font();
    let zeros = [
        ((glyph("A"), glyph("V")), 0.),
        ((glyph("A"), glyph("W")), 0.),
        ((glyph("Aacute"), glyph("V")), 0.),
        ((glyph("Aacute"), glyph("W")), 0.),
    ];
    initial.masters[0].kerning_h.0 = zeros.clone().into();
    with_font(initial, |doc| {
        doc.apply(command(doc, false, Op::Compress), "Compress")
            .unwrap();
        // A zero class record would express nothing, so none is inserted, and
        // Compress keeps explicit zeros. Clean Up is what removes obsolete ones.
        assert_eq!(table(&doc.font, false).0, Kerning(zeros.into()).0);
        doc.apply(command(doc, false, Op::CleanupObsolete), "Clean up")
            .unwrap();
        assert!(table(&doc.font, false).0.is_empty());
    });
}

#[test]
fn pasting_can_keep_existing_destination_records() {
    let mut initial = font();
    initial.masters[0].kerning_h.0 = [
        ((glyph("A"), glyph("V")), -10.),
        ((glyph("X"), glyph("X")), -1.),
    ]
    .into();
    with_font(initial, |doc| {
        let incoming = vec![
            record(glyph("A"), glyph("V"), -99.),
            record(glyph("A"), glyph("W"), -44.),
        ];
        let before = doc.font.clone();
        doc.apply(
            command(
                doc,
                false,
                Op::PasteKeepingExisting {
                    records: incoming.clone(),
                },
            ),
            "Keep existing",
        )
        .unwrap();
        let merged = table(&doc.font, false);
        assert_eq!(merged.0.get(&(glyph("A"), glyph("V"))), Some(&-10.));
        assert_eq!(merged.0.get(&(glyph("A"), glyph("W"))), Some(&-44.));
        assert_eq!(merged.0.get(&(glyph("X"), glyph("X"))), Some(&-1.));
        doc.undo().unwrap();
        assert_eq!(doc.font, before);
        // The overwrite half of the same choice takes the incoming value.
        doc.apply(
            command(
                doc,
                false,
                Op::Paste {
                    records: incoming,
                    replace: false,
                },
            ),
            "Overwrite",
        )
        .unwrap();
        assert_eq!(
            table(&doc.font, false).0.get(&(glyph("A"), glyph("V"))),
            Some(&-99.)
        );
    });
}

#[test]
fn keep_existing_paste_validates_sides_and_rejects_duplicates() {
    with_font(font(), |doc| {
        let before = doc.font.clone();
        for records in [
            vec![record(glyph("A"), glyph("absent"), -5.)],
            vec![
                record(glyph("A"), glyph("V"), -5.),
                record(glyph("A"), glyph("V"), -6.),
            ],
            vec![record(glyph("A"), glyph("V"), f64::INFINITY)],
        ] {
            assert!(doc
                .apply(
                    command(doc, false, Op::PasteKeepingExisting { records }),
                    "Invalid",
                )
                .is_err());
            assert_eq!(doc.font, before);
        }
    });
}
