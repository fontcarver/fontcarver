//! Acceptance of the bounded FEA patterns emitted by the visual lookup forms.
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use fc_api::{Api, ShapedRun};
use fc_doc::{DocId, Session};
use fc_model::{Font, Glyph, Layer};
use serde_json::{json, Value};

struct LookupCase {
    name: &'static str,
    table: &'static str,
    kind: u16,
    tag: &'static str,
    code: &'static str,
    text: &'static str,
    // Glyph name, x/y advances, x/y offsets, in font units.
    expected: &'static [(&'static str, f64, f64, f64, f64)],
}

const CASES: &[LookupCase] = &[
    LookupCase {
        name: "single-substitution",
        table: "GSUB",
        kind: 1,
        tag: "smcp",
        code: r"sub \a by \a.alt;",
        text: "a",
        expected: &[("a.alt", 500.0, 0.0, 0.0, 0.0)],
    },
    LookupCase {
        name: "multiple-substitution",
        table: "GSUB",
        kind: 2,
        tag: "ss01",
        code: r"sub \a by \b \c;",
        text: "a",
        expected: &[("b", 500.0, 0.0, 0.0, 0.0), ("c", 500.0, 0.0, 0.0, 0.0)],
    },
    LookupCase {
        name: "alternate-substitution",
        table: "GSUB",
        kind: 3,
        tag: "salt",
        code: r"sub \a from [\b \c];",
        text: "a",
        expected: &[("b", 500.0, 0.0, 0.0, 0.0)],
    },
    LookupCase {
        name: "ligature-substitution",
        table: "GSUB",
        kind: 4,
        tag: "liga",
        code: r"sub \f \i by \f_i;",
        text: "fi",
        expected: &[("f_i", 540.0, 0.0, 0.0, 0.0)],
    },
    LookupCase {
        name: "contextual-substitution",
        table: "GSUB",
        kind: 6,
        tag: "calt",
        code: r"sub \a \b' \c by \b.alt;",
        text: "abc",
        expected: &[
            ("a", 500.0, 0.0, 0.0, 0.0),
            ("b.alt", 500.0, 0.0, 0.0, 0.0),
            ("c", 500.0, 0.0, 0.0, 0.0),
        ],
    },
    LookupCase {
        name: "single-positioning",
        table: "GPOS",
        kind: 1,
        tag: "dist",
        code: r"pos \A <12 -18 25 7>;",
        text: "A",
        expected: &[("A", 625.0, 0.0, 12.0, -18.0)],
    },
    LookupCase {
        name: "pair-positioning",
        table: "GPOS",
        kind: 2,
        tag: "kern",
        code: r"pos \A <0 0 -80 0> \V <3 -4 15 6>;",
        text: "AV",
        expected: &[("A", 520.0, 0.0, 0.0, 0.0), ("V", 615.0, 0.0, 3.0, -4.0)],
    },
    LookupCase {
        name: "mark-to-base-positioning",
        table: "GPOS",
        kind: 4,
        tag: "mark",
        code: "markClass \\acute <anchor 0 100> @visual_base;\n\
               pos base \\A <anchor 200 700> mark @visual_base;",
        text: "A\u{301}",
        expected: &[
            ("A", 600.0, 0.0, 0.0, 0.0),
            ("acute", 0.0, 0.0, -400.0, 600.0),
        ],
    },
    LookupCase {
        name: "mark-to-mark-positioning",
        table: "GPOS",
        kind: 6,
        tag: "mkmk",
        code: "markClass \\acute <anchor 0 100> @visual_stack;\n\
               pos mark \\grave <anchor 200 700> mark @visual_stack;",
        text: "\u{300}\u{301}",
        expected: &[
            ("grave", 0.0, 0.0, 0.0, 0.0),
            ("acute", 0.0, 0.0, 200.0, 600.0),
        ],
    },
];

