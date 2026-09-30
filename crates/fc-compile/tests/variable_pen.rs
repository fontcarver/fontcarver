use fc_compile::{export, shaping_fingerprint, ExportFormat, ExportOptions};
use fc_model::{
    Component, Contour, EffectCap, EffectJoin, EffectNib, EffectProgression, Font, Glyph, Layer,
    LayerEffect, LayerEffectKind, PenPoint,
};
use kurbo::{Affine, BezPath, Point, Rect, Shape};

fn fixture(nib: EffectNib, progression: EffectProgression) -> Font {
    let mut font = Font::new_empty("Variable Pen Export", 1000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("A");
    glyph.unicodes.push(0x41);
    let mut layer = Layer::new_master(master.clone());
    layer.width = 400.;
    let mut path = BezPath::new();
    path.move_to((0., 0.));
    path.line_to((200., 0.));
    layer.contours = Contour::from_bezpath(&path);
    layer.effects = vec![LayerEffect {
        enabled: true,
        effect: LayerEffectKind::VariableStroke {
            nib,
            cap: EffectCap::Butt,
            join: EffectJoin::Round,
            miter_limit: 4.,
            progression,
            points: vec![
                PenPoint {
                    position: 0.,
                    width: 20.,
                    height: 20.,
                    angle: 0.,
                },
                PenPoint {
                    position: 1.,
                    width: 60.,
                    height: 60.,
                    angle: 0.,
                },
            ],
        },
    }];
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let mut composite = Glyph::new("B");
    composite.unicodes.push(0x42);
    let mut layer = Layer::new_master(master);
    layer.width = 400.;
    layer.components.push(Component {
        base: "A".into(),
        transform: Affine::translate((0., 150.)),
        ..Default::default()
    });
    composite.layers.push(layer);
    font.insert_glyph(composite).unwrap();
    font
}

fn assert_bounds(actual: Rect, expected: Rect) {
    for (actual, expected) in [
        (actual.x0, expected.x0),
        (actual.y0, expected.y0),
        (actual.x1, expected.x1),
        (actual.y1, expected.y1),
    ] {
        assert!((actual - expected).abs() <= 1., "{actual} != {expected}");
    }
}

#[test]
fn ttf_and_otf_bake_variable_pen_and_dependents_without_changing_source() {
    for nib in [EffectNib::Ellipse, EffectNib::Rectangle] {
        for progression in [EffectProgression::Linear, EffectProgression::Smooth] {
            let font = fixture(nib, progression);
            let source = font.clone();
            for format in [ExportFormat::Ttf, ExportFormat::Otf] {
                let report = export(
                    &font,
                    &|_| 0,
                    &ExportOptions {
                        format,
                        ..ExportOptions::default()
                    },
                )
                .unwrap();
                assert_eq!(font, source);
                assert!(report
                    .warnings
                    .iter()
                    .any(|warning| warning.code == "layer-effects-baked"));
                let reopened = fc_sfnt::read_bytes(&report.bytes, &Default::default())
                    .unwrap()
                    .fonts
                    .remove(0);
                let master = &reopened.default_master().id;
                let outline = fc_model::flatten_layer(&reopened, "A", master).unwrap();
                assert_bounds(outline.bounding_box(), Rect::new(0., -30., 200., 30.));
                assert_eq!(outline.winding(Point::new(25., 20.)), 0);
                assert_ne!(outline.winding(Point::new(175., 20.)), 0);
                assert_eq!(
                    outline.winding(Point::new(50., 14.25)) != 0,
                    progression == EffectProgression::Linear,
                    "export must preserve the chosen interpolation progression"
                );
                assert_bounds(
                    fc_model::flatten_layer(&reopened, "B", master)
                        .unwrap()
                        .bounding_box(),
                    Rect::new(0., 120., 200., 180.),
                );
                for name in ["A", "B"] {
                    let layer = reopened.glyph(name).unwrap().master_layer(master).unwrap();
                    assert!(layer.effects.is_empty());
                    assert_eq!(layer.width, 400.);
                    assert!(layer.contours.iter().all(|contour| contour.closed));
                }
                assert_eq!(
                    shaping_fingerprint(&report.bytes, "AB").unwrap(),
                    vec![(1, 400, 0), (2, 400, 0)]
                );
            }
        }
    }
}

#[test]
fn export_rejects_invalid_variable_pen_without_changing_its_source() {
    let mut font = fixture(EffectNib::Ellipse, EffectProgression::Linear);
    if let LayerEffectKind::VariableStroke { points, .. } =
        &mut font.glyph_mut("A").unwrap().layers[0].effects[0].effect
    {
        points[1].position = 0.;
    }
    let source = font.clone();
    for format in [ExportFormat::Ttf, ExportFormat::Otf] {
        assert!(export(
            &font,
            &|_| 0,
            &ExportOptions {
                format,
                ..ExportOptions::default()
            }
        )
        .is_err());
        assert_eq!(font, source);
    }
}
