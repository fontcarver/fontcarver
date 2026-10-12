use fc_model::{Layer, LayerEffect, LayerEffectKind};

#[test]
fn old_layers_default_to_no_effects_and_effects_round_trip() {
    let old: Layer = serde_json::from_value(serde_json::json!({"kind":{"Custom":"old"}})).unwrap();
    assert!(old.effects.is_empty());
    assert!(serde_json::to_value(&old).unwrap().get("effects").is_none());
    let effect: LayerEffect = serde_json::from_value(serde_json::json!({
        "effect":{"kind":"boolean","operation":"union","path":"M0 0L100 0L100 100Z"}
    }))
    .unwrap();
    assert!(effect.enabled);
    let layer = Layer {
        effects: vec![effect],
        ..old
    };
    layer.validate_effects().unwrap();
    assert!(!layer.is_empty());
    assert_eq!(
        layer,
        serde_json::from_value(serde_json::to_value(&layer).unwrap()).unwrap()
    );
}

#[test]
fn validates_disabled_effects_and_bounds_work() {
    let mut layer = Layer::default();
    let parse = |value| serde_json::from_value::<LayerEffect>(value).unwrap();
    for effect in [
        parse(
            serde_json::json!({"enabled":false,"effect":{"kind":"stroke","width":0,"height":20,"angle":0,"nib":"ellipse","cap":"round","join":"round","miter_limit":4}}),
        ),
        parse(
            serde_json::json!({"effect":{"kind":"boolean","operation":"subtract","path":"M0 0L20 20"}}),
        ),
        parse(
            serde_json::json!({"effect":{"kind":"boolean","operation":"union","path":"M0 0L1e100 0Z"}}),
        ),
        parse(serde_json::json!({"effect":{"kind":"boolean","operation":"xor","path":"invalid"}})),
    ] {
        assert!(fc_model::validate_effect_mask("M0 0L20 20ZL40 40").is_err());
        layer.effects = vec![effect];
        assert!(layer.validate_effects().is_err());
    }
    layer.effects = vec![
        LayerEffect {
            enabled: true,
            effect: LayerEffectKind::RemoveOverlap
        };
        17
    ];
    assert!(layer.validate_effects().is_err());
}
