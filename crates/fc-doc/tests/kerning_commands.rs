use fc_doc::{kerning::KerningGroupSide as Side, Command, Document, Session};
use fc_model::{Font, Glyph, KernSide, Master};

fn document(test: impl FnOnce(&mut Document)) {
    let mut font = Font::new_empty("Kerning commands", 1000);
    for name in ["A", "V", "W"] {
        font.insert_glyph(Glyph::new(name)).unwrap();
    }
    font.add_master(Master::default());
    let session = Session::new();
    let id = session.open_font(font, None);
    session.with_doc_mut(id, test).unwrap();
}

fn pair(doc: &Document, value: Option<f64>) -> Command {
    Command::SetHorizontalKerning {
        master: doc.font.masters[0].id.clone(),
        left: KernSide::Glyph("A".into()),
        right: KernSide::Glyph("V".into()),
        value,
    }
}

fn group(side: Side, name: &str, members: &[&str]) -> Command {
    Command::SetKerningGroup {
        side,
        name: name.into(),
        members: members.iter().map(|name| (*name).into()).collect(),
    }
}

#[test]
fn pair_set_zero_remove_undo_redo_and_master_isolation() {
    document(|doc| {
        let original = doc.font.clone();
        let gid = doc.glyph_id("A").unwrap();
        let generation = doc.glyph_generation(gid);
        let change = doc.apply(pair(doc, Some(-80.0)), "Kern").unwrap();
        assert!(change.layout);
        assert!(change.glyphs.is_empty());
        assert!(!change.masters);
        assert_eq!(doc.glyph_generation(gid), generation);
        assert!(doc.font.masters[1].kerning_h.0.is_empty());
        doc.apply(pair(doc, Some(0.0)), "Zero").unwrap();
        assert_eq!(doc.font.masters[0].kerning_h.0.values().next(), Some(&0.0));
        doc.apply(pair(doc, None), "Remove").unwrap();
        assert!(doc.font.masters[0].kerning_h.0.is_empty());
        for _ in 0..3 {
            doc.undo().unwrap();
        }
        assert_eq!(doc.font, original);
        assert!(!doc.dirty);
        for _ in 0..3 {
            doc.redo().unwrap();
        }
        assert!(doc.font.masters[0].kerning_h.0.is_empty());
        assert_eq!(doc.generation, 9);
    });
}

#[test]
fn no_op_does_not_change_history_generation_or_redo() {
    document(|doc| {
        doc.apply(pair(doc, Some(-80.0)), "Kern").unwrap();
        let generation = doc.generation;
        assert_eq!(
            doc.apply(pair(doc, Some(-80.0)), "No-op").unwrap(),
            Default::default()
        );
        assert_eq!((doc.generation, doc.undo_len()), (generation, 1));
        doc.undo().unwrap();
        let generation = doc.generation;
        doc.apply(pair(doc, None), "Absent").unwrap();
        assert_eq!(doc.generation, generation);
        assert_eq!(doc.redo_label(), Some("Kern"));
        assert!(!doc.dirty);
    });
}

#[test]
fn no_op_at_edit_start_does_not_coalesce_into_previous_entry() {
    document(|doc| {
        doc.apply(pair(doc, Some(-80.0)), "First").unwrap();
        let token = doc.begin_edit("Second");
        doc.apply_in_edit(&token, pair(doc, Some(-80.0))).unwrap();
        for value in [-81.0, -82.0, -83.0] {
            doc.apply_in_edit(&token, pair(doc, Some(value))).unwrap();
        }
        doc.end_edit(token);
        assert_eq!(doc.undo_len(), 2);
        doc.undo().unwrap();
        assert_eq!(
            doc.font.masters[0].kerning_h.0.values().next(),
            Some(&-80.0)
        );
        assert_eq!(doc.undo_label(), Some("First"));
    });
}

#[test]
fn unrelated_nan_records_do_not_turn_no_ops_into_edits() {
    document(|doc| {
        let invalid = (KernSide::Glyph("W".into()), KernSide::Glyph("V".into()));
        doc.font.masters[1]
            .kerning_h
            .0
            .insert(invalid.clone(), f64::NAN);
        doc.apply(pair(doc, Some(-80.0)), "First").unwrap();
        doc.apply(group(Side::Side1, "public.kern1.A", &["A"]), "Group")
            .unwrap();
        let before = (doc.generation, doc.undo_len());
        for command in [
            pair(doc, Some(-80.0)),
            group(Side::Side1, "public.kern1.A", &["A"]),
            Command::AssignKerningGroup {
                side: Side::Side1,
                glyph: "A".into(),
                group: Some("public.kern1.A".into()),
            },
        ] {
            assert_eq!(doc.apply(command, "No-op").unwrap(), Default::default());
            assert_eq!((doc.generation, doc.undo_len()), before);
        }
        assert!(doc.font.masters[1].kerning_h.0[&invalid].is_nan());
    });
}

