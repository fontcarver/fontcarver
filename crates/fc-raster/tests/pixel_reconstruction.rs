use fc_model::{Bitmap, Component, Contour, Font, Glyph, Layer, Point, PointKind};
use fc_raster::{detect_pixel_font, reconstruct_pixel_font, trace_bitmap};
use kurbo::Affine;

fn pixel_font(upm: u16, ppem: u16) -> (Font, Vec<(String, Bitmap)>) {
    let mut font = Font::new_empty("Pixel reconstruction", upm);
    let master = font.default_master().id.clone();
    let grid = f64::from(upm) / f64::from(ppem);
    font.masters[0].metrics.ascender = 10.0 * grid;
    font.masters[0].metrics.descender = -3.0 * grid;
    let masks = [
        [
            0b1100000, 0b0110000, 0b0011000, 0b0001100, 0b0000110, 0b0000011, 0b0000001,
        ],
        [
            0b0000011, 0b0000110, 0b0001100, 0b0011000, 0b0110000, 0b1100000, 0b1000000,
        ],
        [
            0b1110000, 0b0111000, 0b0011100, 0b0001110, 0b0000111, 0b0000011, 0b0000001,
        ],
        [
            0b1111111, 0b1000001, 0b1000001, 0b1000001, 0b1000001, 0b1000001, 0b1111111,
        ],
    ];
    let mut expected = Vec::new();
    for (index, rows) in masks.into_iter().enumerate() {
        let mut bitmap = Bitmap::new(7, 7, 1).unwrap();
        bitmap.origin_x = -2;
        bitmap.origin_y = -1;
        for (y, row) in rows.into_iter().enumerate() {
            for x in 0..7 {
                bitmap.set_pixel(x, y as u16, (row >> (6 - x)) & 1).unwrap();
            }
        }
        let name = format!("pixel{index}");
        let mut glyph = Glyph::new(name.as_str());
        let mut layer = Layer::new_master(master.clone());
        layer.width = 10.0 * grid;
        layer.vertical_width = Some(12.0 * grid);
        layer.vertical_origin = Some(8.0 * grid);
        layer.contours = trace_bitmap(&bitmap, ppem, upm, 128).unwrap();
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
        expected.push((name, bitmap));
    }
    let mut space = Glyph::new("space");
    let mut layer = Layer::new_master(master);
    layer.width = 5.0 * grid;
    space.layers.push(layer);
    font.insert_glyph(space).unwrap();
    expected.push(("space".into(), Bitmap::new(0, 0, 1).unwrap()));
    (font, expected)
}

#[test]
fn exact_grid_recovers_size_pixels_holes_origins_and_metrics() {
    for upm in [1000, 1024] {
        let (font, expected) = pixel_font(upm, 16);
        let master = &font.default_master().id;
        let detection = detect_pixel_font(&font, master).unwrap().unwrap();
        assert_eq!(detection.ppem, 16);
        assert_eq!(detection.grid_size, f64::from(upm) / 16.0);
        assert_eq!(detection.glyph_count, 5);
        assert_eq!(detection.distinctive_glyphs, 3);
        let names = expected
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let strike = reconstruct_pixel_font(&font, master, &names)
            .unwrap()
            .unwrap();
        assert_eq!(
            (strike.ppem, strike.depth, strike.ascent, strike.descent),
            (16, 1, 10, 3)
        );
        for (name, bitmap) in expected {
            let actual = &strike.glyphs[name.as_str()];
            assert_eq!(actual.bitmap, bitmap);
            assert_eq!(actual.advance, if name == "space" { 5 } else { 10 });
            if name != "space" {
                assert_eq!(actual.vertical_advance, Some(12));
                assert_eq!(actual.vertical_origin, Some((5, 8)));
            }
        }
    }
}

#[test]
fn translated_components_preserve_recovery_and_do_not_add_evidence() {
    let (mut font, _) = pixel_font(1024, 16);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("component");
    let mut layer = Layer::new_master(master.clone());
    layer.width = 640.0;
    layer.components.push(Component {
        base: "pixel0".into(),
        transform: Affine::translate((64.0, -128.0)),
        ..Default::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let detection = detect_pixel_font(&font, &master).unwrap().unwrap();
    assert_eq!(detection.distinctive_glyphs, 3);
    let strike = reconstruct_pixel_font(&font, &master, &["component".into()])
        .unwrap()
        .unwrap();
    let bitmap = &strike.glyphs["component"].bitmap;
    assert_eq!((bitmap.origin_x, bitmap.origin_y), (-1, -3));
}

#[test]
fn rectangle_only_and_repeated_outlines_are_not_pixel_evidence() {
    let (mut font, _) = pixel_font(1024, 16);
    let master = font.default_master().id.clone();
    let contours = font
        .glyph("pixel3")
        .unwrap()
        .master_layer(&master)
        .unwrap()
        .contours
        .clone();
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].contours = contours.clone();
    }
    assert!(detect_pixel_font(&font, &master).unwrap().is_none());
    let (mut font, _) = pixel_font(1024, 16);
    let master = font.default_master().id.clone();
    let contours = font
        .glyph("pixel0")
        .unwrap()
        .master_layer(&master)
        .unwrap()
        .contours
        .clone();
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].contours = contours.clone();
    }
    assert!(detect_pixel_font(&font, &master).unwrap().is_none());
}

