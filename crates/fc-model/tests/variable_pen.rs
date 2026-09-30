use fc_model::{EffectProgression, Layer, LayerEffect, LayerEffectKind, PenPoint};

fn effect() -> LayerEffect {
    serde_json::from_value(serde_json::json!({"effect": {
        "kind":"variable_stroke", "nib":"ellipse", "cap":"butt", "join":"miter",
        "miter_limit":4, "progression":"linear", "points":[
            {"position":0,"width":20,"height":30,"angle":350},
            {"position":1,"width":60,"height":80,"angle":10}
        ]
    }}))
    .unwrap()
}

#[test]
fn variable_points_round_trip_and_interpolate_shortest_angle() {
    let entry = effect();
    let layer = Layer {
        effects: vec![entry.clone()],
        ..Default::default()
    };
    layer.validate_effects().unwrap();
    assert_eq!(
        entry,
        serde_json::from_value(serde_json::to_value(&entry).unwrap()).unwrap()
    );
    let LayerEffectKind::VariableStroke { points, .. } = entry.effect else {
        unreachable!()
    };
    let middle = PenPoint::interpolate(&points[0], &points[1], 0.25, EffectProgression::Linear);
    assert_eq!(
        (middle.width, middle.height, middle.angle),
        (30., 42.5, 355.)
    );
    let smooth = PenPoint::interpolate(&points[0], &points[1], 0.25, EffectProgression::Smooth);
    assert_eq!(smooth.width, 26.25);
}

#[test]
fn invalid_disabled_pen_points_are_rejected() {
    for case in 0..10 {
        let mut entry = effect();
        entry.enabled = false;
        let LayerEffectKind::VariableStroke {
            points,
            miter_limit,
            ..
        } = &mut entry.effect
        else {
            unreachable!()
        };
        match case {
            0 => points.clear(),
            1 => points[0].position = 0.1,
            2 => points[1].position = 0.9,
            3 => points.insert(1, points[0]),
            4 => points[0].width = f64::NAN,
            5 => points[0].height = 0.,
            6 => points[0].angle = 361.,
            7 => points[0].height = 8193.,
            8 => *miter_limit = 0.,
            _ => *points = vec![points[0]; 65],
        }
        assert!(
            Layer {
                effects: vec![entry],
                ..Default::default()
            }
            .validate_effects()
            .is_err(),
            "case {case}"
        );
    }
}
