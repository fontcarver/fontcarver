//! Small original CFF tables exercise reader precision independently of the writer.

use super::*;
use read_fonts::types::Tag;
use write_fonts::{from_obj::ToOwnedTable, FontBuilder};

fn index(objects: &[Vec<u8>], cff2: bool) -> Vec<u8> {
    let mut data = if cff2 {
        (objects.len() as u32).to_be_bytes().to_vec()
    } else {
        (objects.len() as u16).to_be_bytes().to_vec()
    };
    if objects.is_empty() {
        return data;
    }
    data.push(2);
    let mut offset = 1u16;
    data.extend_from_slice(&offset.to_be_bytes());
    for object in objects {
        offset += object.len() as u16;
        data.extend_from_slice(&offset.to_be_bytes());
    }
    for object in objects {
        data.extend_from_slice(object);
    }
    data
}

fn dict_integer(data: &mut Vec<u8>, value: usize) {
    data.push(29);
    data.extend_from_slice(&(value as i32).to_be_bytes());
}

fn dict_matrix(values: [&str; 6]) -> Vec<u8> {
    let mut data = Vec::new();
    for value in values {
        data.push(30);
        let mut nibbles = value
            .bytes()
            .map(|byte| match byte {
                b'.' => 10,
                b'-' => 14,
                _ => byte - b'0',
            })
            .collect::<Vec<_>>();
        nibbles.push(15);
        if nibbles.len() % 2 != 0 {
            nibbles.push(15);
        }
        data.extend(nibbles.chunks_exact(2).map(|pair| pair[0] << 4 | pair[1]));
    }
    data.extend_from_slice(&[12, 7]);
    data
}

fn charstring_operands(data: &mut Vec<u8>, values: &[f64], operator: u8) {
    for value in values {
        if value.fract() == 0.0 && (-107.0..=107.0).contains(value) {
            data.push((*value as i16 + 139) as u8);
        } else {
            data.push(255);
            data.extend_from_slice(&Fixed::from_f64(*value).to_bits().to_be_bytes());
        }
    }
    data.push(operator);
}

fn cubic_charstring(hints: bool, cff2: bool) -> Vec<u8> {
    let mut data = Vec::new();
    if hints {
        charstring_operands(&mut data, &[0.25, 10.5], 1); // hstem
    }
    charstring_operands(&mut data, &[10.25, -0.125], 21); // rmoveto
    charstring_operands(
        &mut data,
        &[65.25, 800.25, 624.75, 50.375, 199.75, -850.5],
        8, // rrcurveto
    );
    if !cff2 {
        data.push(14);
    }
    data
}

fn name_keyed_cff(matrix: Vec<u8>, hints: bool) -> Vec<u8> {
    let names = index(&[b"PrecisionFixture".to_vec()], false);
    let make_top = |offset| {
        let mut top = matrix.clone();
        dict_integer(&mut top, offset);
        top.push(17);
        top.extend_from_slice(&[139, 139, 18]); // empty Private DICT
        index(&[top], false)
    };
    let offset = 4 + names.len() + make_top(0).len() + 4;
    let mut data = vec![1, 0, 4, 4];
    data.extend(names);
    data.extend(make_top(offset));
    data.extend_from_slice(&[0, 0, 0, 0]); // empty strings and global subrs
    data.extend(index(&[vec![14], cubic_charstring(hints, false)], false));
    data
}