fn setup() -> (Arc<Session>, Api, DocId) {
    let mut font = Font::new_empty("Visual lookup acceptance", 1000);
    let master = font.default_master().id.clone();
    for (name, unicode, width) in [
        (".notdef", None, 500.0),
        ("a", Some(0x61), 500.0),
        ("a.alt", None, 500.0),
        ("b", Some(0x62), 500.0),
        ("b.alt", None, 500.0),
        ("c", Some(0x63), 500.0),
        ("f", Some(0x66), 300.0),
        ("i", Some(0x69), 280.0),
        ("f_i", None, 540.0),
        ("A", Some(0x41), 600.0),
        ("V", Some(0x56), 600.0),
        ("acute", Some(0x301), 0.0),
        ("grave", Some(0x300), 0.0),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(Arc::clone(&session));
    (session, api, doc)
}

fn state(api: &Api, doc: DocId) -> Value {
    api.call("features.get", json!({"doc": doc})).unwrap()
}

fn source(cases: &[LookupCase]) -> Value {
    json!({"features": cases.iter().map(|case| json!({
        "id": case.name, "tag": case.tag, "code": case.code
    })).collect::<Vec<_>>()})
}

fn draft(api: &Api, doc: DocId, source: Value) -> Value {
    let before = state(api, doc);
    api.call(
        "features.set_draft",
        json!({"doc": doc, "source": source,
            "settings": before["active_settings"],
            "expected_generation": before["generation"],
            "expected_draft_revision": before["draft_revision"]}),
    )
    .unwrap()
}

fn revisions(doc: DocId, state: &Value) -> Value {
    json!({"doc": doc, "expected_generation": state["generation"],
        "expected_compile_revision": state["compile_revision"],
        "expected_draft_revision": state["draft_revision"]})
}

fn apply(api: &Api, doc: DocId, draft: &Value) -> Value {
    let validation = api
        .call("features.validate", revisions(doc, draft))
        .unwrap();
    assert_eq!(validation["valid"], true, "{validation}");
    let result = api.call("features.apply", revisions(doc, draft)).unwrap();
    assert_eq!(result["applied"], true, "{result}");
    result["state"].clone()
}

fn shape(api: &Api, doc: DocId, case: &LookupCase, enabled: bool) -> ShapedRun {
    shape_direction(api, doc, case, enabled, "ltr")
}

fn shape_direction(
    api: &Api,
    doc: DocId,
    case: &LookupCase,
    enabled: bool,
    direction: &str,
) -> ShapedRun {
    let features: Vec<_> = CASES
        .iter()
        .map(|other| (other.tag, enabled && other.tag == case.tag))
        .collect();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let run: ShapedRun = serde_json::from_value(
            api.call(
                "text.shape",
                json!({"doc": doc, "text": case.text, "features": features, "direction": direction}),
            )
            .unwrap(),
        )
        .unwrap();
        if !run.stale {
            assert!(run.warnings.is_empty(), "{}: {run:?}", case.name);
            return run;
        }
        assert!(Instant::now() < deadline, "{}: {run:?}", case.name);
        thread::sleep(Duration::from_millis(20));
    }
}

fn assert_shaping(api: &Api, doc: DocId, case: &LookupCase) {
    let run = shape(api, doc, case, true);
    assert_eq!(run.names.len(), case.expected.len(), "{}", case.name);
    for (index, (name, x_advance, y_advance, x_offset, y_offset)) in
        case.expected.iter().enumerate()
    {
        assert_eq!(run.names[index].as_deref(), Some(*name), "{}", case.name);
        let glyph = &run.glyphs[index];
        assert_eq!(glyph.x_advance, *x_advance, "{}: {run:?}", case.name);
        assert_eq!(glyph.y_advance, *y_advance, "{}: {run:?}", case.name);
        assert_eq!(glyph.x_offset, *x_offset, "{}: {run:?}", case.name);
        assert_eq!(glyph.y_offset, *y_offset, "{}: {run:?}", case.name);
    }
}

