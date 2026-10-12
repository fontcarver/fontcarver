use fc_formats::metrics::{read_afm, write_afm};

const SAMPLE: &str = "StartFontMetrics 4.1\nComment Test metrics\nFontName Metrics-Regular\nFamilyName Metrics\nFontBBox -20 -200 900 800\nAscender 800\nDescender -200\nIsFixedPitch false\nEncodingScheme AdobeStandardEncoding\nStartCharMetrics 3\nC 65 ; WX 600.25 ; N A ; B 10 0 590 700 ;\nCH <0056> ; W0X 610 ; N V ; B -10 0 620 700 ;\nC -1 ; WX 500 ; N f ; L i fi ;\nEndCharMetrics\nStartKernData\nStartKernPairs 1\nKPX A V -80.5\nEndKernPairs\nEndKernData\nEndFontMetrics\n";

#[test]
fn afm_preserves_horizontal_metrics_and_ligatures() {
    let parsed = read_afm(SAMPLE).unwrap();
    assert_eq!(parsed.font_name, "Metrics-Regular");
    assert_eq!(parsed.glyphs[0].width, 600.25);
    assert_eq!(parsed.glyphs[1].code, Some(86));
    assert_eq!(parsed.glyphs[2].ligatures[0].replacement.as_ref(), "fi");
    assert_eq!(parsed.kerning[0].value, -80.5);
    assert_eq!(read_afm(&write_afm(&parsed).unwrap()).unwrap(), parsed);
}

#[test]
fn afm_rejects_broken_framing_counts_duplicates_and_numbers() {
    for input in [
        SAMPLE.replace("StartCharMetrics 3", "StartCharMetrics 4"),
        SAMPLE.replace("StartKernPairs 1", "StartKernPairs 2"),
        SAMPLE.replace("EndCharMetrics\n", ""),
        SAMPLE.replace("EndKernData\n", ""),
        SAMPLE.replace("EndFontMetrics\n", ""),
        SAMPLE.replace("600.25", "NaN"),
        SAMPLE.replace("N V", "N A"),
        SAMPLE.replace("CH <0056>", "C 65"),
        SAMPLE.replace(
            "FontName Metrics-Regular",
            "FontName Metrics-Regular\nFontName Duplicate",
        ),
        format!("{SAMPLE}FontName Trailing\n"),
    ] {
        assert!(read_afm(&input).is_err(), "accepted: {input}");
    }
}

#[test]
fn unsupported_afm_content_is_explicitly_rejected() {
    for input in [
        SAMPLE.replace("KPX A V -80.5", "KPY A V -80.5"),
        SAMPLE.replace("WX 600.25", "W1X 600.25"),
        SAMPLE.replace("FontBBox", "UnknownField"),
        SAMPLE.replace(
            "EndFontMetrics",
            "StartComposites 0\nEndComposites\nEndFontMetrics",
        ),
        SAMPLE.replace(
            "EndFontMetrics",
            "StartTrackKern 0\nEndTrackKern\nEndFontMetrics",
        ),
    ] {
        assert!(read_afm(&input).is_err(), "accepted: {input}");
    }
}

#[test]
fn writer_rejects_invalid_model_values_and_record_injection() {
    let mut metrics = read_afm(SAMPLE).unwrap();
    metrics.glyphs[0].width = f64::INFINITY;
    assert!(write_afm(&metrics).is_err());
    metrics.glyphs[0].width = 600.0;
    metrics.font_name = "Bad\nEndFontMetrics".into();
    assert!(write_afm(&metrics).is_err());
    metrics.font_name = "Valid".into();
    metrics
        .properties
        .insert("FamilyName".into(), "Bad\rFontName Other".into());
    assert!(write_afm(&metrics).is_err());
}

#[test]
fn zero_glyph_metrics_and_zero_vertical_components_are_supported() {
    let metrics = read_afm("StartFontMetrics 2.0\nFontName Empty\nStartCharMetrics 0\nEndCharMetrics\nEndFontMetrics\n").unwrap();
    assert!(metrics.glyphs.is_empty());
    assert_eq!(read_afm(&write_afm(&metrics).unwrap()).unwrap(), metrics);
    let combined = SAMPLE
        .replace("WX 600.25", "W 600.25 0")
        .replace("KPX A V -80.5", "KP A V -80.5 0");
    assert_eq!(read_afm(&combined).unwrap(), read_afm(SAMPLE).unwrap());
}

#[test]
fn truncation_and_ambiguous_character_records_fail_without_panicking() {
    for end in 0..SAMPLE.find("EndFontMetrics").unwrap() + "EndFontMetrics".len() {
        assert!(
            read_afm(&SAMPLE[..end]).is_err(),
            "accepted prefix ending at {end}"
        );
    }
    for input in [
        SAMPLE.replace("WX 600.25", "WX 600.25 ; W0X 601"),
        SAMPLE.replace("C 65", "C -2"),
        SAMPLE.replace("C 65", "C 65536"),
        SAMPLE.replace("B 10 0 590 700", "B 590 0 10 700"),
        SAMPLE.replace("L i fi", "L i fi ; L i f_i"),
        SAMPLE.replace(
            "StartKernPairs 1\nKPX A V -80.5",
            "StartKernPairs 2\nKPX A V -80.5\nKPX A V -81",
        ),
    ] {
        assert!(read_afm(&input).is_err(), "accepted: {input}");
    }
}

#[test]
fn writer_rejects_whitespace_that_reader_would_normalize() {
    let mut metrics = read_afm(SAMPLE).unwrap();
    metrics.comments.push("  leading and trailing  ".into());
    assert!(write_afm(&metrics).is_err());
    metrics.comments.pop();
    metrics
        .properties
        .insert("Notice".into(), "  leading and trailing  ".into());
    assert!(write_afm(&metrics).is_err());
}

#[test]
fn afm_is_ascii_and_fixed_pitch_declarations_must_agree() {
    for property in [
        "FamilyName 字体",
        "Comment 字体",
        "CharWidth 600 0",
        "IsFixedPitch true",
    ] {
        let input = SAMPLE.replace("IsFixedPitch false", property);
        assert!(read_afm(&input).is_err(), "accepted {property}");
    }
    let mut metrics = read_afm(SAMPLE).unwrap();
    metrics.comments.push("字体".into());
    assert!(write_afm(&metrics).is_err());
    metrics.comments.pop();
    metrics
        .properties
        .insert("FamilyName".into(), "字体".into());
    assert!(write_afm(&metrics).is_err());
    metrics.properties.remove("FamilyName");
    metrics
        .properties
        .insert("CharWidth".into(), "600.25 0".into());
    assert!(write_afm(&metrics).is_err());
    metrics
        .properties
        .insert("IsFixedPitch".into(), "true".into());
    for glyph in &mut metrics.glyphs {
        glyph.width = 600.25;
    }
    assert!(write_afm(&metrics).is_ok());
}

#[test]
fn thousand_glyph_metrics_roundtrip_within_open_export_budget() {
    use fc_formats::metrics::{GlyphMetrics, MetricsFont};
    let metrics = MetricsFont {
        font_name: "Metrics-Budget".into(),
        glyphs: (0..1024)
            .map(|code| GlyphMetrics {
                code: Some(code),
                name: format!("g{code}").into(),
                width: 500.25 + f64::from(code % 100),
                bbox: Some([0.0, -200.0, 500.0, 800.0]),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let started = std::time::Instant::now();
    assert_eq!(read_afm(&write_afm(&metrics).unwrap()).unwrap(), metrics);
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
}
