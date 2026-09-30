//! Derived glyph-grid status without additional frontend requests.

use fc_model::{Font, Glyph, MasterId};

/// Whether a note or a canonical editor annotation is stored on the glyph.
pub(crate) fn has_annotations(glyph: &Glyph) -> bool {
    !glyph.note.trim().is_empty()
        || glyph.layers.iter().any(|layer| {
            layer.user_data[fc_doc::command::EDITOR_METADATA_KEY]["annotations"]
                .as_array()
                .is_some_and(|annotations| !annotations.is_empty())
        })
}

/// Whether explicit glyph-info overrides or format-specific metadata are stored.
pub(crate) fn has_custom_glyph_info(glyph: &Glyph) -> bool {
    glyph.production_name.is_some()
        || glyph.category.is_some()
        || glyph.sub_category.is_some()
        || glyph.script.is_some()
        || match &glyph.user_data {
            serde_json::Value::Null => false,
            serde_json::Value::Object(value) => !value.is_empty(),
            serde_json::Value::Array(value) => !value.is_empty(),
            serde_json::Value::String(value) => !value.is_empty(),
            _ => true,
        }
}

/// Whether a required master layer is missing or cannot interpolate.
pub(crate) fn incompatible_masters(font: &Font, glyph: &Glyph) -> bool {
    let mut reference = None;
    for master in &font.masters {
        let Some(layer) = glyph.master_layer(&master.id) else {
            return true;
        };
        if let Some(reference) = reference {
            if fc_geom::compatible(reference, layer).is_err() {
                return true;
            }
        } else {
            reference = Some(layer);
        }
    }
    false
}

