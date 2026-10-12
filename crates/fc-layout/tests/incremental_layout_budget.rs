//! Row 72 acceptance: live shaping rebuilds only the layout tables, with a
//! measured budget, and the shaper cache the editing canvas uses takes that path.

use std::time::{Duration, Instant};

use fc_compile::{compile_static_with_metadata, CompileOptions, ShapingCompile};
use fc_layout::{FontKey, ShapeRequest, Shaper};
use fc_model::{Contour, FeatureBlock, Font, Glyph, KernSide, Layer};
use kurbo::BezPath;
use read_fonts::{types::Tag, FontRef};

/// Non-layout tables that an incremental refresh must reuse byte for byte.
const RETAINED: [&[u8; 4]; 6] = [b"glyf", b"loca", b"cmap", b"hmtx", b"maxp", b"post"];

/// A font big enough that a full compile is measurably more expensive than a
/// layout-only refresh, while staying small enough for an ordinary test run.
fn large_font(glyph_count: usize) -> Font {
    let mut font = Font::new_empty("Incremental budget", 1000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new(".notdef");
    glyph.layers.push(Layer::new_master(master.clone()));
    font.insert_glyph(glyph).expect("insert .notdef");
    for index in 0..glyph_count {
        let name = format!("g{index:04}");
        let mut glyph = Glyph::new(name.as_str());
        // Keep the first glyphs on the letters the tests shape with.
        if index < 26 {
            glyph.unicodes.push(0x61 + index as u32);
        }
        let mut layer = Layer::new_master(master.clone());
        layer.width = 520.0;
        let mut path = BezPath::new();
        // Enough points that outline compilation is not free.
        for step in 0..12 {
            let offset = f64::from(step) * 18.0;
            let point = (20.0 + offset, 20.0 + offset);
            if step == 0 {
                path.move_to(point);
            } else {
                path.curve_to(
                    (point.0 + 6.0, point.1),
                    (point.0, point.1 + 6.0),
                    (point.0 + 12.0, point.1 + 12.0),
                );
            }
        }
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert glyph");
    }
    font.features.features.push(FeatureBlock {
        tag: "liga".into(),
        code: "sub g0000 g0001 by g0002;".into(),
        ..Default::default()
    });
    font
}

fn table(bytes: &[u8], tag: &[u8; 4]) -> Vec<u8> {
    let face = FontRef::new(bytes).expect("parse font");
    face.table_data(Tag::new(tag))
        .map(|data| data.as_bytes().to_vec())
        .unwrap_or_default()
}

#[test]
fn a_layout_only_refresh_is_far_cheaper_than_a_full_compile() {
    let mut font = large_font(400);
    let master = font.default_master().id.clone();

    let full_start = Instant::now();
    let initial = ShapingCompile::build(&font, &master, None).expect("initial build");
    let full = full_start.elapsed();
    assert!(!initial.incremental, "the first build must be a full build");

    // Timings on a shared machine are noisy, so both sides are best of three.
    // The byte-identity assertions below are the primary check; these numbers
    // only guard against the layout refresh quietly becoming a full compile.
    let mut baseline = Duration::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        let _ = compile_static_with_metadata(&font, &master, &CompileOptions::default())
            .expect("baseline full compile");
        baseline = baseline.min(start.elapsed());
    }

    font.features.features[0].code = "sub g0001 g0000 by g0002;".into();
    font.masters[0].kerning_h.0.insert(
        (
            KernSide::Glyph("g0000".into()),
            KernSide::Glyph("g0001".into()),
        ),
        -40.0,
    );
    let mut incremental = Duration::MAX;
    let mut refreshed = None;
    for _ in 0..3 {
        let start = Instant::now();
        let built = ShapingCompile::build(&font, &master, Some(&initial)).expect("refresh");
        incremental = incremental.min(start.elapsed());
        refreshed = Some(built);
    }
    let refreshed = refreshed.expect("three refreshes");
    assert!(refreshed.incremental, "the refresh was not incremental");

    for tag in RETAINED {
        assert_eq!(
            table(&initial.bytes, tag),
            table(&refreshed.bytes, tag),
            "table {} was rebuilt",
            std::str::from_utf8(tag).unwrap_or_default()
        );
    }
    // `head` is reused apart from the whole-font checksum adjustment, which any
    // table change necessarily moves.
    let head_without_checksum = |bytes: &[u8]| {
        let mut head = table(bytes, b"head");
        head.splice(8..12, [0, 0, 0, 0]);
        head
    };
    assert_eq!(
        head_without_checksum(&initial.bytes),
        head_without_checksum(&refreshed.bytes)
    );
    assert_ne!(
        table(&initial.bytes, b"GSUB"),
        table(&refreshed.bytes, b"GSUB"),
        "GSUB was not refreshed"
    );
    assert!(
        !table(&refreshed.bytes, b"GPOS").is_empty(),
        "the new kerning produced no GPOS"
    );

    // A wall-clock ceiling keeps live typing responsive, and the comparison
    // proves the saving is real rather than accidental.
    assert!(
        incremental < Duration::from_millis(750),
        "layout refresh took {incremental:?} (full build {full:?}, baseline {baseline:?})"
    );
    assert!(
        incremental * 3 < baseline * 2,
        "layout refresh {incremental:?} is not clearly cheaper than the full compile {baseline:?}"
    );
    eprintln!(
        "400 glyphs: first full build {full:?}, full compile baseline {baseline:?}, layout refresh {incremental:?}"
    );
}

