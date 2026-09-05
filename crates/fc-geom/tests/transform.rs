mod fixtures;

use fc_geom::{round_to_grid, transform};
use fc_model::TtIndex;
use kurbo::Affine;
use proptest::prelude::*;

proptest! {
    #[test]
    fn affine_round_trip(x in -1000.0f64..1000.0, y in -1000.0f64..1000.0,
                         sx in 0.1f64..10.0, sy in 0.1f64..10.0,
                         tx in -100.0f64..100.0, ty in -100.0f64..100.0) {
        let c = fixtures::open_line(x, y, x + 3.0, y - 7.0);
        let affine = Affine::scale_non_uniform(sx, sy).then_translate((tx, ty).into());
        let result = transform(&transform(std::slice::from_ref(&c), affine), affine.inverse());
        for (actual, expected) in result[0].points.iter().zip(&c.points) {
            prop_assert!((actual.x - expected.x).abs() < 1e-8);
            prop_assert!((actual.y - expected.y).abs() < 1e-8);
        }
    }

    #[test]
    fn rounding_is_idempotent(x in -1000.0f64..1000.0, y in -1000.0f64..1000.0,
                              step in 0.01f64..100.0) {
        let mut c = fixtures::open_line(x, y, x + 3.0, y - 7.0);
        c.points[0].tt_index = TtIndex::Index(17);
        let once = round_to_grid(&[c], step);
        let twice = round_to_grid(&once, step);
        prop_assert_eq!(&once, &twice);
        prop_assert_eq!(once[0].points[0].tt_index, TtIndex::Index(17));
    }
}
