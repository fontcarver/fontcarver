use fc_api::Api;
use fc_doc::{command::BitmapCommand, Command, Session};
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
fn waterfall_resolves_vertical_metrics_without_changing_stored_bitmaps() {
    let (session, doc, api) = fixture();
    session
        .with_doc_mut(doc, |d| {
            let master = &mut d.font.masters[0];
            master.metrics.default_vertical_advance = Some(1100.0);
            master.metrics.default_vertical_origin = Some(900.0);
            master.metrics.ascender = 720.0;
            let master_id = master.id.clone();
            let layer = d
                .font
                .glyph_mut("A")
                .unwrap()
                .master_layer_mut(&master_id)
                .unwrap();
            layer.vertical_width = Some(1250.0);
            layer.vertical_origin = Some(850.0);
        })
        .unwrap();
    let preview = || {
        api.call(
            "bitmap.preview",
            json!({"doc":doc,"text":"","glyphs":["A"]}),
        )
        .unwrap()[0]["glyphs"][0]
            .clone()
    };
    // Explicit strike metrics take precedence over the outline layer and VORG.
    assert_eq!(preview()["vertical_origin"], json!([1, 2]));
    assert_eq!(preview()["vertical_advance"], 4);
    session
        .with_doc_mut(doc, |d| {
            let glyph = d.font.bitmap_strikes[0].glyphs.get_mut("A").unwrap();
            glyph.vertical_origin = None;
            glyph.vertical_advance = None;
        })
        .unwrap();
    assert_eq!(preview()["vertical_origin"], json!([2, 9]));
    assert_eq!(preview()["vertical_advance"], 13);
    session
        .with_doc_mut(doc, |d| {
            let master = d.font.default_master().id.clone();
            let layer = d
                .font
                .glyph_mut("A")
                .unwrap()
                .master_layer_mut(&master)
                .unwrap();
            layer.vertical_width = None;
            layer.vertical_origin = None;
        })
        .unwrap();
    assert_eq!(preview()["vertical_origin"], json!([2, 9]));
    assert_eq!(preview()["vertical_advance"], 11);
    session
        .with_doc_mut(doc, |d| {
            d.font.masters[0].metrics.default_vertical_advance = None;
            d.font.masters[0].metrics.default_vertical_origin = None;
        })
        .unwrap();
    assert_eq!(preview()["vertical_origin"], json!([2, 7]));
    assert_eq!(preview()["vertical_advance"], 10);
    session
        .with_doc_mut(doc, |d| {
            d.font.bitmap_strikes[0]
                .glyphs
                .get_mut("A")
                .unwrap()
                .vertical_advance = Some(0);
        })
        .unwrap();
    assert_eq!(preview()["vertical_advance"], 0);
    session
        .with_doc(doc, |d| {
            assert_eq!(d.font.bitmap_strikes[0].glyphs["A"].vertical_origin, None);
            assert_eq!(d.generation, 0);
            assert_eq!(d.undo_len(), 0);
        })
        .unwrap();
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

/// A strike fixture at another units-per-em, with real vertical metrics.
fn fixture_upm(upm: u16) -> (Arc<Session>, fc_doc::DocId, Api) {
    let mut font = Font::new_empty("Pixel grid", upm);
    if let Some(master) = font.masters.first_mut() {
        master.metrics.ascender = f64::from(upm) * 0.8;
        master.metrics.descender = -f64::from(upm) * 0.2;
    }
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("A");
    glyph.layers.push(Layer::new_master(master));
    font.insert_glyph(glyph).unwrap();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session.clone());
    (session, doc, api)
}

fn lock(session: &Session, doc: fc_doc::DocId, glyph: &str, locked: bool) {
    session
        .with_doc_mut(doc, |d| {
            d.apply(
                fc_doc::Command::EditorMetadata(
                    fc_doc::command::EditorMetadataCommand::GlyphLock {
                        glyph: glyph.into(),
                        locked,
                    },
                ),
                "Lock glyph",
            )
        })
        .unwrap()
        .unwrap();
}

