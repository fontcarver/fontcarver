use fc_model::{Bitmap, BitmapGlyph, BitmapRef, BitmapStrike, Font, Glyph};

#[test]
fn packed_samples_cover_all_depths_and_row_padding() {
    for depth in [1, 2, 4, 8, 32] {
        let mut bitmap = Bitmap::new(9, 3, depth).unwrap();
        let max = if depth == 32 {
            u32::MAX
        } else {
            (1 << depth) - 1
        };
        bitmap.set_pixel(8, 2, max).unwrap();
        assert_eq!(bitmap.pixel(8, 2), max);
        assert_eq!(bitmap.pixel(0, 2), 0);
        assert_eq!(bitmap.coverage(8, 2), 255);
        bitmap.validate().unwrap();
        assert!(bitmap.set_pixel(9, 0, 0).is_err());
        if depth < 32 {
            assert!(bitmap.set_pixel(0, 0, max + 1).is_err());
        }
    }
    let mut bitmap = Bitmap::new(1, 1, 1).unwrap();
    bitmap.rows[0] = 1;
    assert!(bitmap.validate().is_err());
    assert!(Bitmap::new(4097, 2, 1).is_err());
    assert!(Bitmap::new(4096, 4096, 32).is_err());
}

#[test]
fn references_resolve_y_up_origins_and_reject_cycles() {
    let mut pixels = Bitmap::new(1, 1, 1).unwrap();
    pixels.set_pixel(0, 0, 1).unwrap();
    let mut strike = BitmapStrike {
        ppem: 16,
        depth: 1,
        ..Default::default()
    };
    strike.glyphs.insert(
        "base".into(),
        BitmapGlyph {
            bitmap: pixels,
            advance: 2,
            ..Default::default()
        },
    );
    strike.glyphs.insert(
        "ref".into(),
        BitmapGlyph {
            bitmap: Bitmap::new(0, 0, 1).unwrap(),
            advance: 3,
            refs: vec![BitmapRef {
                base: "base".into(),
                dx: -1,
                dy: 2,
            }],
            ..Default::default()
        },
    );
    strike.validate().unwrap();
    let result = strike.flattened(&"ref".into()).unwrap();
    assert_eq!(result.advance, 3);
    assert_eq!(result.bitmap.origin_x, -1);
    assert_eq!(result.bitmap.pixel(0, 0), 1);
    strike.glyphs.get_mut("base").unwrap().refs.push(BitmapRef {
        base: "ref".into(),
        ..Default::default()
    });
    assert!(strike.validate().is_err());
    assert!(strike.flattened(&"ref".into()).is_err());
}

#[test]
fn renaming_moves_bitmap_ownership_and_reference_names() {
    let mut font = Font::new_empty("Bitmap", 1000);
    font.insert_glyph(Glyph::new("a")).unwrap();
    let mut strike = BitmapStrike {
        ppem: 16,
        depth: 1,
        ..Default::default()
    };
    strike.glyphs.insert(
        "a".into(),
        BitmapGlyph {
            bitmap: Bitmap::new(0, 0, 1).unwrap(),
            ..Default::default()
        },
    );
    font.bitmap_strikes.push(strike);
    font.rename_glyph("a", "b").unwrap();
    font.validate_bitmaps().unwrap();
    assert!(font.bitmap_strikes[0].glyphs.contains_key("b"));
}
