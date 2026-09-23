mod common;
use fc_compile::{compile_variable, CompileOptions};
use fc_model::{Axis, Instance, LayerId, LayerKind, MasterId, Tag};
use read_fonts::{FontRef, TableProvider};
use skrifa::{instance::Size, MetadataProvider};

#[derive(Default)]
struct Points(Vec<(f32, f32)>);
impl skrifa::outline::OutlinePen for Points {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push((x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push((x, y));
    }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
        self.0.extend([(a, b), (x, y)]);
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        self.0.extend([(a, b), (c, d), (x, y)]);
    }
    fn close(&mut self) {}
}

#[test]
fn cff2_export_draws_variations_and_preserves_required_tables_under_subset() {
    let font = variable_font();
    let report = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            variable: true,
            format: fc_compile::ExportFormat::Otf,
            table_subset: Some(vec![]),
            ..Default::default()
        },
    )
    .unwrap();
    if let Ok(path) = std::env::var("FC_CFF2_TEST_OUTPUT") {
        std::fs::write(path, &report.bytes).unwrap();
    }
    assert_eq!(&report.bytes[..4], b"OTTO");
    let face = FontRef::new(&report.bytes).unwrap();
    assert!(face
        .table_data(read_fonts::types::Tag::new(b"glyf"))
        .is_none());
    assert!(face
        .table_data(read_fonts::types::Tag::new(b"CFF2"))
        .is_some());
    assert!(face
        .table_data(read_fonts::types::Tag::new(b"VORG"))
        .is_some());
    let glyph = face
        .outline_glyphs()
        .get(face.charmap().map('A').unwrap())
        .unwrap();
    for (user, max_x) in [
        (100.0, 550.0),
        (400.0, 687.5),
        (650.0, 756.25),
        (900.0, 825.0),
    ] {
        let location = face.axes().location([("wght", user)]);
        let mut pen = Points::default();
        glyph
            .draw(
                skrifa::outline::DrawSettings::unhinted(Size::new(1000.0), &location),
                &mut pen,
            )
            .unwrap();
        // Skrifa's CFF drawing path quantizes fractional design coordinates;
        // the independent FontTools acceptance check retains their fractions.
        let actual = pen.0.iter().map(|p| p.0).reduce(f32::max).unwrap();
        assert!((actual - max_x).abs() < 1.0, "{user}: {actual} != {max_x}");
    }
}

fn variable_font() -> fc_model::Font {
    let mut font = common::scratch_font();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.0,
        default: 100.0,
        max: 900.0,
        map: vec![(100.0, 0.0), (400.0, 50.0), (900.0, 100.0)],
        ..Default::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 0.0);
    let mut heavy = font.masters[0].clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(Tag(*b"wght"), 100.0);
    heavy.metrics.ascender += 100.0;
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].vertical_width = Some(1000.0);
        glyph.layers[0].vertical_origin = Some(800.0);
        let mut layer = glyph.layers[0].clone();
        layer.id = LayerId::new();
        layer.kind = LayerKind::Master(heavy.id.clone());
        layer.width += 200.0;
        layer.vertical_width = Some(1200.0);
        for contour in &mut layer.contours {
            for point in &mut contour.points {
                point.x *= 1.5;
            }
        }
        glyph.layers.push(layer);
    }
    font.masters.insert(0, heavy);
    font.instances.push(Instance {
        name: "Medium".into(),
        style_name: "Medium".into(),
        location: [(Tag(*b"wght"), 50.0)].into(),
        active: true,
        is_variable_named_instance: true,
        ..Default::default()
    });
    font
}

