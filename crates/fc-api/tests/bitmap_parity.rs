use fc_api::Api;
use fc_doc::Session;
use fc_model::{Bitmap, BitmapGlyph, BitmapStrike, Font, Glyph, Layer};
use serde_json::json;
use std::sync::Arc;

fn fixture() -> (Arc<Session>, fc_doc::DocId, Api) {
    let mut font = Font::new_empty("Pixel parity", 1000);
    let master = font.default_master().id.clone();
    for name in ["A", "B"] {
        let mut glyph = Glyph::new(name);
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    let mut bitmap = Bitmap::new(2, 2, 1).unwrap();
    bitmap.origin_x = -1;
    bitmap.origin_y = -1;
    bitmap.set_pixel(0, 0, 1).unwrap();
    bitmap.set_pixel(1, 1, 1).unwrap();
    let mut strike = BitmapStrike {
        ppem: 10,
        depth: 1,
        ascent: 8,
        descent: 2,
        ..Default::default()
    };
    strike.glyphs.insert(
        "A".into(),
        BitmapGlyph {
            bitmap,
            advance: 3,
            vertical_advance: Some(4),
            vertical_origin: Some((1, 2)),
            ..Default::default()
        },
    );
    font.bitmap_strikes.push(strike);
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    (session, doc, api)
}
fn generation(api: &Api, doc: fc_doc::DocId) -> serde_json::Value {
    api.call("bitmap.state", json!({"doc":doc})).unwrap()["generation"].clone()
}
#[test]
fn cross_strike_rescale_preserves_y_up_origins_metrics_and_undo() {
    let (session, doc, api) = fixture();
    let params = json!({"doc":doc,"expected_generation":generation(&api,doc),"source_ppem":10,"source_depth":1,"target_ppem":20,"target_depth":8,"glyphs":["A"],"scale":true});
    api.call("bitmap.transfer", params.clone()).unwrap();
    assert!(api.call("bitmap.transfer", params).is_err());
    session
        .with_doc(doc, |d| {
            let strike = &d.font.bitmap_strikes[1];
            let value = &strike.glyphs["A"];
            assert_eq!((strike.ascent, strike.descent), (16, 4));
            assert_eq!((value.bitmap.width, value.bitmap.height), (4, 4));
            assert_eq!((value.bitmap.origin_x, value.bitmap.origin_y), (-2, -2));
            assert_eq!(value.advance, 6);
            assert_eq!(value.vertical_advance, Some(8));
            assert_eq!(value.vertical_origin, Some((2, 4)));
            for y in 0..4 {
                for x in 0..4 {
                    assert_eq!(
                        value.bitmap.pixel(x, y),
                        if (x < 2) == (y < 2) { 255 } else { 0 }
                    );
                }
            }
            assert_eq!(d.undo_len(), 1);
        })
        .unwrap();
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    assert_eq!(
        api.call("bitmap.state", json!({"doc":doc})).unwrap()["strikes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bitmap.fcf");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Fcf"}}),
    )
    .unwrap();
    let reopened = fc_doc::fcf::read(&path).unwrap();
    assert_eq!(
        reopened.bitmap_strikes[1].glyphs["A"].bitmap.pixel(3, 3),
        255
    );
}
#[test]
fn transfer_failure_and_sheet_batch_are_atomic() {
    let (session, doc, api) = fixture();
    assert!(api.call("bitmap.transfer",json!({"doc":doc,"expected_generation":generation(&api,doc),"source_ppem":10,"source_depth":1,"target_ppem":20,"target_depth":1,"glyphs":["A","B"],"scale":true})).is_err());
    session
        .with_doc(doc, |d| assert_eq!(d.undo_len(), 0))
        .unwrap();
    let value = api
        .call(
            "bitmap.glyph",
            json!({"doc":doc,"ppem":10,"depth":1,"glyph":"A"}),
        )
        .unwrap()["value"]
        .clone();
    api.call("bitmap.apply",json!({"doc":doc,"expected_generation":generation(&api,doc),"label":"Import sheet","command":{"SetGlyphs":{"ppem":10,"depth":1,"glyphs":{"A":value,"B":value}}}})).unwrap();
    session
        .with_doc(doc, |d| {
            assert_eq!(d.font.bitmap_strikes[0].glyphs.len(), 2)
        })
        .unwrap();
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    session
        .with_doc(doc, |d| {
            assert_eq!(d.font.bitmap_strikes[0].glyphs.len(), 1)
        })
        .unwrap();
}
#[test]
fn pixel_components_reuse_editable_pixel_glyph_and_undo_together() {
    let (session, doc, api) = fixture();
    api.call("bitmap.pixel_components",json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"glyphs":["A"],"master":null,"threshold":128,"pixel_glyph":"pixel"})).unwrap();
    session
        .with_doc(doc, |d| {
            let master = &d.font.default_master().id;
            let layer = d.font.glyph("A").unwrap().master_layer(master).unwrap();
            assert!(layer.contours.is_empty());
            assert_eq!(layer.width, 300.0);
            assert_eq!(layer.components.len(), 2);
            assert!(layer.components.iter().all(|c| c.base.as_ref() == "pixel"));
            assert_eq!(
                layer.components[0].transform.as_coeffs(),
                [0.1, 0.0, 0.0, 0.1, -100.0, 0.0]
            );
            assert_eq!(
                layer.components[1].transform.as_coeffs(),
                [0.1, 0.0, 0.0, 0.1, 0.0, -100.0]
            );
            let pixel = d.font.glyph("pixel").unwrap().master_layer(master).unwrap();
            assert_eq!(pixel.contours.len(), 1);
            assert_eq!(pixel.contours[0].points[2].x, 1000.0);
            assert_eq!(pixel.contours[0].points[2].y, 1000.0);
            assert_eq!(d.undo_len(), 1);
        })
        .unwrap();
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    session
        .with_doc(doc, |d| assert!(d.font.glyph("pixel").is_none()))
        .unwrap();
    api.call("doc.redo", json!({"doc":doc})).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("components.fcf");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Fcf"}}),
    )
    .unwrap();
    let reopened = fc_doc::fcf::read(&path).unwrap();
    assert_eq!(
        reopened
            .glyph("A")
            .unwrap()
            .master_layer(&reopened.default_master().id)
            .unwrap()
            .components
            .len(),
        2
    );
}