#[test]
fn mixed_curves_diagonals_off_grid_geometry_and_metrics_do_not_detect() {
    for case in 0..5 {
        let (mut font, _) = pixel_font(1024, 16);
        let master = font.default_master().id.clone();
        let layer = &mut font.glyphs.get_mut("pixel0").unwrap().layers[0];
        match case {
            0 => layer.contours[0].points[1].kind = PointKind::OffCurve,
            1 => layer.contours[0].points[1].x += 7.0,
            2 => {
                for contour in &mut layer.contours {
                    for point in &mut contour.points {
                        point.x += 1.0;
                    }
                }
            }
            3 => layer.width += 1.0,
            _ => layer.contours[0].closed = false,
        }
        assert!(
            detect_pixel_font(&font, &master).unwrap().is_none(),
            "case {case}"
        );
    }
}

#[test]
fn malformed_inputs_and_component_cycles_fail_without_panics() {
    let (mut font, _) = pixel_font(1024, 16);
    let master = font.default_master().id.clone();
    assert!(detect_pixel_font(&font, &fc_model::MasterId("missing".into())).is_err());
    assert!(reconstruct_pixel_font(&font, &master, &["missing".into()]).is_err());
    font.glyphs.get_mut("pixel0").unwrap().layers[0]
        .components
        .push(Component {
            base: "pixel0".into(),
            ..Default::default()
        });
    assert!(detect_pixel_font(&font, &master).is_err());
    let (mut font, _) = pixel_font(1024, 16);
    let master = font.default_master().id.clone();
    font.glyphs.get_mut("pixel0").unwrap().layers[0].contours[0].points[0].x = f64::NAN;
    assert!(detect_pixel_font(&font, &master).is_err());
    font.info.units_per_em = 0;
    assert!(detect_pixel_font(&font, &master).is_err());
}

#[test]
fn expanded_component_work_is_bounded() {
    let mut font = Font::new_empty("Bounded", 1024);
    let master = font.default_master().id.clone();
    for index in 0..32 {
        let mut glyph = Glyph::new(format!("g{index}").as_str());
        let mut layer = Layer::new_master(master.clone());
        layer.width = 640.0;
        if index == 0 {
            layer.contours.push(Contour {
                closed: true,
                points: [(0., 0.), (64., 0.), (64., 64.), (0., 64.)]
                    .into_iter()
                    .map(|(x, y)| Point {
                        x,
                        y,
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            });
        } else {
            layer.components = vec![
                Component {
                    base: format!("g{}", index - 1).as_str().into(),
                    ..Default::default()
                };
                2
            ];
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    assert!(detect_pixel_font(&font, &master).is_err());
}

#[test]
fn rotated_start_points_and_reversed_winding_do_not_multiply_evidence() {
    let (mut font, _) = pixel_font(1024, 16);
    let master = font.default_master().id.clone();
    let contours = font
        .glyph("pixel0")
        .unwrap()
        .master_layer(&master)
        .unwrap()
        .contours
        .clone();
    for (index, glyph) in font.glyphs.values_mut().enumerate() {
        glyph.layers[0].contours = contours.clone();
        for contour in &mut glyph.layers[0].contours {
            let count = contour.points.len();
            contour.points.rotate_left(index % count);
            if index % 2 == 0 {
                contour.points.reverse();
            }
        }
    }
    assert!(detect_pixel_font(&font, &master).unwrap().is_none());
}

#[test]
fn tiny_detection_has_a_practical_work_budget_and_size_limits() {
    let (font, _) = pixel_font(1024, 16);
    let start = std::time::Instant::now();
    assert!(detect_pixel_font(&font, &font.default_master().id)
        .unwrap()
        .is_some());
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
    for ppem in [2, 512] {
        let (font, _) = pixel_font(1024, ppem);
        assert!(detect_pixel_font(&font, &font.default_master().id)
            .unwrap()
            .is_none());
    }
}
