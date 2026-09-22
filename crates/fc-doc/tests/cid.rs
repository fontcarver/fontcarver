mod common;
use fc_doc::{Command, Session};
use fc_model::{CidInfo, KernSide};

#[test]
fn cid_mapping_survives_rename_remove_undo_and_failed_batches() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let cid = CidInfo {
                registry: "Adobe".into(),
                ordering: "Identity".into(),
                supplement: 0,
                cid_map: [(12, "base".into()), (300, "composite".into())].into(),
            };
            doc.apply(
                Command::SetCidInfo {
                    cid: Some(cid.clone()),
                },
                "CID",
            )
            .unwrap();
            let mapped = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.redo().unwrap();
            assert_eq!(doc.font, mapped);
            doc.apply(
                Command::RenameGlyph {
                    from: "base".into(),
                    to: "renamed".into(),
                },
                "Rename",
            )
            .unwrap();
            assert_eq!(
                doc.font.cid.as_ref().unwrap().cid_map[&12].as_ref(),
                "renamed"
            );
            doc.undo().unwrap();
            assert_eq!(doc.font, mapped);
            doc.apply(
                Command::SetVerticalKerning {
                    master: doc.font.masters[0].id.clone(),
                    left: KernSide::Glyph("base".into()),
                    right: KernSide::Glyph("composite".into()),
                    value: Some(-30.),
                },
                "Pair",
            )
            .unwrap();
            let paired = doc.font.clone();
            doc.apply(
                Command::RemoveGlyphs {
                    names: vec!["composite".into()],
                },
                "Remove",
            )
            .unwrap();
            assert!(!doc.font.cid.as_ref().unwrap().cid_map.contains_key(&300));
            assert!(doc.font.masters[0].kerning_v.0.is_empty());
            doc.undo().unwrap();
            assert_eq!(doc.font, paired);
            for cid in [
                CidInfo {
                    cid_map: [(0, "base".into())].into(),
                    ..cid.clone()
                },
                CidInfo {
                    cid_map: [(12, "base".into()), (13, "base".into())].into(),
                    ..cid.clone()
                },
                CidInfo {
                    cid_map: [(70000, "base".into())].into(),
                    ..cid.clone()
                },
                CidInfo {
                    registry: "bad/name".into(),
                    ..cid.clone()
                },
                CidInfo {
                    cid_map: [(2, "absent".into())].into(),
                    ..cid.clone()
                },
            ] {
                assert!(doc
                    .apply(
                        Command::Batch(vec![
                            Command::SetCidInfo { cid: None },
                            Command::SetCidInfo { cid: Some(cid) }
                        ]),
                        "Invalid"
                    )
                    .is_err());
                assert_eq!(doc.font, paired);
            }
            let path = std::env::temp_dir().join(format!("fc-cid-{}.fcf", std::process::id()));
            fc_doc::fcf::write(&path, &doc.font).unwrap();
            let restored = fc_doc::fcf::read(&path).unwrap();
            assert_eq!(restored.cid, doc.font.cid);
            std::fs::remove_file(path).unwrap();
        })
        .unwrap();
}