fn assert_case(case: &LookupCase) {
    let (session, api, doc) = setup();
    let baseline = shape(&api, doc, case, false);
    let candidate = draft(&api, doc, source(std::slice::from_ref(case)));
    apply(&api, doc, &candidate);
    assert_shaping(&api, doc, case);
    let disabled = shape(&api, doc, case, false);
    assert_eq!(disabled.names, baseline.names, "{}", case.name);
    if case.table != "GPOS" || case.kind != 4 {
        assert_eq!(disabled.glyphs, baseline.glyphs, "{}", case.name);
    }
    // HarfRust enables fallback mark positioning only when GPOS is absent.
    // Installing GPOS changes that baseline even with `mark=0`. Compare the
    // updated cache with a fresh full compilation of the same source instead.
    let font = session.with_doc(doc, |doc| doc.font.clone()).unwrap();
    let fresh_session = Arc::new(Session::new());
    let fresh_doc = fresh_session.open_font(font.clone(), None);
    let fresh_api = Api::new(fresh_session);
    let full_disabled = shape(&fresh_api, fresh_doc, case, false);
    assert_eq!(disabled.names, full_disabled.names, "{}", case.name);
    assert_eq!(disabled.glyphs, full_disabled.glyphs, "{}", case.name);
    assert_ne!(
        shape(&api, doc, case, true).glyphs,
        disabled.glyphs,
        "{}",
        case.name
    );

    if case.table == "GPOS" && matches!(case.kind, 1 | 2) {
        // ValueRecord XAdvance applies only horizontally, YAdvance only
        // vertically. Test the latter against feature-off in the same font;
        // this keeps all four authored values covered without assuming a
        // particular synthetic vertical metric for these outline-free glyphs.
        let before = shape_direction(&api, doc, case, false, "ttb");
        let after = shape_direction(&api, doc, case, true, "ttb");
        assert_eq!(before.names, after.names);
        for (index, (off, on)) in before.glyphs.iter().zip(&after.glyphs).enumerate() {
            let (_, _, _, x_offset, y_offset) = case.expected[index];
            let y_advance_delta = if case.kind == 1 {
                -7.0
            } else if index == 1 {
                -6.0
            } else {
                0.0
            };
            assert_eq!(on.x_advance, off.x_advance, "{}", case.name);
            assert_eq!(
                on.y_advance - off.y_advance,
                y_advance_delta,
                "{}",
                case.name
            );
            assert_eq!(on.x_offset - off.x_offset, x_offset, "{}", case.name);
            assert_eq!(on.y_offset - off.y_offset, y_offset, "{}", case.name);
        }
    }

    let bytes = fc_compile::compile_static(
        &font,
        &font.default_master().id,
        &fc_compile::CompileOptions::default(),
    )
    .unwrap();
    let imported = fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let inventory = fc_layout::LayoutImport::from_font(&imported);
    assert!(
        inventory
            .lookups
            .iter()
            .any(|lookup| lookup.table == case.table && lookup.kind == case.kind),
        "{} did not compile to {}{}: {:?}",
        case.name,
        case.table,
        case.kind,
        inventory.lookups
    );
}

#[test]
fn visual_gsub_single_compiles_and_shapes() {
    assert_case(&CASES[0]);
}

#[test]
fn visual_gsub_multiple_compiles_and_shapes() {
    assert_case(&CASES[1]);
}

#[test]
fn visual_gsub_alternate_compiles_and_shapes() {
    assert_case(&CASES[2]);
}

#[test]
fn visual_gsub_ligature_compiles_and_shapes() {
    assert_case(&CASES[3]);
}

#[test]
fn visual_gsub_contextual_compiles_and_shapes() {
    assert_case(&CASES[4]);
}

#[test]
fn visual_gpos_single_compiles_and_shapes() {
    assert_case(&CASES[5]);
}

