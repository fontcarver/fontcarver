use fc_model::{GlyphName, SvgDocument, SvgTable};
use fc_sfnt::svg::{read_table, write_table};
use std::io::Write;
fn order() -> Vec<GlyphName> {
    [".notdef", "A", "B", "C"].map(Into::into).to_vec()
}
fn table() -> SvgTable {
    SvgTable { documents: vec![SvgDocument {
        source: r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><defs><linearGradient id="ink"><stop stop-color="#abc"/></linearGradient><path id="base" d="M0 0L20 -30"/></defs><g id="glyph1"><use xlink:href="#base" fill="url( '#ink' )"/></g><g id="glyph3"><use href="#glyph1"/></g></svg>"##.into(),
        glyphs: [("A".into(), "glyph1".into()), ("C".into(), "glyph3".into())].into(),
    }] }
}
#[test]
fn sharing_order_remapping_and_duplicate_bindings_preserve_references() {
    let mut table = table();
    let order = ["C", ".notdef", "B", "A"].map(Into::into);
    table.duplicate_glyph(&"A".into(), &"B".into());
    let data = write_table(&table, &order).unwrap();
    let decoded = read_table(&data, &order).unwrap();
    assert_eq!(decoded.documents.len(), 2);
    for doc in &decoded.documents {
        let ids = fc_model::svg::validate_xml(&doc.source).unwrap();
        for (name, id) in &doc.glyphs {
            assert_eq!(
                *id,
                format!("glyph{}", order.iter().position(|n| n == name).unwrap())
            );
            assert!(ids.contains(id));
        }
        assert!(doc.source.contains("stop-color=\"#abc\""));
    }
    assert_eq!(
        decoded
            .documents
            .iter()
            .map(|d| d.glyphs.len())
            .sum::<usize>(),
        3
    );
}
#[test]
fn all_truncations_and_bad_ranges_fail_without_panicking() {
    let data = write_table(&table(), &order()).unwrap();
    for end in 0..data.len() {
        assert!(read_table(&data[..end], &order()).is_err(), "{end}");
    }
    for (offset, bytes) in [
        (0, vec![0, 1]),
        (2, vec![255; 4]),
        (6, vec![0, 0, 0, 1]),
        (12, vec![0, 9]),
        (16, vec![0; 4]),
        (20, vec![255; 4]),
        (24, vec![0, 0]),
    ] {
        let mut bad = data.clone();
        bad[offset..offset + bytes.len()].copy_from_slice(&bytes);
        assert!(read_table(&bad, &order()).is_err(), "offset {offset}");
    }
}
fn compressed(source: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(source).unwrap();
    let payload = encoder.finish().unwrap();
    let mut data = vec![0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 14];
    data.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    data.extend(payload);
    data
}
#[test]
fn gzip_is_bounded_and_invalid_xml_is_rejected() {
    let source = r#"<svg xmlns="http://www.w3.org/2000/svg" id="glyph1"><rect width="10" height="10"/></svg>"#;
    assert_eq!(
        read_table(&compressed(source.as_bytes()), &order())
            .unwrap()
            .documents[0]
            .source,
        source
    );
    assert!(read_table(
        &compressed(&vec![b' '; fc_model::svg::MAX_SVG_DOCUMENT_BYTES + 1]),
        &order()
    )
    .is_err());
    for source in [
        "<svg/>",
        "<!DOCTYPE svg [<!ENTITY a 'x'>]><svg/>",
        r#"<svg xmlns="http://www.w3.org/2000/svg"><script/></svg>"#,
        r##"<svg xmlns="http://www.w3.org/2000/svg"><path id="x"/><path id="x"/></svg>"##,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><use href="https://example.com/x.svg"/></svg>"#,
        r##"<svg xmlns="http://www.w3.org/2000/svg"><path fill="url(#missing)"/></svg>"##,
    ] {
        assert!(fc_model::svg::validate_xml(source).is_err(), "{source}");
    }
}
#[test]
fn attribute_entities_whitespace_and_prefix_collisions_are_rewritten_exactly() {
    let source = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg"><g id{}= 'glyph1'/><g id="fcsvg0"><use href="&#35;glyph1"/></g></svg>"##,
        " ".repeat(300)
    );
    let table = SvgTable {
        documents: vec![SvgDocument {
            source,
            glyphs: [("B".into(), "glyph1".into())].into(),
        }],
    };
    let data = write_table(&table, &order()).unwrap();
    let decoded = read_table(&data, &order()).unwrap();
    assert!(decoded.documents[0].source.contains("href=\"#glyph2\""));
}

#[test]
fn cycles_and_exponential_use_expansion_are_rejected() {
    let cycle = r##"<svg xmlns="http://www.w3.org/2000/svg"><g id="a"><use href="#b"/></g><g id="b"><use href="#a"/></g></svg>"##;
    assert!(fc_model::svg::validate_xml(cycle)
        .unwrap_err()
        .contains("cyclic"));
    let mut bomb = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg"><path id="a0"/>"#);
    for i in 1..24 {
        bomb.push_str(&format!(
            r##"<g id="a{i}"><use href="#a{}"/><use href="#a{}"/></g>"##,
            i - 1,
            i - 1
        ));
    }
    bomb.push_str("</svg>");
    assert!(fc_model::svg::validate_xml(&bomb)
        .unwrap_err()
        .contains("expansion"));
}

#[test]
fn twenty_thousand_glyphs_share_a_document_with_bounded_order_remapping() {
    let order: Vec<GlyphName> = (0..20_001).map(|i| format!("g{i}").into()).collect();
    let mut source = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg">"#);
    let mut glyphs = std::collections::BTreeMap::new();
    for name in order.iter().skip(1) {
        source.push_str(&format!(r#"<path id="{name}" d="M0 0L10 -10"/>"#));
        glyphs.insert(name.clone(), name.to_string());
    }
    source.push_str("</svg>");
    let table = SvgTable {
        documents: vec![SvgDocument { source, glyphs }],
    };
    let mut reversed = order.clone();
    reversed.reverse();
    let start = std::time::Instant::now();
    let bytes = write_table(&table, &reversed).unwrap();
    let read = read_table(&bytes, &reversed).unwrap();
    assert_eq!(read.documents.len(), 1);
    assert_eq!(read.documents[0].glyphs.len(), 20_000);
    assert_eq!(read.documents[0].glyphs["g20000"], "glyph0");
    assert_eq!(read.documents[0].glyphs["g1"], "glyph19999");
    assert!(
        start.elapsed().as_secs_f64() < 10.,
        "SVG 20k remap budget: {:?}",
        start.elapsed()
    );
}
