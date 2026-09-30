mod common;
use fc_doc::{command::DesignSpaceCommand, Command, Session};
use fc_model::{Axis, Font, MasterId, Tag};

fn add_master(font: &Font) -> Command {
    let mut regular = font.masters[0].clone();
    regular.location.insert(Tag(*b"wght"), 100.0);
    let mut heavy = regular.clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(Tag(*b"wght"), 900.0);
    Command::DesignSpace(DesignSpaceCommand::Set {
        axis_renames: Default::default(),
        virtual_masters: None,
        axes: vec![Axis {
            tag: Tag(*b"wght"),
            name: "Weight".into(),
            min: 100.0,
            default: 100.0,
            max: 900.0,
            ..Default::default()
        }],
        masters: vec![regular, heavy],
        instances: vec![],
        copy_from: font.masters[0].id.clone(),
    })
}

#[test]
fn supplemental_sources_duplicate_edit_undo_and_reject_invalid_roles_atomically() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(add_master(&doc.font), "Design space").unwrap();
            let before = doc.font.clone();
            let master = doc.font.masters[0].id.clone();
            let name = doc.font.glyph_order().next().unwrap().clone();
            let source = doc.font.glyph(&name).unwrap().layers[0].id.clone();
            let command = DesignSpaceCommand::Layer {
                glyph: name.clone(),
                source,
                duplicate: true,
                kind: fc_model::LayerKind::Brace {
                    master: master.clone(),
                    location: [(Tag(*b"wght"), 500.)].into(),
                },
                name: "Intermediate".into(),
            };
            doc.apply(Command::DesignSpace(command), "Add source")
                .unwrap();
            let layer = doc.font.glyph(&name).unwrap().layers.last().unwrap();
            assert_eq!(
                layer.contours,
                before.glyph(&name).unwrap().layers[0].contours
            );
            let source = layer.id.clone();
            let duplicated = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, duplicated);
            let mut command = DesignSpaceCommand::Layer {
                glyph: name,
                source,
                duplicate: false,
                kind: fc_model::LayerKind::Bracket {
                    master,
                    axis: Tag(*b"wght"),
                    min: Some(500.),
                    max: None,
                },
                name: "Alternate".into(),
            };
            doc.apply(Command::DesignSpace(command.clone()), "Edit source")
                .unwrap();
            let edited = doc.font.clone();
            if let DesignSpaceCommand::Layer { kind, .. } = &mut command {
                *kind = fc_model::LayerKind::Custom("bad".into());
            }
            assert!(doc
                .apply(Command::DesignSpace(command), "Bad source")
                .is_err());
            assert_eq!(doc.font, edited);
            doc.undo().unwrap();
            assert_eq!(doc.font, duplicated);
        })
        .unwrap();
}

#[test]
fn add_remove_master_undo_and_native_roundtrip_restore_source_layers() {
    let session = Session::new();
    let original = common::test_font();
    let id = session.open_font(original.clone(), None);
    session
        .with_doc_mut(id, |doc| {
            let change = doc.apply(add_master(&doc.font), "Add master").unwrap();
            assert!(change.masters && change.layers && change.affects_compile());
            assert_eq!(doc.font.masters.len(), 2);
            for glyph in doc.font.glyphs.values() {
                assert_eq!(glyph.layers.len(), 2);
                assert_ne!(glyph.layers[0].id, glyph.layers[1].id);
            }
            let added = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.redo().unwrap();
            assert_eq!(doc.font, added);
            let dir = common::temp_dir("p4-designspace");
            let path = dir.join("font.fcf");
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            assert_eq!(fc_doc::fcf::read(&path).unwrap(), doc.font);
            std::fs::remove_dir_all(dir).unwrap();
            doc.apply(
                Command::DesignSpace(DesignSpaceCommand::Set {
                    axis_renames: Default::default(),
                    virtual_masters: None,
                    axes: doc.font.axes.clone(),
                    masters: vec![doc.font.masters[0].clone()],
                    instances: vec![],
                    copy_from: doc.font.masters[0].id.clone(),
                }),
                "Remove master",
            )
            .unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers.len(), 1);
            doc.undo().unwrap();
            assert_eq!(doc.font, added);
        })
        .unwrap();
}

#[test]
fn invalid_space_rolls_back_all_layers_and_coalesced_metrics_restore_endpoints() {
    let session = Session::new();
    let original = common::test_font();
    let id = session.open_font(original.clone(), None);
    session
        .with_doc_mut(id, |doc| {
            let mut command = add_master(&doc.font);
            if let Command::DesignSpace(DesignSpaceCommand::Set { masters, .. }) = &mut command {
                masters[1].location = masters[0].location.clone();
            }
            assert!(doc.apply(command, "Invalid duplicate").is_err());
            assert_eq!(doc.font, original);
            assert_eq!(doc.undo_len(), 0);
            let token = doc.begin_edit("Design space");
            let mut metrics = doc.font.masters[0].metrics.clone();
            metrics.ascender += 10.0;
            doc.apply_in_edit(
                &token,
                Command::SetMasterMetrics {
                    master: doc.font.masters[0].id.clone(),
                    metrics,
                },
            )
            .unwrap();
            doc.apply_in_edit(&token, add_master(&doc.font)).unwrap();
            doc.end_edit(token);
            let final_font = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.redo().unwrap();
            assert_eq!(doc.font, final_font);
        })
        .unwrap();
}