#[test]
fn sfnt_pixel_fonts_are_detected_reconstructed_and_undoable_after_import() {
    let mut font = Font::new_empty("Imported pixel outlines", 1000);
    let master = font.default_master().id.clone();
    // Keep every exported glyph on the intended 100-unit grid. Without an
    // explicit .notdef, the compiler synthesizes a fallback with 50-unit insets;
    // that mixed-grid font is deliberately outside the detector's contract.
    let mut notdef = Glyph::new(".notdef");
    let mut notdef_layer = Layer::new_master(master.clone());
    notdef_layer.width = 500.0;
    notdef.layers.push(notdef_layer);
    font.insert_glyph(notdef).unwrap();
    let masks = [
        [
            0b1100000, 0b0110000, 0b0011000, 0b0001100, 0b0000110, 0b0000011, 0b0000001,
        ],
        [
            0b0000011, 0b0000110, 0b0001100, 0b0011000, 0b0110000, 0b1100000, 0b1000000,
        ],
        [
            0b1110000, 0b0111000, 0b0011100, 0b0001110, 0b0000111, 0b0000011, 0b0000001,
        ],
    ];
    let mut originals = Vec::new();
    for ((name, unicode), rows) in [("A", 65), ("B", 66), ("C", 67)].into_iter().zip(masks) {
        let mut bitmap = Bitmap::new(7, 7, 1).unwrap();
        bitmap.origin_x = -2;
        bitmap.origin_y = -1;
        for (y, row) in rows.into_iter().enumerate() {
            for x in 0..7 {
                bitmap.set_pixel(x, y as u16, (row >> (6 - x)) & 1).unwrap();
            }
        }
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 1000.0;
        layer.contours = fc_raster::trace_bitmap(&bitmap, 10, 1000, 128).unwrap();
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
        originals.push((name, bitmap));
    }
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    let directory = tempfile::tempdir().unwrap();
    for format in ["Ttf", "Otf"] {
        let path = directory
            .path()
            .join(format!("pixel.{}", format.to_lowercase()));
        api.call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":format}}),
        )
        .unwrap();
        let imported = api.call("session.open", json!({"path":path})).unwrap();
        let imported: fc_doc::DocId = serde_json::from_value(imported["doc"].clone()).unwrap();
        let detection = api
            .call(
                "bitmap.detect_pixel_grid",
                json!({"doc":imported,"master":null}),
            )
            .unwrap();
        assert_eq!(detection["ppem"], 10, "{format}");
        assert_eq!(detection["evidence_glyphs"], 3, "{format}");
        let params = json!({"doc":imported,"expected_generation":detection["generation"],"master":null,"glyphs":["A","B","C"]});
        api.call("bitmap.reconstruct", params.clone()).unwrap();
        assert!(api.call("bitmap.reconstruct", params).is_err());
        session
            .with_doc(imported, |d| {
                for (name, bitmap) in &originals {
                    assert_eq!(
                        &d.font.bitmap_strikes[0].glyphs[*name].bitmap, bitmap,
                        "{format}/{name}"
                    );
                }
                assert_eq!(d.undo_len(), 1);
            })
            .unwrap();
        api.call("doc.undo", json!({"doc":imported})).unwrap();
        session
            .with_doc(imported, |d| assert!(d.font.bitmap_strikes.is_empty()))
            .unwrap();
    }
}
