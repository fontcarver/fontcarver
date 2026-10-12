//! SVG editor transactions, native persistence and OpenType export.
use fc_api::Api;
use fc_doc::Session;
use fc_model::{SvgDocument, SvgTable};
use serde_json::{json, Value};
use std::sync::Arc;
fn setup() -> (Api, Value, SvgTable) {
    let mut font = fc_formats::ufo::read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let svg = SvgTable { documents: vec![SvgDocument {
        source: r#"<svg xmlns="http://www.w3.org/2000/svg"><path id="art" d="M50 0L300 -700L550 0Z" fill="red"/></svg>"#.into(),
        glyphs: [("A".into(), "art".into())].into(),
    }] };
    font.svg = Some(svg.clone());
    font.svg_table_owned = true;
    let session = Arc::new(Session::new());
    let doc = serde_json::to_value(session.open_font(font, None)).unwrap();
    (Api::new(session), doc, svg)
}
fn state(api: &Api, doc: &Value) -> Value {
    api.call("svg.state", json!({"doc":doc})).unwrap()
}
#[test]
fn svg_edits_are_validated_revision_checked_undoable_and_persisted() {
    let (api, doc, mut svg) = setup();
    let initial = state(&api, &doc);
    svg.documents[0].source = svg.documents[0].source.replace("red", "blue");
    let params = json!({"doc":doc, "expected_generation":initial["generation"], "svg":svg, "label":"SVG edit"});
    assert_eq!(
        api.call("svg.apply", params.clone()).unwrap()["color"],
        true
    );
    assert!(api.call("svg.apply", params).is_err());
    let edited = state(&api, &doc);
    let mut bad = svg.clone();
    bad.documents[0]
        .glyphs
        .insert("missing".into(), "art".into());
    assert!(api
        .call(
            "svg.apply",
            json!({"doc":doc, "expected_generation":edited["generation"], "svg":bad, "label":"bad"})
        )
        .is_err());
    assert_eq!(state(&api, &doc), edited);
    let dir = tempfile::tempdir().unwrap();
    for ext in ["fcf", "fcf.gz", "ufo"] {
        let path = dir.path().join(format!("edited.{ext}"));
        if ext == "ufo" {
            api.call(
                "session.export",
                json!({"doc":doc,"path":path,"options":{"format":"Ufo"}}),
            )
            .unwrap();
        } else {
            api.call("session.save", json!({"doc":doc, "path":path}))
                .unwrap();
        }
        let opened = api.call("session.open", json!({"path":path})).unwrap();
        assert_eq!(state(&api, &opened["doc"])["svg"], json!(svg));
    }
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(state(&api, &doc)["svg"], initial["svg"]);
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    assert_eq!(state(&api, &doc)["svg"], json!(svg));
    for format in ["Ttf", "Otf", "Woff", "Woff2"] {
        let path = dir.path().join(format!("color.{format}"));
        api.call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":format}}),
        )
        .unwrap();
        let font = fc_sfnt::read_path(&path, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let svg = font.svg.unwrap();
        assert!(svg.documents[0].source.contains("blue"));
        assert!(svg.documents[0].glyphs.contains_key("A"));
    }
    api.call("svg.apply", json!({"doc":doc,"expected_generation":state(&api,&doc)["generation"],"svg":null,"label":"Delete SVG"})).unwrap();
    assert_eq!(state(&api, &doc)["status"], "Deleted");
    let path = dir.path().join("deleted.ttf");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Ttf"}}),
    )
    .unwrap();
    assert!(fc_sfnt::read_path(&path, &Default::default())
        .unwrap()
        .fonts[0]
        .svg
        .is_none());
}
#[test]
fn names_duplication_deletion_and_undo_keep_svg_bindings() {
    let (api, doc, _) = setup();
    for command in [
        json!({"RenameGlyph":{"from":"A","to":"new.A"}}),
        json!({"DuplicateGlyph":{"source":"new.A","name":"copy.A"}}),
    ] {
        api.call(
            "doc.apply",
            json!({"doc":doc,"command":command,"label":"structure"}),
        )
        .unwrap();
    }
    assert!(state(&api, &doc)["svg"]["documents"][0]["glyphs"]["copy.A"].is_string());
    api.call(
        "doc.apply",
        json!({"doc":doc,"command":{"RemoveGlyphs":{"names":["new.A"]}},"label":"delete"}),
    )
    .unwrap();
    assert!(state(&api, &doc)["svg"]["documents"][0]["glyphs"]["new.A"].is_null());
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert!(state(&api, &doc)["svg"]["documents"][0]["glyphs"]["new.A"].is_string());
}

#[test]
fn legacy_svg_conversion_and_unsupported_raw_tables_are_atomic() {
    let mut source = fc_formats::ufo::read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let bytes = fc_compile::export(&source, &|_| 0, &Default::default())
        .unwrap()
        .bytes;
    source = fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let order: Vec<_> = source.glyph_order().cloned().collect();
    let svg = SvgTable { documents: vec![SvgDocument {
        source: r#"<svg xmlns="http://www.w3.org/2000/svg" id="art"><rect width="50" height="50"/></svg>"#.into(),
        glyphs: [("A".into(), "art".into())].into(),
    }] };
    let raw = fc_sfnt::svg::write_table(&svg, &order).unwrap();
    for malformed in [false, true] {
        let mut font = source.clone();
        let mut table = raw.clone();
        if malformed {
            table[1] = 1;
        }
        font.extra
            .raw_tables
            .insert(fc_model::Tag(*b"SVG "), table.clone());
        let session = Arc::new(Session::new());
        let doc = json!(session.open_font(font, None));
        let api = Api::new(session);
        let before = state(&api, &doc);
        assert_eq!(before["status"], "Imported");
        let params =
            json!({"doc": doc, "expected_generation":before["generation"], "label":"Decode"});
        if malformed {
            assert!(before["import_error"].is_string());
            assert!(api.call("svg.decode_imported", params).is_err());
            assert_eq!(state(&api, &doc), before);
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("preserved.ttf");
            api.call(
                "session.export",
                json!({"doc":doc,"path":path,"options":{"format":"Ttf"}}),
            )
            .unwrap();
            let bytes = std::fs::read(path).unwrap();
            assert_eq!(
                read_fonts::FontRef::new(&bytes)
                    .unwrap()
                    .table_data(read_fonts::types::Tag::new(b"SVG "))
                    .unwrap()
                    .as_bytes(),
                table
            );
        } else {
            api.call("svg.decode_imported", params.clone()).unwrap();
            assert_eq!(state(&api, &doc)["status"], "Editable");
            assert!(api.call("svg.decode_imported", params).is_err());
            api.call("doc.undo", json!({"doc":doc})).unwrap();
            assert_eq!(state(&api, &doc)["status"], "Imported");
        }
    }
}
