use std::sync::Arc;

use fc_api::{Api, ApplyParams, ExportReport, GlyphCell, LayerOutline};
use fc_doc::{Command, HandleMode, PointRef, Session};
use read_fonts::FontRef;
use serde_json::json;

fn editable_glyph(api: &Api, doc: fc_doc::DocId, name: &str) -> (GlyphCell, LayerOutline) {
    let cells: Vec<GlyphCell> = serde_json::from_value(
        api.call(
            "glyphs.search",
            json!({ "doc": doc, "query": name, "limit": 8 }),
        )
        .expect("search glyphs"),
    )
    .expect("decode search result");
    let cell = cells
        .into_iter()
        .find(|cell| cell.name == name)
        .unwrap_or_else(|| panic!("missing glyph {name}"));
    let outline = serde_json::from_value(
        api.call("glyphs.outline", json!({ "doc": doc, "gid": cell.gid }))
            .expect("read outline"),
    )
    .expect("decode outline");
    (cell, outline)
}

#[test]
#[ignore = "release acceptance workflow"]
fn edits_three_dejavu_glyphs_and_preserves_layout_on_export() {
    if cfg!(debug_assertions) {
        panic!("acceptance workflow must be run with --release");
    }
    let directory = tempfile::tempdir().expect("temporary directory");
    let original_path = fc_fixtures::path("DejaVuSans.ttf");
    let original_bytes = std::fs::read(&original_path).expect("read DejaVu Sans");
    let output_path = directory.path().join("EditedDejaVuSans.ttf");
    let session = Arc::new(Session::new());
    let api = Api::new(Arc::clone(&session));
    api.configure_config_dir(directory.path().join("config"))
        .expect("configure API");
    let summary = api
        .call(
            "session.open",
            json!({ "path": original_path, "face_index": null }),
        )
        .expect("open DejaVu Sans");
    let doc: fc_doc::DocId = serde_json::from_value(summary["doc"].clone()).expect("document id");

    let mut expected_points = Vec::new();
    for name in ["A", "B", "C"] {
        let (_, outline) = editable_glyph(&api, doc, name);
        let first = &outline.contours[0].points[0];
        expected_points.push((name, first.x + 16.0, first.y + 8.0));
        api.call(
            "doc.apply",
            serde_json::to_value(ApplyParams {
                doc,
                command: Command::MovePoints {
                    glyph: name.into(),
                    layer: outline.layer,
                    points: vec![PointRef {
                        contour: 0,
                        point: 0,
                    }],
                    delta: (16.0, 8.0),
                    handles: HandleMode::PointsOnly,
                },
                label: format!("Move {name}"),
            })
            .expect("serialize edit"),
        )
        .expect("edit glyph");
    }
    let invalidated_glyphs = session
        .with_doc(doc, |document| {
            for name in ["A", "B", "C"] {
                let glyph = document.glyph_id(name).expect("edited glyph handle");
                assert!(
                    document.glyph_generation(glyph) > 0,
                    "{name} was not edited"
                );
            }
            document
                .font
                .glyph_order()
                .filter(|name| {
                    document
                        .glyph_id(name)
                        .is_some_and(|glyph| document.glyph_generation(glyph) > 0)
                })
                .count()
        })
        .expect("read edit generations");

    let report: ExportReport = serde_json::from_value(
        api.call(
            "session.export",
            json!({
                "doc": doc,
                "path": output_path,
                "options": { "format": "Ttf" }
            }),
        )
        .expect("export edited font"),
    )
    .expect("decode export report");
    assert_eq!(report.touched_glyphs as usize, invalidated_glyphs);
    assert!(invalidated_glyphs >= 3);
    let hint_warning = report
        .warnings
        .iter()
        .find(|warning| warning.code == "hinting-dropped")
        .expect("hinting-drop warning");
    for name in ["A", "B", "C"] {
        assert!(hint_warning.message.contains(name), "{hint_warning:?}");
    }

    let output_bytes = std::fs::read(&output_path).expect("read export");
    let input = FontRef::new(&original_bytes).expect("parse original");
    let output = FontRef::new(&output_bytes).expect("parse export");
    for tag in [b"GSUB", b"GPOS", b"GDEF"] {
        let tag = read_fonts::types::Tag::new(tag);
        if let Some(expected) = input.table_data(tag) {
            assert_eq!(
                output.table_data(tag).map(|table| table.as_bytes()),
                Some(expected.as_bytes()),
                "layout table {tag} changed"
            );
        }
    }
    for tag in [b"fpgm", b"prep", b"cvt "] {
        assert!(
            output
                .table_data(read_fonts::types::Tag::new(tag))
                .is_none(),
            "hinting table survived an outline edit"
        );
    }
    assert_eq!(
        fc_compile::shaping_fingerprint(&original_bytes, "Hello fi AVWA").expect("shape original"),
        fc_compile::shaping_fingerprint(&output_bytes, "Hello fi AVWA").expect("shape export")
    );

    let reopened = fc_sfnt::read_path(&output_path, &fc_sfnt::ReadOptions::default())
        .expect("reopen exported font")
        .fonts
        .remove(0);
    for (name, x, y) in expected_points {
        let layer = reopened
            .glyph(name)
            .and_then(|glyph| glyph.master_layer(&reopened.default_master().id))
            .unwrap_or_else(|| panic!("missing reopened glyph {name}"));
        assert!(
            layer
                .contours
                .iter()
                .flat_map(|contour| &contour.points)
                .any(|point| (point.x - x).abs() < 0.01 && (point.y - y).abs() < 0.01),
            "edited coordinate missing from reopened {name}"
        );
    }
}
