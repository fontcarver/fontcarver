mod common;

use std::collections::HashSet;

use fc_compile::{export, shaping_fingerprint, ExportOptions};
use fc_model::{FeaturesSource, GlyphName, Tag};
use read_fonts::{types::GlyphId, FontRef, TableProvider};
use write_fonts::FontBuilder;

#[test]
fn untouched_dejavu_preserves_raw_tables_hinting_and_shaping() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export untouched font");
    let output = FontRef::new(&report.bytes).expect("parse export");
    let input = FontRef::new(&original).expect("parse original");
    for tag in [*b"glyf", *b"loca", *b"maxp"] {
        assert_eq!(
            output
                .table_data(read_fonts::types::Tag::new(&tag))
                .expect("output preserved table")
                .as_bytes(),
            input
                .table_data(read_fonts::types::Tag::new(&tag))
                .expect("input preserved table")
                .as_bytes(),
            "{} was not preserved verbatim",
            Tag(tag)
        );
    }
    for (tag, expected) in &font.extra.raw_tables {
        let actual = output
            .table_data(read_fonts::types::Tag::new(&tag.0))
            .unwrap_or_else(|| panic!("missing preserved {tag}"));
        assert_eq!(actual.as_bytes(), expected, "raw table {tag} changed");
    }
    for tag in [*b"fpgm", *b"prep", *b"cvt "] {
        assert!(
            output
                .table_data(read_fonts::types::Tag::new(&tag))
                .is_some(),
            "missing {}",
            Tag(tag)
        );
    }
    assert!(!glyph_instructions(&output, GlyphId::new(36)).is_empty());
    assert_eq!(
        simple_points(&input, GlyphId::new(36)),
        simple_points(&output, GlyphId::new(36)),
        "A point coordinates/order changed on the preservation path"
    );
    assert_eq!(
        shaping_fingerprint(&original, "Hello fi AVWA").expect("shape original"),
        shaping_fingerprint(&report.bytes, "Hello fi AVWA").expect("shape export")
    );
}

#[test]
fn ambiguous_ttc_order_keeps_the_selected_faces_modeled_outline() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let mut edited = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    edited.glyph_mut("A").expect("A").layers[0].contours[0].points[0].x += 37.0;
    let modeled = export(
        &edited,
        &|name| u64::from(name.as_ref() == "A"),
        &ExportOptions::default(),
    )
    .expect("compile changed face")
    .bytes;
    let selected_bytes = with_original_global_hinting(&original, &modeled);
    let ttc = two_face_ttc(&original, &selected_bytes);
    let selected = fc_sfnt::read_bytes(
        &ttc,
        &fc_sfnt::ReadOptions {
            font_index: Some(1),
            ..fc_sfnt::ReadOptions::default()
        },
    )
    .expect("import selected TTC face")
    .fonts
    .remove(0);

    let report = export(&selected, &|_| 0, &ExportOptions::default())
        .expect("export untouched selected face");
    let first = FontRef::new(&original).expect("parse first face");
    let second = FontRef::new(&selected_bytes).expect("parse selected face");
    let output = FontRef::new(&report.bytes).expect("parse export");
    let gid = GlyphId::new(36);
    assert_ne!(
        sorted_simple_points(&first, gid),
        sorted_simple_points(&second, gid)
    );
    assert_eq!(
        sorted_simple_points(&output, gid),
        sorted_simple_points(&second, gid)
    );
    assert!(!glyph_instructions(&first, gid).is_empty());
    assert!(glyph_instructions(&second, gid).is_empty());
    assert!(glyph_instructions(&output, gid).is_empty());
    assert!(report.warnings.iter().any(|warning| {
        warning.code == "hinting-dropped" && warning.message.contains("outline bytes")
    }));
}

fn with_original_global_hinting(original: &[u8], modeled: &[u8]) -> Vec<u8> {
    let original = FontRef::new(original).expect("parse original");
    let modeled = FontRef::new(modeled).expect("parse modeled face");
    let mut builder = FontBuilder::new();
    for record in modeled.table_directory().table_records() {
        let data = modeled.table_data(record.tag()).expect("modeled table");
        builder.add_raw(record.tag(), data.as_bytes().to_vec());
    }
    for tag in [*b"fpgm", *b"prep", *b"cvt "] {
        let tag = read_fonts::types::Tag::new(&tag);
        let data = original.table_data(tag).expect("fixture hinting table");
        builder.add_raw(tag, data.as_bytes().to_vec());
    }
    builder.build()
}

