mod common;

use fc_doc::{command::DesignSpaceCommand, Command, Session};
use fc_model::{Axis, LayerKind, Tag, VirtualMaster};

fn virtual_font() -> fc_model::Font {
    let mut font = common::test_font();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.,
        default: 100.,
        max: 900.,
        ..Default::default()
    });
    font
}

fn virtual_master() -> VirtualMaster {
    VirtualMaster {
        name: "Heavy support".into(),
        location: [(Tag(*b"wght"), 900.)].into(),
    }
}

#[test]
fn virtual_positions_and_sparse_sources_are_atomic_undoable_and_persisted() {
    let session = Session::new();
    let original = virtual_font();
    let id = session.open_font(original.clone(), None);
    session.with_doc_mut(id, |doc| {
        doc.apply(Command::DesignSpace(DesignSpaceCommand::VirtualMasters {
            masters: vec![virtual_master()],
        }), "Virtual positions").unwrap();
        assert_eq!(doc.font.glyphs, original.glyphs);
        assert_eq!(doc.font.masters, original.masters);
        let positions = doc.font.clone();
        doc.undo().unwrap();
        assert_eq!(doc.font, original);
        doc.redo().unwrap();
        assert_eq!(doc.font, positions);
        let source = doc.font.glyph("base").unwrap().layers[0].id.clone();
        let command = Command::DesignSpace(DesignSpaceCommand::VirtualSource {
            glyph: "base".into(), source, index: 0,
        });
        doc.apply(command.clone(), "Virtual source").unwrap();
        let glyph = doc.font.glyph("base").unwrap();
        assert_eq!(glyph.layers.len(), 2);
        assert!(matches!(&glyph.layers[1].kind, LayerKind::Brace { location, .. } if location[&Tag(*b"wght")] == 900.));
        for (name, glyph) in &doc.font.glyphs {
            if name.as_ref() != "base" { assert_eq!(glyph, &original.glyphs[name]); }
        }
        let added = doc.font.clone();
        assert!(doc.apply(command, "Duplicate virtual source").is_err());
        assert_eq!(doc.font, added);
        doc.undo().unwrap();
        assert_eq!(doc.font, positions);
        doc.redo().unwrap();
        assert_eq!(doc.font, added);
        let dir = common::temp_dir("p5-virtual-masters");
        let path = dir.join("virtual.fcf");
        fc_doc::fcf::write(&path, &doc.font).unwrap();
        assert_eq!(fc_doc::fcf::read(&path).unwrap(), added);
        std::fs::remove_dir_all(dir).unwrap();
        doc.undo().unwrap();
        doc.undo().unwrap();
        assert_eq!(doc.font, original);
        doc.redo().unwrap();
        assert_eq!(doc.font, positions);
        doc.redo().unwrap();
        assert_eq!(doc.font, added);
    }).unwrap();
}

#[test]
fn invalid_positions_roll_back_and_axis_renames_preserve_virtual_coordinates() {
    let session = Session::new();
    let id = session.open_font(virtual_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.clone();
            for masters in [
                vec![VirtualMaster {
                    name: "".into(),
                    ..virtual_master()
                }],
                vec![VirtualMaster {
                    location: [(Tag(*b"wght"), 901.)].into(),
                    ..virtual_master()
                }],
                vec![VirtualMaster {
                    location: [(Tag(*b"wght"), 100.)].into(),
                    ..virtual_master()
                }],
                vec![virtual_master(), virtual_master()],
            ] {
                assert!(doc
                    .apply(
                        Command::DesignSpace(DesignSpaceCommand::VirtualMasters { masters }),
                        "Invalid virtual"
                    )
                    .is_err());
                assert_eq!(doc.font, before);
            }
            doc.apply(
                Command::DesignSpace(DesignSpaceCommand::VirtualMasters {
                    masters: vec![virtual_master()],
                }),
                "Virtual",
            )
            .unwrap();
            let positioned = doc.font.clone();
            let mut axes = doc.font.axes.clone();
            axes[0].tag = Tag(*b"WGHT");
            doc.apply(
                Command::DesignSpace(DesignSpaceCommand::Set {
                    axis_renames: [("wght".into(), "WGHT".into())].into(),
                    virtual_masters: None,
                    axes,
                    masters: doc.font.masters.clone(),
                    instances: vec![],
                    copy_from: doc.font.masters[0].id.clone(),
                }),
                "Rename axis",
            )
            .unwrap();
            assert_eq!(
                doc.font.virtual_masters[0].location,
                [(Tag(*b"WGHT"), 900.)].into()
            );
            let renamed = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, positioned);
            doc.redo().unwrap();
            assert_eq!(doc.font, renamed);
        })
        .unwrap();
}

#[test]
fn failing_batch_restores_virtual_positions_with_other_designspace_metadata() {
    let session = Session::new();
    let id = session.open_font(virtual_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let generation = doc.generation;
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            let result = doc.apply(
                Command::Batch(vec![
                    Command::DesignSpace(DesignSpaceCommand::VirtualMasters {
                        masters: vec![virtual_master()],
                    }),
                    Command::SetWidth {
                        glyph: "missing".into(),
                        layer,
                        width: 500.,
                    },
                ]),
                "Failed sparse source edit",
            );
            assert!(result.is_err());
            assert_eq!(doc.font, original);
            assert_eq!(doc.generation, generation);
            assert_eq!(doc.undo_len(), 0);
            assert_eq!(doc.redo_label(), None);
        })
        .unwrap();
}
