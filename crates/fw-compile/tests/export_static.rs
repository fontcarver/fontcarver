mod common;

use std::time::Instant;

use fw_compile::{export, shaping_fingerprint, ExportFormat, ExportOptions};
use read_fonts::{types::Tag as ReadTag, FontRef};

#[test]
fn scratch_font_compiles_kerning_to_gpos() {
    let font = common::scratch_font();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Ttf,
            ..ExportOptions::default()
        },
    )
    .expect("export scratch font");
    let face = FontRef::new(&report.bytes).expect("parse exported font");
    assert!(face.table_data(ReadTag::new(b"GPOS")).is_some());
    assert_eq!(
        shaping_fingerprint(&report.bytes, "AV").expect("shape exported font"),
        vec![(2, 550, 0), (3, 600, 0)]
    );
}

#[test]
fn exports_dejavu_within_release_budget() {
    let font = fw_sfnt::read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &fw_sfnt::ReadOptions::default(),
    )
    .expect("read DejaVu Sans")
    .fonts
    .remove(0);
    let started = Instant::now();
    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export DejaVu Sans");
    let elapsed = started.elapsed();
    assert!(FontRef::new(&report.bytes).is_ok());
    if cfg!(not(debug_assertions)) {
        assert!(
            elapsed.as_secs_f64() < 5.0,
            "DejaVu export took {elapsed:?}"
        );
    }
    eprintln!("DejaVu Sans export time: {elapsed:?}");
}

#[test]
fn overlap_and_autohint_options_report_deferred_work() {
    let font = common::scratch_font();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            remove_overlap: true,
            autohint: true,
            ..ExportOptions::default()
        },
    )
    .expect("export with deferred options");
    let codes: Vec<_> = report.warnings.iter().map(|warning| warning.code).collect();
    assert!(codes.contains(&"remove-overlap-unavailable"));
    assert!(codes.contains(&"autohint-unavailable"));
}
