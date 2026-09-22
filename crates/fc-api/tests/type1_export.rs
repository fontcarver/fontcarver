//! Export preprocessing must preserve the document and change only native output.

use fc_api::Api;
use fc_doc::Session;
use fc_model::{Contour, Font, Glyph, Layer, MasterId, StemHint};
use kurbo::{Rect, Shape};
use serde_json::json;
use std::sync::Arc;

fn source() -> Font {
    let mut font = Font::new_empty("Type One Preparation", 1000);
    let mut other = font.default_master().clone();
    other.id = MasterId::new();
    other.name = "Alternate".into();
    font.masters.push(other);
    let mut glyph = Glyph::new("H");
    glyph.unicodes.push(72);
    for (index, master) in font.masters.iter().enumerate() {
        let offset = index as f64 * 200.0;
        let mut layer = Layer::new_master(master.id.clone());
        layer.width = 333.0 + index as f64 * 444.0;
        for rect in [
            Rect::new(50.0 + offset, 0.0, 130.0 + offset, 700.0),
            Rect::new(100.0 + offset, 0.0, 180.0 + offset, 700.0),
        ] {
            layer
                .contours
                .extend(Contour::from_bezpath(&rect.to_path(0.1)));
        }
        layer.hints.v_stems.push(StemHint {
            start: 55.0 + offset,
            width: 10.0,
            ..Default::default()
        });
        layer.tt_instructions = Some(vec![0]);
        glyph.layers.push(layer);
    }
    font.insert_glyph(glyph).unwrap();
    font
}

#[test]
fn type1_prepares_only_the_export_copy_and_selected_master() {
    let font = source();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font.clone(), None);
    let api = Api::new(session.clone());
    let dir = tempfile::tempdir().unwrap();
    for format in ["Pfa", "Pfb"] {
        for (index, master) in font.masters.iter().enumerate() {
            for (overlap, autohint) in [(false, false), (true, false), (false, true), (true, true)]
            {
                let path = dir.path().join(format);
                let report = api.call("session.export", json!({"doc":doc, "path":path,
                    "options":{"format":format,"master":master.id,"remove_overlap":overlap,"autohint":autohint}})).unwrap();
                let codes: Vec<_> = report["warnings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|w| w["code"].as_str().unwrap())
                    .collect();
                assert!(!codes.contains(&"remove-overlap-unavailable"));
                assert!(!codes.contains(&"autohint-unavailable"));
                assert!(codes.contains(&"type1-hinting-loss"));
                assert_eq!(
                    codes.contains(&"ps-hints-cleared-overlap"),
                    overlap && !autohint
                );
                let exported = fc_formats::type1::read_type1(&path).unwrap();
                let layer = &exported.glyph("H").unwrap().layers[0];
                assert_eq!(layer.width, 333.0 + index as f64 * 444.0);
                assert_eq!(layer.contours.len(), if overlap { 1 } else { 2 });
                if autohint {
                    assert!(!layer.hints.is_empty());
                    if overlap {
                        assert_eq!(
                            layer
                                .hints
                                .v_stems
                                .iter()
                                .map(|s| (s.start, s.width))
                                .collect::<Vec<_>>(),
                            [(50.0 + index as f64 * 200.0, 130.0)]
                        );
                    }
                } else if overlap {
                    assert!(layer.hints.is_empty());
                } else {
                    assert_eq!(layer.hints, font.glyph("H").unwrap().layers[index].hints);
                }
                session
                    .with_doc(doc, |document| {
                        assert_eq!(document.font, font);
                        assert_eq!(document.generation, 0);
                        assert_eq!(document.undo_len(), 0);
                    })
                    .unwrap();
            }
        }
    }
}

#[test]
fn failed_type1_preprocessing_leaves_the_destination_and_source_intact() {
    let mut font = source();
    font.glyph_mut("H").unwrap().layers[0].contours[0].points[0].x = 40000.0;
    let session = Arc::new(Session::new());
    let doc = session.open_font(font.clone(), None);
    let api = Api::new(session.clone());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.pfb");
    std::fs::write(&path, b"original").unwrap();
    assert!(api
        .call(
            "session.export",
            json!({"doc":doc,"path":path,
        "options":{"format":"Pfb","autohint":true}})
        )
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    session
        .with_doc(doc, |document| {
            assert_eq!(document.font, font);
            assert_eq!(document.generation, 0);
            assert_eq!(document.undo_len(), 0);
        })
        .unwrap();
}
