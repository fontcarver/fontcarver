mod common;

use fc_doc::{fcf, kerning::KerningGroupSide, Command, Session};
use fc_model::{
    Component, EffectBoolean, Glyph, KernSide, LayerEffect, LayerEffectKind, MathGlyphInfo,
    MathTable,
};
use kurbo::{Rect, Shape};

fn replacement(original: &Glyph) -> Glyph {
    let mut glyph = original.clone();
    glyph.unicodes = vec![0x42];
    glyph.note = "Replacement source".into();
    glyph.layers[0].id = fc_model::LayerId::new();
    glyph.layers[0].width = 720.0;
    glyph.layers[0].contours[0].points[1].x = 180.0;
    glyph.layers[0].contours[0].points[2].x = 180.0;
    glyph.kern_group_side1 = None;
    glyph.kern_group_side2 = Some("public.kern2.unrelated".into());
    glyph
}

#[test]
fn replacement_retains_identity_references_and_memberships_across_history() {
    let mut font = common::test_font();
    font.glyph_mut("base").unwrap().unicodes = vec![0x41];
    font.glyph_mut("base").unwrap().kern_group_side1 = Some("public.kern1.base".into());
    font.kerning_groups
        .side1
        .insert("public.kern1.base".into(), ["base".into()].into());
    font.masters[0].kerning_h.0.insert(
        (
            KernSide::Glyph("base".into()),
            KernSide::Glyph("composite".into()),
        ),
        -60.0,
    );
    font.math = Some(MathTable {
        glyphs: [("base".into(), MathGlyphInfo::default())].into(),
        ..Default::default()
    });
    let original = font.clone();
    let mut expected = font.clone();
    let incoming = replacement(font.glyph("base").unwrap());
    let mut retained = incoming.clone();
    retained.kern_group_side1 = font.glyph("base").unwrap().kern_group_side1.clone();
    retained.kern_group_side2 = font.glyph("base").unwrap().kern_group_side2.clone();
    expected.glyphs.insert("base".into(), retained);
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let base = doc.glyph_id("base").unwrap();
            let composite = doc.glyph_id("composite").unwrap();
            let master = doc.font.default_master().id.clone();
            assert_eq!(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 0., 100., 100.)
            );
            let change = doc
                .apply(Command::ReplaceGlyph { glyph: incoming }, "Replace")
                .unwrap();
            assert!(change.layers && change.metrics);
            assert!(!change.glyph_order);
            assert!(change.glyphs.contains(&base) && change.glyphs.contains(&composite));
            assert_eq!(doc.font, expected);
            assert_eq!(doc.glyph_id("base"), Some(base));
            assert_eq!(doc.glyph_id("composite"), Some(composite));
            assert_eq!(
                doc.font.glyphs.keys().collect::<Vec<_>>(),
                original.glyphs.keys().collect::<Vec<_>>()
            );
            assert_eq!(doc.unicode_index().glyphs(0x41).count(), 0);
            assert_eq!(
                doc.unicode_index().glyphs(0x42).collect::<Vec<_>>(),
                vec![&fc_model::GlyphName::from("base")]
            );
            assert_eq!(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 0., 180., 100.)
            );
            assert_eq!(doc.undo_len(), 1);
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            assert_eq!(doc.glyph_id("base"), Some(base));
            assert!(!doc.dirty);
            assert_eq!(doc.unicode_index().glyphs(0x41).count(), 1);
            assert_eq!(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 0., 100., 100.)
            );
            doc.redo().unwrap();
            assert_eq!(doc.font, expected);
            assert_eq!(doc.glyph_id("base"), Some(base));
            assert_eq!(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 0., 180., 100.)
            );
        })
        .unwrap();
}