#[test]
fn smart_components_match_model_preview_and_static_and_variable_exports() {
    let mut font = variable_font();
    let mut part = font.glyph("V").unwrap().clone();
    part.name = "_part.stem".into();
    part.unicodes.clear();
    part.smart_axes = vec![fc_model::SmartAxis {
        name: "Width".into(),
        min: 0.,
        default: 0.,
        max: 100.,
    }];
    let sources: Vec<_> = part
        .layers
        .iter()
        .map(|layer| {
            let mut source = layer.clone();
            source.id = LayerId::new();
            let LayerKind::Master(master) = &layer.kind else {
                panic!("fixture")
            };
            source.kind = LayerKind::Smart {
                master: master.clone(),
                location: [("Width".into(), 100.)].into(),
            };
            for point in source.contours.iter_mut().flat_map(|c| &mut c.points) {
                point.x *= 2.;
            }
            source
        })
        .collect();
    part.layers.extend(sources);
    font.insert_glyph(part).unwrap();
    for layer in &mut font.glyph_mut("A").unwrap().layers {
        layer.contours.clear();
        layer.components = vec![fc_model::Component {
            base: "_part.stem".into(),
            smart: Some([("Width".into(), 50.)].into()),
            ..Default::default()
        }];
    }
    use kurbo::Shape;
    let middle = fc_geom::variation::interpolated_path(&font, "A", &[(Tag(*b"wght"), 50.)].into())
        .unwrap()
        .bounding_box();
    assert_eq!(middle.x1, 1012.5);
    for format in [fc_compile::ExportFormat::Ttf, fc_compile::ExportFormat::Otf] {
        for variable in [false, true] {
            let report = fc_compile::export(
                &font,
                &|_| 0,
                &fc_compile::ExportOptions {
                    format,
                    variable,
                    instance: (!variable).then_some(0),
                    ..Default::default()
                },
            )
            .unwrap();
            let face = FontRef::new(&report.bytes).unwrap();
            let position = face.axes().location([("wght", 400.)]);
            let glyph = face
                .outline_glyphs()
                .get(face.charmap().map('A').unwrap())
                .unwrap();
            let mut pen = Points::default();
            glyph
                .draw(
                    skrifa::outline::DrawSettings::unhinted(Size::new(1000.), &position),
                    &mut pen,
                )
                .unwrap();
            let maximum = pen.0.iter().map(|p| p.0).reduce(f32::max).unwrap();
            assert!(
                (maximum - middle.x1 as f32).abs() < 1.,
                "{format:?} {variable}: {maximum}"
            );
        }
    }
}

