mod fixtures;

use fw_geom::{boolean, remove_overlap, BooleanOp, CurveKind, DEFAULT_BOOLEAN_TOLERANCE};
use fw_model::TtIndex;

#[test]
fn overlapping_rectangles_become_one_eight_point_contour() {
    let contours = [
        fixtures::rect(0.0, 0.0, 100.0, 100.0),
        fixtures::rect(50.0, 50.0, 150.0, 150.0),
    ];
    let result = remove_overlap(&contours, CurveKind::Cubic, DEFAULT_BOOLEAN_TOLERANCE)
        .expect("rectangle union should succeed");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].points.len(), 8);
}

#[test]
fn untouched_contour_keeps_true_type_indices() {
    let mut contour = fixtures::rect(0.0, 0.0, 100.0, 100.0);
    for (index, point) in contour.points.iter_mut().enumerate() {
        point.tt_index = TtIndex::Index(index as u16);
    }
    let result = remove_overlap(std::slice::from_ref(&contour), CurveKind::Cubic, 0.01)
        .expect("single simple contour");
    assert_eq!(result, vec![contour]);
}

#[test]
fn booleans_handle_an_empty_side() {
    let a = [fixtures::rect(0.0, 0.0, 100.0, 100.0)];
    assert_eq!(
        boolean(&a, &[], BooleanOp::Union, CurveKind::Cubic, 0.01)
            .expect("union with empty")
            .len(),
        1
    );
    assert!(
        boolean(&a, &[], BooleanOp::Intersect, CurveKind::Cubic, 0.01)
            .expect("intersection with empty")
            .is_empty()
    );
}

#[test]
fn dejavu_o_outer_and_counter_are_unchanged() {
    let outcome = fw_sfnt::read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &fw_sfnt::ReadOptions::default(),
    )
    .expect("read DejaVu Sans");
    let font = &outcome.fonts[0];
    let glyph = font.glyph("O").expect("O glyph");
    let layer = glyph
        .master_layer(&font.default_master().id)
        .expect("O master layer");
    let result = remove_overlap(
        &layer.contours,
        CurveKind::Quadratic { max_err: 0.01 },
        0.01,
    )
    .expect("remove overlap on O");
    assert_eq!(result, layer.contours);
}

#[test]
fn all_boolean_operations_are_available() {
    let a = [fixtures::rect(0.0, 0.0, 100.0, 100.0)];
    let b = [fixtures::rect(50.0, 0.0, 150.0, 100.0)];
    for op in [
        BooleanOp::Union,
        BooleanOp::Intersect,
        BooleanOp::Exclude,
        BooleanOp::Xor,
    ] {
        assert!(boolean(&a, &b, op, CurveKind::Cubic, 0.01).is_ok());
    }
}

#[test]
fn self_touching_input_never_panics() {
    let bow = fw_model::Contour {
        closed: true,
        points: vec![
            fixtures::point(0.0, 0.0, fw_model::PointKind::Line),
            fixtures::point(100.0, 100.0, fw_model::PointKind::Line),
            fixtures::point(0.0, 100.0, fw_model::PointKind::Line),
            fixtures::point(100.0, 0.0, fw_model::PointKind::Line),
        ],
        name: None,
        id: None,
    };
    let _ = remove_overlap(&[bow], CurveKind::Cubic, 0.01);
}
