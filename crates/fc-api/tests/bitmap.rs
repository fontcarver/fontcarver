use fc_api::Api;
use fc_doc::Session;
use fc_model::{BdfValue, Bitmap, BitmapGlyph, BitmapStrike, Font, Glyph, Layer};
use serde_json::json;
use std::sync::Arc;
fn font(depth: u8) -> Font {
    let mut font = Font::new_empty("Bitmap API", 1000);
    let master = font.default_master().id.clone();
    let mut strike = BitmapStrike {
        ppem: 16,
        depth,
        ascent: 12,
        descent: 4,
        ..Default::default()
    };
    for (name, unicode) in [
        (".notdef", None),
        ("A", Some(65)),
        ("uni4E2D", Some(0x4e2d)),
    ] {
        let mut g = Glyph::new(name);
        g.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 500.0;
        g.layers.push(layer);
        font.insert_glyph(g).unwrap();
        let mut bitmap = Bitmap::new(8, 16, depth).unwrap();
        bitmap.origin_y = -4;
        for y in 0..16 {
            bitmap
                .set_pixel(
                    y % 8,
                    y,
                    if depth == 32 {
                        0xf03080b0
                    } else {
                        (1 << depth) - 1
                    },
                )
                .unwrap();
        }
        strike.glyphs.insert(
            name.into(),
            BitmapGlyph {
                bitmap,
                advance: 8,
                ..Default::default()
            },
        );
    }
    font.bitmap_strikes.push(strike);
    font
}
#[test]
fn editable_embedded_tables_export_and_reopen_without_stale_strikes() {
    for (depth, format, tag) in [
        (1, "Otb", b"EBDT"),
        (2, "Ttf", b"EBDT"),
        (4, "Woff", b"EBDT"),
        (8, "Ttf", b"EBDT"),
        (32, "Cbdt", b"CBDT"),
        (32, "Sbix", b"sbix"),
    ] {
        let mut font = font(depth);
        if format == "Sbix" {
            font.bitmap_strikes[0].properties.insert(
                "FONTCARVER_BITMAP_TABLE".into(),
                BdfValue::Str("sbix".into()),
            );
        }
        let expected = font.bitmap_strikes[0].glyphs.clone();
        let session = Arc::new(Session::new());
        let doc = session.open_font(font, None);
        let api = Api::new(session.clone());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(if format == "Woff" {
            "font.woff"
        } else {
            "font.otb"
        });
        api.call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":format}}),
        )
        .unwrap_or_else(|e| panic!("{format}/{depth}: {e}"));
        let reopened = fc_sfnt::read_path(&path, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        assert_eq!(
            reopened.bitmap_strikes[0].glyphs, expected,
            "{format}/{depth}"
        );
        assert!(reopened.extra.raw_tables.contains_key(&fc_model::Tag(*tag)));
        if matches!(format, "Otb" | "Cbdt" | "Sbix") {
            let mut damaged = std::fs::read(&path).unwrap();
            if format == "Otb" {
                let count = u16::from_be_bytes(damaged[4..6].try_into().unwrap()) as usize;
                let record = (0..count)
                    .map(|i| 12 + i * 16)
                    .find(|&at| &damaged[at..at + 4] == b"EBLC")
                    .unwrap();
                let location =
                    u32::from_be_bytes(damaged[record + 8..record + 12].try_into().unwrap())
                        as usize;
                damaged[location + 8..location + 12].copy_from_slice(&u32::MAX.to_be_bytes());
            } else {
                let png = damaged
                    .windows(8)
                    .position(|b| b == b"\x89PNG\r\n\x1a\n")
                    .unwrap();
                damaged[png] = 0;
            }
            assert!(fc_sfnt::read_bytes(&damaged, &Default::default()).is_err());
            assert!(
                fc_sfnt::read_bytes(
                    &damaged,
                    &fc_sfnt::ReadOptions {
                        decode_bitmaps: false,
                        ..Default::default()
                    }
                )
                .is_ok(),
                "explicit opaque import remains available"
            );
        }
        let opened = api.call("session.open", json!({"path":path})).unwrap();
        let newdoc = serde_json::from_value::<fc_doc::DocId>(opened["doc"].clone()).unwrap();
        session
            .with_doc(newdoc, |d| {
                assert_eq!(d.font.bitmap_strikes[0].glyphs, expected)
            })
            .unwrap();
        let state = api.call("bitmap.state", json!({"doc":newdoc})).unwrap();
        api.call("bitmap.apply",json!({"doc":newdoc,"expected_generation":state["generation"],"command":{"RemoveStrike":{"ppem":16,"depth":depth}},"label":"delete"})).unwrap();
        let output = dir.path().join("deleted.ttf");
        api.call("session.export", json!({"doc":newdoc,"path":output}))
            .unwrap();
        let deleted = fc_sfnt::read_path(&output, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        assert!(deleted.bitmap_strikes.is_empty());
    }
}
#[test]
fn bitmap_edit_trace_raster_export_native_reopen_and_conflict() {
    let session = Arc::new(Session::new());
    let doc = session.open_font(font(1), None);
    let api = Api::new(session.clone());
    let dir = tempfile::tempdir().unwrap();
    let state = api.call("bitmap.state", json!({"doc":doc})).unwrap();
    let image = api
        .call(
            "bitmap.glyph",
            json!({"doc":doc,"ppem":16,"depth":1,"glyph":"A"}),
        )
        .unwrap();
    let mut value = image["value"].clone();
    value["bitmap"]["rows"][0] = json!(255);
    let params = json!({"doc":doc,"expected_generation":state["generation"],"command":{"SetGlyph":{"ppem":16,"depth":1,"glyph":"A","value":value}},"label":"draw"});
    api.call("bitmap.apply", params.clone()).unwrap();
    assert!(api.call("bitmap.apply", params).is_err());
    let state = api.call("bitmap.state", json!({"doc":doc})).unwrap();
    api.call("bitmap.trace",json!({"doc":doc,"expected_generation":state["generation"],"ppem":16,"depth":1,"glyph":"A","master":null,"threshold":128})).unwrap();
    let state = api.call("bitmap.state", json!({"doc":doc})).unwrap();
    api.call("bitmap.rasterize",json!({"doc":doc,"expected_generation":state["generation"],"ppem":16,"depth":1,"master":null,"glyphs":["A"]})).unwrap();
    let traced = api
        .call(
            "bitmap.glyph",
            json!({"doc":doc,"ppem":16,"depth":1,"glyph":"A"}),
        )
        .unwrap();
    assert_eq!(traced["value"]["bitmap"]["rows"], value["bitmap"]["rows"]);
    let preview = api
        .call("bitmap.preview", json!({"doc":doc,"text":"A中?"}))
        .unwrap();
    assert_eq!(preview[0]["glyphs"].as_array().unwrap().len(), 3);
    assert!(preview[0]["glyphs"][2].is_null());
    for format in ["Bdf", "Pcf", "Fcf"] {
        let path = dir.path().join(format!("font.{}", format.to_lowercase()));
        api.call(
            "session.export",
            json!({"doc":doc,"path":path,"options":{"format":format}}),
        )
        .unwrap();
        let opened = api.call("session.open", json!({"path":path})).unwrap();
        assert_eq!(opened["glyph_count"], 3);
    }
}

#[test]
fn optional_cjk_bdf_corpus_roundtrip_and_paged_preview_budget() {
    let Ok(path) = std::env::var("FC_CJK_BDF") else {
        return;
    };
    let started = std::time::Instant::now();
    let font = fc_formats::bitmap::read(std::path::Path::new(&path)).unwrap();
    let expected = font.bitmap_strikes[0].glyphs.clone();
    let count = font.glyphs.len();
    assert!(count > 5000, "use a full CJK corpus");
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(session);
    let dir = tempfile::tempdir().unwrap();
    let open_ms = started.elapsed().as_millis();
    for offset in [0, 3000, count as u32 - 100] {
        let page = api
            .call(
                "glyphs.list",
                json!({"doc":doc,"offset":offset,"limit":100,"include_paths":true}),
            )
            .unwrap();
        assert_eq!(page["items"].as_array().unwrap().len(), 100);
    }
    for format in ["Bdf", "Pcf", "Fcf", "Otb"] {
        let output = dir.path().join(format!("corpus.{}", format.to_lowercase()));
        api.call(
            "session.export",
            json!({"doc":doc,"path":output,"options":{"format":format}}),
        )
        .unwrap_or_else(|e| panic!("{format}: {e}"));
        let font = match format {
            "Fcf" => fc_doc::fcf::read(&output).unwrap(),
            "Otb" => fc_sfnt::read_path(&output, &Default::default())
                .unwrap()
                .fonts
                .remove(0),
            _ => fc_formats::bitmap::read(&output).unwrap(),
        };
        for (name, glyph) in &expected {
            assert_eq!(
                font.bitmap_strikes[0].glyphs.get(name),
                Some(glyph),
                "{format}/{name}"
            );
        }
    }
    eprintln!(
        "CJK bitmap acceptance: {count} glyphs, open {open_ms} ms, complete {} ms",
        started.elapsed().as_millis()
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(120));
}

#[test]
fn embedded_and_automatic_grid_fitting_keep_source_outlines_unchanged() {
    let source = fc_sfnt::read_path(&fc_fixtures::path("DejaVuSans.ttf"), &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let original = source.glyph("A").unwrap().clone();
    let session = Arc::new(Session::new());
    let doc = session.open_font(source, None);
    let api = Api::new(session.clone());
    for (hinting, ppem) in [("Embedded", 13), ("Auto", 15)] {
        let state = api.call("bitmap.state", json!({"doc":doc})).unwrap();
        api.call("bitmap.rasterize",json!({"doc":doc,"expected_generation":state["generation"],"master":null,"ppem":ppem,"depth":1,"glyphs":["A"],"hinting":hinting,"threshold":128})).unwrap();
        let glyph = api
            .call(
                "bitmap.glyph",
                json!({"doc":doc,"ppem":ppem,"depth":1,"glyph":"A"}),
            )
            .unwrap();
        assert!(glyph["value"]["bitmap"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64().unwrap() > 0));
    }
    session
        .with_doc(doc, |d| assert_eq!(d.font.glyph("A").unwrap(), &original))
        .unwrap();
}