#[test]
fn invalid_pairs_are_atomic() {
    document(|doc| {
        let mut commands = vec![pair(doc, Some(f64::NAN)), pair(doc, Some(f64::INFINITY))];
        for (master, left, right) in [
            (
                fc_model::MasterId::new(),
                KernSide::Glyph("A".into()),
                KernSide::Glyph("V".into()),
            ),
            (
                doc.font.masters[0].id.clone(),
                KernSide::Glyph("missing".into()),
                KernSide::Glyph("V".into()),
            ),
            (
                doc.font.masters[0].id.clone(),
                KernSide::Group("public.kern2.bad".into()),
                KernSide::Glyph("V".into()),
            ),
            (
                doc.font.masters[0].id.clone(),
                KernSide::Glyph("A".into()),
                KernSide::Group("public.kern2.missing".into()),
            ),
        ] {
            commands.push(Command::SetHorizontalKerning {
                master,
                left,
                right,
                value: Some(-80.0),
            });
        }
        let original = doc.font.clone();
        for command in commands {
            assert!(doc.apply(command, "Invalid").is_err());
            assert_eq!(doc.font, original);
            assert_eq!((doc.generation, doc.undo_len(), doc.dirty), (0, 0, false));
        }
    });
}

#[test]
fn groups_membership_moves_and_undo_are_atomic_without_outline_changes() {
    document(|doc| {
        let original = doc.font.clone();
        doc.apply(group(Side::Side1, "public.kern1.A", &["A"]), "Group")
            .unwrap();
        doc.apply(group(Side::Side1, "public.kern1.W", &["W"]), "Group")
            .unwrap();
        doc.apply(
            Command::AssignKerningGroup {
                side: Side::Side1,
                glyph: "A".into(),
                group: Some("public.kern1.W".into()),
            },
            "Assign",
        )
        .unwrap();
        assert!(doc.font.kerning_groups.side1["public.kern1.A"].is_empty());
        assert_eq!(
            doc.font.glyph("A").unwrap().kern_group_side1.as_deref(),
            Some("public.kern1.W")
        );
        assert_eq!(doc.glyph_generation(doc.glyph_id("A").unwrap()), 0);
        for _ in 0..3 {
            doc.undo().unwrap();
        }
        assert_eq!(doc.font, original);
        for _ in 0..3 {
            doc.redo().unwrap();
        }
        assert_eq!(
            doc.font.glyph("A").unwrap().kern_group_side1.as_deref(),
            Some("public.kern1.W")
        );
    });
}

#[test]
fn group_rename_updates_all_horizontal_masters_and_keeps_vertical_pairs() {
    document(|doc| {
        doc.apply(group(Side::Side1, "public.kern1.A", &["A"]), "Group")
            .unwrap();
        for master in &mut doc.font.masters {
            let key = (
                KernSide::Group("public.kern1.A".into()),
                KernSide::Glyph("V".into()),
            );
            master.kerning_h.0.insert(key.clone(), -80.0);
            master.kerning_v.0.insert(key, -200.0);
        }
        let before = doc.font.clone();
        doc.apply(
            Command::RenameKerningGroup {
                side: Side::Side1,
                from: "public.kern1.A".into(),
                to: "public.kern1.New".into(),
            },
            "Rename",
        )
        .unwrap();
        for (master, old) in doc.font.masters.iter().zip(&before.masters) {
            assert_eq!(
                master.kerning_h.0.keys().next().unwrap().0,
                KernSide::Group("public.kern1.New".into())
            );
            assert_eq!(master.kerning_v, old.kerning_v);
        }
        assert_eq!(
            doc.font.glyph("A").unwrap().kern_group_side1.as_deref(),
            Some("public.kern1.New")
        );
        let after = doc.font.clone();
        doc.undo().unwrap();
        assert_eq!(doc.font, before);
        doc.redo().unwrap();
        assert_eq!(doc.font, after);
    });
}

#[test]
fn in_use_group_removal_requires_explicit_pair_removal() {
    document(|doc| {
        doc.apply(group(Side::Side2, "public.kern2.V", &["V"]), "Group")
            .unwrap();
        doc.apply(
            Command::SetHorizontalKerning {
                master: doc.font.masters[0].id.clone(),
                left: KernSide::Glyph("A".into()),
                right: KernSide::Group("public.kern2.V".into()),
                value: Some(-70.0),
            },
            "Pair",
        )
        .unwrap();
        let before = doc.font.clone();
        let generation = doc.generation;
        assert!(doc
            .apply(
                Command::RemoveKerningGroup {
                    side: Side::Side2,
                    name: "public.kern2.V".into(),
                    remove_pairs: false
                },
                "Remove"
            )
            .is_err());
        assert_eq!(doc.font, before);
        assert_eq!(doc.generation, generation);
        doc.apply(
            Command::RemoveKerningGroup {
                side: Side::Side2,
                name: "public.kern2.V".into(),
                remove_pairs: true,
            },
            "Remove and pairs",
        )
        .unwrap();
        assert!(doc.font.masters[0].kerning_h.0.is_empty());
        assert_eq!(doc.font.glyph("V").unwrap().kern_group_side2, None);
        doc.undo().unwrap();
        assert_eq!(doc.font, before);
    });
}