fn cid_cff() -> Vec<u8> {
    let names = index(&[b"CidPrecisionFixture".to_vec()], false);
    let strings = index(&[b"Adobe".to_vec(), b"Identity".to_vec()], false);
    let make_top = |charstrings, fd_array, fd_select, charset| {
        let mut top = Vec::new();
        for value in [391, 392, 0] {
            dict_integer(&mut top, value);
        }
        top.extend_from_slice(&[12, 30]); // ROS
        top.extend(dict_matrix(["0.001", "0", "0", "0.001", "0", "0"]));
        for (offset, operator) in [
            (charstrings, &[17][..]),
            (fd_array, &[12, 36]),
            (fd_select, &[12, 37]),
            (charset, &[15]),
        ] {
            dict_integer(&mut top, offset);
            top.extend_from_slice(operator);
        }
        index(&[top], false)
    };
    let make_fd_array = |private0, private1| {
        let entries = [
            ["1", "0", "0", "1", "3", "4"],
            ["2", "0", "0.5", "1", "7", "9"],
        ]
        .into_iter()
        .zip([private0, private1])
        .map(|(matrix, offset)| {
            let mut dict = dict_matrix(matrix);
            dict_integer(&mut dict, 6);
            dict_integer(&mut dict, offset);
            dict.push(18);
            dict
        })
        .collect::<Vec<_>>();
        index(&entries, false)
    };
    let make_private = |start: f64| {
        let mut private = Vec::new();
        dict_integer(&mut private, 6);
        private.push(19); // local Subrs immediately follow Private DICT
        let mut subr = Vec::new();
        charstring_operands(&mut subr, &[start, 20.125], 21);
        charstring_operands(&mut subr, &[30.5, 40.75], 5);
        subr.push(11); // return
        private.extend(index(&[subr], false));
        private
    };
    let charstrings = index(&[vec![32, 10, 14], vec![32, 10, 14]], false);
    let prefix_len = 4 + names.len() + make_top(0, 0, 0, 0).len() + strings.len() + 2;
    let fd_array_offset = prefix_len + charstrings.len();
    let fd_select_offset = fd_array_offset + make_fd_array(0, 0).len();
    let charset_offset = fd_select_offset + 3;
    let private0_offset = charset_offset + 3;
    let private0 = make_private(10.25);
    let private1_offset = private0_offset + private0.len();
    let mut data = vec![1, 0, 4, 4];
    data.extend(names);
    data.extend(make_top(
        prefix_len,
        fd_array_offset,
        fd_select_offset,
        charset_offset,
    ));
    data.extend(strings);
    data.extend_from_slice(&[0, 0]);
    data.extend(charstrings);
    data.extend(make_fd_array(private0_offset, private1_offset));
    data.extend_from_slice(&[0, 0, 1]); // format 0 FDSelect: gid 0 -> FD 0, gid 1 -> FD 1
    data.extend_from_slice(&[0, 0, 1]); // format 0 charset: gid 1 -> CID 1
    data.extend(private0);
    data.extend(make_private(15.25));
    data
}

fn variable_cff2() -> Vec<u8> {
    let make_top = |charstrings, fd_array, var_store| {
        let mut top = Vec::new();
        for (offset, operator) in [
            (charstrings, &[17][..]),
            (fd_array, &[12, 36]),
            (var_store, &[24]),
        ] {
            dict_integer(&mut top, offset);
            top.extend_from_slice(operator);
        }
        top
    };
    let mut glyph = Vec::new();
    // At default coordinates, the positive-axis region has scalar zero.
    // Thus blend retains (10.25, 20.125), excluding the (100, 200) deltas.
    charstring_operands(&mut glyph, &[10.25, 20.125, 100.0, 200.0, 2.0], 16);
    glyph.push(21);
    charstring_operands(&mut glyph, &[30.5, 40.75], 5);
    let charstrings = index(&[Vec::new(), glyph], true);
    let fd_array = index(&[vec![139, 139, 18]], true);
    let charstrings_offset = 5 + make_top(0, 0, 0).len() + 4;
    let fd_array_offset = charstrings_offset + charstrings.len();
    let var_store_offset = fd_array_offset + fd_array.len();
    let top = make_top(charstrings_offset, fd_array_offset, var_store_offset);
    let mut data = vec![2, 0, 5];
    data.extend_from_slice(&(top.len() as u16).to_be_bytes());
    data.extend(top);
    data.extend_from_slice(&[0, 0, 0, 0]); // empty global subr index
    data.extend(charstrings);
    data.extend(fd_array);
    // OpenType ItemVariationStore: one axis, one region, one item data record.
    // Region list offset 12, item data offset 22; no rows, region index zero.
    data.extend_from_slice(&[
        0, 30, // CFF2 VariationStore length
        0, 1, 0, 0, 0, 12, 0, 1, 0, 0, 0, 22, // store header
        0, 1, 0, 1, 0, 0, 0x40, 0, 0x40, 0, // region: start 0, peak 1, end 1
        0, 0, 0, 0, 0, 1, 0, 0, // item data
    ]);
    data
}

