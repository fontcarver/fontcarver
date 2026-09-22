use fc_model::{Component, Contour, Font, Glyph, Layer, LayerKind, Point, PointKind, SmartAxis};
use kurbo::Shape;

fn fixture() -> Font {
    let mut font = Font::new_empty("Smart Sources", 1000);
    let master = font.masters[0].id.clone();
    let mut part = Glyph::new("_part.stem");
    part.smart_axes = vec![SmartAxis {
        name: "Width".into(),
        min: 0.,
        default: 0.,
        max: 100.,
    }];
    for (x, name) in [(0., ""), (100., "Wide")] {
        let mut layer = Layer::new_master(master.clone());
        layer.name = name.into();
        layer.width = 300. + x;
        if x != 0. {
            layer.kind = LayerKind::Smart {
                master: master.clone(),
                location: [("Width".into(), x)].into(),
            };
        }
        layer.contours = vec![Contour {
            closed: true,
            points: [(0., 0.), (100. + x, 0.), (100. + x, 500.), (0., 500.)]
                .into_iter()
                .map(|(x, y)| Point {
                    x,
                    y,
                    kind: PointKind::Line,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }];
        part.layers.push(layer);
    }
    font.insert_glyph(part).unwrap();
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![65];
    let mut layer = Layer::new_master(master);
    layer.width = 600.;
    layer.components.push(Component {
        base: "_part.stem".into(),
        smart: Some([("Width".into(), 50.)].into()),
        ..Default::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}
#[test]
fn glyphs_and_ufo_retain_local_axes_sources_and_component_values() {
    let font = fixture();
    let dir = tempfile::tempdir().unwrap();
    let glyphs = dir.path().join("smart.glyphs");
    fc_formats::glyphs::write_glyphs(&font, &glyphs).unwrap();
    let text = std::fs::read_to_string(&glyphs).unwrap();
    assert!(text.contains("partSelection"));
    assert!(text.contains("partsSettings"));
    assert!(text.contains("piece"));
    let ufo = dir.path().join("smart.ufo");
    fc_formats::ufo::write_ufo(&font, &font.default_master().id, &ufo, &Default::default())
        .unwrap();
    for reopened in [
        fc_formats::glyphs::read_glyphs(&glyphs).unwrap(),
        fc_formats::ufo::read_ufo(&ufo).unwrap(),
    ] {
        assert_eq!(
            reopened.glyph("_part.stem").unwrap().smart_axes,
            font.glyph("_part.stem").unwrap().smart_axes
        );
        assert!(reopened.glyph("_part.stem").unwrap().layers.iter().any(
            |layer| matches!(&layer.kind,LayerKind::Smart {location,..} if location["Width"]==100.)
        ));
        let bounds = fc_model::flatten_layer(&reopened, "A", &reopened.default_master().id)
            .unwrap()
            .bounding_box();
        assert_eq!(bounds.x1, 150.);
    }
    std::fs::write(&glyphs, text.replace("Width = 50;", "Width = 75;")).unwrap();
    let edited = fc_formats::glyphs::read_glyphs(&glyphs).unwrap();
    assert_eq!(
        fc_model::flatten_layer(&edited, "A", &edited.default_master().id)
            .unwrap()
            .bounding_box()
            .x1,
        175.
    );
}
