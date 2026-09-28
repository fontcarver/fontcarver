//! Pack shared paint nodes once, placing explicit children after their parents.
use super::error;
use crate::Error;
use fc_model::{ColrColorLine, ColrGraph, ColrPaint, ColrTransform, GlyphName};
use std::collections::{HashMap, VecDeque};

fn u16out(out: &mut Vec<u8>, n: u16) {
    out.extend(n.to_be_bytes());
}
fn signed(out: &mut Vec<u8>, n: i16) {
    out.extend(n.to_be_bytes());
}
fn f2(out: &mut Vec<u8>, n: f64) {
    signed(out, (n * 16384.).round() as i16);
}
fn point(out: &mut Vec<u8>, p: [i16; 2]) {
    signed(out, p[0]);
    signed(out, p[1]);
}
fn patch(out: &mut [u8], at: usize, n: usize) {
    out[at..at + 4].copy_from_slice(&(n as u32).to_be_bytes());
}
fn offset(out: &mut Vec<u8>, base: usize, target: usize) -> Result<(), Error> {
    let value = target
        .checked_sub(base)
        .filter(|v| *v > 0 && *v <= 0xFFFFFF)
        .ok_or_else(|| error("24-bit paint offset cannot be represented"))?;
    out.extend([(value >> 16) as u8, (value >> 8) as u8, value as u8]);
    Ok(())
}
fn color_line(out: &mut Vec<u8>, line: &ColrColorLine) {
    out.push(line.extend);
    u16out(out, line.stops.len() as u16);
    for stop in &line.stops {
        f2(out, stop.offset);
        u16out(out, stop.palette_index.unwrap_or(0xFFFF));
        f2(out, stop.alpha);
    }
}
fn size(paint: &ColrPaint) -> usize {
    match paint {
        ColrPaint::Layers { .. } => 6,
        ColrPaint::Solid { .. } => 5,
        ColrPaint::Linear { color_line, .. } | ColrPaint::Radial { color_line, .. } => {
            19 + color_line.stops.len() * 6
        }
        ColrPaint::Sweep { color_line, .. } => 15 + color_line.stops.len() * 6,
        ColrPaint::Glyph { .. } => 6,
        ColrPaint::ColrGlyph { .. } => 3,
        ColrPaint::Composite { .. } => 8,
        ColrPaint::Transform { transform, .. } => match transform {
            ColrTransform::Affine { .. } => 31,
            ColrTransform::Translate { .. } => 8,
            ColrTransform::Scale { .. } | ColrTransform::Skew { .. } => 12,
            ColrTransform::Rotate { .. } => 10,
        },
    }
}