fn import_layer(data: Vec<u8>, cff2: bool, gid: u16, upem: u16) -> (Layer, bool) {
    let fixture = std::fs::read(fc_fixtures::path("SourceSerif4Min.otf")).expect("fixture");
    let face = FontRef::new(&fixture).expect("fixture face");
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        if ![Tag::new(b"CFF "), Tag::new(b"head")].contains(&record.tag()) {
            builder.add_raw(
                record.tag(),
                face.table_data(record.tag())
                    .expect("fixture table")
                    .as_bytes(),
            );
        }
    }
    let mut head: write_fonts::tables::head::Head = face.head().expect("head").to_owned_table();
    head.units_per_em = upem;
    builder.add_table(&head).expect("write head");
    builder.add_raw(Tag::new(if cff2 { b"CFF2" } else { b"CFF " }), data);
    let sfnt = builder.build();
    let face = FontRef::new(&sfnt).expect("synthetic face");
    let source = CffSource::new(&face, false)
        .expect("CFF source")
        .expect("CFF");
    let mut layer = Layer::new_master(fc_model::MasterId::default());
    source.apply(gid, &mut layer).expect("read glyph");
    (layer, source.has_hints())
}

fn assert_point(point: &Point, x: f64, y: f64, kind: PointKind) {
    assert!((point.x - x).abs() < 1e-10, "x: {} != {x}", point.x);
    assert!((point.y - y).abs() < 1e-10, "y: {} != {y}", point.y);
    assert_eq!(point.kind, kind);
}

#[test]
fn fractional_cubic_coordinates_and_hints_survive_import() {
    for hints in [false, true] {
        let (layer, has_hints) = import_layer(name_keyed_cff(Vec::new(), hints), false, 1, 1000);
        assert_eq!(has_hints, hints);
        let contour = &layer.contours[0];
        assert!(contour.closed);
        assert_eq!(contour.points.len(), 4);
        assert_point(&contour.points[0], 10.25, -0.125, PointKind::Line);
        assert_point(&contour.points[1], 75.5, 800.125, PointKind::OffCurve);
        assert_point(&contour.points[2], 700.25, 850.5, PointKind::OffCurve);
        assert_point(&contour.points[3], 900.0, 0.0, PointKind::Curve);
    }
}

#[test]
fn fractional_coordinates_use_effective_matrix_and_head_upem() {
    let matrix = dict_matrix(["0.002", "0.00025", "0.0005", "0.001", "0.004", "-0.002"]);
    let (layer, _) = import_layer(name_keyed_cff(matrix, false), false, 1, 2000);
    // FontMatrix * head UPEM: [4, .5, 1, 2, 8, -4].
    assert_point(&layer.contours[0].points[0], 48.875, 0.875, PointKind::Line);
    assert_point(
        &layer.contours[0].points[1],
        1110.125,
        1634.0,
        PointKind::OffCurve,
    );
}

#[test]
fn cid_font_selects_each_subfont_matrix_and_local_subroutines() {
    let (layer0, _) = import_layer(cid_cff(), false, 0, 1000);
    let (layer1, _) = import_layer(cid_cff(), false, 1, 1000);
    assert_point(
        &layer0.contours[0].points[0],
        13.25,
        24.125,
        PointKind::Line,
    );
    assert_point(
        &layer0.contours[0].points[1],
        43.75,
        64.875,
        PointKind::Line,
    );
    assert_point(
        &layer1.contours[0].points[0],
        47.5625,
        29.125,
        PointKind::Line,
    );
    assert_point(
        &layer1.contours[0].points[1],
        128.9375,
        69.875,
        PointKind::Line,
    );
}

#[test]
fn cff2_blend_uses_default_variation_coordinates_without_rounding() {
    let (layer, has_hints) = import_layer(variable_cff2(), true, 1, 1000);
    assert!(!has_hints);
    assert_point(&layer.contours[0].points[0], 10.25, 20.125, PointKind::Line);
    assert_point(&layer.contours[0].points[1], 40.75, 60.875, PointKind::Line);
    assert!(layer.contours[0].closed);
}