fn two_face_ttc(first: &[u8], second: &[u8]) -> Vec<u8> {
    const HEADER_LEN: usize = 20;
    let first_offset = HEADER_LEN;
    let second_offset = (first_offset + first.len() + 3) & !3;
    let first = sfnt_with_collection_offsets(first, first_offset);
    let second = sfnt_with_collection_offsets(second, second_offset);
    let mut ttc = Vec::with_capacity(second_offset + second.len());
    ttc.extend_from_slice(b"ttcf");
    ttc.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    ttc.extend_from_slice(&2_u32.to_be_bytes());
    ttc.extend_from_slice(&(first_offset as u32).to_be_bytes());
    ttc.extend_from_slice(&(second_offset as u32).to_be_bytes());
    ttc.extend_from_slice(&first);
    ttc.resize(second_offset, 0);
    ttc.extend_from_slice(&second);
    ttc
}

fn sfnt_with_collection_offsets(sfnt: &[u8], collection_offset: usize) -> Vec<u8> {
    let mut adjusted = sfnt.to_vec();
    let table_count = usize::from(u16::from_be_bytes([adjusted[4], adjusted[5]]));
    for index in 0..table_count {
        let position = 12 + index * 16 + 8;
        let old = u32::from_be_bytes(
            adjusted[position..position + 4]
                .try_into()
                .expect("table offset"),
        );
        adjusted[position..position + 4]
            .copy_from_slice(&(old + collection_offset as u32).to_be_bytes());
    }
    adjusted
}

fn simple_points(face: &FontRef<'_>, gid: GlyphId) -> Vec<(i16, i16, bool)> {
    let glyf = face.glyf().expect("glyf");
    let loca = face.loca(None).expect("loca");
    let glyph = loca
        .get_glyf(gid, &glyf)
        .expect("glyph data")
        .expect("glyph");
    let read_fonts::tables::glyf::Glyph::Simple(simple) = glyph else {
        panic!("expected simple glyph")
    };
    simple
        .points()
        .map(|point| (point.x, point.y, point.on_curve))
        .collect()
}

fn sorted_simple_points(face: &FontRef<'_>, gid: GlyphId) -> Vec<(i16, i16, bool)> {
    let mut points = simple_points(face, gid);
    points.sort_unstable();
    points
}

#[test]
fn touching_one_glyph_drops_all_true_type_hinting() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let mut font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    font.glyph_mut("A").expect("A").layers[0].contours[0].points[0].x += 1.0;
    let report = export(
        &font,
        &|name| u64::from(name.as_ref() == "A"),
        &ExportOptions::default(),
    )
    .expect("export edited font");
    let warning = report
        .warnings
        .iter()
        .find(|warning| warning.code == "hinting-dropped")
        .expect("hinting warning");
    assert!(warning.message.contains('A'));
    let output = FontRef::new(&report.bytes).expect("parse export");
    for tag in [*b"fpgm", *b"prep", *b"cvt "] {
        assert!(output
            .table_data(read_fonts::types::Tag::new(&tag))
            .is_none());
    }
    let count = output.maxp().expect("maxp").num_glyphs();
    for gid in 0..count {
        assert!(glyph_instructions(&output, GlyphId::new(u32::from(gid))).is_empty());
    }
}

fn glyph_instructions(face: &FontRef<'_>, gid: GlyphId) -> Vec<u8> {
    let glyf = face.glyf().expect("glyf");
    let loca = face.loca(None).expect("loca");
    match loca.get_glyf(gid, &glyf).expect("glyph data") {
        Some(read_fonts::tables::glyf::Glyph::Simple(glyph)) => glyph.instructions().to_vec(),
        Some(read_fonts::tables::glyf::Glyph::Composite(glyph)) => {
            glyph.instructions().unwrap_or_default().to_vec()
        }
        None => Vec::new(),
    }
}

#[test]
fn helper_marks_positive_generations_as_touched() {
    let mut font = fc_model::Font::new_empty("Touch", 1_000);
    font.insert_glyph(fc_model::Glyph::new("A"))
        .expect("insert A");
    let touched = fc_compile::touched_since_import(&font, &|name: &GlyphName| {
        u64::from(name.as_ref() == "A")
    });
    assert_eq!(touched, HashSet::from([GlyphName::from("A")]));
}

