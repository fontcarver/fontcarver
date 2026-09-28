mod common;
use fc_compile::{export, ExportOptions};
use fc_model::{SvgDocument, SvgTable, Tag};

#[test]
fn production_names_exclusions_and_explicit_deletion_keep_svg_ownership() {
    let mut font = common::scratch_font();
    font.rename_glyph("A", "custom.A").unwrap();
    font.svg_table_owned = true;
    font.svg = Some(SvgTable { documents: vec![SvgDocument {
        source: r##"<svg xmlns="http://www.w3.org/2000/svg"><path id="artA" data-meaning="A" d="M0 0L10 -20"/><path id="artV" data-meaning="V" d="M10 0L0 -20"/></svg>"##.into(),
        glyphs: [("custom.A".into(), "artA".into()), ("V".into(), "artV".into())].into(),
    }] });
    for production_names in [false, true] {
        let report = export(
            &font,
            &|_| 0,
            &ExportOptions {
                production_names,
                ..Default::default()
            },
        )
        .unwrap();
        let output = fc_sfnt::read_bytes(&report.bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let name = &output
            .glyphs
            .values()
            .find(|g| g.unicodes.contains(&65))
            .unwrap()
            .name;
        let svg = output.svg.as_ref().unwrap();
        let document = svg
            .documents
            .iter()
            .find(|d| d.glyphs.contains_key(name))
            .unwrap();
        let id = &document.glyphs[name];
        assert!(document
            .source
            .contains(&format!("id=\"{id}\" data-meaning=\"A\"")));
    }
    font.glyph_mut("V").unwrap().export = false;
    let report = export(&font, &|_| 0, &Default::default()).unwrap();
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "svg-references-dropped"));
    let output = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert_eq!(output.svg.unwrap().documents[0].glyphs.len(), 1);
    assert_eq!(font.svg.as_ref().unwrap().documents[0].glyphs.len(), 2);
    font.svg = None;
    font.extra.raw_tables.insert(Tag(*b"SVG "), vec![0; 10]);
    let report = export(&font, &|_| 0, &Default::default()).unwrap();
    assert!(read_fonts::FontRef::new(&report.bytes)
        .unwrap()
        .table_data(read_fonts::types::Tag::new(b"SVG "))
        .is_none());
}
