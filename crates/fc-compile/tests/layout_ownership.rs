mod common;

use fc_compile::{export, ExportOptions};
use fc_model::{FeaturesSource, Tag};
use read_fonts::FontRef;

#[test]
fn source_authority_never_resurrects_any_original_layout_table() {
    let original = export(&common::scratch_font(), &|_| 0, &ExportOptions::default()).unwrap();
    let mut imported = fc_sfnt::read_bytes(&original.bytes, &fc_sfnt::ReadOptions::default())
        .unwrap()
        .fonts
        .remove(0);
    imported.features.source_of_truth = FeaturesSource::Fea;
    imported.masters[0].kerning_h.0.clear();
    // Even absence from the newly compiled layout is an ownership decision.
    for tag in [
        *b"GSUB", *b"GPOS", *b"GDEF", *b"BASE", *b"kern", *b"kerx", *b"morx", *b"mort", *b"JSTF",
    ] {
        imported.extra.raw_tables.insert(Tag(tag), vec![0, 1, 0, 0]);
    }
    let report = export(&imported, &|_| 0, &ExportOptions::default()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    for tag in [
        *b"GSUB", *b"GPOS", *b"GDEF", *b"BASE", *b"kern", *b"kerx", *b"morx", *b"mort", *b"JSTF",
    ] {
        assert!(
            face.table_data(read_fonts::types::Tag::new(&tag)).is_none(),
            "{} restored",
            Tag(tag)
        );
    }
}

#[test]
fn imported_authority_preserves_competing_layout_when_order_is_unchanged() {
    let original = export(&common::scratch_font(), &|_| 0, &ExportOptions::default()).unwrap();
    let mut imported = fc_sfnt::read_bytes(&original.bytes, &fc_sfnt::ReadOptions::default())
        .unwrap()
        .fonts
        .remove(0);
    assert_eq!(
        imported.features.source_of_truth,
        FeaturesSource::ImportedTables
    );
    for tag in [*b"kern", *b"kerx", *b"morx", *b"mort", *b"JSTF"] {
        imported.extra.raw_tables.insert(Tag(tag), vec![0, 1, 0, 0]);
    }
    let report = export(&imported, &|_| 0, &ExportOptions::default()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    for tag in [*b"kern", *b"kerx", *b"morx", *b"mort", *b"JSTF"] {
        assert_eq!(
            face.table_data(read_fonts::types::Tag::new(&tag))
                .unwrap()
                .as_bytes(),
            &[0, 1, 0, 0]
        );
    }
}

#[test]
fn changed_glyph_order_drops_aat_tables_with_glyph_references() {
    let original = export(&common::scratch_font(), &|_| 0, &ExportOptions::default()).unwrap();
    let mut imported = fc_sfnt::read_bytes(&original.bytes, &fc_sfnt::ReadOptions::default())
        .unwrap()
        .fonts
        .remove(0);
    for tag in [*b"kerx", *b"morx", *b"mort"] {
        imported.extra.raw_tables.insert(Tag(tag), vec![0, 1, 0, 0]);
    }
    imported.glyph_mut("V").unwrap().export = false;
    let report = export(&imported, &|_| 0, &ExportOptions::default()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    for tag in [*b"kerx", *b"morx", *b"mort"] {
        assert!(
            face.table_data(read_fonts::types::Tag::new(&tag)).is_none(),
            "{} restored",
            Tag(tag)
        );
    }
}