fn image(api: &Api, doc: fc_doc::DocId, glyph: &str) -> serde_json::Value {
    api.call(
        "bitmap.glyph",
        json!({"doc":doc,"ppem":10,"depth":1,"glyph":glyph}),
    )
    .unwrap()["value"]
        .clone()
}

#[test]
fn nearest_and_area_average_downscaling_are_pixel_exact_and_differ() {
    for (filter, expected) in [("Nearest", 255u32), ("AreaAverage", 128)] {
        let (session, doc, api) = fixture();
        api.call("bitmap.transfer", json!({"doc":doc,"expected_generation":generation(&api,doc),"source_ppem":10,"source_depth":1,"target_ppem":5,"target_depth":8,"glyphs":["A"],"scale":true,"filter":filter})).unwrap();
        session
            .with_doc(doc, |d| {
                let strike = d
                    .font
                    .bitmap_strikes
                    .iter()
                    .find(|s| (s.ppem, s.depth) == (5, 8))
                    .expect("target strike");
                assert_eq!((strike.ascent, strike.descent), (4, 1), "{filter}");
                let glyph = &strike.glyphs["A"];
                assert_eq!(
                    (glyph.bitmap.width, glyph.bitmap.height),
                    (1, 1),
                    "{filter}"
                );
                assert_eq!(glyph.bitmap.pixel(0, 0), expected, "{filter}");
                // Origins and metrics scale with the size, rounding away from zero.
                assert_eq!(
                    (glyph.bitmap.origin_x, glyph.bitmap.origin_y),
                    (-1, -1),
                    "{filter}"
                );
                assert_eq!(glyph.advance, 2, "{filter}");
            })
            .unwrap();
    }
}

#[test]
fn an_unknown_scale_filter_is_rejected_without_touching_the_document() {
    let (session, doc, api) = fixture();
    assert!(api.call("bitmap.transfer", json!({"doc":doc,"expected_generation":generation(&api,doc),"source_ppem":10,"source_depth":1,"target_ppem":5,"target_depth":8,"glyphs":["A"],"scale":true,"filter":"Bicubic"})).is_err());
    session
        .with_doc(doc, |d| {
            assert_eq!(d.font.bitmap_strikes.len(), 1);
            assert_eq!(d.undo_len(), 0);
        })
        .unwrap();
}

#[test]
fn batch_tracing_is_one_undo_step_and_leaves_no_geometry_diagnostics() {
    let (session, doc, api) = fixture();
    let value = image(&api, doc, "A");
    api.call("bitmap.apply",json!({"doc":doc,"expected_generation":generation(&api,doc),"label":"Seed B","command":{"SetGlyphs":{"ppem":10,"depth":1,"glyphs":{"B":value}}}})).unwrap();
    let before = session.with_doc(doc, |d| d.undo_len()).unwrap();
    api.call("bitmap.trace",json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"glyph":"A","glyphs":["A","B"],"master":null,"threshold":128,"mode":"ChamferedCorners","join_diagonals":true})).unwrap();
    session
        .with_doc(doc, |d| {
            assert_eq!(d.undo_len(), before + 1, "one undo entry for the batch");
            let master = &d.font.default_master().id;
            for name in ["A", "B"] {
                let layer = d.font.glyph(name).unwrap().master_layer(master).unwrap();
                assert!(!layer.contours.is_empty(), "{name} has no traced contours");
                assert_eq!(layer.width, 300.0, "{name} advance");
            }
            let report = fc_doc::problems::scan(&d.font, 100);
            assert!(report.issues.is_empty(), "diagnostics: {:?}", report.issues);
        })
        .unwrap();
    api.call("doc.undo", json!({"doc":doc})).unwrap();
    session
        .with_doc(doc, |d| {
            let master = &d.font.default_master().id;
            for name in ["A", "B"] {
                assert!(d
                    .font
                    .glyph(name)
                    .unwrap()
                    .master_layer(master)
                    .unwrap()
                    .contours
                    .is_empty());
            }
        })
        .unwrap();
}