#[test]
fn named_static_instance_resolves_mapping_names_outlines_and_metrics() {
    let mut font = variable_font();
    font.instances[0].family_name = Some("Instance Family".into());
    let resolved =
        fc_compile::instantiate(&font, &font.instances[0].location, Some(&font.instances[0]))
            .unwrap();
    assert_eq!(resolved.info.family_name, "Instance Family");
    assert_eq!(resolved.info.style_name, "Medium");
    assert_eq!(
        resolved.default_master().metrics.ascender,
        font.design_default_master().unwrap().metrics.ascender + 50.
    );
    assert_eq!(resolved.glyph("A").unwrap().layers[0].width, 700.);
    for format in [fc_compile::ExportFormat::Ttf, fc_compile::ExportFormat::Otf] {
        let report = fc_compile::export(
            &font,
            &|_| 0,
            &fc_compile::ExportOptions {
                instance: Some(0),
                format,
                ..Default::default()
            },
        )
        .unwrap();
        let face = FontRef::new(&report.bytes).unwrap();
        assert!(face.fvar().is_err());
        let gid = face.charmap().map('A').unwrap();
        assert_eq!(
            face.glyph_metrics(Size::unscaled(), skrifa::instance::LocationRef::default())
                .advance_width(gid),
            Some(700.)
        );
        assert_eq!(
            face.vmtx().unwrap().v_metrics()
                [(gid.to_u32() as usize).min(face.vmtx().unwrap().v_metrics().len() - 1)]
            .advance(),
            1100
        );
        assert_eq!(face.os2().unwrap().us_weight_class(), 400);
    }
    assert!(fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            instance: Some(0),
            variable: true,
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn bracket_layers_substitute_real_glyphs_at_the_axis_threshold() {
    let mut font = variable_font();
    let glyph = font.glyphs.get_mut("A").unwrap();
    let alternates: Vec<_> = glyph
        .layers
        .iter()
        .map(|source| {
            let mut layer = source.clone();
            let LayerKind::Master(master) = &source.kind else {
                panic!("fixture master")
            };
            layer.id = LayerId::new();
            layer.kind = LayerKind::Bracket {
                master: master.clone(),
                axis: Tag(*b"wght"),
                min: Some(50.0),
                max: None,
            };
            layer.width += 100.0;
            layer
        })
        .collect();
    glyph.layers.extend(alternates);
    for format in [fc_compile::ExportFormat::Ttf, fc_compile::ExportFormat::Otf] {
        let report = fc_compile::export(
            &font,
            &|_| 0,
            &fc_compile::ExportOptions {
                variable: true,
                format,
                ..Default::default()
            },
        )
        .unwrap();
        let face = FontRef::new(&report.bytes).unwrap();
        let data = harfrust::ShaperData::new(&face);
        for (weight, expected) in [(100.0, 600), (400.0, 800), (900.0, 900)] {
            let instance = harfrust::ShaperInstance::from_variations(&face, [("wght", weight)]);
            let shaper = data.shaper(&face).instance(Some(&instance)).build();
            let mut buffer = harfrust::UnicodeBuffer::new();
            buffer.push_str("A");
            buffer.guess_segment_properties();
            let result = shaper.shape(buffer, harfrust::ShapeOptions::new());
            assert_eq!(
                result.glyph_positions()[0].x_advance,
                expected,
                "weight {weight}"
            );
        }
    }
    let preview = fc_geom::variation::interpolate_glyph(
        &font,
        font.glyph("A").unwrap(),
        &[(Tag(*b"wght"), 50.0)].into(),
    )
    .unwrap();
    assert_eq!(preview.width, 800.0);
}

#[test]
fn variable_export_has_real_axes_instances_mapping_and_interpolated_metrics() {
    let font = variable_font();
    let compiled = compile_variable(&font, &CompileOptions::default()).unwrap();
    let face = FontRef::new(&compiled.bytes).unwrap();
    for tag in [b"gvar", b"fvar", b"avar", b"HVAR", b"VVAR", b"MVAR"] {
        assert!(
            face.table_data(read_fonts::types::Tag::new(tag)).is_some(),
            "missing {}",
            String::from_utf8_lossy(tag)
        );
    }
    assert_eq!(face.fvar().unwrap().instance_count(), 1);
    let gid = face.charmap().map('A').unwrap();
    for (user, expected) in [
        (100.0, 600.0),
        (400.0, 700.0),
        (650.0, 750.0),
        (900.0, 800.0),
    ] {
        let location = face.axes().location([("wght", user)]);
        assert!(
            (face
                .glyph_metrics(Size::unscaled(), &location)
                .advance_width(gid)
                .unwrap()
                - expected)
                .abs()
                < 0.1
        );
    }
    if let Ok(path) = std::env::var("FC_VARIABLE_TEST_OUTPUT") {
        std::fs::write(path, &compiled.bytes).unwrap();
    }
}

#[test]
fn incompatible_or_missing_default_sources_fail_before_compilation() {
    let mut font = variable_font();
    font.glyphs.get_mut("A").unwrap().layers[0].contours.clear();
    assert!(compile_variable(&font, &CompileOptions::default()).is_err());
    let mut font = variable_font();
    font.glyphs.get_mut("A").unwrap().layers.remove(0);
    assert!(compile_variable(&font, &CompileOptions::default()).is_err());
}

#[test]
fn sparse_glyph_and_intermediate_source_compile() {
    let mut font = variable_font();
    font.glyphs.get_mut("space").unwrap().layers.pop();
    let glyph = font.glyphs.get_mut("A").unwrap();
    let mut layer = glyph.layers[0].clone();
    layer.id = LayerId::new();
    layer.kind = LayerKind::Brace {
        master: font.masters[1].id.clone(),
        location: [(Tag(*b"wght"), 50.0)].into(),
    };
    layer.width = 760.0;
    glyph.layers.push(layer);
    let compiled = compile_variable(&font, &CompileOptions::default()).unwrap();
    let face = FontRef::new(&compiled.bytes).unwrap();
    let location = face.axes().location([("wght", 400.0)]);
    assert_eq!(
        face.glyph_metrics(Size::unscaled(), &location)
            .advance_width(face.charmap().map('A').unwrap()),
        Some(760.0)
    );
    assert_eq!(
        face.glyph_metrics(Size::unscaled(), &location)
            .advance_width(face.charmap().map(' ').unwrap()),
        Some(250.0)
    );
}

#[test]
fn vertical_kerning_interpolates_mapped_locations_and_group_exceptions() {
    let mut font = variable_font();
    font.kerning_groups
        .side1
        .insert("public.kern1.Top".into(), ["A".into(), "V".into()].into());
    font.kerning_groups.side2.insert(
        "public.kern2.Bottom".into(),
        ["A".into(), "V".into()].into(),
    );
    for master in &mut font.masters {
        let heavy = master.location[&Tag(*b"wght")] == 100.;
        master.kerning_v.0.insert(
            (
                fc_model::KernSide::Group("public.kern1.Top".into()),
                fc_model::KernSide::Group("public.kern2.Bottom".into()),
            ),
            if heavy { -120. } else { -40. },
        );
        master.kerning_v.0.insert(
            (
                fc_model::KernSide::Glyph("V".into()),
                fc_model::KernSide::Glyph("A".into()),
            ),
            0.,
        );
    }
    for format in [fc_compile::ExportFormat::Ttf, fc_compile::ExportFormat::Otf] {
        let report = fc_compile::export(
            &font,
            &|_| 0,
            &fc_compile::ExportOptions {
                format,
                variable: true,
                ..Default::default()
            },
        )
        .unwrap();
        if let Ok(path) = std::env::var("FC_VERTICAL_TEST_OUTPUT") {
            std::fs::write(path, &report.bytes).unwrap();
        }
        let face = FontRef::new(&report.bytes).unwrap();
        let data = harfrust::ShaperData::new(&face);
        for (weight, height, kern) in [(100., 1000, 40), (400., 1100, 80), (900., 1200, 120)] {
            let instance = harfrust::ShaperInstance::from_variations(&face, [("wght", weight)]);
            let shaper = data.shaper(&face).instance(Some(&instance)).build();
            let mut buffer = harfrust::UnicodeBuffer::new();
            buffer.push_str("AVA");
            buffer.set_direction(harfrust::Direction::TopToBottom);
            buffer.guess_segment_properties();
            let shaped = shaper.shape(
                buffer,
                harfrust::ShapeOptions::new().features(&["vkrn=1".parse().unwrap()]),
            );
            assert_eq!(
                shaped
                    .glyph_positions()
                    .iter()
                    .map(|p| p.y_advance)
                    .collect::<Vec<_>>(),
                [-height + kern, -height, -height],
                "{format:?} {weight}"
            );
            assert!(shaped.glyph_positions().iter().all(|p| p.x_advance == 0));
        }
    }
}

#[test]
fn master_vertical_defaults_create_metrics_and_manual_vkrn_retains_ownership() {
    let mut font = common::scratch_font();
    font.masters[0].metrics.default_vertical_advance = Some(1100.);
    font.masters[0].metrics.default_vertical_origin = Some(900.);
    font.masters[0].kerning_v.0.insert(
        (
            fc_model::KernSide::Glyph("A".into()),
            fc_model::KernSide::Glyph("V".into()),
        ),
        -70.,
    );
    font.features.prefixes.push(fc_model::FeatureBlock {
        tag: "Languages".into(),
        code: "languagesystem DFLT dflt; languagesystem latn dflt;".into(),
        ..Default::default()
    });
    font.features.features.push(fc_model::FeatureBlock {
        tag: "vkrn".into(),
        code: "pos A V <0 0 0 25>;".into(),
        ..Default::default()
    });
    for format in [fc_compile::ExportFormat::Ttf, fc_compile::ExportFormat::Otf] {
        let report = fc_compile::export(
            &font,
            &|_| 0,
            &fc_compile::ExportOptions {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        if let Ok(path) = std::env::var("FC_VERTICAL_TEST_OUTPUT") {
            std::fs::write(path, &report.bytes).unwrap();
        }
        let face = FontRef::new(&report.bytes).unwrap();
        assert!(face.vhea().is_ok());
        let data = harfrust::ShaperData::new(&face);
        let shaper = data.shaper(&face).build();
        let mut buffer = harfrust::UnicodeBuffer::new();
        buffer.push_str("AV");
        buffer.set_direction(harfrust::Direction::TopToBottom);
        buffer.guess_segment_properties();
        let shaped = shaper.shape(
            buffer,
            harfrust::ShapeOptions::new().features(&["vkrn=1".parse().unwrap()]),
        );
        assert_eq!(shaped.glyph_positions()[0].y_advance, -1125);
        assert_eq!(shaped.glyph_positions()[0].y_offset, -900);
    }
}

#[test]
fn static_instance_uses_master_defaults_for_mixed_optional_metrics() {
    let mut font = variable_font();
    font.masters[0].metrics.ascender = 800.;
    font.masters[0].metrics.descender = -200.;
    font.masters[0].metrics.default_vertical_advance = None;
    font.masters[0].metrics.default_vertical_origin = None;
    font.masters[0].metrics.hhea_ascender = None;
    font.masters[1].metrics.default_vertical_advance = Some(1200.);
    font.masters[1].metrics.default_vertical_origin = Some(900.);
    font.masters[1].metrics.hhea_ascender = Some(1400.);
    let result = fc_compile::instantiate(&font, &[(Tag(*b"wght"), 50.)].into(), None).unwrap();
    assert_eq!(
        result.masters[0].metrics.default_vertical_advance,
        Some(1100.)
    );
    assert_eq!(
        result.masters[0].metrics.default_vertical_origin,
        Some(850.)
    );
    assert_eq!(result.masters[0].metrics.hhea_ascender, Some(1200.));
    assert!(fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            master_subset: Some(vec![font.masters[0].id.clone()]),
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn vertical_writer_accepts_reserved_glyph_names_and_rejects_ambiguous_groups() {
    let mut font = common::scratch_font();
    font.rename_glyph("A", "base").unwrap();
    font.masters[0].kerning_v.0.insert(
        (
            fc_model::KernSide::Glyph("base".into()),
            fc_model::KernSide::Glyph("V".into()),
        ),
        -50.,
    );
    fc_compile::export(&font, &|_| 0, &Default::default()).unwrap();
    font.kerning_groups
        .side1
        .insert("public.kern1.a".into(), ["base".into()].into());
    font.kerning_groups
        .side1
        .insert("public.kern1.b".into(), ["base".into()].into());
    assert!(fc_compile::export(&font, &|_| 0, &Default::default()).is_err());
}

#[test]
fn static_instance_updates_registered_width_and_style_metadata() {
    let mut font = variable_font();
    for (tag, min, default, max, at) in [
        (*b"wdth", 50., 100., 200., 75.),
        (*b"ital", 0., 0., 1., 1.),
        (*b"slnt", -20., 0., 0., -12.),
    ] {
        font.axes.push(Axis {
            tag: Tag(tag),
            name: String::from_utf8(tag.to_vec()).unwrap(),
            min,
            default,
            max,
            ..Default::default()
        });
        for master in &mut font.masters {
            master.location.insert(Tag(tag), default);
        }
        // A sparse axis can still drive static instance metadata.
        font.instances[0].location.insert(Tag(tag), at);
    }
    font.info.open_type_os2_selection = vec![6, 7];
    font.instances[0].location.insert(Tag(*b"wght"), 100.);
    let result =
        fc_compile::instantiate(&font, &font.instances[0].location, Some(&font.instances[0]))
            .unwrap();
    assert_eq!(result.info.open_type_os2_width_class, Some(3));
    assert_eq!(result.info.open_type_os2_weight_class, Some(900));
    assert_eq!(result.info.open_type_os2_selection, [0, 5, 7]);
    assert_eq!(result.default_master().metrics.italic_angle, -12.);
}

#[test]
fn overlapping_conditional_component_dependencies_fail_instead_of_losing_one_alternate() {
    let mut font = variable_font();
    for (name, min) in [("A", 40.), ("V", 60.)] {
        let sources: Vec<_> = font
            .glyph(name)
            .unwrap()
            .layers
            .iter()
            .map(|source| {
                let LayerKind::Master(master) = &source.kind else {
                    panic!("fixture source");
                };
                let mut layer = source.clone();
                layer.id = LayerId::new();
                layer.kind = LayerKind::Bracket {
                    master: master.clone(),
                    axis: Tag(*b"wght"),
                    min: Some(min),
                    max: None,
                };
                layer
            })
            .collect();
        font.glyph_mut(name).unwrap().layers.extend(sources);
    }
    let mut glyph = fc_model::Glyph::new("combined");
    for master in &font.masters {
        let mut layer = fc_model::Layer::new_master(master.id.clone());
        for base in ["A", "V"] {
            layer.components.push(fc_model::Component {
                base: base.into(),
                ..Default::default()
            });
        }
        glyph.layers.push(layer);
    }
    font.insert_glyph(glyph).unwrap();
    let error = compile_variable(&font, &CompileOptions::default()).unwrap_err();
    assert!(error
        .to_string()
        .contains("conditional component dependencies"));
}

#[test]
fn variable_ttf_and_cff2_exports_write_editable_math() {
    use fc_model::{MathConstruction, MathGlyphInfo, MathTable, MathValue, MathVariant};
    let mut font = variable_font();
    let mut math = MathTable {
        min_connector_overlap: 12.0,
        ..Default::default()
    };
    math.glyphs.insert(
        "A".into(),
        MathGlyphInfo {
            italic_correction: Some(MathValue::from(15.0)),
            vertical: Some(MathConstruction {
                variants: vec![MathVariant {
                    glyph: "V".into(),
                    advance: 800.0,
                }],
                assembly: None,
            }),
            ..Default::default()
        },
    );
    font.math = Some(math.clone());
    font.math_table_owned = true;
    for format in [fc_compile::ExportFormat::Ttf, fc_compile::ExportFormat::Otf] {
        let report = fc_compile::export(
            &font,
            &|_| 0,
            &fc_compile::ExportOptions {
                variable: true,
                format,
                ..Default::default()
            },
        )
        .unwrap();
        let output = fc_sfnt::read_bytes(&report.bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        // CFF2 output uses post 3.0, so names are synthesized; compare by ID.
        let mut decoded = output.math.clone().expect("MATH");
        let names: Vec<_> = output.glyph_order().cloned().collect();
        for (name, source) in names.iter().zip(font.glyph_order()) {
            if name != source {
                decoded.rename_glyph(name, source);
            }
        }
        assert_eq!(decoded, math, "{format:?}");
    }
}
