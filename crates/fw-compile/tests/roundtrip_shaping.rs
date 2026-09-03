use std::time::Instant;

use fw_compile::{export, shaping_fingerprint, ExportOptions};
use read_fonts::{FontRef, TableProvider};
use skrifa::MetadataProvider;

#[test]
fn noto_sans_shapes_identically_after_export() {
    let original = std::fs::read(fw_fixtures::path("NotoSans-Regular.ttf")).expect("read fixture");
    let font = fw_sfnt::read_bytes(&original, &fw_sfnt::ReadOptions::default())
        .expect("import Noto Sans")
        .fonts
        .remove(0);
    let output = export(&font, &|_| 0, &ExportOptions::default()).expect("export Noto Sans");
    assert_eq!(
        shaping_fingerprint(&original, "Hello fi AVWA").expect("shape original"),
        shaping_fingerprint(&output.bytes, "Hello fi AVWA").expect("shape export")
    );
}

#[test]
fn source_han_export_meets_release_budget_when_available() {
    let path = fw_fixtures::skip_unless_external!("source-han-sans-sc-otf");
    if cfg!(debug_assertions) {
        eprintln!("skipping Source Han export timing in debug mode");
        return;
    }
    let font = fw_sfnt::read_path(&path, &fw_sfnt::ReadOptions::default())
        .expect("read Source Han Sans SC")
        .fonts
        .remove(0);
    let started = Instant::now();
    let output = export(&font, &|_| 0, &ExportOptions::default()).expect("export Source Han");
    let elapsed = started.elapsed();
    let face = FontRef::new(&output.bytes).expect("parse exported Source Han");
    assert_eq!(face.maxp().expect("maxp").num_glyphs(), 65_535);
    assert!(face.charmap().map('\u{4e00}').is_some());
    assert!(
        elapsed.as_secs_f64() < 60.0,
        "Source Han export took {elapsed:?}"
    );
    eprintln!("Source Han Sans SC export time: {elapsed:?}");
}