#[test]
fn tracing_a_glyph_without_an_image_fails_and_changes_nothing() {
    let (session, doc, api) = fixture();
    assert!(api.call("bitmap.trace",json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"glyph":"A","glyphs":["A","B"],"master":null,"threshold":128})).is_err());
    session
        .with_doc(doc, |d| {
            assert_eq!(d.undo_len(), 0);
            let master = &d.font.default_master().id;
            assert!(d
                .font
                .glyph("A")
                .unwrap()
                .master_layer(master)
                .unwrap()
                .contours
                .is_empty());
        })
        .unwrap();
}

#[test]
fn locked_glyphs_block_every_bitmap_write_including_sheet_batches() {
    let (session, doc, api) = fixture();
    let value = image(&api, doc, "A");
    lock(&session, doc, "A", true);
    let entries = session.with_doc(doc, |d| d.undo_len()).unwrap();
    let strike = json!({"ppem":10,"depth":1,"ascent":8,"descent":2,"properties":{},"glyphs":{}});
    for command in [
        json!({"SetGlyphs":{"ppem":10,"depth":1,"glyphs":{"A":value.clone()}}}),
        json!({"SetGlyph":{"ppem":10,"depth":1,"glyph":"A","value":value.clone()}}),
        json!({"SetStrike":{"strike":strike}}),
        json!({"RemoveStrike":{"ppem":10,"depth":1}}),
    ] {
        let error = api
            .call(
                "bitmap.apply",
                json!({"doc":doc,"expected_generation":generation(&api,doc),"label":"Write","command":command}),
            )
            .expect_err("a locked glyph must reject the write");
        assert!(
            format!("{error:?}").contains("locked"),
            "unexpected error: {error:?}"
        );
    }
    // Every higher-level bitmap route honours the same lock.
    assert!(api.call("bitmap.transfer", json!({"doc":doc,"expected_generation":generation(&api,doc),"source_ppem":10,"source_depth":1,"target_ppem":20,"target_depth":1,"glyphs":["A"],"scale":true})).is_err());
    assert!(api.call("bitmap.rasterize", json!({"doc":doc,"expected_generation":generation(&api,doc),"master":null,"ppem":10,"depth":1,"glyphs":["A"],"hinting":null,"threshold":128})).is_err());
    assert!(api.call("bitmap.trace", json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"glyph":"A","master":null,"threshold":128})).is_err());
    assert!(api.call("bitmap.pixel_components", json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"glyphs":["A"],"master":null,"threshold":128,"pixel_glyph":"pixel"})).is_err());
    session
        .with_doc(doc, |d| {
            assert_eq!(d.undo_len(), entries, "no write created history");
            assert_eq!(d.font.bitmap_strikes.len(), 1);
            assert_eq!(d.font.bitmap_strikes[0].glyphs.len(), 1);
        })
        .unwrap();
    lock(&session, doc, "A", false);
    api.call("bitmap.apply",json!({"doc":doc,"expected_generation":generation(&api,doc),"label":"Write","command":{"SetGlyphs":{"ppem":10,"depth":1,"glyphs":{"A":value}}}})).unwrap();
}

#[test]
fn invalid_strike_properties_are_rejected_and_leave_the_document_intact() {
    let (session, doc, api) = fixture();
    let configure = |properties: serde_json::Value| {
        api.call(
            "bitmap.configure",
            json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"ascent":8,"descent":2,"properties":properties}),
        )
    };
    assert!(configure(json!({"FAMILY NAME": {"Str": "Pixel"}})).is_err());
    assert!(configure(json!({"": {"Int": 1}})).is_err());
    assert!(configure(json!({"NOTE": {"Str": "one\ntwo"}})).is_err());
    assert!(configure(json!({"TAB\tNAME": {"Int": 1}})).is_err());
    session
        .with_doc(doc, |d| {
            assert!(d.font.bitmap_strikes[0].properties.is_empty());
            assert_eq!(d.undo_len(), 0);
        })
        .unwrap();
    configure(json!({"FOUNDRY": {"Str": "FontCarver"}, "RESOLUTION_X": {"Int": 72}})).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("properties.fcf");
    api.call(
        "session.export",
        json!({"doc":doc,"path":path,"options":{"format":"Fcf"}}),
    )
    .unwrap();
    let reopened = fc_doc::fcf::read(&path).unwrap();
    let strike = &reopened.bitmap_strikes[0];
    assert_eq!((strike.ppem, strike.depth), (10, 1));
    assert_eq!((strike.ascent, strike.descent), (8, 2));
    assert_eq!(
        strike.properties["FOUNDRY"],
        fc_model::BdfValue::Str("FontCarver".into())
    );
    assert_eq!(
        strike.properties["RESOLUTION_X"],
        fc_model::BdfValue::Int(72)
    );
}

