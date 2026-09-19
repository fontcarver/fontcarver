mod common;

use std::sync::OnceLock;

use fc_compile::{export, Error, ExportFormat, ExportOptions};
use fc_model::{FeaturesSource, Font, Tag};
use read_fonts::FontRef;

fn imported_font() -> Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| {
        let compiled = export(&common::scratch_font(), &|_| 0, &ExportOptions::default()).unwrap();
        fc_sfnt::read_bytes(&compiled.bytes, &fc_sfnt::ReadOptions::default())
            .unwrap()
            .fonts
            .remove(0)
    })
    .clone()
}

fn options() -> ExportOptions {
    ExportOptions {
        format: ExportFormat::Otf,
        ..ExportOptions::default()
    }
}

fn with_table(tag: [u8; 4], bytes: Vec<u8>) -> Font {
    let mut font = imported_font();
    font.features.source_of_truth = FeaturesSource::ImportedTables;
    font.extra.raw_tables.insert(Tag(tag), bytes);
    font
}

fn rejected(tag: [u8; 4], bytes: Vec<u8>, detail: &str) {
    let font = with_table(tag, bytes);
    let error = export(&font, &|_| 0, &options())
        .expect_err("unsafe CFF layout")
        .to_string();
    assert!(error.contains(detail), "{error}");
    assert!(error.contains("disable preservation"), "{error}");
    export(&font, &|_| 0, &ExportOptions::default()).expect("TTF preservation remains supported");
    export(
        &font,
        &|_| 0,
        &ExportOptions {
            preserve_untouched_tables: false,
            ..options()
        },
    )
    .expect("explicit preservation opt-out recovers CFF export");
}

fn preserved_font(font: &Font, tag: [u8; 4]) {
    let report = export(font, &|_| 0, &options()).expect("coordinate layout exports");
    let face = FontRef::new(&report.bytes).unwrap();
    assert_eq!(
        face.table_data(read_fonts::types::Tag::new(&tag))
            .unwrap()
            .as_bytes(),
        font.extra.raw_tables[&Tag(tag)]
    );
}

fn preserved(tag: [u8; 4], bytes: Vec<u8>) {
    preserved_font(&with_table(tag, bytes), tag);
}

fn parse_error(tag: [u8; 4], bytes: Vec<u8>) {
    let error =
        export(&with_table(tag, bytes), &|_| 0, &options()).expect_err("malformed AAT table");
    assert!(matches!(error, Error::Parse { .. }), "{error}");
    assert!(
        error
            .to_string()
            .contains(std::str::from_utf8(&tag).unwrap()),
        "{error}"
    );
}