#[test]
fn invalid_groups_and_duplicate_memberships_are_rejected() {
    document(|doc| {
        doc.apply(group(Side::Side1, "public.kern1.A", &["A"]), "Group")
            .unwrap();
        let before = doc.font.clone();
        let generation = doc.generation;
        for command in [
            group(Side::Side1, "public.kern2.wrong", &[]),
            group(Side::Side1, "public.kern1.", &[]),
            group(Side::Side2, "public.kern2.V", &["missing"]),
            group(Side::Side1, "public.kern1.other", &["A"]),
            group(Side::Side2, "public.kern2.V", &["V", "V"]),
        ] {
            assert!(doc.apply(command, "Invalid").is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, generation);
        }
    });
}

#[test]
fn failed_batch_restores_group_membership_and_pairs() {
    document(|doc| {
        let before = doc.font.clone();
        assert!(doc
            .apply(
                Command::Batch(vec![
                    group(Side::Side1, "public.kern1.A", &["A"]),
                    pair(doc, Some(-80.0)),
                    group(Side::Side2, "public.kern2.V", &["missing"]),
                ]),
                "Batch"
            )
            .is_err());
        assert_eq!(doc.font, before);
        assert_eq!((doc.generation, doc.undo_len()), (0, 0));
    });
}

#[test]
fn glyph_rename_delete_and_undo_keep_exact_kerning_references() {
    document(|doc| {
        doc.apply(group(Side::Side1, "public.kern1.A", &["A"]), "Group")
            .unwrap();
        doc.apply(pair(doc, Some(-80.0)), "Pair").unwrap();
        let before = doc.font.clone();
        doc.apply(
            Command::RenameGlyph {
                from: "A".into(),
                to: "A.new".into(),
            },
            "Rename",
        )
        .unwrap();
        assert!(doc.font.kerning_groups.side1["public.kern1.A"].contains("A.new"));
        let renamed = doc.font.clone();
        doc.apply(
            Command::RemoveGlyphs {
                names: vec!["A.new".into()],
            },
            "Delete",
        )
        .unwrap();
        assert!(doc.font.kerning_groups.side1["public.kern1.A"].is_empty());
        assert!(doc.font.masters[0].kerning_h.0.is_empty());
        doc.undo().unwrap();
        assert_eq!(doc.font, renamed);
        doc.undo().unwrap();
        assert_eq!(doc.font, before);
    });
}

#[test]
fn imported_and_source_owned_layout_reject_every_kerning_command() {
    document(|doc| {
        for imported in [true, false] {
            if imported {
                doc.font.features.source_of_truth = fc_model::FeaturesSource::ImportedTables;
            } else {
                doc.font.features.source_of_truth = fc_model::FeaturesSource::Fea;
                doc.font.features.features.push(fc_model::FeatureBlock {
                    tag: "kern".into(),
                    code: "pos A V -80;".into(),
                    ..Default::default()
                });
            }
            let before = doc.font.clone();
            for command in [
                pair(doc, Some(-80.0)),
                Command::CreateKerningGroup {
                    side: Side::Side1,
                    name: "public.kern1.New".into(),
                    members: vec![],
                },
                group(Side::Side1, "public.kern1.A", &["A"]),
                Command::AssignKerningGroup {
                    side: Side::Side1,
                    glyph: "A".into(),
                    group: None,
                },
                Command::RenameKerningGroup {
                    side: Side::Side1,
                    from: "public.kern1.A".into(),
                    to: "public.kern1.B".into(),
                },
                Command::RemoveKerningGroup {
                    side: Side::Side1,
                    name: "public.kern1.A".into(),
                    remove_pairs: true,
                },
            ] {
                assert!(doc.apply(command, "Blocked").is_err());
                assert_eq!(doc.font, before);
            }
        }
    });
}