#[test]
fn strike_identity_depth_and_size_are_validated_without_duplicates() {
    let (session, doc, api) = fixture();
    let apply = |strike: serde_json::Value| {
        api.call(
            "bitmap.apply",
            json!({"doc":doc,"expected_generation":generation(&api,doc),"label":"Strike","command":{"SetStrike":{"strike":strike}}}),
        )
    };
    assert!(
        apply(json!({"ppem":10,"depth":3,"ascent":8,"descent":2,"properties":{},"glyphs":{}}))
            .is_err()
    );
    assert!(
        apply(json!({"ppem":0,"depth":1,"ascent":8,"descent":2,"properties":{},"glyphs":{}}))
            .is_err()
    );
    assert!(apply(
        json!({"ppem":5000,"depth":1,"ascent":8,"descent":2,"properties":{},"glyphs":{}})
    )
    .is_err());
    assert!(apply(
        json!({"ppem":10,"depth":1,"ascent":-1,"descent":2,"properties":{},"glyphs":{}})
    )
    .is_err());
    // Writing the same key again replaces that strike instead of duplicating it.
    apply(json!({"ppem":10,"depth":1,"ascent":7,"descent":3,"properties":{},"glyphs":{}})).unwrap();
    session
        .with_doc(doc, |d| {
            assert_eq!(d.font.bitmap_strikes.len(), 1);
            assert_eq!(
                (
                    d.font.bitmap_strikes[0].ascent,
                    d.font.bitmap_strikes[0].descent
                ),
                (7, 3)
            );
            d.font.validate_bitmaps().expect("font stays valid");
        })
        .unwrap();
    // A second key is a separate strike, kept in (ppem, depth) order.
    apply(json!({"ppem":8,"depth":4,"ascent":6,"descent":2,"properties":{},"glyphs":{}})).unwrap();
    session
        .with_doc(doc, |d| {
            assert_eq!(
                d.font
                    .bitmap_strikes
                    .iter()
                    .map(|s| (s.ppem, s.depth))
                    .collect::<Vec<_>>(),
                vec![(8, 4), (10, 1)]
            );
        })
        .unwrap();
}

#[test]
fn rasterizing_validates_threshold_size_and_depth_and_covers_every_glyph() {
    let (session, doc, api) = fixture();
    let rasterize = |body: serde_json::Value| api.call("bitmap.rasterize", body);
    let base = |extra: serde_json::Value| {
        let mut value = json!({"doc":doc,"expected_generation":generation(&api,doc),"master":null,"ppem":12,"depth":1,"glyphs":[],"hinting":null,"threshold":128});
        for (key, item) in extra.as_object().unwrap() {
            value[key] = item.clone();
        }
        value
    };
    assert!(rasterize(base(json!({"threshold":0}))).is_err());
    assert!(rasterize(base(json!({"depth":32}))).is_err());
    assert!(rasterize(base(json!({"ppem":5000}))).is_err());
    assert!(rasterize(base(json!({"depth":3}))).is_err());
    session
        .with_doc(doc, |d| assert_eq!(d.font.bitmap_strikes.len(), 1))
        .unwrap();
    // An empty selection is the batch: every glyph in the font gets an image.
    rasterize(base(json!({}))).unwrap();
    session
        .with_doc(doc, |d| {
            let strike = d
                .font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (12, 1))
                .expect("new strike");
            assert_eq!(strike.glyphs.len(), d.font.glyphs.len());
            assert_eq!(d.undo_len(), 1);
        })
        .unwrap();
}

