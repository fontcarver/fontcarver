//! Variable-font export configurations: master, axis and named-instance subsets.

mod common;

use fc_compile::{export, ExportFormat, ExportOptions};
use fc_model::{Axis, Font, Instance, LayerId, LayerKind, MasterId, Tag};
use read_fonts::{FontRef, TableProvider};

/// Three weights and two widths, so a subset can collapse one axis completely.
fn two_axis_font() -> Font {
    let mut font = common::scratch_font();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.0,
        default: 400.0,
        max: 900.0,
        map: vec![(100.0, 0.0), (400.0, 40.0), (900.0, 100.0)],
        ..Default::default()
    });
    font.axes.push(Axis {
        tag: Tag(*b"wdth"),
        name: "Width".into(),
        min: 75.0,
        default: 100.0,
        max: 125.0,
        map: Vec::new(),
        ..Default::default()
    });
    let regular = font.masters[0].id.clone();
    font.masters[0].name = "Regular".into();
    font.masters[0].location.insert(Tag(*b"wght"), 40.0);
    font.masters[0].location.insert(Tag(*b"wdth"), 100.0);
    for (name, wght, wdth, factor) in [
        ("Light", 0.0, 100.0, 0.8),
        ("Bold", 100.0, 100.0, 1.4),
        ("Condensed", 40.0, 75.0, 0.7),
    ] {
        let mut master = font.masters[0].clone();
        master.id = MasterId::new();
        master.name = name.into();
        master.location.insert(Tag(*b"wght"), wght);
        master.location.insert(Tag(*b"wdth"), wdth);
        for glyph in font.glyphs.values_mut() {
            let mut layer = glyph.master_layer(&regular).expect("regular layer").clone();
            layer.id = LayerId::new();
            layer.kind = LayerKind::Master(master.id.clone());
            for contour in &mut layer.contours {
                for point in &mut contour.points {
                    point.x *= factor;
                }
            }
            glyph.layers.push(layer);
        }
        font.masters.push(master);
    }
    // Instance locations are design coordinates, like master locations.
    for (name, wght, wdth) in [
        ("Light", 0.0, 100.0),
        ("Regular", 40.0, 100.0),
        ("Bold", 100.0, 100.0),
        ("Condensed", 40.0, 75.0),
    ] {
        font.instances.push(Instance {
            name: name.into(),
            style_name: name.into(),
            location: [(Tag(*b"wght"), wght), (Tag(*b"wdth"), wdth)]
                .into_iter()
                .collect(),
            active: true,
            is_variable_named_instance: true,
            ..Default::default()
        });
    }
    font
}

fn master_ids(font: &Font, names: &[&str]) -> Vec<MasterId> {
    names
        .iter()
        .map(|name| {
            font.masters
                .iter()
                .find(|master| master.name == *name)
                .unwrap_or_else(|| panic!("master {name}"))
                .id
                .clone()
        })
        .collect()
}

fn variable(options: ExportOptions) -> ExportOptions {
    ExportOptions {
        variable: true,
        format: ExportFormat::Ttf,
        ..options
    }
}

fn axis_ranges(bytes: &[u8]) -> Vec<(String, f32, f32, f32)> {
    let face = FontRef::new(bytes).expect("compiled font");
    face.fvar()
        .expect("fvar")
        .axes()
        .expect("axes")
        .iter()
        .map(|axis| {
            (
                axis.axis_tag().to_string(),
                axis.min_value().to_f32(),
                axis.default_value().to_f32(),
                axis.max_value().to_f32(),
            )
        })
        .collect()
}

fn instance_count(bytes: &[u8]) -> usize {
    let face = FontRef::new(bytes).expect("compiled font");
    face.fvar()
        .expect("fvar")
        .instances()
        .expect("instances")
        .iter()
        .count()
}

#[test]
fn a_master_subset_collapses_an_axis_that_no_longer_varies() {
    let font = two_axis_font();
    let subset = master_ids(&font, &["Regular", "Light", "Bold"]);
    let report = export(
        &font,
        &|_| 0,
        &variable(ExportOptions {
            master_subset: Some(subset),
            ..Default::default()
        }),
    )
    .expect("variable export");
    let axes = axis_ranges(&report.bytes);
    assert_eq!(
        axes,
        vec![("wght".to_owned(), 100.0, 400.0, 900.0)],
        "wdth has no remaining variation and must be dropped"
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "designspace-axis-dropped"),
        "{:?}",
        report.warnings
    );
    // The Condensed named instance no longer exists in the subset design space.
    assert_eq!(instance_count(&report.bytes), 3);
}

#[test]
fn a_master_subset_clamps_the_retained_axis_range() {
    let font = two_axis_font();
    let subset = master_ids(&font, &["Regular", "Light"]);
    let report = export(
        &font,
        &|_| 0,
        &variable(ExportOptions {
            master_subset: Some(subset),
            ..Default::default()
        }),
    )
    .expect("variable export");
    assert_eq!(
        axis_ranges(&report.bytes),
        vec![("wght".to_owned(), 100.0, 400.0, 400.0)]
    );
    assert_eq!(
        instance_count(&report.bytes),
        2,
        "Bold and Condensed fall outside the retained range"
    );
}

#[test]
fn an_explicit_axis_subset_drops_the_axis_and_its_off_default_masters() {
    let font = two_axis_font();
    let report = export(
        &font,
        &|_| 0,
        &variable(ExportOptions {
            axis_subset: Some(vec![Tag(*b"wght")]),
            ..Default::default()
        }),
    )
    .expect("variable export");
    assert_eq!(
        axis_ranges(&report.bytes),
        vec![("wght".to_owned(), 100.0, 400.0, 900.0)]
    );
    assert_eq!(instance_count(&report.bytes), 3);
}