#[test]
fn serialized_provenance_keeps_layout_but_drops_unavailable_hinting_safely() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    let reopened: fc_model::Font =
        serde_json::from_str(&serde_json::to_string(&font).expect("serialize font"))
            .expect("reopen font");

    let report = export(&reopened, &|_| 0, &ExportOptions::default()).expect("export reopened");
    let output = FontRef::new(&report.bytes).expect("parse export");
    assert!(output
        .table_data(read_fonts::types::Tag::new(b"GSUB"))
        .is_some());
    assert!(!report
        .warnings
        .iter()
        .any(|warning| warning.code == "preservation-unavailable-no-provenance"));
    assert!(report.warnings.iter().any(|warning| {
        warning.code == "hinting-dropped" && warning.message.contains("outline bytes")
    }));
}

#[test]
fn missing_order_provenance_uses_one_distinct_preservation_warning() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let mut font = fc_sfnt::read_bytes(
        &original,
        &fc_sfnt::ReadOptions {
            keep_original_bytes: false,
            ..fc_sfnt::ReadOptions::default()
        },
    )
    .expect("import fixture")
    .fonts
    .remove(0);
    font.extra
        .source
        .as_mut()
        .expect("provenance")
        .glyph_order
        .clear();

    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export font");
    assert_eq!(
        report
            .warnings
            .iter()
            .filter(|warning| warning.code == "preservation-unavailable-no-provenance")
            .count(),
        1
    );
    assert!(!report
        .warnings
        .iter()
        .any(|warning| warning.code == "layout-dropped-glyph-order-changed"));
}

#[test]
fn changed_order_drops_all_gid_indexed_and_variation_raw_tables() {
    const GID_TAGS: [[u8; 4]; 17] = [
        *b"kern", *b"MATH", *b"hdmx", *b"VDMX", *b"LTSH", *b"EBDT", *b"EBLC", *b"CBDT", *b"CBLC",
        *b"sbix", *b"COLR", *b"CPAL", *b"SVG ", *b"VORG", *b"JSTF", *b"BASE", *b"GDEF",
    ];
    const VAR_TAGS: [[u8; 4]; 7] = [
        *b"gvar", *b"HVAR", *b"MVAR", *b"cvar", *b"avar", *b"fvar", *b"STAT",
    ];
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let mut font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    font.features.source_of_truth = FeaturesSource::ImportedTables;
    font.glyph_mut("V").expect("V").export = false;
    for tag in GID_TAGS.into_iter().chain(VAR_TAGS) {
        font.extra.raw_tables.insert(Tag(tag), vec![0, 1, 2, 3]);
    }

    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export changed font");
    let output = FontRef::new(&report.bytes).expect("parse export");
    for tag in GID_TAGS.into_iter().chain(VAR_TAGS) {
        assert!(
            output
                .table_data(read_fonts::types::Tag::new(&tag))
                .is_none(),
            "{} survived changed-order export",
            Tag(tag)
        );
    }
    let raw_warning = report
        .warnings
        .iter()
        .find(|warning| warning.code == "raw-table-dropped-glyph-order-changed")
        .expect("gid-indexed warning");
    assert!(raw_warning.message.contains("kern"));
    assert!(raw_warning.message.contains("MATH"));
    let variation_warning = report
        .warnings
        .iter()
        .find(|warning| warning.code == "variation-tables-dropped")
        .expect("variation warning");
    assert!(variation_warning.message.contains("gvar"));
    assert!(variation_warning.message.contains("STAT"));
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.code == "hinting-dropped-glyph-set-changed"));
}

#[test]
fn touching_an_unhinted_font_does_not_claim_hinting_was_dropped() {
    let font = common::scratch_font();
    let report = export(
        &font,
        &|name| u64::from(name.as_ref() == "A"),
        &ExportOptions::default(),
    )
    .expect("export unhinted font");
    assert!(!report
        .warnings
        .iter()
        .any(|warning| warning.code.starts_with("hinting-dropped")));
}

#[test]
fn disabling_table_preservation_reports_dropped_tags() {
    let original = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            preserve_untouched_tables: false,
            ..ExportOptions::default()
        },
    )
    .expect("export without preservation");
    let warning = report
        .warnings
        .iter()
        .find(|warning| warning.code == "tables-not-preserved")
        .expect("tables warning");
    assert!(warning.message.contains("GSUB"));
    assert!(warning.message.contains("GPOS"));
}
