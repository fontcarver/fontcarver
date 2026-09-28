use super::{ColrColorLine, ColrGraph, ColrPaint, ColrTransform, MAX_COLR_PAINTS};
use crate::GlyphName;
use std::collections::HashSet;

fn fixed(value: f64, scale: f64, min: f64, max: f64) -> bool {
    value.is_finite() && (min..=max).contains(&(value * scale).round())
}
fn f2(value: f64) -> bool {
    fixed(value, 16384., -32768., 32767.)
}
fn angle(value: f64) -> bool {
    f2(value / 180.)
}
fn color(index: Option<u16>, alpha: f64, entries: usize) -> bool {
    index.is_none_or(|i| usize::from(i) < entries)
        && alpha.is_finite()
        && (0. ..=1.).contains(&alpha)
}
fn line(line: &ColrColorLine, entries: usize) -> bool {
    line.extend <= 2
        && line.stops.len() <= 65535
        && line
            .stops
            .iter()
            .all(|s| f2(s.offset) && color(s.palette_index, s.alpha, entries))
}
fn transform(transform: &ColrTransform) -> bool {
    match transform {
        ColrTransform::Affine { matrix } => matrix
            .iter()
            .all(|v| fixed(*v, 65536., i32::MIN as f64, i32::MAX as f64)),
        ColrTransform::Translate { .. } => true,
        ColrTransform::Scale { scale, .. } => scale.iter().all(|v| f2(*v)),
        ColrTransform::Rotate { angle: a, .. } => angle(*a),
        ColrTransform::Skew { angles, .. } => angles.iter().all(|a| angle(*a)),
    }
}

pub(super) fn graph(graph: &ColrGraph, order: &[GlyphName], entries: usize) -> Result<(), String> {
    if graph.paints.len() > MAX_COLR_PAINTS || graph.glyphs.len() > 65535 {
        return Err("COLRv1 paint/root limit exceeded".into());
    }
    let names: HashSet<_> = order.iter().collect();
    for (name, root) in &graph.glyphs {
        if !names.contains(name) || *root as usize >= graph.paints.len() {
            return Err(format!("COLRv1 missing glyph or root: {name}"));
        }
    }
    for (name, clip) in &graph.clips {
        if !names.contains(name) || clip[0] > clip[2] || clip[1] > clip[3] {
            return Err(format!("COLRv1 invalid clip: {name}"));
        }
    }
    let mut parents = vec![Vec::new(); graph.paints.len()];
    let mut remaining = vec![0usize; graph.paints.len()];
    let mut items = graph.paints.len();
    for (index, paint) in graph.paints.iter().enumerate() {
        let valid = match paint {
            ColrPaint::Layers { paints } => paints.len() <= 255,
            ColrPaint::Solid {
                palette_index,
                alpha,
            } => color(*palette_index, *alpha, entries),
            ColrPaint::Linear { color_line, .. } | ColrPaint::Radial { color_line, .. } => {
                items = items.saturating_add(color_line.stops.len());
                line(color_line, entries)
            }
            ColrPaint::Sweep {
                color_line, angles, ..
            } => {
                items = items.saturating_add(color_line.stops.len());
                line(color_line, entries) && angles.iter().all(|a| f2(*a / 180. - 1.))
            }
            ColrPaint::Glyph { glyph, .. } => names.contains(glyph),
            ColrPaint::ColrGlyph { glyph } => graph.glyphs.contains_key(glyph),
            ColrPaint::Transform { transform: t, .. } => transform(t),
            ColrPaint::Composite { mode, .. } => *mode <= 27,
        };
        if !valid {
            return Err(format!("COLRv1 invalid paint {index}"));
        }
        let mut children = paint.children();
        if let ColrPaint::ColrGlyph { glyph } = paint {
            children.push(graph.glyphs[glyph]);
        }
        items = items.saturating_add(children.len());
        if items > 1_000_000 {
            return Err("COLRv1 item budget exceeded".into());
        }
        for child in children {
            let child = child as usize;
            if child >= graph.paints.len() {
                return Err(format!("COLRv1 paint {index} has a missing child"));
            }
            parents[child].push(index);
            remaining[index] += 1;
        }
    }
    let mut ready: Vec<_> = (0..graph.paints.len())
        .filter(|i| remaining[*i] == 0)
        .collect();
    let mut sizes = vec![1usize; graph.paints.len()];
    let mut depth = vec![1usize; graph.paints.len()];
    let mut bounded = vec![false; graph.paints.len()];
    let mut visited = 0;
    while let Some(index) = ready.pop() {
        visited += 1;
        let bound = |i: u32| bounded[i as usize];
        bounded[index] = match &graph.paints[index] {
            ColrPaint::Glyph { .. } => true,
            ColrPaint::Layers { paints } => paints.iter().all(|i| bound(*i)),
            ColrPaint::ColrGlyph { glyph } => {
                graph.clips.contains_key(glyph) || bound(graph.glyphs[glyph])
            }
            ColrPaint::Transform { paint, .. } => bound(*paint),
            ColrPaint::Composite {
                source,
                backdrop,
                mode,
            } => match mode {
                0 => true,
                1 | 7 => bound(*source),
                2 | 8 => bound(*backdrop),
                5 | 6 => bound(*source) || bound(*backdrop),
                _ => bound(*source) && bound(*backdrop),
            },
            _ => false,
        };
        for parent in &parents[index] {
            sizes[*parent] = sizes[*parent].saturating_add(sizes[index]);
            depth[*parent] = depth[*parent].max(depth[index] + 1);
            if sizes[*parent] > 1_000_000 || depth[*parent] > 64 {
                return Err("COLRv1 depth or paint expansion budget exceeded".into());
            }
            remaining[*parent] -= 1;
            if remaining[*parent] == 0 {
                ready.push(*parent);
            }
        }
    }
    if visited != graph.paints.len() {
        return Err("COLRv1 paint graph contains a cycle".into());
    }
    for (name, root) in &graph.glyphs {
        if !graph.clips.contains_key(name) && !bounded[*root as usize] {
            return Err(format!(
                "COLRv1 root {name} requires a finite clip or bounded paint"
            ));
        }
    }
    let work = graph.glyphs.values().fold(0usize, |sum, root| {
        sum.saturating_add(sizes[*root as usize])
    });
    if work > 1_000_000 {
        return Err("COLRv1 total root evaluation budget exceeded".into());
    }
    Ok(())
}
