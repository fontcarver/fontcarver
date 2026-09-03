mod common;

use fw_doc::{metrics, Command, Error, MetricsSource, Session};
use fw_model::{Anchor, Component, Glyph, GlyphName, MetricsKeys};
use kurbo::Affine;

use common::test_font;

#[test]
fn set_metrics_moves_all_positioned_content_and_applies_width_lsb_rsb_order() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = GlyphName::from("base");
            let layer = doc.font.glyph(&glyph).unwrap().layers[0].id.clone();
            let target = &mut doc.font.glyph_mut(&glyph).unwrap().layers[0];
            target.components.push(Component {
                base: "missing-until-query".into(),
                transform: Affine::translate((5.0, 7.0)),
                ..Component::default()
            });
            target.anchors.push(Anchor {
                name: "top".to_owned(),
                x: 50.0,
                y: 100.0,
            });
            // Keep bounds flattenable while still observing component translation.
            doc.font
                .insert_glyph(Glyph::new("missing-until-query"))
                .unwrap();
            let before = doc.font.glyph(&glyph).unwrap().clone();
            let change = doc
                .apply(
                    Command::SetMetrics {
                        glyph: glyph.clone(),
                        layer,
                        width: Some(600.0),
                        lsb: Some(20.0),
                        rsb: Some(30.0),
                    },
                    "Metrics",
                )
                .unwrap();
            assert!(change.metrics);
            let target = &doc.font.glyph(&glyph).unwrap().layers[0];
            assert_eq!(target.contours[0].points[0].x, 20.0);
            assert_eq!(target.components[0].transform.as_coeffs()[4], 25.0);
            assert_eq!(target.anchors[0].x, 70.0);
            assert_eq!(target.width, 150.0);
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph(&glyph).unwrap(), &before);
        })
        .unwrap();
}

#[test]
fn metrics_keys_resolve_mirroring_offsets_literals_and_cycles() {
    let mut font = test_font();
    let master = font.default_master().id.clone();
    font.insert_glyph(Glyph::new("keyed")).unwrap();
    font.glyph_mut("keyed").unwrap().metrics_keys = MetricsKeys {
        left: Some("=|base+10".to_owned()),
        right: Some("=base-5".to_owned()),
        width: Some("=700".to_owned()),
        ..MetricsKeys::default()
    };
    let resolved = metrics::resolve(&font, &GlyphName::from("keyed"), &master).unwrap();
    assert_eq!(resolved.lsb, 410.0);
    assert_eq!(resolved.rsb, 395.0);
    assert_eq!(resolved.width, 700.0);
    assert_eq!(resolved.source, MetricsSource::Key);

    font.glyph_mut("base").unwrap().metrics_keys.left = Some("=keyed".to_owned());
    font.glyph_mut("keyed").unwrap().metrics_keys.left = Some("=base".to_owned());
    assert!(matches!(
        metrics::resolve(&font, &GlyphName::from("keyed"), &master),
        Err(Error::MetricsKeyCycle)
    ));
}

#[test]
fn set_metrics_keys_is_undoable() {
    let session = Session::new();
    let id = session.open_font(test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let before = doc.font.glyph("base").unwrap().clone();
            let change = doc
                .apply(
                    Command::SetMetricsKeys {
                        glyph: "base".into(),
                        keys: MetricsKeys {
                            width: Some("=600".to_owned()),
                            ..MetricsKeys::default()
                        },
                    },
                    "Keys",
                )
                .unwrap();
            assert!(change.metrics);
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap(), &before);
        })
        .unwrap();
}
