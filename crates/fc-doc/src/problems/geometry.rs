use std::collections::HashMap;

use fc_model::{Contour, Font, GlyphName, Layer, LayerKind, PointKind};

use super::{layer_master, Collector, Location, Severity};

// Full compiler validation can flatten mixed contour/component glyphs. Count
// expanded records without materializing them so small repeated-reference DAGs
// cannot allocate exponentially. Include empty nodes as work, not only points.
const MAX_EXPANDED_LAYER_RECORDS: usize = 100_000;
const MAX_EXPANDED_FONT_RECORDS: usize = 5_000_000;
const MAX_COMPONENT_DEPTH: usize = 64;

pub(super) fn metrics(font: &Font, out: &mut Collector) {
    if font.info.units_per_em == 0 {
        out.error(
            "invalid-upem",
            "Units per em must be positive.",
            &Location::default(),
        );
    }
    if font.masters.is_empty() {
        out.error(
            "missing-masters",
            "The font has no masters.",
            &Location::default(),
        );
    }
    for master in &font.masters {
        let m = &master.metrics;
        let values = [
            Some(m.ascender),
            Some(m.descender),
            Some(m.x_height),
            Some(m.cap_height),
            Some(m.italic_angle),
            m.hhea_ascender,
            m.hhea_descender,
            m.hhea_line_gap,
            m.typo_ascender,
            m.typo_descender,
            m.typo_line_gap,
            m.win_ascent,
            m.win_descent,
            m.vhea_ascender,
            m.vhea_descender,
            m.vhea_line_gap,
            m.default_vertical_origin,
            m.default_vertical_advance,
        ];
        if values.into_iter().flatten().any(|value| !value.is_finite()) {
            out.error(
                "nonfinite-metrics",
                "Master metrics contain a nonfinite value.",
                &Location {
                    master: Some(master.id.clone()),
                    ..Location::default()
                },
            );
        }
    }
}

pub(super) fn layer(font: &Font, name: &GlyphName, layer: &Layer, out: &mut Collector) {
    let at = Location::layer(font, name, layer);
    if at
        .master
        .as_ref()
        .is_some_and(|id| !font.masters.iter().any(|master| &master.id == id))
    {
        out.error(
            "missing-layer-master",
            "Layer refers to a master absent from the font.",
            &at,
        );
    }
    if !layer.width.is_finite() || layer.width < 0.0 {
        out.error(
            "invalid-width",
            "Horizontal advance must be finite and nonnegative.",
            &at,
        );
    }
    if layer
        .vertical_width
        .is_some_and(|width| !width.is_finite() || width < 0.0)
    {
        out.error(
            "invalid-vertical-width",
            "Vertical advance must be finite and nonnegative.",
            &at,
        );
    }
    if layer
        .vertical_origin
        .is_some_and(|origin| !origin.is_finite())
    {
        out.error("nonfinite-metrics", "Vertical origin must be finite.", &at);
    }
    for anchor in &layer.anchors {
        if !anchor.x.is_finite() || !anchor.y.is_finite() {
            out.error(
                "nonfinite-anchor",
                format!("Anchor {} has nonfinite coordinates.", anchor.name),
                &at,
            );
        }
    }
    contours(&layer.contours, &at, out);
    for (index, color) in layer.color_layers.iter().flatten().enumerate() {
        contours(
            &color.contours,
            &Location {
                color_layer: Some(index),
                ..at.clone()
            },
            out,
        );
    }
}

fn contours(contours: &[Contour], location: &Location, out: &mut Collector) {
    for (index, contour) in contours.iter().enumerate() {
        let at = Location {
            contour: Some(index),
            ..location.clone()
        };
        if !contour.closed {
            out.push(
                "open-contour",
                Severity::Warning,
                "Contour is open; outline exports may require closure.",
                &at,
            );
        }
        if malformed(contour) {
            out.error(
                "malformed-contour",
                "Contour has an invalid sequence of endpoints and control points.",
                &at,
            );
        }
        for (index, point) in contour.points.iter().enumerate() {
            if !point.x.is_finite() || !point.y.is_finite() {
                out.error(
                    "nonfinite-point",
                    "Point coordinates must be finite.",
                    &Location {
                        point: Some(index),
                        ..at.clone()
                    },
                );
            }
        }
    }
}

// Validate segment roles directly, without allocating or evaluating a path.
fn malformed(contour: &Contour) -> bool {
    let points = &contour.points;
    if points.is_empty() {
        return true;
    }
    if contour.closed && points.iter().any(|point| point.kind == PointKind::Move) {
        return true;
    }
    let Some(start) = points
        .iter()
        .position(|point| point.kind != PointKind::OffCurve)
    else {
        return !contour.closed;
    };
    if !contour.closed && start != 0 {
        return true;
    }
    let mut controls = 0;
    let end = if contour.closed {
        points.len() + 1
    } else {
        points.len()
    };
    for offset in 1..end {
        let point = &points[(start + offset) % points.len()];
        if point.kind == PointKind::OffCurve {
            controls += 1;
        } else {
            if point.kind == PointKind::Move
                || (point.kind == PointKind::Curve && controls != 2)
                || (point.kind == PointKind::Line && controls != 0)
            {
                return true;
            }
            controls = 0;
        }
    }
    controls != 0
}

