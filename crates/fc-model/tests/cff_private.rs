use fc_model::{Layer, Master, PsPrivate};

#[test]
fn cff_private_data_round_trips_and_legacy_data_default_to_fd_zero() {
    let mut master = Master::default();
    master.cff_private_dicts.push(PsPrivate {
        blue_values: vec![-20.0, 0.0],
        std_vw: Some(90.0),
        ..PsPrivate::default()
    });
    let mut layer = Layer::new_master(master.id.clone());
    layer.cff_private_dict_index = Some(1);
    let json = serde_json::to_string(&(master.clone(), layer.clone())).expect("serialize");
    let decoded: (Master, Layer) = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, (master, layer));
    let legacy_master: Master = serde_json::from_str("{}").expect("legacy master");
    let legacy_layer: Layer =
        serde_json::from_str(r#"{"kind":{"Custom":"legacy"}}"#).expect("legacy layer");
    assert!(legacy_master.cff_private_dicts.is_empty());
    assert_eq!(legacy_layer.cff_private_dict_index, None);
}

#[test]
fn nondefault_cff_private_assignment_is_meaningful_on_an_empty_outline() {
    let mut layer = Layer::default();
    assert!(layer.is_empty());
    layer.cff_private_dict_index = Some(0);
    assert!(layer.is_empty());
    layer.cff_private_dict_index = Some(1);
    assert!(!layer.is_empty());
}
