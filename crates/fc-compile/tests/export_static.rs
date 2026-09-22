mod common;

use std::time::Instant;

use fc_compile::{
    compile_static_with_metadata, export, export_with_progress, shaping_fingerprint,
    CompileOptions, ExportFormat, ExportOptions, ExportStage,
};
use fc_model::{Font, Glyph, GlyphName, Layer, NamePlatform, NameRecord, Tag};
use read_fonts::{tables::cmap::CmapSubtable, types::Tag as ReadTag, FontRef, TableProvider};
use skrifa::MetadataProvider;

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
    let font = fc_sfnt::read_path(
        &fc_fixtures::path("DejaVuSans.ttf"),
        &fc_sfnt::ReadOptions::default(),
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
fn overlap_is_applied_and_optional_hinter_availability_is_reported() {
    let font = common::scratch_font();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            remove_overlap: true,
            autohint: fc_hint::external::executable().is_none(),
            ..ExportOptions::default()
        },
    )
    .expect("export with overlap removal");
    let codes: Vec<_> = report.warnings.iter().map(|warning| warning.code).collect();
    assert!(!codes.contains(&"remove-overlap-unavailable"));
    assert_eq!(
        codes.contains(&"autohint-unavailable"),
        fc_hint::external::executable().is_none()
    );
}

#[test]
fn fontc_order_is_read_by_name_and_notdef_is_moved_to_gid_zero() {
    let mut font = Font::new_empty("Order Test", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicode) in [("A", Some(0x41)), (".notdef", None), ("B", Some(0x42))] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).expect("insert glyph");
    }

    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export reordered");
    assert_eq!(post_names(&report.bytes), [".notdef", "A", "B"]);
    let face = FontRef::new(&report.bytes).expect("parse export");
    assert_eq!(face.charmap().map('A').expect("map A").to_u32(), 1);
    assert_eq!(face.charmap().map('B').expect("map B").to_u32(), 2);
}

#[test]
fn missing_notdef_is_synthesized_without_renaming_a_user_glyph() {
    let mut font = Font::new_empty("Synthesis Test", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicode) in [("A", 0x41), ("B", 0x42)] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(unicode);
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).expect("insert glyph");
    }

    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export font");
    assert_eq!(post_names(&report.bytes), [".notdef", "A", "B"]);
    let face = FontRef::new(&report.bytes).expect("parse export");
    assert_eq!(face.charmap().map('A').expect("map A").to_u32(), 1);
}

#[test]
fn static_compile_metadata_tracks_model_identity_and_exclusions() {
    let mut font = Font::new_empty("Identity Test", 1_000);
    let master = font.default_master().id.clone();
    for (name, unicode, export) in [
        ("B", Some(0x42), true),
        ("hidden", None, false),
        ("A", Some(0x41), true),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        glyph.export = export;
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).expect("insert glyph");
    }
    font.glyph_mut("A").expect("A").production_name = Some("uni0041".to_owned());

    let compiled = compile_static_with_metadata(
        &font,
        &master,
        &CompileOptions {
            production_names: true,
        },
    )
    .expect("compile with identity metadata");

    assert_eq!(
        compiled.glyph_order,
        [
            GlyphName::from(".notdef"),
            GlyphName::from("B"),
            GlyphName::from("A"),
        ]
    );
    assert_eq!(post_names(&compiled.bytes), [".notdef", "B", "uni0041"]);
}

#[test]
fn localized_windows_names_survive_and_mac_records_are_reported_once() {
    let mut font = common::scratch_font();
    font.info.copyright = None;
    font.info.names = vec![
        NameRecord {
            name_id: 0,
            language: "en".into(),
            text: "English copyright".into(),
            platform: NamePlatform::Windows,
        },
        NameRecord {
            name_id: 0,
            language: "de".into(),
            text: "Deutsches Copyright".into(),
            platform: NamePlatform::Windows,
        },
        NameRecord {
            name_id: 7,
            language: "en".into(),
            text: "Mac trademark".into(),
            platform: NamePlatform::Mac,
        },
        NameRecord {
            name_id: 8,
            language: "x-private".into(),
            text: "Unknown language".into(),
            platform: NamePlatform::Windows,
        },
    ];

    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export names");
    let records = decoded_names(&report.bytes);
    assert!(records.contains(&(0, 0x0409, "English copyright".into())));
    assert!(records.contains(&(0, 0x0407, "Deutsches Copyright".into())));
    assert_eq!(
        report
            .warnings
            .iter()
            .filter(|warning| warning.code == "mac-name-records-dropped")
            .count(),
        1
    );
    assert_eq!(
        report
            .warnings
            .iter()
            .filter(|warning| warning.code == "name-language-unmapped")
            .count(),
        1
    );
}

#[test]
fn german_only_names_still_get_english_defaults() {
    let mut font = common::scratch_font();
    font.info.family_name = "Kompilierprobe".to_owned();
    font.info.style_name = "Fett".to_owned();
    font.info.names = [1_u16, 2, 4, 6]
        .into_iter()
        .map(|name_id| NameRecord {
            name_id,
            language: "de".into(),
            text: format!("Deutscher Name {name_id}"),
            platform: NamePlatform::Windows,
        })
        .collect();

    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export names");
    let records = decoded_names(&report.bytes);

    for name_id in [1_u16, 2, 4, 6] {
        assert!(
            records.contains(&(name_id, 0x0409, format!("Deutscher Name {name_id}"))),
            "name {name_id} has no English default in {records:?}"
        );
        assert!(
            records.contains(&(name_id, 0x0407, format!("Deutscher Name {name_id}"))),
            "name {name_id} lost its German record in {records:?}"
        );
    }
}

