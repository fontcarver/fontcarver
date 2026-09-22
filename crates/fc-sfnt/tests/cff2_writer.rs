use fc_sfnt::write::{cff2_outline, write_cff2, Cff2Delta, Cff2Glyph, Cff2Outline};

fn glyph() -> Cff2Glyph {
    Cff2Glyph {
        outline: Cff2Outline {
            operators: vec![21, 5, 5],
            operands: vec![0., 0., 100., 0., 0., 200.],
        },
        deltas: vec![Cff2Delta {
            region: vec![[0., 1., 1.]],
            values: vec![0., 0., 20., 0., 0., 40.],
        }],
    }
}

#[test]
fn malformed_dimensions_regions_programs_and_numbers_fail_cleanly() {
    assert!(write_cff2(1000, 1, &[glyph()]).is_ok());
    for malformed in 0..8 {
        let mut glyph = glyph();
        match malformed {
            0 => {
                glyph.outline.operands.pop();
            }
            1 => {
                glyph.deltas[0].values.pop();
            }
            2 => {
                glyph.deltas[0].region[0] = [0., f64::NAN, 1.];
            }
            3 => {
                glyph.deltas[0].region[0] = [0., 0.0000001, 1.];
            }
            4 => {
                glyph.deltas[0].region[0] = [-1., 0.5, 1.];
            }
            5 => {
                glyph.outline.operators[0] = 5;
            }
            6 => {
                glyph.outline.operands[2] = f64::INFINITY;
            }
            _ => {
                glyph.deltas[0].values[2] = f64::NAN;
            }
        }
        assert!(write_cff2(1000, 1, &[glyph]).is_err(), "case {malformed}");
    }
    let mut path = kurbo::BezPath::new();
    path.move_to((0., 0.));
    path.line_to((100., 100.));
    assert!(cff2_outline(&path).is_err());
}

#[test]
fn charstrings_obey_the_specification_length_limit() {
    let mut glyph = glyph();
    glyph.deltas.clear();
    glyph.outline.operators = std::iter::once(21)
        .chain(std::iter::repeat_n(5, 22_000))
        .collect();
    glyph.outline.operands = vec![0.; 44_002];
    assert!(write_cff2(1000, 1, &[glyph])
        .unwrap_err()
        .to_string()
        .contains("65535"));
}