#[test]
fn visual_gpos_pair_compiles_and_shapes() {
    assert_case(&CASES[6]);
}

#[test]
fn visual_gpos_mark_to_base_compiles_and_shapes() {
    assert_case(&CASES[7]);
}

#[test]
fn visual_gpos_mark_to_mark_compiles_and_shapes() {
    assert_case(&CASES[8]);
}

#[test]
fn visual_class_substitution_compiles_and_shapes_every_member() {
    assert_case(&LookupCase {
        code: r"sub [\a \b] by [\a.alt \b.alt];",
        text: "ab",
        expected: &[
            ("a.alt", 500.0, 0.0, 0.0, 0.0),
            ("b.alt", 500.0, 0.0, 0.0, 0.0),
        ],
        ..CASES[0]
    });
}

#[test]
fn visual_class_positioning_compiles_and_shapes_every_member() {
    assert_case(&LookupCase {
        code: r"pos [\A \V] <12 -18 25 7>;",
        text: "AV",
        expected: &[
            ("A", 625.0, 0.0, 12.0, -18.0),
            ("V", 625.0, 0.0, 12.0, -18.0),
        ],
        ..CASES[5]
    });
}

#[test]
fn all_visual_rules_survive_apply_undo_redo_and_native_save_with_invalid_draft() {
    let (session, api, doc) = setup();
    let baselines: Vec<_> = CASES
        .iter()
        .map(|case| shape(&api, doc, case, true))
        .collect();
    let candidate = draft(&api, doc, source(CASES));
    let active = apply(&api, doc, &candidate);
    for case in CASES {
        assert_shaping(&api, doc, case);
    }

    api.call("doc.undo", json!({"doc": doc})).unwrap();
    let undone = state(&api, doc);
    assert_eq!(undone["active_source"], candidate["active_source"]);
    assert_eq!(undone["draft"]["source"], candidate["draft"]["source"]);
    for (case, baseline) in CASES.iter().zip(baselines) {
        let undone_run = shape(&api, doc, case, true);
        assert_eq!(undone_run.names, baseline.names, "{}", case.name);
        assert_eq!(undone_run.glyphs, baseline.glyphs, "{}", case.name);
    }
    api.call("doc.redo", json!({"doc": doc})).unwrap();
    let redone = state(&api, doc);
    assert_eq!(redone["active_source"], active["active_source"]);
    assert_eq!(redone["draft"], Value::Null);
    for case in CASES {
        assert_shaping(&api, doc, case);
    }

    let mut broken = source(CASES);
    broken["features"][4]["code"] = json!(r"sub \a \b' \c by \missing;");
    let invalid = draft(&api, doc, broken);
    let undo_len = session.with_doc(doc, |doc| doc.undo_len()).unwrap();
    let result = api
        .call("features.apply", revisions(doc, &invalid))
        .unwrap();
    assert_eq!(result["applied"], false, "{result}");
    assert_eq!(result["validation"]["valid"], false, "{result}");
    assert_eq!(state(&api, doc), invalid);
    assert_eq!(invalid["active_source"], active["active_source"]);
    assert_eq!(invalid["compile_revision"], redone["compile_revision"]);
    assert_eq!(
        session.with_doc(doc, |doc| doc.undo_len()).unwrap(),
        undo_len
    );
    for case in CASES {
        assert_shaping(&api, doc, case);
    }

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("visual-lookups.fcf");
    api.call("session.save", json!({"doc": doc, "path": path}))
        .unwrap();
    let reopened = session.open_path(&path, None).unwrap();
    let restored = state(&api, reopened);
    assert_eq!(restored["active_source"], invalid["active_source"]);
    assert_eq!(restored["draft"]["source"], invalid["draft"]["source"]);
    assert_eq!(restored["has_unapplied_changes"], true);
    for case in CASES {
        assert_shaping(&api, reopened, case);
    }
}
