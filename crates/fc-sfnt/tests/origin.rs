#[path = "support/tt.rs"]
mod support;
use fc_model::flatten_layer;
use kurbo::Shape;
use read_fonts::types::Tag;

#[test]
fn side_bearings_normalize_outline_origins_without_moving_component_assembly() {
    let child = support::simple(&[(60, 0, true), (160, 0, true), (160, 100, true)], &[]);
    let mut parent = support::composite(&[(2 | 0x80, 0, 30, 20, &[16384, 8192, 0, 16384])]);
    for (at, value) in [(2, 90i16), (4, 50), (6, 190), (8, 200)] {
        parent[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }
    let bytes = support::font(&[child, parent]);
    let bytes = support::replace(
        &bytes,
        Tag::new(b"hmtx"),
        [600u16, 10, 600, 40]
            .into_iter()
            .flat_map(u16::to_be_bytes)
            .collect(),
    );
    let font = fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let names: Vec<_> = font.glyph_order().collect();
    let master = &font.default_master().id;
    let simple = flatten_layer(&font, names[0], master)
        .unwrap()
        .bounding_box();
    let composite = flatten_layer(&font, names[1], master)
        .unwrap()
        .bounding_box();
    assert_eq!((simple.x0, simple.x1), (10., 110.));
    assert_eq!(
        (composite.x0, composite.x1, composite.y0, composite.y1),
        (40., 140., 50., 200.)
    );
}

#[test]
fn header_only_and_omitted_empty_glyphs_need_no_origin_adjustment() {
    for glyph in [vec![], vec![0; 10], support::simple(&[], &[])] {
        let font = fc_sfnt::read_bytes(&support::font(&[glyph]), &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let layer = &font.glyphs.values().next().unwrap().layers[0];
        assert!(layer.contours.is_empty());
        assert_eq!(layer.width, 600.);
    }
}