#[test]
fn axis_renames_migrate_sources_atomically_and_restore_them_on_undo() {
    use fc_model::{LayerId, LayerKind};
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session.with_doc_mut(id, |doc| {
        doc.apply(add_master(&doc.font), "Design space").unwrap();
        let master = doc.font.masters[0].id.clone();
        let glyph = doc.font.glyph_mut("base").unwrap();
        for kind in [
            LayerKind::Brace { master: master.clone(), location: [(Tag(*b"wght"), 500.)].into() },
            LayerKind::Bracket { master: master.clone(), axis: Tag(*b"wght"), min: Some(600.), max: None },
        ] {
            let mut source = glyph.layers[0].clone();
            source.id = LayerId::new();
            source.kind = kind;
            glyph.layers.push(source);
        }
        let original = doc.font.clone();
        let mut axes = doc.font.axes.clone();
        axes[0].tag = Tag(*b"WGHT");
        let mut masters = doc.font.masters.clone();
        for master in &mut masters {
            let value = master.location.remove(&Tag(*b"wght")).unwrap();
            master.location.insert(Tag(*b"WGHT"), value);
        }
        let command = DesignSpaceCommand::Set {
            axes, masters, instances: vec![], copy_from: master,
            axis_renames: [("wght".into(), "WGHT".into())].into(),
            virtual_masters: None,
        };
        let mut invalid = command.clone();
        if let DesignSpaceCommand::Set { axis_renames, .. } = &mut invalid {
            axis_renames.insert("wdth".into(), "WGHT".into());
        }
        assert!(doc.apply(Command::DesignSpace(invalid), "Invalid rename").is_err());
        assert_eq!(doc.font, original);
        doc.apply(Command::DesignSpace(command), "Rename axis").unwrap();
        let changed = doc.font.clone();
        assert!(changed.glyph("base").unwrap().layers.iter().any(|l| matches!(&l.kind, LayerKind::Brace {location,..} if location.contains_key(&Tag(*b"WGHT")))));
        assert!(changed.glyph("base").unwrap().layers.iter().any(|l| matches!(&l.kind, LayerKind::Bracket {axis,..} if *axis == Tag(*b"WGHT"))));
        doc.undo().unwrap();
        assert_eq!(doc.font, original);
        doc.redo().unwrap();
        assert_eq!(doc.font, changed);
    }).unwrap();
}

#[test]
fn repairs_smart_sources_and_conditional_groups_without_changing_geometry() {
    use fc_model::{LayerId, LayerKind, SmartAxis};
    use kurbo::Shape;
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            doc.apply(add_master(&doc.font), "Design space").unwrap();
            let masters = doc.font.masters.clone();
            let glyph = doc.font.glyph_mut("base").unwrap();
            glyph.smart_axes.push(SmartAxis {
                name: "Size".into(),
                min: 0.,
                default: 0.,
                max: 1.,
            });
            for master in &masters {
                for kind in [
                    LayerKind::Smart {
                        master: master.id.clone(),
                        location: [("Size".into(), 1.)].into(),
                    },
                    LayerKind::Bracket {
                        master: master.id.clone(),
                        axis: Tag(*b"wght"),
                        min: Some(600.),
                        max: None,
                    },
                ] {
                    let mut source = glyph.layers[0].clone();
                    source.id = LayerId::new();
                    source.kind = kind;
                    source.contours[0].points.rotate_left(2);
                    if master.id == masters[1].id {
                        source.contours[0].points.reverse();
                    }
                    glyph.layers.push(source);
                }
            }
            let original = doc.font.clone();
            let bounds: Vec<_> = original
                .glyph("base")
                .unwrap()
                .layers
                .iter()
                .map(|l| l.contours[0].to_bezpath().bounding_box())
                .collect();
            doc.apply(
                Command::DesignSpace(DesignSpaceCommand::Repair {
                    glyphs: vec!["base".into()],
                }),
                "Repair",
            )
            .unwrap();
            let repaired = doc.font.glyph("base").unwrap();
            for (layer, bounds) in repaired.layers.iter().zip(bounds) {
                assert_eq!(layer.contours[0].to_bezpath().bounding_box(), bounds);
                assert_eq!(layer.contours, repaired.layers[0].contours);
            }
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
        })
        .unwrap();
}
