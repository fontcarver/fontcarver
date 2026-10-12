use fc_model::{ConstructionKind, Contour, Font, Glyph, GlyphConstruction, Layer, NodeRef};
use kurbo::{BezPath, Rect, Shape};
fn reference(id: &str) -> NodeRef {
    NodeRef { id: id.into() }
}
fn fixture(kind: ConstructionKind, keep: bool) -> Font {
    let mut font = Font::new_empty("Constructions", 1000);
    let master = font.default_master().id.clone();
    let mut source = Glyph::new("stamp");
    let mut source_layer = Layer::new_master(master.clone());
    source_layer.contours = Contour::from_bezpath(&Rect::new(0., 0., 10., 5.).to_path(0.1));
    source.layers.push(source_layer);
    font.insert_glyph(source).unwrap();
    let mut target = Glyph::new("target");
    let mut layer = Layer::new_master(master);
    layer.contours = Contour::from_bezpath(&BezPath::from_svg("M100 100L100 200L200 200").unwrap());
    for (i, p) in layer.contours[0].points.iter_mut().enumerate() {
        p.id = Some(format!("n{i}"));
    }
    layer.constructions.push(GlyphConstruction {
        id: "test".into(),
        source: "stamp".into(),
        kind,
        keep_carrier: keep,
    });
    target.layers.push(layer);
    font.insert_glyph(target).unwrap();
    font
}
#[test]
fn segment_maps_baseline_and_bakes_without_mutating_carrier() {
    let font = fixture(
        ConstructionKind::Segment {
            start: reference("n0"),
            end: reference("n1"),
            source_length: 10.,
        },
        false,
    );
    let before = font.clone();
    let path = fc_geom::flatten_layer(&font, "target", &font.default_master().id).unwrap();
    assert_eq!(path.bounding_box(), Rect::new(50., 100., 100., 200.));
    let (baked, _) = fc_geom::bake_effects_font(&font).unwrap();
    assert!(baked.glyph("target").unwrap().layers[0]
        .constructions
        .is_empty());
    assert_eq!(
        fc_model::flatten_layer(&baked, "target", &font.default_master().id).unwrap(),
        path
    );
    assert_eq!(font, before);
}
#[test]
fn brush_count_is_bounded_and_missing_nodes_fail_explicitly() {
    let mut font = fixture(
        ConstructionKind::Brush {
            at: reference("n0"),
            spacing: 50.,
            scale: 1.,
        },
        false,
    );
    let path = fc_geom::flatten_layer(&font, "target", &font.default_master().id).unwrap();
    assert_eq!(Contour::from_bezpath(&path).len(), 5);
    font.glyph_mut("target").unwrap().layers[0].contours[0].points[0].id = None;
    assert!(fc_geom::flatten_layer(&font, "target", &font.default_master().id).is_err());
}
#[test]
fn attachments_follow_node_tangents_and_source_edits() {
    let mut font = fixture(
        ConstructionKind::Cap {
            at: reference("n0"),
            scale: 1.,
        },
        false,
    );
    let master = font.default_master().id.clone();
    let bounds = fc_geom::flatten_layer(&font, "target", &master)
        .unwrap()
        .bounding_box();
    assert!((bounds.x0 - 100.).abs() < 1e-9 && (bounds.y0 - 90.).abs() < 1e-9);
    font.glyph_mut("stamp").unwrap().layers[0].contours[0].points[0].x = -10.;
    assert_ne!(
        fc_geom::flatten_layer(&font, "target", &master)
            .unwrap()
            .bounding_box(),
        bounds
    );
}
