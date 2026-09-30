use fc_model::{Component, Contour, Error, Font, Glyph, Layer, Point, PointKind};

fn carrier(font: &Font, name: &str, source: &str) -> Glyph {
    let mut glyph = Glyph::new(name);
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.contours.push(Contour {
        closed: false,
        points: [(0., "start"), (100., "end")]
            .into_iter()
            .enumerate()
            .map(|(index, (x, id))| Point {
                id: Some(id.into()),
                x,
                kind: if index == 0 {
                    PointKind::Move
                } else {
                    PointKind::Line
                },
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    });
    layer.constructions = serde_json::from_value(serde_json::json!([{
        "id":"cap", "source":source, "keep_carrier":true,
        "kind":{"Cap":{"at":{"id":"start"},"scale":1.0}}
    }]))
    .unwrap();
    glyph.layers.push(layer);
    glyph
}

#[test]
fn construction_sources_participate_in_rename_dependents_and_missing_reference_checks() {
    let mut font = Font::default();
    font.insert_glyph(Glyph::new("_cap")).unwrap();
    font.insert_glyph(carrier(&font, "A", "_cap")).unwrap();
    assert_eq!(
        fc_model::dependents(&font)["_cap"],
        vec![fc_model::GlyphName::from("A")]
    );
    font.rename_glyph("_cap", "_cap.alt").unwrap();
    assert_eq!(
        font.glyph("A").unwrap().layers[0].constructions[0].source,
        "_cap.alt".into()
    );
    assert!(!fc_model::dependents(&font).contains_key("_cap"));
    font.remove_glyph("_cap.alt");
    assert!(fc_model::validate(&font).iter().any(|issue| matches!(issue,
        fc_model::Issue::DanglingComponent {glyph,base} if glyph.as_ref()=="A" && base.as_ref()=="_cap.alt"
    )));
}

#[test]
fn insertion_rejects_mixed_construction_component_cycles_atomically() {
    let mut font = Font::default();
    font.insert_glyph(carrier(&font, "A", "_cap")).unwrap();
    let mut source = Glyph::new("_cap");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.components.push(Component {
        base: "A".into(),
        ..Default::default()
    });
    source.layers.push(layer);
    assert!(matches!(
        font.insert_glyph(source),
        Err(Error::ComponentCycle { .. })
    ));
    assert!(font.glyph("_cap").is_none());
}
