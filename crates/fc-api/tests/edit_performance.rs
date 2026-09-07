use std::{sync::Arc, time::Duration, time::Instant};

use fc_api::{Api, DocParams, EditApplyParams, EditBeginParams, EditEndParams, EditTokenId};
use fc_doc::{Command, HandleMode, PointRef, Session};
use fc_model::{Contour, Font, Glyph, Layer, LayerId, Point, PointKind};

const GLYPH_COUNT: usize = 20_000;
const POINT_COUNT: usize = 300;
const WARM_SAMPLES: usize = 20;
const MEASURED_SAMPLES: usize = 200;

fn large_edit_font() -> (Font, LayerId) {
    let mut font = Font::new_empty("Edit performance", 1_000);
    let master = font.default_master().id.clone();
    let mut target_layer = Layer::new_master(master.clone());
    target_layer.width = 1_000.0;
    target_layer.contours.push(Contour {
        closed: true,
        points: (0..POINT_COUNT)
            .map(|index| {
                let angle = index as f64 / POINT_COUNT as f64 * std::f64::consts::TAU;
                Point {
                    x: angle.cos() * 400.0,
                    y: angle.sin() * 400.0,
                    kind: PointKind::Line,
                    ..Point::default()
                }
            })
            .collect(),
        ..Contour::default()
    });
    let layer_id = target_layer.id.clone();
    let mut target = Glyph::new("target");
    target.layers.push(target_layer);
    font.insert_glyph(target).expect("insert target glyph");

    for index in 1..GLYPH_COUNT {
        let mut layer = Layer::new_master(master.clone());
        layer.width = 500.0;
        layer.contours.push(Contour {
            closed: true,
            points: vec![
                Point {
                    x: 50.0,
                    y: 0.0,
                    kind: PointKind::Line,
                    ..Point::default()
                },
                Point {
                    x: 50.0,
                    y: 500.0,
                    kind: PointKind::Line,
                    ..Point::default()
                },
                Point {
                    x: 450.0,
                    y: 500.0,
                    kind: PointKind::Line,
                    ..Point::default()
                },
                Point {
                    x: 450.0,
                    y: 0.0,
                    kind: PointKind::Line,
                    ..Point::default()
                },
            ],
            ..Contour::default()
        });
        let mut glyph = Glyph::new(format!("g{index:05}"));
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert supporting glyph");
    }
    (font, layer_id)
}

fn percentile(sorted: &[Duration], numerator: usize, denominator: usize) -> Duration {
    let rank = (sorted.len() * numerator).div_ceil(denominator);
    sorted[rank.saturating_sub(1)]
}

#[test]
#[ignore = "release-only acceptance performance gate"]
fn edit_apply_meets_budget_at_twenty_thousand_glyphs() {
    if cfg!(debug_assertions) {
        panic!("performance gate must be run with --release");
    }
    let (font, layer) = large_edit_font();
    let session = Arc::new(Session::new());
    let doc = session.open_font(font, None);
    let api = Api::new(Arc::clone(&session));
    let token: EditTokenId = serde_json::from_value(
        api.call(
            "doc.edit_begin",
            serde_json::to_value(EditBeginParams {
                doc,
                label: "Move 300 points".to_owned(),
            })
            .expect("serialize edit begin"),
        )
        .expect("begin edit"),
    )
    .expect("decode edit token");
    let points = (0..POINT_COUNT)
        .map(|point| PointRef { contour: 0, point })
        .collect::<Vec<_>>();
    let request = || {
        serde_json::to_value(EditApplyParams {
            doc,
            token,
            command: Command::MovePoints {
                glyph: "target".into(),
                layer: layer.clone(),
                points: points.clone(),
                delta: (0.125, 0.25),
                handles: HandleMode::PointsOnly,
            },
        })
        .expect("serialize edit apply")
    };

    for _ in 0..WARM_SAMPLES {
        api.call("doc.edit_apply", request())
            .expect("warm edit apply");
    }
    let measured_requests = (0..MEASURED_SAMPLES).map(|_| request()).collect::<Vec<_>>();
    let mut elapsed = Vec::with_capacity(MEASURED_SAMPLES);
    for params in measured_requests {
        let started = Instant::now();
        api.call("doc.edit_apply", params)
            .expect("measured edit apply");
        elapsed.push(started.elapsed());
    }
    api.call(
        "doc.edit_end",
        serde_json::to_value(EditEndParams { doc, token }).expect("serialize edit end"),
    )
    .expect("end edit");

    elapsed.sort_unstable();
    let median = percentile(&elapsed, 50, 100);
    let p95 = percentile(&elapsed, 95, 100);
    let max = *elapsed.last().expect("measured durations");
    eprintln!(
        "edit_apply 20k glyphs/300 points/{MEASURED_SAMPLES} samples: median={median:?} p95={p95:?} max={max:?}"
    );
    assert!(
        p95 <= Duration::from_millis(4),
        "edit_apply p95 was {p95:?}"
    );

    session
        .with_doc(doc, |document| {
            assert_eq!(document.undo_len(), 1);
            assert_eq!(document.undo_label(), Some("Move 300 points"));
            let first = &document.font.glyph("target").expect("target glyph").layers[0].contours[0]
                .points[0];
            assert_eq!(first.x, 400.0 + 27.5);
            assert_eq!(first.y, 55.0);
        })
        .expect("read edited document");
    api.call(
        "doc.undo",
        serde_json::to_value(DocParams { doc }).expect("serialize undo"),
    )
    .expect("undo edit");
    session
        .with_doc(doc, |document| {
            assert_eq!(document.undo_len(), 1);
            assert_eq!(document.undo_label(), None);
            assert_eq!(document.redo_label(), Some("Move 300 points"));
            let first = &document.font.glyph("target").expect("target glyph").layers[0].contours[0]
                .points[0];
            assert_eq!(first.x, 400.0);
            assert_eq!(first.y, 0.0);
        })
        .expect("read undone document");
}
