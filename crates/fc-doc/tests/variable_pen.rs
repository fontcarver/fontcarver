use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use fc_doc::{Command, Session};
use fc_model::{
    Component, Contour, EffectCap, EffectJoin, EffectNib, EffectProgression, Font, Glyph, Layer,
    LayerEffect, LayerEffectKind, PenPoint,
};
use kurbo::{Affine, BezPath, Point, Rect, Shape};

fn fixture() -> Font {
    let mut font = Font::new_empty("Variable Pen", 1000);
    let master = font.default_master().id.clone();
    let mut base = Glyph::new("base");
    let mut layer = Layer::new_master(master.clone());
    let mut path = BezPath::new();
    path.move_to((0., 0.));
    path.line_to((200., 0.));
    layer.contours = Contour::from_bezpath(&path);
    base.layers.push(layer);
    font.insert_glyph(base).unwrap();
    let mut composite = Glyph::new("composite");
    let mut layer = Layer::new_master(master);
    layer.components.push(Component {
        base: "base".into(),
        transform: Affine::translate((0., 200.)),
        ..Default::default()
    });
    composite.layers.push(layer);
    font.insert_glyph(composite).unwrap();
    font
}

fn point(position: f64, width: f64) -> PenPoint {
    PenPoint {
        position,
        width,
        height: width,
        angle: 0.,
    }
}

fn effect(points: Vec<PenPoint>) -> LayerEffect {
    LayerEffect {
        enabled: true,
        effect: LayerEffectKind::VariableStroke {
            nib: EffectNib::Ellipse,
            cap: EffectCap::Butt,
            join: EffectJoin::Round,
            miter_limit: 4.,
            progression: EffectProgression::Linear,
            points,
        },
    }
}

fn tapered_effect() -> LayerEffect {
    effect(vec![point(0., 20.), point(1., 60.)])
}

fn assert_bounds(actual: Rect, expected: Rect) {
    for (actual, expected) in [
        (actual.x0, expected.x0),
        (actual.y0, expected.y0),
        (actual.x1, expected.x1),
        (actual.y1, expected.y1),
    ] {
        assert!((actual - expected).abs() < 0.05, "{actual} != {expected}");
    }
}

#[test]
fn source_edits_recompute_variable_pen_and_component_caches_with_exact_history() {
    let session = Session::new();
    let id = session.open_font(fixture(), None);
    session
        .with_doc_mut(id, |doc| {
            let master = doc.font.default_master().id.clone();
            let composite = doc.glyph_id("composite").unwrap();
            let source = doc.font.glyph("base").unwrap().layers[0].clone();
            assert_bounds(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 200., 200., 200.),
            );
            doc.apply(
                Command::SetLayerEffects {
                    glyph: "base".into(),
                    layer: source.id.clone(),
                    effects: vec![tapered_effect()],
                },
                "Variable pen",
            )
            .unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].contours,
                source.contours
            );
            assert_bounds(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 170., 200., 230.),
            );
            let with_pen = doc.font.glyph("base").unwrap().layers[0].clone();
            let mut contours = source.contours.clone();
            contours[0].points[1].x = 300.;
            doc.apply(
                Command::ReplaceLayerOutline {
                    glyph: "base".into(),
                    layer: source.id.clone(),
                    contours: contours.clone(),
                    components: vec![],
                    anchors: vec![],
                },
                "Lengthen centerline",
            )
            .unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0].contours, contours);
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].effects,
                with_pen.effects
            );
            assert_bounds(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 170., 300., 230.),
            );
            assert_eq!(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .winding(Point::new(150., 223.)),
                0,
                "pen positions must resample against the edited centerline length"
            );
            let edited = doc.font.glyph("base").unwrap().layers[0].clone();
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0], with_pen);
            assert_bounds(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 170., 200., 230.),
            );
            doc.undo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0], source);
            doc.redo().unwrap();
            doc.redo().unwrap();
            assert_eq!(doc.font.glyph("base").unwrap().layers[0], edited);
            assert_bounds(
                doc.flattened_path(composite, &master)
                    .unwrap()
                    .bounding_box(),
                Rect::new(0., 170., 300., 230.),
            );
        })
        .unwrap();
}

