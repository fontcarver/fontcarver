use std::collections::BTreeSet;

use fc_doc::problems::scan;
use fc_model::{Component, Contour, Font, Glyph, GlyphName, KernSide, Layer, Point, PointKind};

fn font() -> Font {
    let mut font = Font::new_empty("Problems", 1000);
    for (name, unicode) in [("A", 65), ("B", 66)] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(unicode);
        let mut layer = Layer::new_master(font.default_master().id.clone());
        layer.width = 500.0;
        layer.contours.push(Contour {
            closed: true,
            points: [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)]
                .into_iter()
                .map(|(x, y)| Point {
                    x,
                    y,
                    kind: PointKind::Line,
                    ..Point::default()
                })
                .collect(),
            ..Contour::default()
        });
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font
}

#[test]
fn clean_scan_is_read_only() {
    let font = font();
    let before = font.clone();
    let report = scan(&font, 200);
    assert!(report.issues.is_empty(), "{report:?}");
    assert_eq!(report.total, 0);
    assert!(!report.truncated);
    assert_eq!(font, before);
}

#[test]
fn diagnostics_are_located_deterministic_and_bounded() {
    let mut font = font();
    let a = font.glyphs.get_mut("A").unwrap();
    a.unicodes.extend([66, 0xd800, 0x110000]);
    a.layers[0].width = -1.0;
    a.layers[0].contours[0].closed = false;
    a.layers[0].contours[0].points[1].x = f64::NAN;
    a.layers[0].components.push(Component {
        base: "absent".into(),
        ..Component::default()
    });
    let b = font.glyphs.get_mut("B").unwrap();
    b.layers[0].contours[0].points[1].kind = PointKind::Curve;
    let report = scan(&font, 200);
    let codes: BTreeSet<_> = report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect();
    for code in [
        "duplicate-unicode",
        "invalid-unicode",
        "invalid-width",
        "open-contour",
        "nonfinite-point",
        "missing-component",
        "malformed-contour",
    ] {
        assert!(codes.contains(code), "missing {code}: {report:?}");
    }
    let nonfinite = report
        .issues
        .iter()
        .find(|issue| issue.code == "nonfinite-point")
        .unwrap();
    assert_eq!(nonfinite.glyph.as_ref(), Some(&GlyphName::from("A")));
    assert_eq!(nonfinite.master.as_ref(), Some(&font.default_master().id));
    assert_eq!(
        nonfinite.layer.as_ref(),
        Some(&font.glyphs["A"].layers[0].id)
    );
    assert_eq!(nonfinite.contour, Some(0));
    assert_eq!(nonfinite.point, Some(1));
    assert_eq!(report, scan(&font, 200));
    let short = scan(&font, 2);
    assert_eq!(short.total, report.total);
    assert_eq!(short.issues, report.issues[..2]);
    assert!(short.truncated);
}

#[test]
fn checks_master_coverage_and_kerning_consistency() {
    let mut font = font();
    font.glyphs.get_mut("A").unwrap().layers.clear();
    font.glyphs.get_mut("B").unwrap().kern_group_side1 = Some("different".into());
    font.kerning_groups
        .side1
        .insert("left".into(), ["B".into(), "missing".into()].into());
    font.kerning_groups
        .side1
        .insert("duplicate".into(), ["B".into()].into());
    font.masters[0].kerning_h.0.insert(
        (
            KernSide::Glyph("absent".into()),
            KernSide::Group("absent".into()),
        ),
        f64::INFINITY,
    );
    let report = scan(&font, 200);
    let codes: BTreeSet<_> = report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect();
    for code in [
        "missing-master-layer",
        "missing-kerning-glyph",
        "missing-kerning-group",
        "invalid-kerning-group",
        "invalid-kerning-value",
        "missing-group-member",
        "duplicate-group-membership",
        "inconsistent-group-membership",
    ] {
        assert!(codes.contains(code), "missing {code}: {report:?}");
    }
}

#[test]
fn long_component_cycles_terminate_without_recursive_expansion() {
    let mut font = Font::new_empty("Cycle", 1000);
    let count = 3000;
    for index in 0..count {
        let mut glyph = Glyph::new(format!("g{index}"));
        let mut layer = Layer::new_master(font.default_master().id.clone());
        layer.components.push(Component {
            base: format!("g{}", (index + 1) % count).into(),
            ..Component::default()
        });
        glyph.layers.push(layer);
        // A hand-edited native document can bypass the mutation API's cycle guard.
        font.glyphs.insert(glyph.name.clone(), glyph);
    }
    let start = std::time::Instant::now();
    let report = scan(&font, 200);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "component-cycle"));
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn color_geometry_is_located_and_self_fallback_is_not_a_component_cycle() {
    let mut font = font();
    let layer = &mut font.glyphs.get_mut("A").unwrap().layers[0];
    layer.color_layers = Some(vec![fc_model::ColorLayer {
        components: vec![Component {
            base: "A".into(),
            ..Component::default()
        }],
        contours: vec![Contour {
            closed: true,
            points: vec![Point {
                x: f64::INFINITY,
                ..Point::default()
            }],
            ..Contour::default()
        }],
        ..fc_model::ColorLayer::default()
    }]);
    let report = scan(&font, 200);
    assert!(!report
        .issues
        .iter()
        .any(|issue| issue.code == "component-cycle"));
    let issue = report
        .issues
        .iter()
        .find(|issue| issue.code == "nonfinite-point")
        .unwrap();
    assert_eq!(issue.color_layer, Some(0));
    assert_eq!(issue.contour, Some(0));
    assert_eq!(issue.point, Some(0));
}

#[test]
fn result_cap_applies_even_when_caller_requests_more() {
    let mut font = font();
    font.glyphs.get_mut("A").unwrap().unicodes = vec![0xd800; 1500];
    let report = scan(&font, usize::MAX);
    assert_eq!(report.issues.len(), fc_doc::problems::MAX_ISSUES);
    assert_eq!(report.total, 2999);
    assert!(report.truncated);
    assert!(report.has_errors);
}

#[test]
fn omitted_error_cannot_make_a_warning_only_page_look_clean() {
    let mut font = font();
    let layer = &mut font.glyphs.get_mut("A").unwrap().layers[0];
    layer.contours[0].closed = false;
    layer.components.push(Component {
        base: "missing".into(),
        ..Component::default()
    });
    let report = scan(&font, 1);
    assert_eq!(
        report.issues[0].severity,
        fc_doc::problems::Severity::Warning
    );
    assert!(report.truncated);
    assert!(report.has_errors);
}

#[test]
fn repeated_component_dag_is_budgeted_without_expanding_geometry() {
    let mut font = font();
    let triangle = font.glyphs["A"].layers[0].contours[0].clone();
    for index in 0..24 {
        let mut glyph = Glyph::new(format!("branch{index}"));
        let mut layer = Layer::new_master(font.default_master().id.clone());
        layer.contours.push(triangle.clone());
        if index > 0 {
            layer.components = vec![
                Component {
                    base: format!("branch{}", index - 1).into(),
                    ..Component::default()
                };
                2
            ];
        }
        glyph.layers.push(layer);
        font.glyphs.insert(glyph.name.clone(), glyph);
    }
    let report = scan(&font, 200);
    assert!(report.has_errors);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "component-expansion-limit"));
    assert!(!report
        .issues
        .iter()
        .any(|issue| issue.code == "component-cycle"));
}
