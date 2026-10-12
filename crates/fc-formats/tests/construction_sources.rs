use fc_model::{Contour, Font, Glyph, Layer, Point, PointKind};

fn fixture() -> Font {
    let mut font = Font::new_empty("Construction sources", 1000);
    font.insert_glyph(Glyph::new("_cap")).unwrap();
    let mut glyph = Glyph::new("A");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.contours.push(Contour {
        closed: false,
        points: [(0., 0., "start"), (100., 0., "end")]
            .into_iter()
            .enumerate()
            .map(|(index, (x, y, id))| Point {
                id: Some(id.into()),
                x,
                y,
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
        "id":"cap", "source":"_cap", "keep_carrier":true,
        "kind":{"Cap":{"at":{"id":"start"},"scale":1.0}}
    }]))
    .unwrap();
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

#[test]
fn ufo_retains_construction_sources_and_stable_node_identifiers() {
    let font = fixture();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("construction.ufo");
    fc_formats::ufo::write_ufo(&font, &font.default_master().id, &path, &Default::default())
        .unwrap();
    let reopened = fc_formats::ufo::read_ufo(&path).unwrap();
    let before = &font.glyph("A").unwrap().layers[0];
    let after = &reopened.glyph("A").unwrap().layers[0];
    assert_eq!(before.constructions, after.constructions);
    assert_eq!(before.contours, after.contours);
}

#[test]
fn unsupported_sources_fail_before_touching_destination() {
    let font = fixture();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source");
    std::fs::write(&path, b"existing source").unwrap();
    let error = fc_formats::glyphs::write_glyphs(&font, &path).unwrap_err();
    assert!(error.to_string().contains("construction"));
    assert_eq!(std::fs::read(&path).unwrap(), b"existing source");
    let error = fc_formats::sfd::write_sfd(&font, &font.default_master().id, &path).unwrap_err();
    assert!(error.to_string().contains("construction"));
    assert_eq!(std::fs::read(&path).unwrap(), b"existing source");
}

#[test]
fn ufo_retains_servants_linked_guides_and_rejects_dangling_metadata() {
    use fc_model::{Guide, GuidedNode, NodeAxis, NodeLink, NodeLinkedGuide, NodeRef};
    let mut font = fixture();
    let layer = &mut font.glyph_mut("A").unwrap().layers[0];
    layer.contours[0].points.insert(
        1,
        Point {
            id: Some("middle".into()),
            x: 50.,
            kind: PointKind::Line,
            ..Default::default()
        },
    );
    let node = |id: &str| NodeRef { id: id.into() };
    layer.node_links.push(NodeLink {
        node: node("middle"),
        first: node("start"),
        second: node("end"),
        axis: NodeAxis::X,
        ratio: 0.5,
        handles: vec![],
    });
    layer.guides.push(Guide {
        id: Some("baseline".into()),
        ..Default::default()
    });
    layer.linked_guides.push(NodeLinkedGuide {
        guide_id: "baseline".into(),
        nodes: vec![GuidedNode {
            node: node("start"),
            offset: 0.,
        }],
    });
    let before = layer.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("links.ufo");
    fc_formats::ufo::write_ufo(&font, &font.default_master().id, &path, &Default::default())
        .unwrap();
    let reopened = fc_formats::ufo::read_ufo(&path).unwrap();
    let after = &reopened.glyph("A").unwrap().layers[0];
    assert_eq!(before.node_links, after.node_links);
    assert_eq!(before.linked_guides, after.linked_guides);
    assert_eq!(before.guides, after.guides);

    let mut raw = norad::Font::load(&path).unwrap();
    let glyph = raw.default_layer_mut().get_glyph_mut("A").unwrap();
    let text = glyph.lib["com.fontcarver.constructionData"]
        .as_string()
        .unwrap();
    let invalid = text.replace("\"middle\"", "\"missing\"");
    glyph.lib.insert(
        "com.fontcarver.constructionData".into(),
        plist::Value::String(invalid),
    );
    raw.save(&path).unwrap();
    assert!(fc_formats::ufo::read_ufo(&path).is_err());
}
