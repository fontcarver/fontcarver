use std::sync::Arc;

use fc_api::{Api, TtHintDraft, TtHintState};
use fc_doc::{DocId, Session};
use fc_model::{Contour, Font, Glyph, TtInstructionBinding, TtMasterData};
use kurbo::{Rect, Shape};
use serde_json::{json, Value};

fn setup() -> (Api, Arc<Session>, DocId, Value, Font) {
    let mut font = Font::new_empty("TT editing", 1000);
    let master = font.default_master().id.clone();
    for name in ["I", "space"] {
        font.insert_glyph(Glyph::new(name)).unwrap();
        let layer = font
            .glyph_mut(name)
            .unwrap()
            .master_layer_mut(&master)
            .unwrap();
        layer.width = 300.0;
        if name == "I" {
            layer.contours =
                Contour::from_bezpath(&Rect::new(100.0, 0.0, 180.0, 700.0).to_path(0.1));
        }
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font.clone(), None);
    let api = Api::new(session.clone());
    let target = json!({"doc":doc,"master":master,"glyph":"I"});
    (api, session, doc, target, font)
}

fn state(api: &Api, target: &Value) -> TtHintState {
    serde_json::from_value(api.call("hints.tt_state", target.clone()).unwrap()).unwrap()
}

fn draft(state: &TtHintState, point: u32) -> TtHintDraft {
    TtHintDraft {
        fingerprint: state.fingerprint.clone(),
        program: format!("SVTCA[x]\nPUSHB[0] {point}\nMDAP[rnd]\nPUSHB[1] {point} 64\nSHPIX"),
        fpgm: "PUSHB[0] 0\nFDEF\nSVTCA[y]\nENDF".into(),
        prep: "PUSHB[0] 0\nCALL".into(),
        globals: TtMasterData {
            cvt: vec![80],
            max_zones: Some(2),
            max_twilight_points: Some(0),
            max_storage: Some(0),
            max_function_defs: Some(1),
            max_instruction_defs: Some(0),
            max_stack_elements: Some(32),
            ..Default::default()
        },
    }
}

#[test]
fn compiled_state_preview_and_apply_are_revision_checked_read_only_and_undoable() {
    let (api, session, doc, target, before) = setup();
    let state = state(&api, &target);
    assert_eq!(state.points.len(), 4);
    assert_eq!(state.phantom_start, 4);
    assert_eq!(state.horizontal_phantoms, [0.0, 300.0]);
    assert!(!state.path.is_empty());
    assert!(!state.authored);
    let draft = draft(&state, 0);
    let preview = api.call("hints.tt_preview", json!({"target":target,"expected_generation":state.generation,"draft":draft,"ppems":[12,16],"mono":true})).unwrap();
    assert_eq!(preview["sizes"].as_array().unwrap().len(), 2);
    assert_ne!(
        preview["sizes"][0]["hinted"],
        preview["sizes"][0]["unhinted"]
    );
    session
        .with_doc(doc, |doc| {
            assert_eq!(doc.font, before);
            assert_eq!(doc.generation, state.generation);
        })
        .unwrap();
    let params = json!({"target":target,"expected_generation":state.generation,"draft":draft,"label":"TrueType instructions"});
    api.call("hints.tt_apply", params.clone()).unwrap();
    assert!(api.call("hints.tt_apply", params).is_err());
    let after_state = self::state(&api, &target);
    assert!(after_state.authored && after_state.binding_matches);
    assert_eq!(
        after_state.program.bytes,
        fc_hint::tt::assemble(&draft.program).unwrap()
    );
    session
        .with_doc_mut(doc, |doc| {
            let after = doc.font.clone();
            doc.undo().unwrap();
            assert_eq!(doc.font, before);
            doc.redo().unwrap();
            assert_eq!(doc.font, after);
        })
        .unwrap();
}

#[test]
fn invalid_binding_assembly_and_globals_never_partially_apply() {
    let (api, session, doc, target, before) = setup();
    let state = state(&api, &target);
    for issue in 0..4 {
        let mut draft = draft(&state, 0);
        match issue {
            0 => draft.fingerprint = format!("tt-points-v1:{}", "0".repeat(64)),
            1 => draft.program = "PUSHB[0]".into(),
            2 => draft.prep = "NOT_AN_OPCODE".into(),
            _ => draft.globals.max_zones = Some(3),
        }
        assert!(api.call("hints.tt_apply", json!({"target":target,"expected_generation":state.generation,"draft":draft,"label":"Invalid"})).is_err());
        session
            .with_doc(doc, |doc| {
                assert_eq!(doc.font, before);
                assert_eq!(doc.generation, state.generation);
            })
            .unwrap();
    }
}

