use fc_model::{ColorPalette, ColorTable, ColrLayer, GlyphName, PaletteColor};

fn fixture() -> (ColorTable, Vec<GlyphName>) {
    let color = ColorTable {
        graph: None,
        palette_version: 1,
        palettes: vec![ColorPalette {
            colors: vec![
                PaletteColor {
                    red: 240,
                    green: 12,
                    blue: 32,
                    alpha: 128,
                },
                PaletteColor {
                    red: 0,
                    green: 42,
                    blue: 200,
                    alpha: 255,
                },
            ],
            flags: 2,
            label: Some(256),
        }],
        entry_labels: vec![Some(257), None],
        glyphs: [(
            "base".into(),
            vec![
                ColrLayer {
                    glyph: "base".into(),
                    palette_index: Some(0),
                },
                ColrLayer {
                    glyph: "overlay".into(),
                    palette_index: None,
                },
            ],
        )]
        .into(),
    };
    (
        color,
        vec![".notdef".into(), "base".into(), "overlay".into()],
    )
}

#[test]
fn color_roundtrip_retains_order_alpha_foreground_and_v1_metadata() {
    let (mut color, order) = fixture();
    // Shared palettes and stacks are valid and must stay within packed limits.
    color.palettes.push(color.palettes[0].clone());
    color
        .glyphs
        .insert("overlay".into(), color.glyphs["base"].clone());
    let (colr, cpal) = fc_sfnt::color::write_tables(&color, &order).unwrap();
    assert_eq!(u16::from_be_bytes([cpal[6], cpal[7]]), 2);
    assert_eq!(&cpal[28..32], &[32, 12, 240, 128]);
    assert_eq!(
        fc_sfnt::color::read_tables(colr.as_deref(), &cpal, &order).unwrap(),
        color
    );
    let reversed = vec![order[2].clone(), order[0].clone(), order[1].clone()];
    let (colr, cpal) = fc_sfnt::color::write_tables(&color, &reversed).unwrap();
    assert_eq!(
        fc_sfnt::color::read_tables(colr.as_deref(), &cpal, &reversed).unwrap(),
        color
    );
}

#[test]
fn palette_only_and_v0_are_supported() {
    let (mut color, order) = fixture();
    color.palette_version = 0;
    color.entry_labels.clear();
    color.palettes[0].flags = 0;
    color.palettes[0].label = None;
    color.glyphs.clear();
    let (colr, cpal) = fc_sfnt::color::write_tables(&color, &order).unwrap();
    assert!(colr.is_none());
    assert_eq!(
        fc_sfnt::color::read_tables(None, &cpal, &order).unwrap(),
        color
    );
}

#[test]
fn malformed_and_unsupported_tables_fail_atomically() {
    let (color, order) = fixture();
    let (colr, cpal) = fc_sfnt::color::write_tables(&color, &order).unwrap();
    let colr = colr.unwrap();
    for end in 0..colr.len() {
        assert!(
            fc_sfnt::color::read_tables(Some(&colr[..end]), &cpal, &order).is_err(),
            "COLR truncation {end}"
        );
    }
    for end in 0..cpal.len() {
        assert!(
            fc_sfnt::color::read_tables(Some(&colr), &cpal[..end], &order).is_err(),
            "CPAL truncation {end}"
        );
    }
    for (at, value) in [(1, 1), (4, 255), (12, 255), (14, 255), (18, 255), (22, 255)] {
        let mut corrupt = colr.clone();
        corrupt[at] = value;
        assert!(
            fc_sfnt::color::read_tables(Some(&corrupt), &cpal, &order).is_err(),
            "offset {at}"
        );
    }
    let mut bad_palette = cpal.clone();
    bad_palette[12..14].copy_from_slice(&65535u16.to_be_bytes());
    assert!(fc_sfnt::color::read_tables(Some(&colr), &bad_palette, &order).is_err());
}

#[test]
fn invalid_model_cannot_write_partial_tables() {
    let (color, order) = fixture();
    for mode in 0..6 {
        let mut invalid = color.clone();
        match mode {
            0 => invalid.palettes.clear(),
            1 => invalid.palettes.push(ColorPalette::default()),
            2 => invalid.glyphs.get_mut("base").unwrap()[0].palette_index = Some(65535),
            3 => invalid.glyphs.get_mut("base").unwrap()[0].glyph = "missing".into(),
            4 => invalid.glyphs.get_mut("base").unwrap().clear(),
            _ => invalid.palettes[0].flags = 4,
        }
        assert!(
            fc_sfnt::color::write_tables(&invalid, &order).is_err(),
            "case {mode}"
        );
    }
}

#[test]
fn shared_ranges_cannot_amplify_into_unbounded_allocations() {
    use std::time::{Duration, Instant};
    let (_, order) = fixture();
    let start = Instant::now();
    // 65535 palettes sharing 65535 colors would expand to over four billion entries.
    let mut cpal = vec![0, 0, 255, 255, 255, 255, 255, 255, 0, 2, 0, 10];
    cpal.resize(12 + 65535 * 2 + 65535 * 4, 0);
    assert!(fc_sfnt::color::read_tables(None, &cpal, &order)
        .unwrap_err()
        .to_string()
        .contains("limit"));
    let (mut color, _) = fixture();
    color.glyphs.clear();
    let (_, cpal) = fc_sfnt::color::write_tables(&color, &order).unwrap();
    // 1001 bases share the same 1000-layer range, crossing the expansion cap.
    let bases = 1001usize;
    let mut colr = vec![0, 0];
    colr.extend((bases as u16).to_be_bytes());
    colr.extend(14u32.to_be_bytes());
    colr.extend((14 + bases as u32 * 6).to_be_bytes());
    colr.extend(1000u16.to_be_bytes());
    for gid in 0..bases {
        colr.extend((gid as u16).to_be_bytes());
        colr.extend([0, 0, 3, 232]);
    }
    for _ in 0..1000 {
        colr.extend([0, 0, 0, 0]);
    }
    let order = (0..bases)
        .map(|i| GlyphName::from(format!("g{i}")))
        .collect::<Vec<_>>();
    assert!(fc_sfnt::color::read_tables(Some(&colr), &cpal, &order)
        .unwrap_err()
        .to_string()
        .contains("excessive"));
    assert!(start.elapsed() < Duration::from_secs(10));
}

#[test]
fn large_color_table_roundtrip_has_a_bounded_work_budget() {
    use std::time::{Duration, Instant};
    let start = Instant::now();
    let (mut color, _) = fixture();
    let order = (0..20_000)
        .map(|i| GlyphName::from(format!("g{i}")))
        .collect::<Vec<_>>();
    color.glyphs = order
        .iter()
        .map(|name| {
            (
                name.clone(),
                vec![ColrLayer {
                    glyph: name.clone(),
                    palette_index: Some(1),
                }],
            )
        })
        .collect();
    let (colr, cpal) = fc_sfnt::color::write_tables(&color, &order).unwrap();
    assert_eq!(
        fc_sfnt::color::read_tables(colr.as_deref(), &cpal, &order).unwrap(),
        color
    );
    assert!(start.elapsed() < Duration::from_secs(10));
}
