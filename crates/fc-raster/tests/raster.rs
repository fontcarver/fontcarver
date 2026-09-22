use fc_model::{Bitmap, Font, Glyph, Layer};
#[test]
fn exact_trace_roundtrip_preserves_holes_diagonal_contacts_and_origins() {
    for rows in [
        [0b11100, 0b10100, 0b11100, 0b00010, 0b00001],
        [0b11111, 0, 0b10101, 0, 0b11111],
    ] {
        let mut bitmap = Bitmap::new(5, 5, 1).unwrap();
        bitmap.origin_x = -2;
        bitmap.origin_y = -1;
        for (y, row) in rows.into_iter().enumerate() {
            for x in 0..5 {
                bitmap.set_pixel(x, y as u16, (row >> (4 - x)) & 1).unwrap();
            }
        }
        let contours = fc_raster::trace_bitmap(&bitmap, 10, 1000, 128).unwrap();
        assert!(contours.iter().all(|c| c.closed));
        let mut font = Font::new_empty("Trace", 1000);
        let master = font.default_master().id.clone();
        let mut g = Glyph::new("test");
        let mut layer = Layer::new_master(master.clone());
        layer.contours = contours;
        layer.width = 600.0;
        g.layers.push(layer);
        font.insert_glyph(g).unwrap();
        let strike = fc_raster::rasterize(&font, &master, 10, 1, &["test".into()]).unwrap();
        assert_eq!(strike.glyphs["test"].bitmap, bitmap);
    }
}
#[test]
fn coverage_and_invalid_limits() {
    let mut bitmap = Bitmap::new(3, 3, 8).unwrap();
    bitmap.set_pixel(1, 1, 127).unwrap();
    assert!(fc_raster::trace_bitmap(&bitmap, 16, 1000, 128)
        .unwrap()
        .is_empty());
    assert_eq!(
        fc_raster::trace_bitmap(&bitmap, 16, 1000, 127)
            .unwrap()
            .len(),
        1
    );
    assert!(fc_raster::trace_bitmap(&bitmap, 0, 1000, 1).is_err());
    let font = Font::new_empty("Limits", 1000);
    assert!(fc_raster::rasterize(&font, &font.default_master().id, 5000, 1, &[]).is_err());
}