#[test]
fn a_named_instance_subset_keeps_only_the_listed_instances() {
    let font = two_axis_font();
    let report = export(
        &font,
        &|_| 0,
        &variable(ExportOptions {
            instance_subset: Some(vec!["Light".to_owned(), "Bold".to_owned()]),
            ..Default::default()
        }),
    )
    .expect("variable export");
    assert_eq!(instance_count(&report.bytes), 2);
    assert_eq!(axis_ranges(&report.bytes).len(), 2);
}

#[test]
fn an_unknown_subset_entry_is_rejected_instead_of_ignored() {
    let font = two_axis_font();
    let error = export(
        &font,
        &|_| 0,
        &variable(ExportOptions {
            instance_subset: Some(vec!["Nonexistent".to_owned()]),
            ..Default::default()
        }),
    )
    .expect_err("unknown instance name");
    assert!(error.to_string().contains("Nonexistent"), "{error}");
    let error = export(
        &font,
        &|_| 0,
        &variable(ExportOptions {
            axis_subset: Some(vec![Tag(*b"ital")]),
            ..Default::default()
        }),
    )
    .expect_err("unknown axis tag");
    assert!(error.to_string().contains("ital"), "{error}");
}

#[test]
fn a_subset_without_a_varying_axis_is_refused_with_a_clear_message() {
    let font = two_axis_font();
    let subset = master_ids(&font, &["Regular"]);
    let error = export(
        &font,
        &|_| 0,
        &variable(ExportOptions {
            master_subset: Some(subset),
            ..Default::default()
        }),
    )
    .expect_err("single-master variable export");
    assert!(error.to_string().contains("static"), "{error}");
}

#[test]
fn a_subset_without_a_master_at_the_default_names_the_axis() {
    let font = two_axis_font();
    let subset = master_ids(&font, &["Light", "Bold"]);
    let error = export(
        &font,
        &|_| 0,
        &variable(ExportOptions {
            master_subset: Some(subset),
            ..Default::default()
        }),
    )
    .expect_err("no default master");
    assert!(error.to_string().contains("wght"), "{error}");
}

#[test]
fn export_scope_overrides_reach_the_compiled_instance() {
    let mut font = two_axis_font();
    let index = font
        .instances
        .iter()
        .position(|instance| instance.name == "Bold")
        .expect("Bold instance");
    font.instances[index].overrides = fc_model::InstanceOverrides {
        weight_class: Some(700),
        width_class: Some(4),
        postscript_font_name: Some("Override-Bold".into()),
        preferred_family_name: Some("Override Family".into()),
        preferred_subfamily_name: Some("Heavy".into()),
    };
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            instance: Some(index),
            format: ExportFormat::Ttf,
            ..Default::default()
        },
    )
    .expect("static instance export");
    let face = FontRef::new(&report.bytes).expect("compiled font");
    let os2 = face.os2().expect("OS/2");
    assert_eq!(os2.us_weight_class(), 700);
    assert_eq!(os2.us_width_class(), 4);
    let names: Vec<(u16, String)> = face
        .name()
        .expect("name")
        .name_record()
        .iter()
        .filter_map(|record| {
            let string = record
                .string(face.name().expect("name").string_data())
                .ok()?;
            Some((record.name_id().to_u16(), string.chars().collect()))
        })
        .collect();
    let value = |id: u16| {
        names
            .iter()
            .find(|(name_id, _)| *name_id == id)
            .map(|(_, text)| text.clone())
    };
    assert_eq!(value(6).as_deref(), Some("Override-Bold"));
    assert_eq!(value(16).as_deref(), Some("Override Family"));
    assert_eq!(value(17).as_deref(), Some("Heavy"));
    // Name 4 is the combination of 16 and 17, so the override reaches the full
    // name a menu shows and the CFF FullName derived from it.
    assert_eq!(value(4).as_deref(), Some("Override Family Heavy"));
}

#[test]
fn a_preferred_subfamily_override_still_writes_its_family_name() {
    let mut font = two_axis_font();
    let index = font
        .instances
        .iter()
        .position(|instance| instance.name == "Bold")
        .expect("Bold instance");
    // Only name 17 is overridden; name 16 has to follow, because a typographic
    // subfamily without its family is not a valid pair.
    font.instances[index].overrides = fc_model::InstanceOverrides {
        preferred_subfamily_name: Some("Heavy".into()),
        ..Default::default()
    };
    let family = font.info.family_name.clone();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            instance: Some(index),
            format: ExportFormat::Ttf,
            ..Default::default()
        },
    )
    .expect("static instance export");
    let face = FontRef::new(&report.bytes).expect("compiled font");
    let name = face.name().expect("name");
    let value = |id: u16| -> Option<String> {
        name.name_record()
            .iter()
            .find(|record| record.name_id().to_u16() == id)
            .and_then(|record| record.string(name.string_data()).ok())
            .map(|string| string.chars().collect())
    };
    assert_eq!(value(16).as_deref(), Some(family.as_str()));
    assert_eq!(value(17).as_deref(), Some("Heavy"));
    assert_eq!(value(4), Some(format!("{family} Heavy")));
}

#[test]
fn an_out_of_range_export_override_is_refused() {
    let mut font = two_axis_font();
    font.instances[0].overrides = fc_model::InstanceOverrides {
        width_class: Some(12),
        ..Default::default()
    };
    let error = export(
        &font,
        &|_| 0,
        &ExportOptions {
            instance: Some(0),
            format: ExportFormat::Ttf,
            ..Default::default()
        },
    )
    .expect_err("invalid width class");
    assert!(error.to_string().contains("width class"), "{error}");
}