#[test]
fn invalid_replacements_and_failed_batches_leave_the_document_unchanged() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let base_id = doc.glyph_id("base").unwrap();
            let base = original.glyph("base").unwrap();
            let mut bad_outline = replacement(base);
            bad_outline.layers[0].contours[0].points[0].x = f64::INFINITY;
            let mut malformed = replacement(base);
            malformed.layers[0].contours[0].points[1].kind = fc_model::PointKind::Curve;
            let mut bad_width = replacement(base);
            bad_width.layers[0].width = f64::INFINITY;
            let mut bad_vertical = replacement(base);
            bad_vertical.layers[0].vertical_width = Some(f64::NAN);
            let mut bad_origin = replacement(base);
            bad_origin.layers[0].vertical_origin = Some(f64::NEG_INFINITY);
            let mut bad_keys = replacement(base);
            bad_keys.metrics_keys.width = Some("==composite".into());
            let mut bad_local_keys = replacement(base);
            bad_local_keys.layers[0].metrics_keys = Some(fc_model::MetricsKeys {
                left: Some("=base/0".into()),
                ..Default::default()
            });
            let mut bad_component = replacement(base);
            bad_component.layers[0].components.push(Component {
                base: "composite".into(),
                transform: kurbo::Affine::scale(f64::NAN),
                ..Default::default()
            });
            let mut bad_effect = replacement(base);
            bad_effect.layers[0].effects.push(LayerEffect {
                enabled: true,
                effect: LayerEffectKind::Boolean {
                    operation: EffectBoolean::Union,
                    path: "invalid path".into(),
                },
            });
            let mut cycle = replacement(base);
            cycle.layers[0].components.push(Component {
                base: "composite".into(),
                ..Default::default()
            });
            for invalid in [
                Glyph::new("missing"),
                bad_outline,
                malformed,
                bad_width,
                bad_vertical,
                bad_origin,
                bad_keys,
                bad_local_keys,
                bad_component,
                bad_effect,
                cycle,
            ] {
                assert!(doc
                    .apply(
                        Command::ReplaceGlyph { glyph: invalid },
                        "Invalid replacement"
                    )
                    .is_err());
                assert_eq!(doc.font, original);
                assert_eq!(doc.glyph_id("base"), Some(base_id));
                assert_eq!(doc.generation, 0);
                assert_eq!(doc.undo_len(), 0);
                assert!(!doc.dirty);
            }
            assert!(doc
                .apply(
                    Command::Batch(vec![
                        Command::ReplaceGlyph {
                            glyph: replacement(base)
                        },
                        Command::ReplaceGlyph {
                            glyph: Glyph::new("missing")
                        },
                    ]),
                    "Failed replacement batch"
                )
                .is_err());
            assert_eq!(doc.font, original);
            assert_eq!(
                doc.font.glyphs.keys().collect::<Vec<_>>(),
                original.glyphs.keys().collect::<Vec<_>>()
            );
            assert_eq!(doc.glyph_id("base"), Some(base_id));
            assert_eq!(doc.generation, 0);
            assert_eq!(doc.undo_len(), 0);
        })
        .unwrap();
}

#[test]
fn replacement_under_a_dependent_live_effect_rolls_back_invalid_source() {
    let mut font = common::test_font();
    font.glyph_mut("composite").unwrap().layers[0]
        .effects
        .push(LayerEffect {
            enabled: true,
            // Filled Boolean operations require closed source contours;
            // RemoveOverlap deliberately preserves open contours unchanged.
            effect: LayerEffectKind::Boolean {
                operation: EffectBoolean::Union,
                path: "M0 0L100 0L100 100L0 100Z".into(),
            },
        });
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let master = doc.font.default_master().id.clone();
            let dependent = doc.glyph_id("composite").unwrap();
            let original_path = doc.flattened_path(dependent, &master).unwrap();
            let original_generation = doc.glyph_generation(dependent);
            let mut incoming = replacement(original.glyph("base").unwrap());
            incoming.layers[0].contours[0].closed = false;
            let error = doc
                .apply(
                    Command::ReplaceGlyph { glyph: incoming },
                    "Invalid effect source",
                )
                .unwrap_err();
            assert!(error.to_string().contains("closed source contours"));
            assert_eq!(doc.font, original);
            assert_eq!(doc.generation, 0);
            assert_eq!(doc.undo_len(), 0);
            assert_eq!(doc.glyph_generation(dependent), original_generation);
            assert_eq!(
                doc.flattened_path(dependent, &master).unwrap(),
                original_path
            );
        })
        .unwrap();
}