pub(in crate::color) fn upgrade(
    v0: &[u8],
    graph: &ColrGraph,
    order: &[GlyphName],
) -> Result<Vec<u8>, Error> {
    let gids: HashMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(i, n)| (n, i as u16))
        .collect();
    let mut out = vec![0; 34];
    out[..14].copy_from_slice(&v0[..14]);
    out[1] = 1;
    let old = super::Data {
        bytes: v0,
        tag: "COLR",
    };
    for field in [4, 8] {
        patch(&mut out, field, old.u32(field)? as usize + 20);
    }
    out.extend_from_slice(&v0[14..]);
    let mut roots: Vec<_> = graph.glyphs.iter().collect();
    roots.sort_by_key(|(name, _)| gids[name]);
    let bases = out.len();
    patch(&mut out, 14, bases);
    out.extend((roots.len() as u32).to_be_bytes());
    out.resize(out.len() + roots.len() * 6, 0);
    let mut layers = Vec::new();
    let mut layer_starts = HashMap::new();
    for (index, paint) in graph.paints.iter().enumerate() {
        if let ColrPaint::Layers { paints } = paint {
            layer_starts.insert(index, layers.len());
            layers.extend(paints.iter().copied());
        }
    }
    let layer_list = out.len();
    if !layer_starts.is_empty() {
        patch(&mut out, 18, layer_list);
        out.extend((layers.len() as u32).to_be_bytes());
        out.resize(out.len() + layers.len() * 4, 0);
    }
    if !graph.clips.is_empty() {
        let at = out.len();
        patch(&mut out, 22, at);
        out.push(1);
        out.extend((graph.clips.len() as u32).to_be_bytes());
        let mut clips: Vec<_> = graph.clips.iter().collect();
        clips.sort_by_key(|(name, _)| gids[name]);
        let first = at + 5 + clips.len() * 7;
        for (index, (name, _)) in clips.iter().enumerate() {
            u16out(&mut out, gids[name]);
            u16out(&mut out, gids[name]);
            offset(&mut out, at, first + index * 9)?;
        }
        for (_, clip) in clips {
            out.push(1);
            for value in clip {
                signed(&mut out, *value);
            }
        }
    }
    let mut incoming = vec![0usize; graph.paints.len()];
    for paint in &graph.paints {
        for child in paint.children() {
            incoming[child as usize] += 1;
        }
    }
    let mut ready: VecDeque<_> = (0..graph.paints.len())
        .filter(|i| incoming[*i] == 0)
        .collect();
    let mut positions = vec![0; graph.paints.len()];
    let mut sorted = Vec::new();
    let mut total = out.len();
    while let Some(index) = ready.pop_front() {
        positions[index] = total;
        total = total.saturating_add(size(&graph.paints[index]));
        if total > 32 * 1024 * 1024 {
            return Err(error("encoded graph exceeds 32 MiB"));
        }
        sorted.push(index);
        for child in graph.paints[index].children() {
            incoming[child as usize] -= 1;
            if incoming[child as usize] == 0 {
                ready.push_back(child as usize);
            }
        }
    }
    if sorted.len() != graph.paints.len() {
        return Err(error("cyclic paint offsets"));
    }
    for (i, (name, root)) in roots.iter().enumerate() {
        let at = bases + 4 + i * 6;
        out[at..at + 2].copy_from_slice(&gids[name].to_be_bytes());
        patch(&mut out, at + 2, positions[**root as usize] - bases);
    }
    for (i, paint) in layers.iter().enumerate() {
        patch(
            &mut out,
            layer_list + 4 + i * 4,
            positions[*paint as usize] - layer_list,
        );
    }
    for index in sorted {
        let at = out.len();
        let child = |paint: u32| positions[paint as usize];
        match &graph.paints[index] {
            ColrPaint::Layers { paints } => {
                out.push(1);
                out.push(paints.len() as u8);
                out.extend((layer_starts[&index] as u32).to_be_bytes());
            }
            ColrPaint::Solid {
                palette_index,
                alpha,
            } => {
                out.push(2);
                u16out(&mut out, palette_index.unwrap_or(0xFFFF));
                f2(&mut out, *alpha);
            }
            ColrPaint::Linear {
                color_line: line,
                points,
            } => {
                out.push(4);
                offset(&mut out, at, at + 16)?;
                for p in points {
                    point(&mut out, *p);
                }
                color_line(&mut out, line);
            }
            ColrPaint::Radial {
                color_line: line,
                centers,
                radii,
            } => {
                out.push(6);
                offset(&mut out, at, at + 16)?;
                for i in 0..2 {
                    point(&mut out, centers[i]);
                    u16out(&mut out, radii[i]);
                }
                color_line(&mut out, line);
            }
            ColrPaint::Sweep {
                color_line: line,
                center,
                angles,
            } => {
                out.push(8);
                offset(&mut out, at, at + 12)?;
                point(&mut out, *center);
                for a in angles {
                    f2(&mut out, *a / 180. - 1.);
                }
                color_line(&mut out, line);
            }
            ColrPaint::Glyph { glyph, paint } => {
                out.push(10);
                offset(&mut out, at, child(*paint))?;
                u16out(&mut out, gids[glyph]);
            }
            ColrPaint::ColrGlyph { glyph } => {
                out.push(11);
                u16out(&mut out, gids[glyph]);
            }
            ColrPaint::Transform { paint, transform } => match transform {
                ColrTransform::Affine { matrix } => {
                    out.push(12);
                    offset(&mut out, at, child(*paint))?;
                    offset(&mut out, at, at + 7)?;
                    for v in matrix {
                        out.extend(((*v * 65536.).round() as i32).to_be_bytes());
                    }
                }
                ColrTransform::Translate {
                    offset: translation,
                } => {
                    out.push(14);
                    offset(&mut out, at, child(*paint))?;
                    point(&mut out, *translation);
                }
                ColrTransform::Scale { scale, center } => {
                    out.push(18);
                    offset(&mut out, at, child(*paint))?;
                    for v in scale {
                        f2(&mut out, *v);
                    }
                    point(&mut out, *center);
                }
                ColrTransform::Rotate { angle, center } => {
                    out.push(26);
                    offset(&mut out, at, child(*paint))?;
                    f2(&mut out, *angle / 180.);
                    point(&mut out, *center);
                }
                ColrTransform::Skew { angles, center } => {
                    out.push(30);
                    offset(&mut out, at, child(*paint))?;
                    for a in angles {
                        f2(&mut out, *a / 180.);
                    }
                    point(&mut out, *center);
                }
            },
            ColrPaint::Composite {
                source,
                backdrop,
                mode,
            } => {
                out.push(32);
                offset(&mut out, at, child(*source))?;
                out.push(*mode);
                offset(&mut out, at, child(*backdrop))?;
            }
        }
    }
    Ok(out)
}
