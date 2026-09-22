use fc_layout::{widths_for_text, widths_for_text_indexed};
use fc_model::{Font, Glyph, Layer, UnicodeIndex};
#[test]
fn indexed_widths_agree_with_full_scan_and_scale_with_text() {
    let mut font = Font::new_empty("Indexed CJK", 1000);
    let master = font.masters[0].id.clone();
    for cp in 0x10000..0x20000 {
        let mut glyph = Glyph::new(format!("u{cp:X}"));
        glyph.unicodes = vec![cp];
        let mut layer = Layer::new_master(master.clone());
        layer.width = 900.;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    let index = UnicodeIndex::new(&font);
    let text = "\u{10000}\u{10004}\u{1fffe}";
    let expected = widths_for_text(&font, &master, text);
    assert_eq!(
        widths_for_text_indexed(&font, &index, &master, text),
        expected
    );
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        assert_eq!(
            widths_for_text_indexed(&font, &index, &master, text),
            expected
        );
    }
    assert!(
        start.elapsed() < std::time::Duration::from_secs(1),
        "1000 indexed queries: {:?}",
        start.elapsed()
    );
}