#[test]
fn replacement_with_open_contours_remains_valid_under_dependent_remove_overlap() {
    let mut font = common::test_font();
    font.glyph_mut("composite").unwrap().layers[0]
        .effects
        .push(LayerEffect {
            enabled: true,
            effect: LayerEffectKind::RemoveOverlap,
        });
    let session = Session::new();
    let id = session.open_font(font, None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let master = doc.font.default_master().id.clone();
            let dependent = doc.glyph_id("composite").unwrap();
            let old_path = doc.flattened_path(dependent, &master).unwrap();
            let mut incoming = replacement(original.glyph("base").unwrap());
            incoming.layers[0].contours[0].closed = false;
            doc.apply(
                Command::ReplaceGlyph { glyph: incoming },
                "Open effect source",
            )
            .unwrap();
            let path = doc.flattened_path(dependent, &master).unwrap();
            assert_ne!(path, old_path);
            assert!(!path.elements().contains(&kurbo::PathEl::ClosePath));
            assert_eq!(doc.undo_len(), 1);
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            assert_eq!(doc.flattened_path(dependent, &master).unwrap(), old_path);
        })
        .unwrap();
}

#[test]
fn explicit_membership_changes_can_follow_replacement_in_one_batch() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let original = doc.font.clone();
            let glyph = replacement(original.glyph("base").unwrap());
            doc.apply(
                Command::Batch(vec![
                    Command::ReplaceGlyph { glyph },
                    Command::CreateKerningGroup {
                        side: KerningGroupSide::Side1,
                        name: "public.kern1.new".into(),
                        members: vec![],
                    },
                    Command::AssignKerningGroup {
                        side: KerningGroupSide::Side1,
                        glyph: "base".into(),
                        group: Some("public.kern1.new".into()),
                    },
                ]),
                "Replace with explicit membership",
            )
            .unwrap();
            let replaced = doc.font.clone();
            assert_eq!(
                doc.font.glyph("base").unwrap().kern_group_side1.as_deref(),
                Some("public.kern1.new")
            );
            assert!(doc.font.kerning_groups.side1["public.kern1.new"].contains("base"));
            doc.undo().unwrap();
            assert_eq!(doc.font, original);
            doc.redo().unwrap();
            assert_eq!(doc.font, replaced);
        })
        .unwrap();
}

#[test]
fn replacement_accepts_unresolved_metric_references_with_valid_syntax() {
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let mut glyph = replacement(doc.font.glyph("base").unwrap());
            glyph.metrics_keys.width = Some("=missing".into());
            glyph.layers[0].metrics_keys = Some(fc_model::MetricsKeys {
                left: Some("==missing+10".into()),
                ..Default::default()
            });
            let global = glyph.metrics_keys.clone();
            let local = glyph.layers[0].metrics_keys.clone();
            doc.apply(
                Command::ReplaceGlyph { glyph },
                "Replace unresolved metrics",
            )
            .unwrap();
            let actual = doc.font.glyph("base").unwrap();
            assert_eq!(actual.metrics_keys, global);
            assert_eq!(actual.layers[0].metrics_keys, local);
        })
        .unwrap();
}

#[test]
fn replacement_round_trips_plain_and_compressed_native_files() {
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let scratch = Scratch(common::temp_dir("replace-glyph"));
    let session = Session::new();
    let id = session.open_font(common::test_font(), None);
    session
        .with_doc_mut(id, |doc| {
            let glyph = replacement(doc.font.glyph("base").unwrap());
            doc.apply(Command::ReplaceGlyph { glyph }, "Replace")
                .unwrap();
            for file in ["replaced.fcf", "replaced.fcf.gz"] {
                let path = scratch.0.join(file);
                fcf::write(&path, &doc.font).unwrap();
                assert_eq!(fcf::read(&path).unwrap(), doc.font);
            }
        })
        .unwrap();
}
