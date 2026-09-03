use std::time::{Duration, Instant};

use fw_model::glyph_for_unicode;
use fw_sfnt::{read_path, ReadOptions};

#[test]
fn source_han_sans_opens_within_the_release_budget() {
    let path = fw_fixtures::skip_unless_external!("source-han-sans-sc-otf");
    let started = Instant::now();
    let outcome = read_path(&path, &ReadOptions::default()).expect("read Source Han Sans");
    let elapsed = started.elapsed();
    let font = &outcome.fonts[0];

    assert_eq!(font.glyphs.len(), 65_535);
    assert!(glyph_for_unicode(font, 0x4E00).is_some());
    assert!(font.cid.is_some());
    if cfg!(not(debug_assertions)) {
        assert!(
            elapsed < Duration::from_secs(3),
            "Source Han Sans took {elapsed:?}"
        );
    }
    eprintln!("Source Han Sans open time: {elapsed:?}");
}