#[test]
fn exact_grid_presets_cover_every_divisor_and_suggest_the_nearest_size() {
    for (upm, expected) in [
        (
            1000u16,
            vec![
                4u16, 5, 8, 10, 20, 25, 40, 50, 100, 125, 200, 250, 500, 1000,
            ],
        ),
        (1024, vec![4, 8, 16, 32, 64, 128, 256, 512, 1024]),
        (2048, vec![4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048]),
    ] {
        let (_session, doc, api) = fixture_upm(upm);
        let info = api.call("bitmap.grid", json!({"doc":doc})).unwrap();
        assert_eq!(info["units_per_em"], upm);
        let presets = info["presets"].as_array().unwrap();
        assert_eq!(
            presets
                .iter()
                .map(|preset| preset["ppem"].as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
            expected,
            "upm {upm}"
        );
        for preset in presets {
            assert_eq!(preset["exact"], true, "upm {upm}");
            let ppem = preset["ppem"].as_u64().unwrap() as u16;
            assert_eq!(
                preset["units_per_pixel"].as_f64().unwrap(),
                f64::from(upm) / f64::from(ppem)
            );
            // A preset carries the strike metrics it would create.
            assert_eq!(
                preset["ascent"].as_i64().unwrap(),
                (f64::from(ppem) * 0.8).round() as i64
            );
            assert_eq!(
                preset["descent"].as_i64().unwrap(),
                (f64::from(ppem) * 0.2).round() as i64
            );
        }
        assert!(info["requested"].is_null());
        assert!(info["nearest"].is_null());
    }
}

#[test]
fn a_non_integer_grid_choice_is_reported_with_the_nearest_exact_size() {
    let (_session, doc, api) = fixture_upm(1000);
    let judge = |ppem: u16| {
        api.call("bitmap.grid", json!({"doc":doc,"ppem":ppem}))
            .unwrap()
    };
    let info = judge(16);
    assert_eq!(info["requested"]["exact"], false);
    assert_eq!(info["requested"]["units_per_pixel"], 62.5);
    assert_eq!(info["nearest"], 20);
    assert_eq!(judge(12)["nearest"], 10);
    assert_eq!(judge(3)["nearest"], 4);
    // Every exact divisor up to the em itself is offered, so a size inside the
    // range needs no suggestion and is in the list.
    let exact = judge(1000);
    assert_eq!(exact["requested"]["exact"], true);
    assert_eq!(exact["requested"]["units_per_pixel"], 1.0);
    assert!(exact["nearest"].is_null());
    assert!(exact["presets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|preset| preset["ppem"] == 1000));
    // Beyond the em a pixel is smaller than a font unit, so nothing is offered
    // and the nearest exact size is the em.
    let beyond = judge(2000);
    assert_eq!(beyond["requested"]["exact"], false);
    assert_eq!(beyond["nearest"], 1000);
    // Zero is not a size, and an unknown master is rejected.
    assert!(judge(0)["requested"].is_null());
    assert!(api
        .call("bitmap.grid", json!({"doc":doc,"master":"nope"}))
        .is_err());
}

#[test]
fn a_new_pixel_glyph_name_must_be_a_conventional_glyph_name() {
    let (session, doc, api) = fixture();
    for name in ["my pixel", "1pixel", "pixel/one", ""] {
        assert!(
            api.call("bitmap.pixel_components", json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"glyphs":["A"],"master":null,"threshold":128,"pixel_glyph":name})).is_err(),
            "{name:?} must be rejected"
        );
    }
    session
        .with_doc(doc, |d| {
            assert_eq!(d.font.glyphs.len(), 2);
            assert_eq!(d.undo_len(), 0);
        })
        .unwrap();
    api.call("bitmap.pixel_components", json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"glyphs":["A"],"master":null,"threshold":128,"pixel_glyph":"_pixel.alt"})).unwrap();
    session
        .with_doc(doc, |d| assert!(d.font.glyph("_pixel.alt").is_some()))
        .unwrap();
}

