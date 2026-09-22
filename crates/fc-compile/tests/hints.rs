mod common;

use fc_compile::{export, ExportFormat, ExportOptions};
use fc_model::{Contour, HintMask, PsHints, StemHint};
use kurbo::{Rect, Shape};

#[test]
fn cff_hints_private_values_masks_and_ghosts_survive_binary_round_trip() {
    let mut font = common::scratch_font();
    let master = font.default_master().id.clone();
    let layer = font
        .glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap();
    layer.contours = Contour::from_bezpath(&Rect::new(50.0, 0.0, 130.0, 700.0).to_path(0.1));
    layer.hints = PsHints {
        h_stems: vec![StemHint {
            start: 700.0,
            width: -20.0,
            ranges: vec![],
        }],
        v_stems: vec![
            StemHint {
                start: 50.0,
                width: 80.0,
                ranges: vec![],
            },
            StemHint {
                start: 171.0,
                width: -21.0,
                ranges: vec![],
            },
        ],
        hint_masks: vec![
            HintMask {
                counter: false,
                closing: false,
                contour: 0,
                point: 0,
                mask: vec![0xe0],
            },
            HintMask {
                counter: false,
                closing: false,
                contour: 0,
                point: 2,
                mask: vec![0x60],
            },
        ],
        ..Default::default()
    };
    font.masters[0].private_dict.blue_values = vec![-10.0, 0.0, 700.0, 710.0];
    font.masters[0].private_dict.std_vw = Some(80.0);
    let before = font.clone();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(font, before);
    let imported = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert_eq!(
        imported.default_master().private_dict.blue_values,
        font.default_master().private_dict.blue_values
    );
    assert_eq!(imported.default_master().private_dict.std_vw, Some(80.0));
    let hints = &imported.glyph("A").unwrap().layers[0].hints;
    assert_eq!(hints, &font.glyph("A").unwrap().layers[0].hints);
}

#[test]
fn autohint_and_remove_overlap_work_on_export_copies() {
    let mut font = common::scratch_font();
    let layer = &mut font.glyph_mut("A").unwrap().layers[0];
    layer.contours = Contour::from_bezpath(&Rect::new(50.0, 0.0, 130.0, 700.0).to_path(0.1));
    layer.contours.extend(Contour::from_bezpath(
        &Rect::new(100.0, 0.0, 180.0, 700.0).to_path(0.1),
    ));
    let before = font.clone();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            remove_overlap: true,
            autohint: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(font, before);
    let imported = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let layer = &imported.glyph("A").unwrap().layers[0];
    assert_eq!(layer.contours.len(), 1);
    assert_eq!(
        layer
            .hints
            .v_stems
            .iter()
            .map(|s| (s.start, s.width))
            .collect::<Vec<_>>(),
        [(50.0, 130.0)]
    );
    assert!(report.warnings.iter().all(|w| ![
        "autohint-unavailable",
        "remove-overlap-unavailable"
    ]
    .contains(&w.code)));
}

#[test]
fn large_stem_sets_are_split_without_overflowing_type2_stack_or_shifting_positions() {
    let mut font = common::scratch_font();
    let hints = &mut font.glyph_mut("A").unwrap().layers[0].hints;
    hints.v_stems = (0..48)
        .map(|index| StemHint {
            start: f64::from(index * 100),
            width: 20.0,
            ranges: vec![],
        })
        .collect();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            ..Default::default()
        },
    )
    .unwrap();
    let imported = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    assert_eq!(
        imported.glyph("A").unwrap().layers[0].hints.v_stems,
        font.glyph("A").unwrap().layers[0].hints.v_stems
    );
}

#[test]
fn counter_masks_and_masks_before_closing_lines_and_curves_keep_their_locations() {
    for curved in [false, true] {
        let mut font = common::scratch_font();
        let layer = &mut font.glyph_mut("A").unwrap().layers[0];
        let mut path = Rect::new(50.0, 0.0, 130.0, 700.0).to_path(0.1);
        if curved {
            path = kurbo::BezPath::from_svg("M50 0L130 0L130 700C80 700 50 200 50 0Z").unwrap();
        }
        layer.contours = Contour::from_bezpath(&path);
        layer.hints.v_stems = vec![StemHint {
            start: 50.0,
            width: 80.0,
            ranges: vec![],
        }];
        layer.hints.hint_masks = vec![
            HintMask {
                contour: 0,
                point: 0,
                mask: vec![0x80],
                ..Default::default()
            },
            HintMask {
                contour: 0,
                point: 1,
                counter: true,
                mask: vec![0x80],
                ..Default::default()
            },
            HintMask {
                contour: 0,
                point: 0,
                closing: true,
                mask: vec![0],
                ..Default::default()
            },
        ];
        let expected = layer.hints.clone();
        let report = export(
            &font,
            &|_| 0,
            &ExportOptions {
                format: ExportFormat::Otf,
                ..Default::default()
            },
        )
        .unwrap();
        let imported = fc_sfnt::read_bytes(&report.bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        assert_eq!(
            imported.glyph("A").unwrap().layers[0].hints,
            expected,
            "curved={curved}"
        );
    }
}

#[test]
fn overlapping_hints_require_valid_active_sets_from_the_first_drawing_operator() {
    let mut font = common::scratch_font();
    let layer = &mut font.glyph_mut("A").unwrap().layers[0];
    layer.contours = Contour::from_bezpath(&Rect::new(0.0, 0.0, 200.0, 700.0).to_path(0.1));
    layer.hints.v_stems = [0.0, 40.0]
        .map(|start| StemHint {
            start,
            width: 80.0,
            ranges: vec![],
        })
        .into();
    let options = ExportOptions {
        format: ExportFormat::Otf,
        ..Default::default()
    };
    assert!(export(&font, &|_| 0, &options).is_err());
    font.glyph_mut("A").unwrap().layers[0].hints.hint_masks = vec![
        HintMask {
            contour: 0,
            point: 0,
            mask: vec![0x80],
            ..Default::default()
        },
        HintMask {
            contour: 0,
            point: 2,
            mask: vec![0x40],
            ..Default::default()
        },
    ];
    export(&font, &|_| 0, &options).unwrap();
    font.glyph_mut("A").unwrap().layers[0].hints.hint_masks[1].mask[0] = 0xc0;
    assert!(export(&font, &|_| 0, &options).is_err());
}
