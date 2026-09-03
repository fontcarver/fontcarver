mod fixtures;

use fw_geom::{compatible, interpolate};
use fw_model::{Anchor, Component, GlyphName, Layer};
use kurbo::Affine;

fn layers() -> (Layer, Layer) {
    let a = Layer {
        width: 500.0,
        contours: vec![fixtures::rect(0.0, 0.0, 100.0, 100.0)],
        components: vec![Component {
            base: GlyphName::from("acute"),
            transform: Affine::translate((10.0, 20.0)),
            ..Component::default()
        }],
        anchors: vec![Anchor {
            name: "top".into(),
            x: 50.0,
            y: 100.0,
        }],
        ..Layer::default()
    };

    let mut b = a.clone();
    b.width = 600.0;
    b.contours[0].points[1].x = 200.0;
    b.components[0].transform = Affine::translate((30.0, 40.0));
    b.anchors[0].x = 70.0;
    b.anchors[0].y = 120.0;
    (a, b)
}

#[test]
fn compatible_layers_interpolate_at_half() {
    let (a, b) = layers();
    compatible(&a, &b).expect("compatible");
    let middle = interpolate(&a, &b, 0.5).expect("interpolation");
    assert_eq!(middle.width, 550.0);
    assert_eq!(middle.contours[0].points[1].x, 150.0);
    assert_eq!(middle.components[0].transform.as_coeffs()[4], 20.0);
    assert_eq!(middle.anchors[0].x, 60.0);
}

#[test]
fn point_count_mismatch_names_the_contour() {
    let (a, mut b) = layers();
    b.contours[0].points.pop();
    let errors = compatible(&a, &b).expect_err("must be incompatible");
    assert!(errors.iter().any(|error| error.contour == Some(0)));
}
