use std::collections::HashSet;

use fw_compile::{export, shaping_fingerprint, ExportOptions};
use fw_model::{GlyphName, Tag};
use read_fonts::{types::GlyphId, FontRef, TableProvider};

#[test]
fn untouched_dejavu_preserves_raw_tables_hinting_and_shaping() {
    let original = std::fs::read(fw_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let font = fw_sfnt::read_bytes(&original, &fw_sfnt::ReadOptions::default())
        .expect("import fixture")
        .fonts
        .remove(0);
    let report = export(&font, &|_| 0, &ExportOptions::default()).expect("export untouched font");
    let output = FontRef::new(&report.bytes).expect("parse export");
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
        shaping_fingerprint(&original, "Hello fi AVWA").expect("shape original"),
        shaping_fingerprint(&report.bytes, "Hello fi AVWA").expect("shape export")
    );
}

#[test]
fn touching_one_glyph_drops_all_true_type_hinting() {
    let original = std::fs::read(fw_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let mut font = fw_sfnt::read_bytes(&original, &fw_sfnt::ReadOptions::default())
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
    let mut font = fw_model::Font::new_empty("Touch", 1_000);
    font.insert_glyph(fw_model::Glyph::new("A"))
        .expect("insert A");
    let touched = fw_compile::touched_since_import(&font, &|name: &GlyphName| {
        u64::from(name.as_ref() == "A")
    });
    assert_eq!(touched, HashSet::from([GlyphName::from("A")]));
}
