//! Export options must either have an effect or fail before replacing output.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Contour, Font, Glyph, Layer};
use kurbo::BezPath;
use serde_json::json;

fn source() -> Font {
    let mut font = Font::new_empty("Export Options", 1000);
    let mut glyph = Glyph::new("A");
    glyph.unicodes.push(65);
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 600.4;
    let mut path = BezPath::new();
    path.move_to((10.3, 0.2));
    path.curve_to((900.4, 800.2), (-400.3, 800.4), (590.6, 0.2));
    path.close_path();
    layer.contours = Contour::from_bezpath(&path);
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

#[test]
fn unsupported_export_options_fail_before_replacing_the_destination() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(source(), None);
    let api = Api::new(session);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing-output");
    for (format, option, value) in [
        ("Ufo", "remove_overlap", json!(true)),
        ("Fcf", "round_coordinates", json!(true)),
        ("Sfd", "add_extrema", json!(true)),
        ("Glyphs", "decompose_components", json!(true)),
        ("Bdf", "autohint", json!(true)),
        ("Otb", "add_extrema", json!(true)),
        ("Ttf", "subroutinize", json!(true)),
        ("Pfb", "subroutinize", json!(true)),
        ("Ufo", "table_subset", json!(["kern"])),
        ("Ttf", "bitmap_strike", json!([16, 1])),
        ("Ufo", "master_subset", json!([])),
    ] {
        std::fs::write(&path, b"previous export").unwrap();
        let mut options = json!({"format": format});
        options[option] = value;
        let error = api
            .call(
                "session.export",
                json!({"doc":doc,"path":path,"options":options}),
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_params", "{format}/{option}: {error}");
        assert!(error.message.contains(option), "{format}/{option}: {error}");
        assert_eq!(std::fs::read(&path).unwrap(), b"previous export");
    }
}

#[test]
fn geometry_flags_reach_static_and_type1_export_without_editing_the_document() {
    let font = source();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font.clone(), None);
    let api = Api::new(session.clone());
    let dir = tempfile::tempdir().unwrap();
    for (format, extension) in [("Otf", "otf"), ("Pfb", "pfb")] {
        let path = dir.path().join(format!("prepared.{extension}"));
        api.call("session.export", json!({"doc":doc,"path":path,"options":{
            "format":format,"add_extrema":true,"round_coordinates":true,"decompose_components":true
        }})).unwrap();
        let exported = if format == "Pfb" {
            fc_formats::type1::read_type1(&path).unwrap()
        } else {
            fc_sfnt::read_path(&path, &Default::default())
                .unwrap()
                .fonts
                .remove(0)
        };
        let layer = &exported.glyphs["A"].layers[0];
        assert_eq!(layer.width, 600.0);
        assert!(
            layer.contours[0].points.len() > font.glyphs["A"].layers[0].contours[0].points.len()
        );
        assert!(layer
            .contours
            .iter()
            .flat_map(|c| &c.points)
            .all(|p| p.x.fract() == 0.0 && p.y.fract() == 0.0));
        session
            .with_doc(doc, |document| {
                assert_eq!(document.font, font);
                assert_eq!(document.generation, 0);
                assert_eq!(document.undo_len(), 0);
            })
            .unwrap();
    }
}

#[test]
fn ttx_and_collection_exports_share_compilation_and_keep_existing_outputs_on_failure() {
    let session = Arc::new(Session::new());
    let first = session.open_font(source(), None);
    let mut second_font = source();
    second_font.info.family_name = "Second Face".into();
    let second = session.open_font(second_font, None);
    let api = Api::new(session.clone());
    let dir = tempfile::tempdir().unwrap();
    let xml = dir.path().join("font.ttx");
    api.call(
        "session.export",
        json!({"doc":first,"path":xml,"options":{"format":"Ttx"}}),
    )
    .unwrap();
    let parsed = fc_sfnt::ttx::read_path(&xml, &Default::default()).unwrap();
    assert!(parsed.fonts[0].glyph("A").is_some());

    let path = dir.path().join("family.ttc");
    api.call(
        "session.export",
        json!({"doc":first,"collection_docs":[second],"path":path,"options":{"format":"Ttc"}}),
    )
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(fc_sfnt::face_count(&bytes).unwrap(), 2);
    let face = fc_sfnt::read_bytes(
        &bytes,
        &fc_sfnt::ReadOptions {
            font_index: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(face.fonts[0].info.family_name, "Second Face");

    session.close(second).unwrap();
    assert!(api
        .call(
            "session.export",
            json!({"doc":first,"collection_docs":[second],"path":path,"options":{"format":"Ttc"}})
        )
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(api
        .call(
            "session.export",
            json!({"doc":first,"collection_docs":[first],"path":path,"options":{"format":"Ttf"}})
        )
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn eps_requires_an_explicit_glyph_and_preserves_the_source() {
    let font = source();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font.clone(), None);
    let api = Api::new(session.clone());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("A.eps");
    let error = api
        .call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":"Eps"}}),
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
    assert!(!path.exists());
    let report = api
        .call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":"Eps","glyph":"A"}}),
        )
        .unwrap();
    assert!(report["bytes_written"].as_u64().unwrap() > 0);
    assert!(std::fs::read_to_string(&path)
        .unwrap()
        .starts_with("%!PS-Adobe"));
    session
        .with_doc(doc, |document| assert_eq!(document.font, font))
        .unwrap();
}