#[test]
fn glyph_rename_rejects_dangling_destination_kerning_references_atomically() {
    for pair_reference in [true, false] {
        document(|doc| {
            doc.apply(group(Side::Side1, "public.kern1.A", &["A"]), "Group")
                .unwrap();
            doc.apply(pair(doc, Some(-80.0)), "Pair").unwrap();
            if pair_reference {
                doc.font.masters[0].kerning_h.0.insert(
                    (KernSide::Glyph("A.new".into()), KernSide::Glyph("V".into())),
                    -90.0,
                );
            } else {
                doc.font
                    .kerning_groups
                    .side1
                    .insert("public.kern1.Other".into(), ["A.new".into()].into());
            }
            let before = doc.font.clone();
            let generation = doc.generation;
            let rename = Command::RenameGlyph {
                from: "A".into(),
                to: "A.new".into(),
            };
            for command in [
                rename.clone(),
                Command::Batch(vec![pair(doc, Some(-120.0)), rename]),
            ] {
                assert!(doc.apply(command, "Conflicting rename").is_err());
                assert_eq!(doc.font, before);
                assert_eq!(doc.generation, generation);
            }
        });
    }
}

#[test]
fn master_metadata_edits_preserve_invalid_existing_kerning() {
    document(|doc| {
        doc.font.features.source_of_truth = fc_model::FeaturesSource::ImportedTables;
        let key = (KernSide::Glyph("A".into()), KernSide::Glyph("V".into()));
        doc.font.masters[0]
            .kerning_h
            .0
            .insert(key.clone(), f64::NAN);
        let mut master = doc.font.masters[0].clone();
        master.name = "New master name".into();
        let change = doc
            .apply(Command::SetMaster { master }, "Name master")
            .unwrap();
        assert!(!change.layout);
        assert_eq!(doc.font.masters[0].name, "New master name");
        assert!(doc.font.masters[0].kerning_h.0[&key].is_nan());
        doc.undo().unwrap();
        assert!(doc.font.masters[0].kerning_h.0[&key].is_nan());
    });
}

#[test]
fn mixed_glyph_and_group_batches_restore_exact_references_on_failure_and_undo() {
    document(|doc| {
        doc.apply(group(Side::Side1, "public.kern1.A", &["A"]), "Group")
            .unwrap();
        doc.apply(pair(doc, Some(-80.0)), "Pair").unwrap();
        let before = doc.font.clone();
        let before_generation = doc.generation;
        let commands = vec![
            Command::RenameGlyph {
                from: "A".into(),
                to: "A.new".into(),
            },
            Command::RenameKerningGroup {
                side: Side::Side1,
                from: "public.kern1.A".into(),
                to: "public.kern1.New".into(),
            },
            Command::RemoveGlyphs {
                names: vec!["V".into()],
            },
        ];
        let mut invalid = commands.clone();
        invalid.push(group(Side::Side2, "public.kern2.V", &["V"]));
        assert!(doc
            .apply(Command::Batch(invalid), "Invalid mixed batch")
            .is_err());
        assert_eq!(doc.font, before);
        assert_eq!(doc.generation, before_generation);
        doc.apply(Command::Batch(commands), "Mixed batch").unwrap();
        let after = doc.font.clone();
        assert!(after.glyph("V").is_none());
        assert!(after.masters[0].kerning_h.0.is_empty());
        doc.undo().unwrap();
        assert_eq!(doc.font, before);
        doc.redo().unwrap();
        assert_eq!(doc.font, after);
    });
}

#[test]
fn creating_a_group_never_overwrites_existing_members_even_in_a_batch() {
    document(|doc| {
        let create = Command::CreateKerningGroup {
            side: Side::Side1,
            name: "public.kern1.A".into(),
            members: vec!["A".into()],
        };
        doc.apply(create, "Create group").unwrap();
        let before = doc.font.clone();
        let generation = doc.generation;
        let command = Command::CreateKerningGroup {
            side: Side::Side1,
            name: "public.kern1.A".into(),
            members: vec![],
        };
        for command in [
            command.clone(),
            Command::Batch(vec![
                command,
                Command::AssignKerningGroup {
                    side: Side::Side1,
                    glyph: "W".into(),
                    group: Some("public.kern1.A".into()),
                },
            ]),
        ] {
            assert!(doc.apply(command, "Duplicate create").is_err());
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, generation);
        }
        doc.undo().unwrap();
        assert!(doc.font.kerning_groups.side1.is_empty());
        assert_eq!(doc.font.glyph("A").unwrap().kern_group_side1, None);
        doc.redo().unwrap();
        assert_eq!(doc.font, before);
    });
}

#[test]
fn set_master_cannot_bypass_kerning_eligibility() {
    document(|doc| {
        doc.font.features.source_of_truth = fc_model::FeaturesSource::ImportedTables;
        let mut master = doc.font.masters[0].clone();
        master.kerning_h.0.insert(
            (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
            -80.0,
        );
        let before = doc.font.clone();
        assert!(doc.apply(Command::SetMaster { master }, "Bypass").is_err());
        assert_eq!(doc.font, before);
    });
}
