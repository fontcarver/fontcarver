mod common;

use fc_compile::{export, prepare_geometry_export, ExportFormat, ExportOptions};
use fc_model::{Component, Contour, Font, Glyph, HintMask, Layer, PointKind};
use kurbo::{Affine, BezPath, Shape};

fn curved_font() -> Font {
    let mut font = common::scratch_font();
    let layer = &mut font.glyph_mut("A").unwrap().layers[0];
    layer.contours = Contour::from_bezpath(
        &BezPath::from_svg("M0.25 0.25 C0.25 100.25 100.25 100.25 100.25 0.25 Z").unwrap(),
    );
    layer.tt_instructions = Some(vec![0]);
    layer.tt_binding = Some(fc_model::TtInstructionBinding {
        fingerprint: "source-point-order".into(),
        authored: false,
    });
    layer.hints.hint_masks.push(HintMask::default());
    font
}

fn composite_font() -> Font {
    let mut font = curved_font();
    let mut glyph = Glyph::new("composite");
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 600.0;
    layer.components.push(Component {
        base: "A".into(),
        transform: Affine::translate((150.25, 200.75)),
        ..Default::default()
    });
    layer.tt_instructions = Some(vec![0]);
    layer.hints.hint_masks.push(HintMask::default());
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

#[test]
fn extrema_insert_real_curve_endpoints_without_changing_source_or_other_layers() {
    let mut font = curved_font();
    let master = font.default_master().id.clone();
    let mut background = font.glyph("A").unwrap().layers[0].clone();
    background.kind = fc_model::LayerKind::Background(master.clone());
    font.glyph_mut("A").unwrap().layers.push(background);
    let before = font.clone();
    let prepared = prepare_geometry_export(
        &font,
        &master,
        &ExportOptions {
            add_extrema: true,
            ..Default::default()
        },
    )
    .unwrap();
    let old = &before.glyph("A").unwrap().layers[0];
    let new = &prepared.glyph("A").unwrap().layers[0];
    assert!(new.contours[0].points.len() > old.contours[0].points.len());
    assert!(new.contours[0].points.iter().any(|point| {
        point.kind != PointKind::OffCurve
            && (point.x - 50.25).abs() < 1e-8
            && (point.y - 75.25).abs() < 1e-8
    }));
    assert_eq!(
        new.contours[0].to_bezpath().bounding_box(),
        old.contours[0].to_bezpath().bounding_box()
    );
    assert!(new.tt_instructions.is_none());
    assert!(new.tt_binding.is_none());
    assert!(new.hints.is_empty());
    assert_eq!(
        prepared.glyph("A").unwrap().layers[1],
        before.glyph("A").unwrap().layers[1]
    );
    assert_eq!(prepared.glyph("V"), before.glyph("V"));
    assert_eq!(font, before);
}

#[test]
fn rounding_handles_and_component_offsets_clears_dependent_hints_on_copy() {
    let font = composite_font();
    let before = font.clone();
    let prepared = prepare_geometry_export(
        &font,
        &font.default_master().id,
        &ExportOptions {
            round_coordinates: true,
            ..Default::default()
        },
    )
    .unwrap();
    for point in &prepared.glyph("A").unwrap().layers[0].contours[0].points {
        assert_eq!(point.x.fract(), 0.0);
        assert_eq!(point.y.fract(), 0.0);
    }
    let composite = &prepared.glyph("composite").unwrap().layers[0];
    assert_eq!(
        composite.components[0].transform.as_coeffs(),
        [1.0, 0.0, 0.0, 1.0, 150.0, 201.0]
    );
    assert!(composite.tt_instructions.is_none());
    assert!(composite.hints.is_empty());
    assert_eq!(font, before);
}

#[test]
fn hidden_component_bases_are_prepared_and_unchanged_layers_keep_hints() {
    let mut font = composite_font();
    font.glyph_mut("A").unwrap().export = false;
    let composite = &mut font.glyph_mut("composite").unwrap().layers[0];
    composite.components[0].transform = Affine::IDENTITY;
    let untouched = &mut font.glyph_mut("V").unwrap().layers[0];
    untouched.tt_instructions = Some(vec![0]);
    untouched.hints.hint_masks.push(HintMask::default());
    let before = font.clone();
    let prepared = prepare_geometry_export(
        &font,
        &font.default_master().id,
        &ExportOptions {
            round_coordinates: true,
            ..Default::default()
        },
    )
    .unwrap();
    let base = &prepared.glyph("A").unwrap().layers[0];
    assert_eq!(base.contours[0].points[0].x, 0.0);
    let composite = &prepared.glyph("composite").unwrap().layers[0];
    assert!(composite.tt_instructions.is_none());
    assert!(composite.hints.is_empty());
    assert_eq!(prepared.glyph("V"), font.glyph("V"));
    assert_eq!(font, before);
}

#[test]
fn invalid_geometry_fails_without_mutating_source() {
    let mut font = composite_font();
    font.glyph_mut("composite").unwrap().layers[0].components[0].base = "composite".into();
    let before = font.clone();
    let result = prepare_geometry_export(
        &font,
        &font.default_master().id,
        &ExportOptions {
            decompose_components: true,
            ..Default::default()
        },
    );
    assert!(result.is_err());
    assert_eq!(font, before);
}

#[test]
fn decomposition_resolves_nested_components_before_extrema_and_rounding() {
    let mut font = composite_font();
    let master = font.default_master().id.clone();
    let mut nested = Glyph::new("nested");
    let mut layer = Layer::new_master(master.clone());
    layer.width = 600.0;
    layer.components.push(Component {
        base: "composite".into(),
        transform: Affine::translate((10.0, 20.0)),
        ..Default::default()
    });
    nested.layers.push(layer);
    font.insert_glyph(nested).unwrap();
    let before = font.clone();
    let started = std::time::Instant::now();
    let prepared = prepare_geometry_export(
        &font,
        &master,
        &ExportOptions {
            add_extrema: true,
            round_coordinates: true,
            decompose_components: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    let nested = &prepared.glyph("nested").unwrap().layers[0];
    assert!(nested.components.is_empty());
    assert!(nested.contours[0]
        .points
        .iter()
        .any(|point| point.kind != PointKind::OffCurve && point.x == 211.0 && point.y == 296.0));
    assert_eq!(font, before);
}

#[test]
fn unsupported_variable_geometry_and_subroutinization_fail_explicitly() {
    let font = common::scratch_font();
    for options in [
        ExportOptions {
            add_extrema: true,
            variable: true,
            ..Default::default()
        },
        ExportOptions {
            round_coordinates: true,
            variable: true,
            ..Default::default()
        },
        ExportOptions {
            decompose_components: true,
            variable: true,
            ..Default::default()
        },
    ] {
        let error = export(&font, &|_| 0, &options).unwrap_err().to_string();
        assert!(error.contains("static"), "{error}");
    }
    let options = ExportOptions {
        subroutinize: true,
        ..Default::default()
    };
    let error = export(&font, &|_| 0, &options).unwrap_err().to_string();
    assert!(error.contains("subroutin"), "{error}");
    assert!(prepare_geometry_export(&font, &font.default_master().id, &options).is_err());
}

#[test]
fn compiled_otf_contains_added_extrema_and_rounded_coordinates() {
    let font = curved_font();
    let before = font.clone();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            add_extrema: true,
            round_coordinates: true,
            ..Default::default()
        },
    )
    .unwrap();
    let imported = fc_sfnt::read_bytes(&report.bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let points = &imported.glyph("A").unwrap().layers[0].contours[0].points;
    assert!(points
        .iter()
        .any(|point| point.kind != PointKind::OffCurve && point.x == 50.0 && point.y == 75.0));
    assert!(points
        .iter()
        .all(|point| point.x.fract() == 0.0 && point.y.fract() == 0.0));
    assert_eq!(font, before);
}

#[test]
fn compiled_ttf_and_webfonts_honor_component_decomposition() {
    let font = composite_font();
    let before = font.clone();
    for format in [ExportFormat::Ttf, ExportFormat::Woff, ExportFormat::Woff2] {
        let report = export(
            &font,
            &|_| 0,
            &ExportOptions {
                format,
                decompose_components: true,
                ..Default::default()
            },
        )
        .unwrap();
        let imported = fc_sfnt::read_bytes(&report.bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let composite = &imported.glyph("composite").unwrap().layers[0];
        assert!(composite.components.is_empty(), "{format:?}");
        assert!(!composite.contours.is_empty(), "{format:?}");
    }
    assert_eq!(font, before);
}
