use std::time::{Duration, Instant};

use fw_layout::{widths_for_text, FontKey, ShapeRequest, ShapedRun, Shaper};
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

/// Polls `attempt` until it yields a run or the deadline passes.
fn wait_for(mut attempt: impl FnMut() -> Option<ShapedRun>) -> ShapedRun {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(run) = attempt() {
            return run;
        }
        assert!(
            Instant::now() < deadline,
            "the shaper never reached the expected state"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_scheduled_rebuild_is_not_copied_again_for_the_same_generation() {
    let font = ligature_font();
    // A long debounce keeps the scheduled builds pending for the whole test.
    let shaper = Shaper::with_debounce(Duration::from_secs(30));
    let key = key(&font);
    let widths = widths_for_text(&font, &key.master, "f");
    shaper
        .shape(&font, &key, 1, &request("f"))
        .expect("compile the first shaping font");

    let first = shaper.plan(&key, 2);
    assert!(
        first.needs_font(),
        "a new generation must schedule a rebuild"
    );
    let run = shaper
        .shape_planned(first, Some(font.clone()), &widths, &request("f"))
        .expect("stale shape");
    assert!(run.stale, "{run:?}");

    assert!(
        !shaper.plan(&key, 2).needs_font(),
        "a pending build for the same generation must not copy the font again"
    );

    let newer = shaper.plan(&key, 3);
    assert!(
        newer.needs_font(),
        "a generation nothing is building for must schedule a rebuild"
    );
    shaper
        .shape_planned(newer, Some(font.clone()), &widths, &request("f"))
        .expect("stale shape at the newer generation");
    assert!(
        !shaper.plan(&key, 2).needs_font(),
        "a pending build for a newer generation must not copy the font again"
    );
}

#[test]
fn a_cached_generation_never_copies_the_font() {
    let font = ligature_font();
    let shaper = Shaper::new();
    let key = key(&font);
    shaper
        .shape(&font, &key, 4, &request("f"))
        .expect("compile the first shaping font");

    assert!(!shaper.plan(&key, 4).needs_font());
}

#[test]
fn a_failed_rebuild_is_reported_and_retried_on_the_next_generation() {
    let mut font = ligature_font();
    let shaper = Shaper::with_debounce(Duration::from_millis(10));
    let key = key(&font);
    shaper
        .shape(&font, &key, 1, &request("f"))
        .expect("compile the first shaping font");

    font.features.features[0].code = "this is not valid feature code".to_owned();
    shaper
        .shape(&font, &key, 2, &request("f"))
        .expect("serve the stale font while the rebuild fails");

    let failed = wait_for(|| {
        let run = shaper
            .shape(&font, &key, 2, &request("f"))
            .expect("shape while the rebuild fails");
        run.warnings
            .iter()
            .any(|warning| warning.starts_with("compile-failed"))
            .then_some(run)
    });
    assert!(failed.stale, "{failed:?}");

    font.features.features[0].code = "sub f i by f_i;".to_owned();
    shaper
        .shape(&font, &key, 3, &request("f"))
        .expect("shape after repairing the feature source");
    let fresh = wait_for(|| {
        shaper.is_current(&key, 3).then(|| {
            shaper
                .shape(&font, &key, 3, &request("fi"))
                .expect("shape with the rebuilt font")
        })
    });

    assert!(!fresh.stale, "{fresh:?}");
    assert!(fresh.warnings.is_empty(), "{:?}", fresh.warnings);
    assert_eq!(fresh.names, vec![Some("f_i".to_owned())]);
}
