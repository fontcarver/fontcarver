use fc_glyphdata::{
    cjk_charset_codepoints, cjk_charset_membership, iicore_codepoints, normalize_reading, unihan,
    unihan_codepoints, CjkCharsets, RadicalStroke,
};

#[test]
fn preserves_original_properties_and_all_alternatives() {
    let water = unihan(0x6C34).unwrap();
    assert_eq!(water.mandarin, "shuǐ");
    assert_eq!(water.japanese_kun, "MIZU");
    assert_eq!(
        water.radicals().collect::<Vec<_>>(),
        [RadicalStroke {
            radical: 85,
            simplified: 0,
            residual: 0
        }]
    );
    assert_eq!(water.strokes().collect::<Vec<_>>(), [4]);
    assert!(unihan(0x30000).is_some());
    assert!(unihan(0x41).is_none());
    assert!(unihan(0xD800).is_none());
    assert!(unihan(u32::MAX).is_none());
    let records: Vec<_> = unihan_codepoints().map(|cp| unihan(cp).unwrap()).collect();
    assert_eq!(records.len(), 102_998);
    assert!(records.iter().any(|r| r.radicals().count() > 1));
    assert!(records
        .iter()
        .any(|r| r.radicals().any(|rs| rs.residual < 0)));
    assert!(records
        .iter()
        .any(|r| r.radicals().any(|rs| rs.simplified > 0)));
    let alternatives = fc_glyphdata::UnihanRecord {
        total_strokes: "8 9",
        ..water
    };
    assert_eq!(alternatives.strokes().collect::<Vec<_>>(), [8, 9]);
}

#[test]
fn matches_tones_languages_and_umlaut_spellings() {
    let water = unihan(0x6C34).unwrap();
    for query in ["SHUI", "shuǐ", "shui3", "shui\u{030c}"] {
        assert!(water.matches_reading(query, "mandarin"));
    }
    assert!(water.matches_reading("mizu", "japanese"));
    assert!(!water.matches_reading("mizu", "mandarin"));
    assert!(water.matches_reading("seoi2", "any"));
    for query in ["lǚ", "lu:3", "lv", "lu\u{0308}\u{030c}"] {
        assert_eq!(normalize_reading(query), "lv");
        assert!(unihan(0x5415).unwrap().matches_reading(query, "mandarin"));
    }
    assert!(!water.matches_reading("123", "any"));
}

#[test]
fn repertoires_are_sorted_unique_and_agree_with_membership() {
    for (charset, count) in [
        (CjkCharsets::GB18030_LEVEL1, 28_606),
        (CjkCharsets::JIS_X_0208, 6_879),
        (CjkCharsets::KS_X_1001, 8_224),
    ] {
        let points = cjk_charset_codepoints(charset);
        assert_eq!(points.len(), count);
        assert!(points.windows(2).all(|p| p[0] < p[1]));
        assert!(points
            .iter()
            .all(|cp| cjk_charset_membership(*cp).contains(charset)));
    }
    let gb = cjk_charset_codepoints(CjkCharsets::GB18030_LEVEL1);
    for cp in [0x3400, 0x4DB5, 0x9FEF, 0x20AC] {
        assert!(gb.binary_search(&cp).is_ok());
    }
    for cp in [0xE000, 0xF92C, 0x9FF0, 0x4DB6] {
        assert!(gb.binary_search(&cp).is_err());
    }
    assert_eq!(iicore_codepoints().len(), 9_810);
}

#[test]
fn full_unihan_lookup_is_bounded() {
    let start = std::time::Instant::now();
    let count = unihan_codepoints()
        .filter(|cp| unihan(*cp).is_some())
        .count();
    assert_eq!(count, 102_998);
    assert!(
        start.elapsed() < std::time::Duration::from_secs(2),
        "full database lookup: {:?}",
        start.elapsed()
    );
}