#[test]
fn editing_the_pixel_glyph_changes_every_pixel_component_and_both_export_modes() {
    let (session, doc, api) = fixture();
    api.call("bitmap.pixel_components", json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"glyphs":["A"],"master":null,"threshold":128,"pixel_glyph":"pixel"})).unwrap();
    let bounds = |session: &Session| {
        session
            .with_doc(doc, |d| {
                let master = d.font.default_master().id.clone();
                let name = fc_model::GlyphName::from("A");
                let path = fc_model::flatten_layer(&d.font, &name, &master).unwrap();
                kurbo::Shape::bounding_box(&path)
            })
            .unwrap()
    };
    let before = bounds(&session);
    // Halving the pixel square must halve every placed pixel, with no edit to A.
    session
        .with_doc_mut(doc, |d| {
            let master = d.font.default_master().id.clone();
            let layer = d
                .font
                .glyph("pixel")
                .unwrap()
                .master_layer(&master)
                .unwrap();
            let id = layer.id.clone();
            let mut contours = layer.contours.clone();
            for point in &mut contours[0].points {
                point.x /= 2.0;
                point.y /= 2.0;
            }
            d.apply(
                fc_doc::Command::ReplaceLayerOutline {
                    glyph: "pixel".into(),
                    layer: id,
                    contours,
                    components: Vec::new(),
                    anchors: Vec::new(),
                },
                "Edit the pixel glyph",
            )
        })
        .unwrap()
        .unwrap();
    let after = bounds(&session);
    // The two pixels sit at (-100, 0) and (0, -100) in font units; halving the
    // square keeps their origins and halves each one's extent.
    assert_eq!(
        (before.x0, before.y0, before.x1, before.y1),
        (-100.0, -100.0, 100.0, 100.0)
    );
    assert_eq!(
        (after.x0, after.y0, after.x1, after.y1),
        (-100.0, -100.0, 50.0, 50.0)
    );
    let directory = tempfile::tempdir().unwrap();
    for (decompose, components, contours) in [(false, 2usize, 0usize), (true, 0, 2)] {
        let path = directory.path().join(format!("pixels-{decompose}.ttf"));
        api.call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":"Ttf","decompose_components":decompose}}),
        )
        .unwrap();
        let decoded = fc_sfnt::read_path(&path, &Default::default()).unwrap();
        let font = decoded.fonts.first().expect("one font");
        let layer = font
            .glyphs
            .get("A")
            .unwrap()
            .master_layer(&font.default_master().id)
            .unwrap();
        assert_eq!(layer.components.len(), components, "decompose={decompose}");
        assert_eq!(layer.contours.len(), contours, "decompose={decompose}");
    }
}

#[test]
fn a_normal_outline_font_is_not_mistaken_for_a_pixel_font() {
    let session = Arc::new(Session::new());
    let api = Api::new(session.clone());
    for fixture in ["DejaVuSans.ttf", "NotoSans-Regular.ttf"] {
        let opened = api
            .call("session.open", json!({"path":fc_fixtures::path(fixture)}))
            .unwrap();
        let doc: fc_doc::DocId = serde_json::from_value(opened["doc"].clone()).unwrap();
        let detection = api
            .call("bitmap.detect_pixel_grid", json!({"doc":doc,"master":null}))
            .unwrap();
        assert!(detection["ppem"].is_null(), "{fixture} detected a grid");
        assert_eq!(detection["evidence_glyphs"], 0, "{fixture}");
        assert!(
            api.call(
                "bitmap.reconstruct",
                json!({"doc":doc,"expected_generation":detection["generation"],"master":null,"glyphs":[]}),
            )
            .is_err(),
            "{fixture} must refuse reconstruction"
        );
        session
            .with_doc(doc, |d| {
                assert!(d.font.bitmap_strikes.is_empty(), "{fixture}");
                assert_eq!(d.undo_len(), 0, "{fixture}");
            })
            .unwrap();
    }
}