#[test]
fn twenty_consecutive_feature_edits_stay_within_the_live_editing_budget() {
    let mut font = large_font(200);
    let master = font.default_master().id.clone();
    let mut compiled = ShapingCompile::build(&font, &master, None).expect("initial build");
    let start = Instant::now();
    for index in 0..20 {
        font.features.features[0].code = format!("sub g0001 g000{} by g0002;", index % 2);
        compiled = ShapingCompile::build(&font, &master, Some(&compiled)).expect("refresh");
        assert!(
            compiled.incremental,
            "edit {index} fell back to a full build"
        );
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(6),
        "20 layout refreshes took {elapsed:?}"
    );
    eprintln!("200 glyphs: 20 layout refreshes in {elapsed:?}");
}

#[test]
fn the_live_shaper_cache_takes_the_incremental_path_after_a_feature_edit() {
    let mut font = large_font(60);
    let shaper = Shaper::with_debounce(Duration::from_millis(20));
    let key = FontKey {
        doc: 1,
        master: font.default_master().id.clone(),
    };
    let request = |text: &str| ShapeRequest {
        text: text.to_owned(),
        ..ShapeRequest::default()
    };
    let first = shaper
        .shape(&font, &key, 1, &request("ab"))
        .expect("first shaping font");
    assert_eq!(
        first.names,
        vec![Some("g0002".to_owned())],
        "the ligature did not apply: {first:?}"
    );
    assert_eq!(shaper.used_incremental_layout(&key), Some(false));

    // A feature-only edit must publish a font built from the layout work graph.
    font.features.features[0].code = "sub g0000 g0001 by g0003;".into();
    let stale = shaper
        .shape(&font, &key, 2, &request("ab"))
        .expect("stale shaping");
    assert!(stale.stale, "{stale:?}");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !shaper.is_current(&key, 2) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let fresh = shaper
        .shape(&font, &key, 2, &request("ab"))
        .expect("refreshed shaping");
    assert!(
        !fresh.stale,
        "the background refresh did not land: {fresh:?}"
    );
    assert_eq!(fresh.names, vec![Some("g0003".to_owned())]);
    assert_eq!(
        shaper.used_incremental_layout(&key),
        Some(true),
        "the live cache did not use the layout-only path"
    );

    // An outline edit must fall back to the verified full build.
    let master = key.master.clone();
    font.glyph_mut("g0000")
        .and_then(|glyph| glyph.master_layer_mut(&master))
        .expect("master layer")
        .contours[0]
        .points[0]
        .y = 300.0;
    let _ = shaper.shape(&font, &key, 3, &request("ab"));
    let deadline = Instant::now() + Duration::from_secs(30);
    while !shaper.is_current(&key, 3) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        shaper.used_incremental_layout(&key),
        Some(false),
        "an outline edit must not reuse the previous outline tables"
    );
}