#[test]
fn phantom_preview_uses_interpreted_advance_and_vm_failures_do_not_fall_back() {
    let (api, session, doc, mut target, before) = setup();
    target["glyph"] = json!("space");
    let state = state(&api, &target);
    assert!(state.points.is_empty());
    let mut draft = draft(&state, state.phantom_start + 1);
    let params = |draft: &TtHintDraft, sizes: Vec<u16>| json!({"target":target,"expected_generation":state.generation,"draft":draft,"ppems":sizes,"mono":true});
    let preview = api
        .call("hints.tt_preview", params(&draft, vec![10]))
        .unwrap();
    assert_eq!(preview["sizes"][0]["hinted_advance"], 4.0);
    assert_eq!(preview["sizes"][0]["unhinted_advance"], 3.0);
    assert_eq!(preview["sizes"][0]["hinted"], "");
    for source in ["POP", "PUSHW[0] -3\nJMPR"] {
        draft.program = source.into();
        assert!(
            api.call("hints.tt_preview", params(&draft, vec![10]))
                .is_err(),
            "{source}"
        );
    }
    draft.program.clear();
    for field in ["fpgm", "prep"] {
        let mut invalid = draft.clone();
        if field == "fpgm" {
            invalid.fpgm = "PUSHB[0] 99\nCALL".into();
        } else {
            invalid.prep = "PUSHB[0] 99\nCALL".into();
        }
        assert!(
            api.call("hints.tt_preview", params(&invalid, vec![10]))
                .is_err(),
            "{field}"
        );
    }
    for sizes in [vec![], vec![0], vec![129], vec![10; 9]] {
        assert!(api.call("hints.tt_preview", params(&draft, sizes)).is_err());
    }
    session
        .with_doc(doc, |doc| assert_eq!(doc.font, before))
        .unwrap();
}

#[test]
fn authored_api_edits_save_reopen_export_and_fail_without_replacing_destination() {
    let (api, session, doc, target, _) = setup();
    let directory = tempfile::tempdir().unwrap();
    api.configure_config_dir(directory.path().join("config"))
        .unwrap();
    let state = state(&api, &target);
    let draft = draft(&state, 0);
    api.call("hints.tt_apply", json!({"target":target,"expected_generation":state.generation,"draft":draft,"label":"TrueType"})).unwrap();
    let path = directory.path().join("authored.fcf.gz");
    api.call("session.save", json!({"doc":doc,"path":path}))
        .unwrap();
    let reopened = api.call("session.open", json!({"path":path})).unwrap();
    let restored = self::state(
        &api,
        &json!({"doc":reopened["doc"],"master":target["master"],"glyph":"I"}),
    );
    assert!(restored.authored && restored.binding_matches);
    assert_eq!(
        restored.program.bytes,
        fc_hint::tt::assemble(&draft.program).unwrap()
    );
    for format in ["Ttf", "Woff", "Woff2"] {
        let path = directory.path().join(format);
        api.call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":format,"keep_tt_instructions":false}}),
        )
        .unwrap();
        let result =
            fc_sfnt::read_bytes(&std::fs::read(path).unwrap(), &Default::default()).unwrap();
        assert_eq!(
            result.fonts[0].glyph("I").unwrap().layers[0].tt_instructions,
            Some(restored.program.bytes.clone())
        );
    }
    session
        .with_doc_mut(doc, |doc| {
            doc.font.glyph_mut("I").unwrap().layers[0].width += 20.0;
        })
        .unwrap();
    let path = directory.path().join("existing.ttf");
    std::fs::write(&path, b"previous font").unwrap();
    let error = api
        .call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":"Ttf"}}),
        )
        .unwrap_err();
    assert!(error.message.contains("stale"), "{error}");
    assert_eq!(std::fs::read(path).unwrap(), b"previous font");
}

#[test]
fn malformed_and_stale_stored_programs_remain_visible_without_breaking_ps_state() {
    let (api, session, doc, target, _) = setup();
    let current = state(&api, &target);
    session
        .with_doc_mut(doc, |doc| {
            let layer = &mut doc.font.glyph_mut("I").unwrap().layers[0];
            layer.tt_instructions = Some(vec![0xb0]);
            layer.tt_binding = Some(TtInstructionBinding {
                fingerprint: current.fingerprint.clone(),
                authored: false,
            });
            doc.font.masters[0].tt.prep = vec![0xb8];
        })
        .unwrap();
    let malformed = state(&api, &target);
    assert!(malformed.binding_matches);
    assert!(malformed.program.error.is_some() && malformed.prep.error.is_some());
    assert_eq!(malformed.program.bytes, vec![0xb0]);
    assert!(api
        .call(
            "hints.state",
            json!({"doc":doc,"master":target["master"],"glyph":"I","layer":null})
        )
        .is_ok());
    session
        .with_doc_mut(doc, |doc| {
            let layer = &mut doc.font.glyph_mut("I").unwrap().layers[0];
            layer.width += 25.0;
            layer.tt_binding.as_mut().unwrap().authored = true;
        })
        .unwrap();
    let stale = state(&api, &target);
    assert!(stale.authored && !stale.binding_matches);
    assert_eq!(stale.program.bytes, vec![0xb0]);
}
