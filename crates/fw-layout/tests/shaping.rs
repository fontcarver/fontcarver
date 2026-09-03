use std::time::{Duration, Instant};

use fw_layout::{FontKey, ShapeRequest, Shaper};
use fw_model::{Contour, FeatureBlock, Font, Glyph, Layer, MasterId};
use kurbo::BezPath;

fn ligature_font() -> Font {
    let mut font = Font::new_empty("Shape Test", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicode, width) in [
        (".notdef", None, 600.0),
        ("space", Some(0x20), 250.0),
        ("f", Some(0x66), 300.0),
        ("i", Some(0x69), 280.0),
        ("f_i", None, 540.0),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        let mut path = BezPath::new();
        path.move_to((40.0, 0.0));
        path.line_to((40.0, 700.0));
        path.line_to((width - 40.0, 700.0));
        path.line_to((width - 40.0, 0.0));
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert test glyph");
    }
    font.features.features.push(FeatureBlock {
        tag: "liga".to_owned(),
        code: "sub f i by f_i;".to_owned(),
        ..FeatureBlock::default()
    });
    font
}

fn key(font: &Font) -> FontKey {
    FontKey {
        doc: 1,
        master: font.default_master().id.clone(),
    }
}

fn request(text: &str) -> ShapeRequest {
    ShapeRequest {
        text: text.to_owned(),
        ..ShapeRequest::default()
    }
}

#[test]
fn shapes_the_ligature_from_the_model_feature_source() {
    let font = ligature_font();
    let shaper = Shaper::new();

    let run = shaper
        .shape(&font, &key(&font), 1, &request("fi"))
        .expect("shape fi");

    assert_eq!(run.glyphs.len(), 1, "expected one ligature glyph: {run:?}");
    assert_eq!(run.names, vec![Some("f_i".to_owned())]);
    assert_eq!(run.glyphs[0].x_advance, 540.0);
    assert!(!run.stale);
    assert_eq!(run.units_per_em, 1_000);
    assert!(run.warnings.is_empty(), "{:?}", run.warnings);
}

#[test]
fn disabled_features_keep_the_unligated_glyphs() {
    let font = ligature_font();
    let shaper = Shaper::new();

    let run = shaper
        .shape(
            &font,
            &key(&font),
            1,
            &ShapeRequest {
                features: vec![("liga".to_owned(), false)],
                ..request("fi")
            },
        )
        .expect("shape fi without liga");

    assert_eq!(
        run.names,
        vec![Some("f".to_owned()), Some("i".to_owned())],
        "{run:?}"
    );
}

#[test]
fn size_scales_positions_from_font_units() {
    let font = ligature_font();
    let shaper = Shaper::new();

    let run = shaper
        .shape(
            &font,
            &key(&font),
            1,
            &ShapeRequest {
                size: Some(100.0),
                ..request("fi")
            },
        )
        .expect("shape at size");

    assert_eq!(run.size, Some(100.0));
    assert!((run.glyphs[0].x_advance - 54.0).abs() < 1e-9, "{run:?}");
}

#[test]
fn width_edits_are_served_before_the_recompile_lands() {
    let mut font = ligature_font();
    let shaper = Shaper::with_debounce(Duration::from_millis(20));
    let key = key(&font);
    let master: MasterId = key.master.clone();
    shaper
        .shape(&font, &key, 1, &request("i"))
        .expect("compile the first shaping font");

    font.glyph_mut("i")
        .and_then(|glyph| glyph.master_layer_mut(&master))
        .expect("master layer of i")
        .width = 480.0;
    let stale = shaper
        .shape(&font, &key, 2, &request("i"))
        .expect("shape with the stale font");

    assert!(stale.stale, "{stale:?}");
    assert_eq!(stale.glyphs[0].x_advance, 480.0);

    let deadline = Instant::now() + Duration::from_secs(30);
    while !shaper.is_current(&key, 2) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let fresh = shaper
        .shape(&font, &key, 2, &request("i"))
        .expect("shape with the rebuilt font");

    assert!(!fresh.stale, "background recompile did not land: {fresh:?}");
    assert_eq!(fresh.glyphs[0].x_advance, 480.0);
}

#[test]
fn dejavu_shapes_with_the_compiled_advances() {
    let font = fw_sfnt::read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &fw_sfnt::ReadOptions::default(),
    )
    .expect("read DejaVu Sans")
    .fonts
    .remove(0);
    let shaper = Shaper::new();

    let run = shaper
        .shape(&font, &key(&font), 1, &request("AV"))
        .expect("shape AV");

    assert_eq!(run.names, vec![Some("A".to_owned()), Some("V".to_owned())]);
    assert_eq!(run.units_per_em, 2_048);
    for glyph in &run.glyphs {
        assert!(glyph.x_advance > 0.0, "{run:?}");
    }
}

#[test]
fn unknown_scripts_and_feature_tags_are_reported() {
    let font = ligature_font();
    let shaper = Shaper::new();

    let run = shaper
        .shape(
            &font,
            &key(&font),
            1,
            &ShapeRequest {
                features: vec![("toolong".to_owned(), true)],
                direction: Some("sideways".to_owned()),
                ..request("f")
            },
        )
        .expect("shape with invalid options");

    assert_eq!(run.warnings.len(), 2, "{:?}", run.warnings);
}

#[test]
fn forget_drops_the_cached_font_of_one_document() {
    let font = ligature_font();
    let shaper = Shaper::new();
    let key = key(&font);
    shaper.shape(&font, &key, 7, &request("f")).expect("shape");
    assert!(shaper.is_current(&key, 7));

    shaper.forget(key.doc);

    assert!(!shaper.is_current(&key, 7));
}
