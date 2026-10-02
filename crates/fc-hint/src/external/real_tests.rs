use super::*;

#[test]
#[ignore = "requires real ttfautohint on PATH; run explicitly with --ignored"]
fn the_real_installed_hinter_produces_glyph_programs() {
    let available = availability().expect("install real ttfautohint before running this test");
    eprintln!(
        "executing {}: {}",
        available.path.display(),
        available.version
    );
    let outcome = autohint(
        include_bytes!("../../../../fixtures/fonts/DejaVuSans.ttf"),
        &AutohintOptions {
            hinting_range_min: Some(8),
            hinting_range_max: Some(50),
            ..AutohintOptions::default()
        },
    )
    .expect("the installed hinter must succeed on the committed fixture");
    assert_eq!(outcome.version, available.version);
    let hinted = FontRef::new(&outcome.bytes).unwrap();
    assert!(hinted
        .table_data(read_fonts::types::Tag::new(b"prep"))
        .is_some());
    assert!(hinted
        .table_data(read_fonts::types::Tag::new(b"fpgm"))
        .is_some());
}

#[test]
#[ignore = "requires real ttfautohint on PATH; run explicitly with --ignored"]
fn the_real_hinter_supports_composites_options_and_dehinting() {
    let available = availability().expect("install real ttfautohint before running this test");
    eprintln!(
        "executing {}: {}",
        available.path.display(),
        available.version
    );
    let source = include_bytes!("../../../../fixtures/fonts/DejaVuSans.ttf");
    let original = FontRef::new(source).unwrap();
    let options = AutohintOptions {
        hinting_range_min: Some(8),
        hinting_range_max: Some(50),
        hinting_limit: Some(200),
        fallback_stem_width: Some(70),
        increase_x_height: Some(0),
        x_height_snapping_exceptions: "10,14-16".into(),
        default_script: "latn".into(),
        fallback_script: "none".into(),
        strong_stem_width: "gGD".into(),
        composites: true,
        symbol: true,
        ttfa_table: true,
        windows_compatibility: true,
        info: AutohintInfo::Detailed,
        ..AutohintOptions::default()
    };
    let outcome = autohint(source, &options).expect("composite helper glyph must be accepted");
    let hinted = FontRef::new(&outcome.bytes).unwrap();
    assert_eq!(
        hinted.maxp().unwrap().num_glyphs(),
        original.maxp().unwrap().num_glyphs() + 1
    );
    for tag in [b"prep", b"fpgm", b"TTFA"] {
        assert!(hinted
            .table_data(read_fonts::types::Tag::new(tag))
            .is_some());
    }

    let adjusted = autohint(
        source,
        &AutohintOptions {
            adjust_subglyphs: true,
            ..options
        },
    )
    .expect("subglyph adjustment must succeed without an extra helper");
    assert_eq!(
        FontRef::new(&adjusted.bytes)
            .unwrap()
            .maxp()
            .unwrap()
            .num_glyphs(),
        original.maxp().unwrap().num_glyphs()
    );

    let dehinted = autohint(
        source,
        &AutohintOptions {
            dehint: true,
            ..AutohintOptions::default()
        },
    )
    .expect("real dehinting must succeed");
    let unhinted = FontRef::new(&dehinted.bytes).unwrap();
    for tag in [b"prep", b"fpgm", b"cvt "] {
        assert!(unhinted
            .table_data(read_fonts::types::Tag::new(tag))
            .is_none());
    }

    let error = autohint(
        source,
        &AutohintOptions {
            default_script: "zzzz".into(),
            ..AutohintOptions::default()
        },
    )
    .expect_err("the real tool must reject an unknown script");
    assert!(matches!(error, Error::External(_)));
    assert!(error.to_string().contains("zzzz"), "{error}");
    assert!(autohint(b"invalid font", &AutohintOptions::default()).is_err());
}
