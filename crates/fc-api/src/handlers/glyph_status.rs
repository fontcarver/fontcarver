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

/// The tag a cross-document transfer writes on a stand-in glyph.
///
/// Documented in `crates/fc-doc/src/fcf/schema.md`: a transfer that needs a
/// component base the destination lacks creates an empty, non-exporting glyph
/// carrying this tag, and nothing else writes it.
pub(crate) const PLACEHOLDER_TAG: &str = "missing-component-base";

/// Whether the glyph is a stand-in for a component base another document had.
pub(crate) fn is_placeholder(glyph: &Glyph) -> bool {
    glyph.tags.iter().any(|tag| tag == PLACEHOLDER_TAG)
}

/// Whether the glyph's stored metadata departs from the effective glyph database.
///
/// Presence alone is not custom: `Glyph → Update Glyph Info` writes the
/// database's own category, subcategory, script and production name onto every
/// selected glyph, and that must not mark the whole font as customised. A value
/// counts only when it differs from what [`fc_glyphdata::resolve_info`] derives
/// for the name and its encodings, which is the same rule `glyphdata.state`
/// reports as its `custom` flag — hence the document's own overlay rather than
/// the bundled data alone, so the badge does not relight after a refresh driven
/// by that overlay. Format-specific `user_data` is always custom, since the
/// database has nothing to compare it against.
pub(crate) fn has_custom_glyph_info(
    glyph: &Glyph,
    overlay: &fc_glyphdata::GlyphDataOverlay,
) -> bool {
    let info = fc_glyphdata::resolve_info(glyph.name.as_ref(), &glyph.unicodes, overlay);
    differs(&glyph.production_name, &info.production_name)
        || differs(&glyph.category, &info.category)
        || differs(&glyph.sub_category, &info.sub_category)
        || differs(&glyph.script, &info.script)
        || match &glyph.user_data {
            serde_json::Value::Null => false,
            serde_json::Value::Object(value) => !value.is_empty(),
            serde_json::Value::Array(value) => !value.is_empty(),
            serde_json::Value::String(value) => !value.is_empty(),
            _ => true,
        }
}

/// An absent stored value inherits the database, so only a present one can differ.
fn differs(stored: &Option<String>, derived: &Option<String>) -> bool {
    stored
        .as_ref()
        .is_some_and(|value| Some(value) != derived.as_ref())
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

    /// The bundled database alone: these tests carry no project overlay.
    fn overlay() -> fc_glyphdata::GlyphDataOverlay {
        fc_glyphdata::GlyphDataOverlay::default()
    }
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
        assert!(!has_custom_glyph_info(glyph, &overlay()));
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
        assert!(!has_custom_glyph_info(&glyph, &overlay()));
        glyph.production_name = Some("A.custom".into());
        assert!(has_custom_glyph_info(&glyph, &overlay()));
        glyph.production_name = None;
        glyph.category = Some("Symbol".into());
        assert!(has_custom_glyph_info(&glyph, &overlay()));
        glyph.category = None;
        glyph.user_data = json!({"vendor.flag": true});
        assert!(has_custom_glyph_info(&glyph, &overlay()));
    }

    #[test]
    fn glyph_status_custom_info_ignores_values_the_database_supplies() {
        // Exactly what `Command::UpdateGlyphInfo` writes for this glyph.
        let info = fc_glyphdata::resolve_info("Aacute", &[0x00C1], &overlay());
        let mut glyph = Glyph::new("Aacute");
        glyph.unicodes = vec![0x00C1];
        glyph.category = info.category.clone();
        glyph.sub_category = info.sub_category.clone();
        glyph.script = info.script.clone();
        glyph.production_name = info.production_name.clone();
        assert_eq!(glyph.category.as_deref(), Some("Letter"));
        assert!(
            !has_custom_glyph_info(&glyph, &overlay()),
            "refreshing from the database is not custom glyph info"
        );
        glyph.script = Some("Grek".into());
        assert!(has_custom_glyph_info(&glyph, &overlay()));
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
                    &overlay(),
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
                    &overlay(),
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
            assert!(!std::hint::black_box(has_custom_glyph_info(
                glyph,
                &overlay()
            )));
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }
}