/// Compares horizontal formulas with stored metrics, reusing the outline bounds.
///
/// Unresolvable formulas are stale. Formula-free cells avoid geometry resolution;
/// the resolver bounds reference chains and uses per-layer overrides.
pub(crate) fn metrics_out_of_sync(
    font: &Font,
    glyph: &Glyph,
    master: &MasterId,
    outline_bbox: Option<[f64; 4]>,
) -> bool {
    let Ok(keys) = fc_doc::metrics::effective_keys(font, &glyph.name, master, None) else {
        return true;
    };
    if keys.left.is_none() && keys.right.is_none() && keys.width.is_none() {
        return false;
    }
    let Some(layer) = glyph.master_layer(master) else {
        return true;
    };
    let Ok(resolved) = fc_doc::metrics::resolve(font, &glyph.name, master) else {
        return true;
    };
    let explicit = [
        outline_bbox.map_or(0.0, |bounds| bounds[0]),
        outline_bbox.map_or(layer.width, |bounds| layer.width - bounds[2]),
        layer.width,
    ];
    [resolved.lsb, resolved.rsb, resolved.width]
        .into_iter()
        .zip(explicit)
        .any(|(resolved, stored)| {
            !resolved.is_finite() || !stored.is_finite() || (resolved - stored).abs() > 1e-6
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fc_model::{Anchor, Contour, Font, Glyph, Layer, LayerKind, Master, MetricsKeys};
    use kurbo::Shape;
    use serde_json::json;

    fn font_with_glyph() -> (Font, fc_model::MasterId) {
        let mut font = Font::new_empty("Status", 1000);
        let master = font.default_master().id.clone();
        font.insert_glyph(Glyph::new("A")).unwrap();
        (font, master)
    }

    #[test]
    fn glyph_status_defaults_are_clear() {
        let (font, master) = font_with_glyph();
        let glyph = font.glyph("A").unwrap();
        assert!(!has_annotations(glyph));
        assert!(!has_custom_glyph_info(glyph));
        assert!(!incompatible_masters(&font, glyph));
        assert!(!metrics_out_of_sync(&font, glyph, &master, None));
    }

    #[test]
    fn glyph_status_annotation_sources_are_specific() {
        let mut glyph = Glyph::new("A");
        glyph.note = "  \n".into();
        assert!(!has_annotations(&glyph));
        glyph.note = "Review this curve".into();
        assert!(has_annotations(&glyph));
        glyph.note.clear();
        glyph.layers.push(Layer {
            user_data: json!({"unrelated": {"annotations": [1]}}),
            ..Layer::default()
        });
        assert!(!has_annotations(&glyph));
        glyph.layers[0].user_data = json!({"com.fontcarver.editor": {"annotations": []}});
        assert!(!has_annotations(&glyph));
        glyph.layers[0].user_data =
            json!({"com.fontcarver.editor": {"annotations": [{"id": "note"}]}});
        assert!(has_annotations(&glyph));
    }

    #[test]
    fn glyph_status_custom_info_excludes_ordinary_organization() {
        let mut glyph = Glyph::new("A");
        glyph.tags.insert("review".into());
        glyph.kern_group_side1 = Some("A".into());
        glyph.user_data = json!({});
        assert!(!has_custom_glyph_info(&glyph));
        glyph.production_name = Some("A.custom".into());
        assert!(has_custom_glyph_info(&glyph));
        glyph.production_name = None;
        glyph.category = Some("Letter".into());
        assert!(has_custom_glyph_info(&glyph));
        glyph.category = None;
        glyph.user_data = json!({"vendor.flag": true});
        assert!(has_custom_glyph_info(&glyph));
    }

    #[test]
    fn glyph_status_metrics_detect_mismatch_and_invalid_references() {
        let (mut font, master) = font_with_glyph();
        font.glyph_mut("A")
            .unwrap()
            .master_layer_mut(&master)
            .unwrap()
            .width = 500.0;
        font.glyph_mut("A").unwrap().metrics_keys.width = Some("=500".into());
        assert!(!metrics_out_of_sync(
            &font,
            font.glyph("A").unwrap(),
            &master,
            None
        ));
        font.glyph_mut("A").unwrap().metrics_keys.width = Some("=501".into());
        assert!(metrics_out_of_sync(
            &font,
            font.glyph("A").unwrap(),
            &master,
            None
        ));
        font.glyph_mut("A").unwrap().metrics_keys.width = Some("=missing".into());
        assert!(metrics_out_of_sync(
            &font,
            font.glyph("A").unwrap(),
            &master,
            None
        ));
        font.glyph_mut("A").unwrap().metrics_keys.width = Some("=A".into());
        assert!(metrics_out_of_sync(
            &font,
            font.glyph("A").unwrap(),
            &master,
            None
        ));
    }

    #[test]
    fn glyph_status_metrics_compare_sidebearings_and_layer_overrides() {
        let (mut font, master) = font_with_glyph();
        let glyph = font.glyph_mut("A").unwrap();
        glyph.metrics_keys.width = Some("=missing".into());
        let layer = glyph.master_layer_mut(&master).unwrap();
        layer.width = 500.0;
        layer.contours =
            Contour::from_bezpath(&kurbo::Rect::new(50.0, 0.0, 450.0, 700.0).to_path(0.1));
        layer.metrics_keys = Some(MetricsKeys {
            left: Some("=50".into()),
            width: Some("=500".into()),
            ..MetricsKeys::default()
        });
        let bbox = Some([50.0, 0.0, 450.0, 700.0]);
        assert!(!metrics_out_of_sync(
            &font,
            font.glyph("A").unwrap(),
            &master,
            bbox
        ));
        font.glyph_mut("A")
            .unwrap()
            .master_layer_mut(&master)
            .unwrap()
            .metrics_keys
            .as_mut()
            .unwrap()
            .left = Some("=60".into());
        assert!(metrics_out_of_sync(
            &font,
            font.glyph("A").unwrap(),
            &master,
            bbox
        ));
    }

    #[test]
    fn glyph_status_compatibility_includes_missing_masters_and_anchors() {
        let (mut font, _) = font_with_glyph();
        let second = Master {
            id: fc_model::MasterId("bold".into()),
            ..Master::default()
        };
        font.masters.push(second.clone());
        assert!(incompatible_masters(&font, font.glyph("A").unwrap()));
        let glyph = font.glyph_mut("A").unwrap();
        glyph.layers.push(Layer {
            id: fc_model::LayerId("bold-layer".into()),
            kind: LayerKind::Master(second.id.clone()),
            ..Layer::default()
        });
        assert!(!incompatible_masters(&font, font.glyph("A").unwrap()));
        font.glyph_mut("A").unwrap().layers[1].anchors.push(Anchor {
            name: "top".into(),
            x: 100.0,
            y: 700.0,
        });
        assert!(incompatible_masters(&font, font.glyph("A").unwrap()));
        font.glyph_mut("A").unwrap().layers[1].anchors.clear();
        font.glyph_mut("A").unwrap().layers[1].contours =
            Contour::from_bezpath(&kurbo::Rect::new(0.0, 0.0, 100.0, 100.0).to_path(0.1));
        assert!(incompatible_masters(&font, font.glyph("A").unwrap()));
    }

    #[test]
    fn glyph_status_cells_expose_metadata_without_a_master_layer() {
        let (mut font, _) = font_with_glyph();
        let master = Master {
            id: fc_model::MasterId("missing".into()),
            ..Master::default()
        };
        font.masters.push(master.clone());
        let glyph = font.glyph_mut("A").unwrap();
        glyph.note = "Review".into();
        glyph.locked = true;
        glyph.production_name = Some("uni0041".into());
        glyph.metrics_keys.width = Some("=500".into());
        let session = fc_doc::Session::new();
        let doc = session.open_font(font, None);
        let cell = session
            .with_doc(doc, |document| {
                super::super::glyphs::build_cell(
                    document,
                    document.glyph_id("A").unwrap(),
                    &master.id,
                    false,
                )
                .unwrap()
            })
            .unwrap();
        assert!(cell.has_annotations);
        assert!(cell.has_custom_glyph_info);
        assert!(cell.incompatible_masters);
        assert!(cell.metrics_out_of_sync);
        assert!(cell.locked);
        assert!(cell.is_empty);
    }

    #[test]
    fn glyph_status_old_cell_payloads_default_to_false() {
        let (font, master) = font_with_glyph();
        let session = fc_doc::Session::new();
        let doc = session.open_font(font, None);
        let cell = session
            .with_doc(doc, |document| {
                super::super::glyphs::build_cell(
                    document,
                    document.glyph_id("A").unwrap(),
                    &master,
                    false,
                )
                .unwrap()
            })
            .unwrap();
        let mut payload = serde_json::to_value(&cell).unwrap();
        for field in [
            "metrics_out_of_sync",
            "has_annotations",
            "has_custom_glyph_info",
            "incompatible_masters",
            "locked",
        ] {
            assert_eq!(
                payload.as_object_mut().unwrap().remove(field),
                Some(json!(false))
            );
        }
        assert_eq!(
            serde_json::from_value::<crate::GlyphCell>(payload).unwrap(),
            cell
        );
    }

    #[test]
    fn glyph_status_default_path_is_bounded_for_large_pages() {
        let (font, master) = font_with_glyph();
        let glyph = font.glyph("A").unwrap();
        let started = std::time::Instant::now();
        for _ in 0..4096 {
            assert!(!std::hint::black_box(metrics_out_of_sync(
                &font, glyph, &master, None
            )));
            assert!(!std::hint::black_box(incompatible_masters(&font, glyph)));
            assert!(!std::hint::black_box(has_annotations(glyph)));
            assert!(!std::hint::black_box(has_custom_glyph_info(glyph)));
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }
}