fn u16s(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

fn u32s(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

fn lookup(format: u16, value: u16) -> Vec<u8> {
    match format {
        0 => u16s(&[0, value, value, value, value]),
        2 => u16s(&[2, 6, 1, 6, 0, 0, 2, 2, value, 0xffff, 0xffff, 0]),
        4 => u16s(&[4, 6, 1, 6, 0, 0, 2, 2, 24, 0xffff, 0xffff, 0, value]),
        6 => u16s(&[6, 4, 1, 4, 0, 0, 2, value, 0xffff, 0]),
        8 => u16s(&[8, 2, 1, value]),
        _ => panic!("unsupported fixture lookup"),
    }
}

fn bsln(format: u16) -> Vec<u8> {
    let mut bytes = u16s(&[1, 0, format, 0]);
    if format >= 2 {
        bytes.extend(u16s(&[2])); // Standard glyph.
    }
    bytes.extend(u16s(&[0; 32]));
    if matches!(format, 1 | 3) {
        bytes.extend(lookup(8, 0));
    }
    bytes
}

fn positioned_table(format: u16, lookup_format: u16, caret: bool) -> Vec<u8> {
    let mut bytes = u16s(&[1, 0, format]);
    let offset = (bytes.len() + lookup(lookup_format, 0).len()) as u16;
    bytes.extend(lookup(lookup_format, offset));
    bytes.extend(if caret {
        u16s(&[1, 200])
    } else {
        u16s(&[10, 20, 30, 40])
    });
    bytes
}

fn kerx_subtable(format: u32, body: Vec<u8>) -> Vec<u8> {
    let mut bytes = u32s(&[12 + body.len() as u32, format, 0]);
    bytes.extend(body);
    bytes
}

fn kerx(subtables: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = u32s(&[0x00020000, subtables.len() as u32]);
    for subtable in subtables {
        bytes.extend(subtable);
    }
    bytes
}

fn kerx_pair() -> Vec<u8> {
    let mut body = u32s(&[1, 6, 0, 0]);
    body.extend(u16s(&[2, 3, (-50i16) as u16]));
    kerx_subtable(0, body)
}

fn kerx_action(action_type: u32) -> Vec<u8> {
    // State header, flags, class lookup, two states, one entry, one action.
    let mut body = u32s(&[4, 20, 28, 44, (action_type << 30) | 50]);
    body.extend(lookup(8, 3));
    body.extend(u16s(&[0; 8]));
    body.extend(u16s(&[0, 0, 0]));
    body.extend(if action_type == 2 {
        u16s(&[100, 200, 300, 400])
    } else {
        u16s(&[0, 0])
    });
    kerx_subtable(4, body)
}

fn kerx_contextual() -> Vec<u8> {
    let mut body = u32s(&[4, 20, 28, 44, 50]);
    body.extend(lookup(8, 3));
    body.extend(u16s(&[0; 8]));
    body.extend(u16s(&[0, 0, 0]));
    body.extend(u16s(&[(-49i16) as u16])); // Low bit terminates the value list.
    kerx_subtable(1, body)
}

fn kerx_classes(format: u32) -> Vec<u8> {
    let mut body = if format == 2 {
        u32s(&[2, 28, 36, 44])
    } else {
        let mut header = u32s(&[0]);
        header.extend(u16s(&[1, 1]));
        header.extend(u32s(&[32, 40, 48]));
        header
    };
    body.extend(lookup(8, 0));
    body.extend(lookup(8, 0));
    body.extend(u16s(&[0]));
    kerx_subtable(format, body)
}

#[test]
fn baseline_point_formats_are_rejected() {
    for format in [2, 3] {
        rejected(*b"bsln", bsln(format), &format!("bsln format {format}"));
    }
}

#[test]
fn distance_baselines_and_all_standard_lookup_formats_are_preserved() {
    preserved(*b"bsln", bsln(0));
    for format in [0, 2, 4, 6, 8] {
        let mut bytes = bsln(0);
        bytes[5] = 1;
        bytes.extend(lookup(format, 0));
        preserved(*b"bsln", bytes);
    }
}

#[test]
fn caret_and_optical_bounds_point_formats_are_rejected() {
    rejected(*b"lcar", positioned_table(1, 8, true), "lcar format 1");
    rejected(*b"opbd", positioned_table(1, 8, false), "opbd format 1");
}

#[test]
fn coordinate_carets_and_optical_bounds_keep_all_standard_lookups() {
    for format in [0, 2, 4, 6, 8] {
        preserved(*b"lcar", positioned_table(0, format, true));
        preserved(*b"opbd", positioned_table(0, format, false));
    }
}

#[test]
fn kerx_control_point_actions_are_rejected_even_after_safe_subtables() {
    rejected(
        *b"kerx",
        kerx(&[kerx_pair(), kerx_action(0)]),
        "kerx format 4 control-point actions",
    );
}

#[test]
fn kerx_coordinate_actions_and_pair_values_are_preserved() {
    preserved(
        *b"kerx",
        kerx(&[
            kerx_pair(),
            kerx_contextual(),
            kerx_classes(2),
            kerx_action(2),
            kerx_classes(6),
        ]),
    );
}

#[test]
fn kerx_old_versions_ignore_reserved_tuple_count() {
    for version in [2, 3] {
        let mut subtable = kerx_classes(6);
        subtable[11] = 1;
        let mut bytes = kerx(&[subtable]);
        bytes[1] = version;
        if version == 3 {
            bytes.extend(u32s(&[0]));
        }
        preserved(*b"kerx", bytes);
    }
}

#[test]
fn kerx_anchor_indices_use_coordinate_ankr_data() {
    let mut font = with_table(*b"kerx", kerx(&[kerx_action(1)]));
    let mut anchors = u32s(&[0, 12, 22]);
    anchors.extend(lookup(0, 0));
    anchors.extend(u32s(&[1]));
    anchors.extend(u16s(&[100, 200]));
    font.extra.raw_tables.insert(Tag(*b"ankr"), anchors);
    preserved_font(&font, *b"kerx");
    preserved_font(&font, *b"ankr");
}

#[test]
fn kerx_anchor_actions_require_ankr() {
    let error = export(
        &with_table(*b"kerx", kerx(&[kerx_action(1)])),
        &|_| 0,
        &options(),
    )
    .unwrap_err();
    assert!(
        matches!(
            error,
            Error::Parse {
                context: "ankr",
                ..
            }
        ),
        "{error}"
    );
}

#[test]
fn kerx_variation_kerning_requires_instantiation() {
    let mut subtable = kerx_pair();
    subtable[4] |= 0x20;
    rejected(*b"kerx", kerx(&[subtable]), "kerx variation kerning");
    let mut subtable = kerx_pair();
    subtable[11] = 1;
    let mut bytes = kerx(&[subtable]);
    bytes[1] = 4;
    rejected(*b"kerx", bytes, "kerx variation kerning");
}

#[test]
fn kerx_format_four_ignores_tuple_count_as_specified() {
    let mut subtable = kerx_action(2);
    subtable[11] = 1;
    let mut bytes = kerx(&[subtable]);
    bytes[1] = 4;
    // Version 3+ adds the subtableGlyphCoverageArray. Zero offset = all glyphs.
    bytes.extend(u32s(&[0]));
    preserved(*b"kerx", bytes);
}

#[test]
fn fmtx_metric_points_require_rebuilding() {
    let mut bytes = u32s(&[0x00020000, 2]);
    bytes.extend([0, 1, 2, 3, 0, 1, 2, 3]);
    rejected(*b"fmtx", bytes, "fmtx font metric control points");
}

#[test]
fn jstf_requires_explicit_rebuild_or_preservation_opt_out() {
    rejected(*b"JSTF", u16s(&[1, 0, 0]), "JSTF");
}

#[test]
fn truncated_tables_and_unsupported_discriminants_are_errors() {
    for tag in [*b"bsln", *b"lcar", *b"opbd", *b"kerx", *b"JSTF", *b"fmtx"] {
        parse_error(tag, vec![0, 1]);
    }
    let mut bytes = bsln(0);
    bytes.pop();
    parse_error(*b"bsln", bytes);
    for tag in [*b"bsln", *b"lcar", *b"opbd"] {
        parse_error(tag, u16s(&[1, 0, 99]));
    }
    parse_error(*b"kerx", kerx(&[kerx_action(3)]));
}

#[test]
fn truncated_lookup_arrays_and_coordinate_records_are_errors() {
    for tag in [*b"lcar", *b"opbd"] {
        let mut bytes = positioned_table(0, 8, tag == *b"lcar");
        bytes.pop();
        parse_error(tag, bytes);
        parse_error(tag, u16s(&[1, 0, 0, 8, 0, 3, 12]));
    }
    let mut baseline = bsln(1);
    baseline.pop();
    parse_error(*b"bsln", baseline);
}

#[test]
fn kerx_declared_subtable_lengths_and_counts_cannot_hide_truncation() {
    parse_error(*b"kerx", u32s(&[0x00020000, 1]));
    parse_error(*b"kerx", u32s(&[0x00020000, 1, 0, 0, 0]));
    let mut bytes = kerx(&[kerx_pair()]);
    bytes.pop();
    parse_error(*b"kerx", bytes);
    let mut pair = kerx_pair();
    pair[15] = 2; // Two pairs declared; only one present.
    parse_error(*b"kerx", kerx(&[pair]));
    let mut action = kerx_action(2);
    action[31] = 0xff; // Action data offset past subtable end.
    parse_error(*b"kerx", kerx(&[action]));
}