pub(super) fn components(font: &Font, out: &mut Collector) {
    let nodes: Vec<_> = font
        .glyphs
        .iter()
        .flat_map(|(name, glyph)| glyph.layers.iter().map(move |layer| (name, layer)))
        .collect();
    let mut masters = HashMap::new();
    for (index, (name, layer)) in nodes.iter().enumerate() {
        if let LayerKind::Master(master) = &layer.kind {
            masters.entry((*name, master)).or_insert(index);
        }
    }
    let mut edges = vec![Vec::new(); nodes.len()];
    for (index, (name, layer)) in nodes.iter().enumerate() {
        let at = Location::layer(font, name, layer);
        let sets = std::iter::once((None, layer.components.as_slice())).chain(
            layer
                .color_layers
                .iter()
                .flatten()
                .enumerate()
                .map(|(i, color)| (Some(i), color.components.as_slice())),
        );
        for (color_layer, components) in sets {
            let at = Location {
                color_layer,
                ..at.clone()
            };
            for component in components {
                if component
                    .transform
                    .as_coeffs()
                    .iter()
                    .any(|value| !value.is_finite())
                {
                    out.error(
                        "nonfinite-component",
                        format!("Component {} has a nonfinite transform.", component.base),
                        &at,
                    );
                }
                if !font.glyphs.contains_key(&component.base) {
                    out.error(
                        "missing-component",
                        format!("Component base {} does not exist.", component.base),
                        &at,
                    );
                } else if let Some(master) = layer_master(font, layer) {
                    if let Some(target) = masters.get(&(&component.base, master)) {
                        // Color entries refer to ordinary master outlines, not
                        // recursively to the target's color stack. Referencing
                        // one's own fallback outline is therefore valid.
                        if color_layer.is_none() {
                            edges[index].push(*target);
                        }
                    } else {
                        out.error(
                            "missing-component-layer",
                            format!(
                                "Component {} has no layer for master {master}.",
                                component.base
                            ),
                            &at,
                        );
                    }
                }
            }
        }
    }
    // Three-color DFS examines each edge once and never expands its outlines.
    let mut states = vec![0u8; nodes.len()];
    let mut expanded = vec![0usize; nodes.len()];
    let mut depths = vec![0usize; nodes.len()];
    let mut total_expanded = 0usize;
    let mut font_limit_reported = false;
    for root in 0..nodes.len() {
        if states[root] != 0 {
            continue;
        }
        states[root] = 1;
        let mut stack = vec![(root, 0)];
        while let Some((node, next)) = stack.last_mut() {
            let node = *node;
            if *next == edges[node].len() {
                let (name, layer) = nodes[node];
                let own_records = layer.contours.iter().fold(1usize, |sum, contour| {
                    sum.saturating_add(contour.points.len())
                });
                expanded[node] = edges[node]
                    .iter()
                    .fold(own_records, |sum, target| {
                        sum.saturating_add(expanded[*target])
                    })
                    .min(MAX_EXPANDED_FONT_RECORDS + 1);
                depths[node] = edges[node]
                    .iter()
                    .map(|target| depths[*target].saturating_add(1))
                    .max()
                    .unwrap_or(0);
                let at = Location::layer(font, name, layer);
                let compiled_layer = matches!(layer.kind, LayerKind::Master(_))
                    && font.glyphs.get(name).is_some_and(|glyph| glyph.export);
                if compiled_layer && expanded[node] > MAX_EXPANDED_LAYER_RECORDS {
                    out.error("component-expansion-limit", "Expanded layer geometry exceeds the 100000-record compilation validation budget.", &at);
                }
                if depths[node] > MAX_COMPONENT_DEPTH {
                    out.error(
                        "component-depth-limit",
                        "Component references exceed the supported depth of 64.",
                        &at,
                    );
                }
                if compiled_layer {
                    total_expanded = total_expanded.saturating_add(expanded[node]);
                }
                if total_expanded > MAX_EXPANDED_FONT_RECORDS && !font_limit_reported {
                    out.error("component-expansion-limit", "Expanded font geometry exceeds the 5000000-record compilation validation budget.", &Location::default());
                    font_limit_reported = true;
                }
                states[node] = 2;
                stack.pop();
                continue;
            }
            let target = edges[node][*next];
            *next += 1;
            if states[target] == 1 {
                let (name, layer) = nodes[node];
                out.error(
                    "component-cycle",
                    format!("Component reference to {} forms a cycle.", nodes[target].0),
                    &Location::layer(font, name, layer),
                );
            } else if states[target] == 0 {
                states[target] = 1;
                stack.push((target, 0));
            }
        }
    }
}
