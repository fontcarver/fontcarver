use fc_model::{ColrGraph, ColrPaint as Paint, ColrTransform as Transform, *};

fn fixture() -> (ColorTable, Vec<GlyphName>) {
    let line = ColrColorLine {
        extend: 2,
        stops: vec![
            ColrColorStop {
                offset: 1.,
                palette_index: Some(0),
                alpha: 0.5,
            },
            ColrColorStop {
                offset: 0.,
                palette_index: None,
                alpha: 1.,
            },
            ColrColorStop {
                offset: 1.,
                palette_index: Some(1),
                alpha: 1.,
            },
        ],
    };
    let mut paints = vec![
        Paint::Solid {
            palette_index: Some(0),
            alpha: 0.5,
        },
        Paint::Linear {
            color_line: line.clone(),
            points: [[0, 0], [600, 0], [0, 700]],
        },
        Paint::Radial {
            color_line: line.clone(),
            centers: [[300, 350], [100, 100]],
            radii: [0, 600],
        },
        Paint::Sweep {
            color_line: line,
            center: [300, 350],
            angles: [0., 360.],
        },
    ];
    for transform in [
        Transform::Affine {
            matrix: [1., 0.25, -0.5, 1., 30., -50.],
        },
        Transform::Translate { offset: [-30, 50] },
        Transform::Scale {
            scale: [-1., 0.5],
            center: [300, 350],
        },
        Transform::Rotate {
            angle: 45.,
            center: [300, 350],
        },
        Transform::Skew {
            angles: [22.5, -45.],
            center: [300, 350],
        },
    ] {
        paints.push(Paint::Transform {
            paint: 1,
            transform,
        });
    }
    for mode in 0..28 {
        paints.push(Paint::Composite {
            source: 2,
            backdrop: 3,
            mode,
        });
    }
    let layers = paints.len() as u32;
    paints.push(Paint::Layers {
        paints: (0..layers).collect(),
    });
    paints.push(Paint::Glyph {
        glyph: "shape".into(),
        paint: layers,
    });
    paints.push(Paint::ColrGlyph { glyph: "A".into() });
    let graph = ColrGraph {
        glyphs: [("A".into(), layers + 1), ("B".into(), layers + 2)].into(),
        paints,
        clips: [
            ("A".into(), [-20, -50, 700, 800]),
            ("shape".into(), [0, 0, 700, 700]),
        ]
        .into(),
    };
    (
        ColorTable {
            graph: Some(graph),
            palettes: vec![ColorPalette {
                colors: vec![
                    PaletteColor {
                        red: 255,
                        green: 0,
                        blue: 0,
                        alpha: 255,
                    },
                    PaletteColor {
                        red: 0,
                        green: 0,
                        blue: 255,
                        alpha: 128,
                    },
                ],
                ..Default::default()
            }],
            glyphs: [(
                "A".into(),
                vec![ColrLayer {
                    glyph: "shape".into(),
                    palette_index: Some(0),
                }],
            )]
            .into(),
            ..Default::default()
        },
        [".notdef", "A", "B", "shape"].map(Into::into).into(),
    )
}
#[derive(Debug, PartialEq)]
struct Semantic {
    paint: Paint,
    children: Vec<Semantic>,
}
fn semantic(graph: &ColrGraph, root: u32) -> Semantic {
    let mut paint = graph.paints[root as usize].clone();
    let children = paint
        .children()
        .iter()
        .map(|i| semantic(graph, *i))
        .collect();
    match &mut paint {
        Paint::Layers { paints } => paints.clear(),
        Paint::Glyph { paint, .. } | Paint::Transform { paint, .. } => *paint = 0,
        Paint::Composite {
            source, backdrop, ..
        } => {
            *source = 0;
            *backdrop = 0;
        }
        _ => {}
    }
    Semantic { paint, children }
}
#[test]
fn all_static_paints_share_nodes_and_remap_named_glyphs() {
    let (color, mut order) = fixture();
    order.reverse();
    let (colr, cpal) = fc_sfnt::color::write_tables(&color, &order).unwrap();
    let decoded = fc_sfnt::color::read_tables(colr.as_deref(), &cpal, &order).unwrap();
    assert_eq!(decoded.glyphs, color.glyphs);
    assert_eq!(decoded.palettes, color.palettes);
    let expected = color.graph.unwrap();
    let actual = decoded.graph.unwrap();
    assert_eq!(actual.clips, expected.clips);
    assert_eq!(
        actual.paints.len(),
        expected.paints.len(),
        "shared DAG must not expand"
    );
    for (name, root) in &expected.glyphs {
        assert_eq!(
            semantic(&actual, actual.glyphs[name]),
            semantic(&expected, *root)
        );
    }
}
#[test]
fn malformed_and_variable_graphs_fail_atomically() {
    let (color, order) = fixture();
    let (bytes, cpal) = fc_sfnt::color::write_tables(&color, &order).unwrap();
    let bytes = bytes.unwrap();
    for end in 0..bytes.len() {
        assert!(
            fc_sfnt::color::read_tables(Some(&bytes[..end]), &cpal, &order).is_err(),
            "truncation {end}"
        );
    }
    for field in [14, 18, 22, 26, 30] {
        let mut bad = bytes.clone();
        bad[field..field + 4].copy_from_slice(&1u32.to_be_bytes());
        assert!(
            fc_sfnt::color::read_tables(Some(&bad), &cpal, &order).is_err(),
            "header {field}"
        );
    }
    let mut graph = color.graph.unwrap();
    graph.paints.push(Paint::ColrGlyph { glyph: "B".into() });
    graph
        .glyphs
        .insert("A".into(), (graph.paints.len() - 1) as u32);
    assert!(graph.validate(&order, 2).unwrap_err().contains("cycle"));
    graph.glyphs.insert("A".into(), 0);
    graph.paints[0] = Paint::Layers { paints: vec![0] };
    assert!(graph.validate(&order, 2).is_err());
}
#[test]
fn deleting_an_outline_removes_dependent_roots_and_preserves_other_roots() {
    let (mut color, order) = fixture();
    let graph = color.graph.as_mut().unwrap();
    graph.glyphs.insert(".notdef".into(), 0);
    graph.clips.insert(".notdef".into(), [0, 0, 600, 700]);
    assert_eq!(graph.retain_glyphs(|name| name.as_ref() != "shape"), 2);
    assert_eq!(graph.glyphs.len(), 1);
    assert_eq!(graph.paints.len(), 1);
    graph.validate(&order, 2).unwrap();
}
#[test]
fn expansion_is_bounded_and_twenty_thousand_shared_roots_stay_fast() {
    use std::time::{Duration, Instant};
    let start = Instant::now();
    let (mut color, _) = fixture();
    color.glyphs.clear();
    let names: Vec<GlyphName> = (0..20_000).map(|i| format!("g{i}").into()).collect();
    color.graph = Some(ColrGraph {
        glyphs: names.iter().map(|n| (n.clone(), 0)).collect(),
        clips: names
            .iter()
            .map(|n| (n.clone(), [0, 0, 600, 700]))
            .collect(),
        paints: vec![Paint::Solid {
            palette_index: None,
            alpha: 1.,
        }],
    });
    let (colr, cpal) = fc_sfnt::color::write_tables(&color, &names).unwrap();
    assert_eq!(
        fc_sfnt::color::read_tables(colr.as_deref(), &cpal, &names).unwrap(),
        color
    );
    assert!(start.elapsed() < Duration::from_secs(10));
    let graph = color.graph.as_mut().unwrap();
    for i in 1..22 {
        graph.paints.push(Paint::Layers {
            paints: vec![i - 1, i - 1],
        });
    }
    graph.glyphs = [(names[0].clone(), 21)].into();
    assert!(graph.validate(&names, 2).unwrap_err().contains("budget"));
}

#[test]
fn unbounded_roots_require_clips_and_empty_layers_have_a_layer_list() {
    let (mut color, order) = fixture();
    color.graph = Some(ColrGraph {
        glyphs: [("A".into(), 0)].into(),
        paints: vec![Paint::Solid {
            palette_index: None,
            alpha: 1.,
        }],
        ..Default::default()
    });
    assert!(fc_sfnt::color::write_tables(&color, &order)
        .unwrap_err()
        .to_string()
        .contains("bounded"));
    let graph = color.graph.as_mut().unwrap();
    graph.clips.insert("A".into(), [0, 0, 700, 700]);
    assert!(fc_sfnt::color::write_tables(&color, &order).is_ok());
    let graph = color.graph.as_mut().unwrap();
    graph.clips.clear();
    graph.paints[0] = Paint::Layers { paints: vec![] };
    let (colr, cpal) = fc_sfnt::color::write_tables(&color, &order).unwrap();
    assert_eq!(
        fc_sfnt::color::read_tables(colr.as_deref(), &cpal, &order).unwrap(),
        color
    );
    let mut colr = colr.unwrap();
    colr[18..22].fill(0);
    assert!(fc_sfnt::color::read_tables(Some(&colr), &cpal, &order).is_err());
}