#[test]
fn dejavu_keeps_every_windows_name_record() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    let expected = font
        .info
        .names
        .iter()
        .filter(|record| record.platform == NamePlatform::Windows)
        .count();
    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export names");

    assert_eq!(decoded_names(&report.bytes).len(), expected);
}

#[test]
fn variation_sequences_are_diagnosed() {
    let mut font = common::scratch_font();
    font.glyph_mut("A")
        .expect("A")
        .variation_selectors
        .push((0x41, 0xfe0f));
    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export UVS font");
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.code == "variation-sequences-dropped"));
}

#[test]
fn imported_created_time_is_used_for_both_head_timestamps() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export font");
    let input_head = FontRef::new(&original)
        .expect("parse original")
        .head()
        .expect("input head");
    let output_head = FontRef::new(&report.bytes)
        .expect("parse output")
        .head()
        .expect("output head");
    assert_eq!(output_head.created(), input_head.created());
    assert_eq!(output_head.modified(), input_head.created());
}

#[test]
fn coordinate_and_advance_clamps_are_reported_per_table() {
    let mut font = common::scratch_font();
    let layer = font
        .glyph_mut("A")
        .expect("A")
        .layers
        .first_mut()
        .expect("layer");
    layer.width = 70_000.0;
    layer.contours[0].points[0].x = 90_000.0;

    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export clamped font");
    let clamp_messages: Vec<_> = report
        .warnings
        .iter()
        .filter(|warning| warning.code == "value-clamped")
        .map(|warning| warning.message.as_str())
        .collect();
    assert_eq!(clamp_messages.len(), 2);
    assert!(clamp_messages
        .iter()
        .any(|message| message.contains("glyf")));
    assert!(clamp_messages
        .iter()
        .any(|message| message.contains("hmtx")));
}

#[test]
fn table_subset_retains_global_hint_programs() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            table_subset: Some(vec![Tag(*b"GSUB")]),
            ..ExportOptions::default()
        },
    )
    .expect("subset export");
    let face = FontRef::new(&report.bytes).expect("parse subset");
    for tag in [*b"fpgm", *b"prep", *b"cvt "] {
        assert!(
            face.table_data(ReadTag::new(&tag)).is_some(),
            "missing {}",
            Tag(tag)
        );
    }
}

#[test]
fn imported_gasp_version_is_retained() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let input = FontRef::new(&original).expect("parse original");
    let font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export font");
    let output = FontRef::new(&report.bytes).expect("parse output");
    assert_eq!(
        output.gasp().expect("output gasp").version(),
        input.gasp().expect("input gasp").version()
    );
}

fn post_names(bytes: &[u8]) -> Vec<String> {
    let face = FontRef::new(bytes).expect("parse font");
    let post = face.post().expect("post");
    (0..face.maxp().expect("maxp").num_glyphs())
        .map(|gid| {
            post.glyph_name(read_fonts::types::GlyphId16::new(gid))
                .expect("post name")
                .to_owned()
        })
        .collect()
}

fn decoded_names(bytes: &[u8]) -> Vec<(u16, u16, String)> {
    let face = FontRef::new(bytes).expect("parse font");
    let name = face.name().expect("name");
    name.name_record()
        .iter()
        .map(|record| {
            (
                record.name_id().to_u16(),
                record.language_id(),
                record
                    .string(name.string_data())
                    .expect("decode name")
                    .to_string(),
            )
        })
        .collect()
}

#[allow(dead_code)]
fn cmap_formats(bytes: &[u8]) -> Vec<u16> {
    let face = FontRef::new(bytes).expect("parse font");
    let cmap = face.cmap().expect("cmap");
    cmap.encoding_records()
        .iter()
        .map(
            |record| match record.subtable(cmap.offset_data()).expect("subtable") {
                CmapSubtable::Format4(_) => 4,
                CmapSubtable::Format12(_) => 12,
                _ => 0,
            },
        )
        .collect()
}

#[test]
fn progress_reports_every_stage_that_actually_runs() {
    let font = common::scratch_font();
    let mut stages = Vec::new();
    export_with_progress(
        &font,
        &|_| 0,
        &ExportOptions::default(),
        &mut |stage, percent| stages.push((stage, percent)),
    )
    .expect("export with progress");

    assert_eq!(
        stages,
        vec![
            (ExportStage::Compile, 0),
            (ExportStage::Preserve, 60),
            (ExportStage::Write, 90),
        ]
    );
    assert_eq!(ExportStage::Compile.as_str(), "compile");
    assert_eq!(ExportStage::Preserve.as_str(), "preserve");
    assert_eq!(ExportStage::Write.as_str(), "write");
}

#[test]
fn a_failing_export_stops_reporting_at_the_stage_that_failed() {
    let font = common::scratch_font();
    let mut stages = Vec::new();
    let error = export_with_progress(
        &font,
        &|_| 0,
        &ExportOptions {
            master: Some(fc_model::MasterId("missing".into())),
            ..ExportOptions::default()
        },
        &mut |stage, percent| stages.push((stage, percent)),
    )
    .expect_err("unknown master");

    assert!(error.to_string().contains("missing"), "{error}");
    assert!(stages.is_empty(), "{stages:?}");
}