#[test]
fn native_roundtrip_retains_editable_pen_points_and_rejects_malformed_profiles() {
    let directory = TestDirectory::new();
    let mut font = fixture();
    let mut pen = effect(vec![point(0., 20.), point(0.4, 80.), point(1., 40.)]);
    if let LayerEffectKind::VariableStroke {
        nib,
        progression,
        points,
        ..
    } = &mut pen.effect
    {
        *nib = EffectNib::Rectangle;
        *progression = EffectProgression::Smooth;
        points[1].height = 30.;
        points[1].angle = 35.;
    }
    font.glyph_mut("base").unwrap().layers[0].effects = vec![pen];
    let source = font.glyph("base").unwrap().layers[0].clone();
    let expected = fc_geom::flatten_layer(&font, "base", &font.default_master().id).unwrap();
    for name in ["variable-pen.fcf", "variable-pen.fcf.gz"] {
        let path = directory.0.join(name);
        fc_doc::fcf::write(&path, &font).unwrap();
        let reopened = fc_doc::fcf::read(&path).unwrap();
        assert_eq!(reopened.glyph("base").unwrap().layers[0], source);
        assert_eq!(
            fc_geom::flatten_layer(&reopened, "base", &reopened.default_master().id).unwrap(),
            expected
        );
    }
    let path = directory.0.join("variable-pen.fcf");
    let mut encoded: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    encoded["font"]["glyphs"]["base"]["layers"][0]["effects"][0]["effect"]["points"][1]
        ["position"] = serde_json::json!(-0.1);
    std::fs::write(&path, serde_json::to_vec(&encoded).unwrap()).unwrap();
    assert!(fc_doc::fcf::read(&path).is_err());

    let existing = directory.0.join("variable-pen.fcf.gz");
    let saved = std::fs::read(&existing).unwrap();
    if let LayerEffectKind::VariableStroke { points, .. } =
        &mut font.glyph_mut("base").unwrap().layers[0].effects[0].effect
    {
        points[1].position = -0.1;
    }
    assert!(fc_doc::fcf::write(&existing, &font).is_err());
    assert_eq!(std::fs::read(&existing).unwrap(), saved);
}

#[test]
fn invalid_pen_edits_leave_font_generation_and_redo_history_untouched() {
    let mut cases = vec![
        vec![],
        vec![point(0., 20.)],
        vec![point(0.1, 20.), point(1., 60.)],
        vec![point(0., 20.), point(0.9, 60.)],
        vec![point(0., 20.), point(0., 30.), point(1., 60.)],
        vec![
            point(0., 20.),
            point(0.8, 30.),
            point(0.2, 40.),
            point(1., 60.),
        ],
        vec![point(0., 0.), point(1., 60.)],
        vec![point(0., 8193.), point(1., 60.)],
        (0..65)
            .map(|index| point(f64::from(index) / 64., 20.))
            .collect(),
    ];
    for (height, angle) in [(0.1, 0.), (20., 361.), (20., f64::INFINITY)] {
        let mut invalid = vec![point(0., 20.), point(1., 60.)];
        invalid[0].height = height;
        invalid[0].angle = angle;
        cases.push(invalid);
    }
    let session = Session::new();
    let id = session.open_font(fixture(), None);
    session
        .with_doc_mut(id, |doc| {
            let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
            doc.apply(
                Command::SetLayerEffects {
                    glyph: "base".into(),
                    layer: layer.clone(),
                    effects: vec![tapered_effect()],
                },
                "Variable pen",
            )
            .unwrap();
            doc.undo().unwrap();
            let before = doc.font.clone();
            let generation = doc.generation;
            let undo_len = doc.undo_len();
            for (index, points) in cases.into_iter().enumerate() {
                let mut invalid = effect(points);
                // Disabled profiles remain validated so enabling later cannot expose invalid data.
                invalid.enabled = index % 2 == 0;
                assert!(
                    doc.apply(
                        Command::SetLayerEffects {
                            glyph: "base".into(),
                            layer: layer.clone(),
                            effects: vec![invalid],
                        },
                        "Invalid variable pen"
                    )
                    .is_err(),
                    "accepted malformed profile {index}"
                );
                assert_eq!(doc.font, before);
                assert_eq!(doc.generation, generation);
                assert_eq!(doc.undo_len(), undo_len);
                assert_eq!(doc.redo_label(), Some("Variable pen"));
            }
            doc.redo().unwrap();
            assert_eq!(
                doc.font.glyph("base").unwrap().layers[0].effects,
                vec![tapered_effect()]
            );
        })
        .unwrap();
}

#[test]
fn closing_a_tapered_centerline_rejects_discontinuous_seam_without_mutation() {
    for effect_owner in ["base", "composite"] {
        let session = Session::new();
        let id = session.open_font(fixture(), None);
        session
            .with_doc_mut(id, |doc| {
                let master = doc.font.default_master().id.clone();
                let composite = doc.glyph_id("composite").unwrap();
                let layer = doc.font.glyph("base").unwrap().layers[0].id.clone();
                let effect_layer = doc.font.glyph(effect_owner).unwrap().layers[0].id.clone();
                doc.apply(
                    Command::SetLayerEffects {
                        glyph: effect_owner.into(),
                        layer: effect_layer,
                        effects: vec![tapered_effect()],
                    },
                    "Variable pen",
                )
                .unwrap();
                let cached = doc.flattened_path(composite, &master).unwrap();
                let before = doc.font.clone();
                let generation = doc.generation;
                let undo_len = doc.undo_len();
                assert!(doc
                    .apply(
                        Command::ReplaceLayerOutline {
                            glyph: "base".into(),
                            layer,
                            contours: Contour::from_bezpath(
                                &Rect::new(0., 0., 200., 100.).to_path(0.01)
                            ),
                            components: vec![],
                            anchors: vec![],
                        },
                        "Close tapered centerline"
                    )
                    .is_err());
                assert_eq!(doc.font, before);
                assert_eq!(doc.generation, generation);
                assert_eq!(doc.undo_len(), undo_len);
                assert_eq!(doc.flattened_path(composite, &master).unwrap(), cached);
            })
            .unwrap();
    }
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fc-doc-variable-pen-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