#[test]
fn a_locked_glyph_still_allows_strike_metric_and_property_edits() {
    let (session, doc, api) = fixture();
    lock(&session, doc, "A", true);
    let entries = session.with_doc(doc, |d| d.undo_len()).unwrap();
    // Metrics and properties belong to the strike, not to the locked image.
    api.call(
        "bitmap.configure",
        json!({"doc":doc,"expected_generation":generation(&api,doc),"ppem":10,"depth":1,"ascent":9,"descent":1,"properties":{"FOUNDRY":{"Str":"FontCarver"}}}),
    )
    .unwrap();
    session
        .with_doc(doc, |d| {
            let strike = &d.font.bitmap_strikes[0];
            assert_eq!((strike.ascent, strike.descent), (9, 1));
            assert_eq!(
                strike.properties["FOUNDRY"],
                fc_model::BdfValue::Str("FontCarver".into())
            );
            // The locked image is untouched by a metadata-only write.
            assert_eq!(strike.glyphs["A"].bitmap.pixel(0, 0), 1);
            assert_eq!(d.undo_len(), entries + 1);
        })
        .unwrap();
    // Changing, removing or adding that glyph's image is still refused.
    let mut altered = session
        .with_doc(doc, |d| d.font.bitmap_strikes[0].clone())
        .unwrap();
    altered.glyphs.get_mut("A").unwrap().advance = 7;
    let strike = serde_json::to_value(&altered).unwrap();
    assert!(api
        .call(
            "bitmap.apply",
            json!({"doc":doc,"expected_generation":generation(&api,doc),"label":"Edit","command":{"SetStrike":{"strike":strike}}}),
        )
        .is_err());
    let mut emptied = altered.clone();
    emptied.glyphs.clear();
    let strike = serde_json::to_value(&emptied).unwrap();
    assert!(api
        .call(
            "bitmap.apply",
            json!({"doc":doc,"expected_generation":generation(&api,doc),"label":"Drop","command":{"SetStrike":{"strike":strike}}}),
        )
        .is_err());
    session
        .with_doc(doc, |d| {
            assert_eq!(d.font.bitmap_strikes[0].glyphs["A"].advance, 3);
            assert_eq!(d.undo_len(), entries + 1);
        })
        .unwrap();
}

#[test]
fn the_waterfall_previews_an_unencoded_glyph_by_name() {
    let (session, doc, api) = fixture();
    // `.notdef` has no Unicode value, so no sample string can reach it.
    session
        .with_doc_mut(doc, |d| {
            let master = d.font.default_master().id.clone();
            let mut glyph = Glyph::new(".notdef");
            glyph.layers.push(Layer::new_master(master));
            d.font.insert_glyph(glyph).unwrap();
            let mut strike = d.font.bitmap_strikes[0].clone();
            let image = strike.glyphs["A"].clone();
            strike.glyphs.insert(".notdef".into(), image);
            d.apply(
                Command::Bitmap(BitmapCommand::SetStrike { strike }),
                "Seed .notdef",
            )
        })
        .unwrap()
        .unwrap();
    let lines = api
        .call(
            "bitmap.preview",
            json!({"doc":doc,"text":"","glyphs":[".notdef"]}),
        )
        .unwrap();
    assert_eq!(lines.as_array().unwrap().len(), 1);
    let glyphs = lines[0]["glyphs"].as_array().unwrap();
    assert_eq!(glyphs.len(), 1);
    assert_eq!(glyphs[0]["advance"], 3);
    // Names win over the text, and an unknown name is reported.
    let named = api
        .call(
            "bitmap.preview",
            json!({"doc":doc,"text":"AAAA","glyphs":["A"]}),
        )
        .unwrap();
    assert_eq!(named[0]["glyphs"].as_array().unwrap().len(), 1);
    assert!(api
        .call(
            "bitmap.preview",
            json!({"doc":doc,"text":"","glyphs":["nope"]})
        )
        .is_err());
    // A glyph with no image in this strike leaves a space, as text does.
    let empty = api
        .call(
            "bitmap.preview",
            json!({"doc":doc,"text":"","glyphs":["B"]}),
        )
        .unwrap();
    assert!(empty[0]["glyphs"][0].is_null());
    assert!(api
        .call(
            "bitmap.preview",
            json!({"doc":doc,"text":"","glyphs":vec!["A"; 129]}),
        )
        .is_err());
}
