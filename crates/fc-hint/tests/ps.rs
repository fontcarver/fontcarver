use fc_hint::ps;
use fc_model::{Contour, Font, Glyph};
use kurbo::{Rect, Shape};

#[test]
fn detects_ink_stems_without_hinting_counter_space() {
    let mut font = Font::new_empty("Hints", 1000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("H");
    font.insert_glyph(glyph.clone()).unwrap();
    glyph = font.glyph("H").unwrap().clone();
    let layer = glyph.master_layer_mut(&master).unwrap();
    for rectangle in [
        Rect::new(50.0, 0.0, 130.0, 700.0),
        Rect::new(430.0, 0.0, 510.0, 700.0),
        Rect::new(130.0, 300.0, 430.0, 380.0),
    ] {
        layer
            .contours
            .extend(Contour::from_bezpath(&rectangle.to_path(0.1)));
    }
    font.glyphs.insert(glyph.name.clone(), glyph);
    let hints = ps::detect(&font, &"H".into(), &master).unwrap();
    assert_eq!(
        hints
            .v_stems
            .iter()
            .map(|s| (s.start, s.width))
            .collect::<Vec<_>>(),
        [(50.0, 80.0), (430.0, 80.0)]
    );
    assert!(hints
        .h_stems
        .iter()
        .any(|s| s.start == 300.0 && s.width == 80.0));
    let private = ps::infer_private(&font, &master, &[hints]).unwrap();
    assert_eq!(private.std_vw, Some(80.0));
    assert!(!private.blue_values.is_empty());
    private.validate().unwrap();
}

#[test]
fn rejects_invalid_and_extreme_curve_coordinates_before_subdivision() {
    let mut font = Font::new_empty("Invalid hints", 1000);
    let master = font.default_master().id.clone();
    font.insert_glyph(Glyph::new("A")).unwrap();
    for value in [f64::NAN, f64::INFINITY, 1e100] {
        let mut path = kurbo::BezPath::new();
        path.move_to((0.0, 0.0));
        path.curve_to((value, 10.0), (50.0, value), (100.0, 0.0));
        path.close_path();
        font.glyph_mut("A").unwrap().layers[0].contours = Contour::from_bezpath(&path);
        assert!(ps::detect(&font, &"A".into(), &master).is_err());
    }
}

#[test]
fn detects_round_bowl_stems_at_exact_curve_extrema() {
    let mut font = Font::new_empty("Round hints", 1000);
    let master = font.default_master().id.clone();
    font.insert_glyph(Glyph::new("O")).unwrap();
    let mut path = kurbo::Circle::new((300.0, 300.0), 250.0).to_path(0.1);
    path.extend(
        kurbo::Circle::new((300.0, 300.0), 170.0)
            .to_path(0.1)
            .reverse_subpaths(),
    );
    font.glyph_mut("O").unwrap().layers[0].contours = Contour::from_bezpath(&path);
    let hints = ps::detect(&font, &"O".into(), &master).unwrap();
    for stems in [&hints.h_stems, &hints.v_stems] {
        assert_eq!(stems.len(), 2);
        for (stem, start) in stems.iter().zip([50.0, 470.0]) {
            assert!((stem.start - start).abs() < 0.01);
            assert!((stem.width - 80.0).abs() < 0.01);
        }
    }
}
