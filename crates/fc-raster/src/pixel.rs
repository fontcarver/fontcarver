//! Conservative recovery of bitmap strikes from exact grid-aligned outlines.

use std::collections::{HashMap, HashSet};

use fc_model::{Bitmap, BitmapStrike, Font, Layer, MasterId, PointKind};
use kurbo::{BezPath, PathEl, Point};

use crate::Error;

const MAX_GLYPHS: usize = 65_536;
const MAX_POINTS: usize = 1_000_000;
const MAX_PPEM: u16 = 256;
const EPSILON: f64 = 1e-7;
type Outline = Vec<Vec<Point>>;

/// Evidence for a shared pixel grid in a master’s outline geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelFontDetection {
    /// Recovered pixels per em.
    pub ppem: u16,
    /// One pixel’s size in font units.
    pub grid_size: f64,
    /// Number of master glyphs checked, including empty glyphs.
    pub glyph_count: usize,
    /// Geometrically different glyphs containing repeated unit-grid staircases.
    pub distinctive_glyphs: usize,
}

/// Detect a shared exact pixel grid conservatively, without changing the font.
///
/// Requires at least three different staircase outlines, all glyphs aligned to
/// the same origin-based grid, and a recovered size of 4..=256 pixels per em.
/// Curves, live effects and near-grid outlines produce no suggestion. Invalid
/// geometry, missing component sources and exceeded work limits return errors.
pub fn detect_pixel_font(
    font: &Font,
    master: &MasterId,
) -> Result<Option<PixelFontDetection>, Error> {
    detect(font, master)
}

/// Reconstruct selected monochrome glyphs if the entire master proves a pixel grid.
pub fn reconstruct_pixel_font(
    font: &Font,
    master: &MasterId,
    names: &[String],
) -> Result<Option<BitmapStrike>, Error> {
    detect_pixel_font(font, master)?
        .map(|detection| crate::rasterize(font, master, detection.ppem, 1, names))
        .transpose()
}

fn detect(font: &Font, master: &MasterId) -> Result<Option<PixelFontDetection>, Error> {
    if font.info.units_per_em == 0 || font.master(master).is_none() {
        return Err(crate::invalid("invalid pixel-font units per em or master"));
    }
    if font.glyphs.len() > MAX_GLYPHS {
        return Err(crate::invalid("pixel-font glyph work limit"));
    }
    let mut total = 0usize;
    let mut memo = HashMap::new();
    let mut active = HashSet::new();
    // Preflight the entire component graph before allocating expanded geometry.
    for name in font.glyphs.keys() {
        total = total.saturating_add(crate::check_work(
            font,
            name,
            master,
            &mut active,
            &mut memo,
        )?);
        if total > MAX_POINTS {
            return Err(crate::invalid("pixel-font expanded outline work limit"));
        }
    }
    // Reject unsupported sources before flattening any component that could
    // evaluate an effect or smart layer outside the counted source outlines.
    for glyph in font.glyphs.values() {
        let layer = glyph
            .master_layer(master)
            .ok_or_else(|| crate::invalid("missing master layer"))?;
        if !plain_layer(layer)? || !glyph.smart_axes.is_empty() {
            return Ok(None);
        }
    }
    let mut outlines = Vec::with_capacity(font.glyphs.len());
    let mut shortest = f64::INFINITY;
    for (name, glyph) in &font.glyphs {
        let layer = glyph
            .master_layer(master)
            .ok_or_else(|| crate::invalid("missing master layer"))?;
        let path = fc_geom::flatten_layer(font, name, master)?;
        let Some(outline) = orthogonal_outline(&path, &mut shortest)? else {
            return Ok(None);
        };
        outlines.push((layer, outline));
    }
    if !shortest.is_finite() || shortest <= 0.0 {
        return Ok(None);
    }
    // A repeated unit staircase proves that the shortest edge is one pixel.
    // This avoids a potentially expensive search through every possible ppem.
    let raw_ppem = f64::from(font.info.units_per_em) / shortest;
    let ppem = raw_ppem.round();
    if !(4.0..=f64::from(MAX_PPEM)).contains(&ppem) || (raw_ppem - ppem).abs() > EPSILON {
        return Ok(None);
    }
    let scale = ppem / f64::from(font.info.units_per_em);
    let mut evidence = HashSet::new();
    for (layer, outline) in outlines {
        if [
            Some(layer.width),
            layer.vertical_width,
            layer.vertical_origin,
        ]
        .into_iter()
        .flatten()
        .any(|value| grid_coordinate(value, scale).is_none())
        {
            return Ok(None);
        }
        let mut pixels = Vec::with_capacity(outline.len());
        for contour in outline {
            let mut points = Vec::with_capacity(contour.len());
            for point in contour {
                let (Some(x), Some(y)) = (
                    grid_coordinate(point.x, scale),
                    grid_coordinate(point.y, scale),
                ) else {
                    return Ok(None);
                };
                points.push((x, y));
            }
            let area: i64 = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(a, b)| i64::from(a.0) * i64::from(b.1) - i64::from(b.0) * i64::from(a.1))
                .sum();
            if area == 0 {
                return Ok(None);
            }
            pixels.push(points);
        }
        if pixels.iter().any(|contour| has_staircase(contour)) {
            evidence.insert(signature(&pixels));
        }
    }
    Ok((evidence.len() >= 3).then_some(PixelFontDetection {
        ppem: ppem as u16,
        grid_size: 1.0 / scale,
        glyph_count: font.glyphs.len(),
        distinctive_glyphs: evidence.len(),
    }))
}

fn plain_layer(layer: &Layer) -> Result<bool, Error> {
    if [
        Some(layer.width),
        layer.vertical_width,
        layer.vertical_origin,
    ]
    .into_iter()
    .flatten()
    .any(|v| !v.is_finite())
    {
        return Err(crate::invalid("non-finite pixel-font metric"));
    }
    if layer.components.iter().any(|component| {
        component
            .transform
            .as_coeffs()
            .iter()
            .any(|value| !value.is_finite())
    }) {
        return Err(crate::invalid("non-finite pixel-font component transform"));
    }
    if layer.effects.iter().any(|effect| effect.enabled)
        || layer
            .components
            .iter()
            .any(|component| component.smart.is_some())
    {
        return Ok(false);
    }
    for contour in &layer.contours {
        if contour
            .points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        {
            return Err(crate::invalid("non-finite pixel-font coordinate"));
        }
        if !contour.closed
            || contour.points.iter().any(|point| {
                !matches!(point.kind, PointKind::Line | PointKind::Move)
                    || point.corner_radius != 0.0
            })
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn orthogonal_outline(path: &BezPath, shortest: &mut f64) -> Result<Option<Outline>, Error> {
    if path.elements().len() > 100_000 {
        return Err(crate::invalid("pixel-font glyph outline work limit"));
    }
    let mut outline = Vec::new();
    let mut points = Vec::new();
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) if points.is_empty() => points.push(point),
            PathEl::LineTo(point) if !points.is_empty() => {
                if points.last() != Some(&point) {
                    points.push(point);
                }
            }
            PathEl::ClosePath if points.len() >= 3 => {
                if points.first() == points.last() {
                    points.pop();
                }
                if points.len() < 4 {
                    return Ok(None);
                }
                for index in 0..points.len() {
                    let point = points[index];
                    let next = points[(index + 1) % points.len()];
                    if !point.is_finite() {
                        return Err(crate::invalid("non-finite flattened pixel-font coordinate"));
                    }
                    let delta = next - point;
                    if delta.x.abs() > EPSILON && delta.y.abs() > EPSILON {
                        return Ok(None);
                    }
                    let length = delta.x.abs().max(delta.y.abs());
                    if length > EPSILON {
                        *shortest = shortest.min(length);
                    }
                }
                outline.push(std::mem::take(&mut points));
            }
            _ => return Ok(None),
        }
    }
    Ok(points.is_empty().then_some(outline))
}

fn grid_coordinate(value: f64, scale: f64) -> Option<i32> {
    let value = value * scale;
    let rounded = value.round();
    (value.is_finite()
        && (value - rounded).abs() <= EPSILON
        && rounded.abs() <= f64::from(i16::MAX))
    .then_some(rounded as i32)
}

fn has_staircase(contour: &[(i32, i32)]) -> bool {
    let count = contour.len();
    if count < 8 {
        return false;
    }
    let edge = |index: usize| {
        let a = contour[index % count];
        let b = contour[(index + 1) % count];
        (b.0 - a.0, b.1 - a.1)
    };
    (0..count).any(|index| {
        let a = edge(index);
        let b = edge(index + 1);
        a.0.abs() + a.1.abs() == 1
            && b.0.abs() + b.1.abs() == 1
            && a.0 * b.0 + a.1 * b.1 == 0
            && edge(index + 2) == a
            && edge(index + 3) == b
    })
}

// Ignore glyph name and translation when counting independent evidence.
fn signature(outline: &[Vec<(i32, i32)>]) -> Vec<(i32, i32)> {
    let x = outline
        .iter()
        .flatten()
        .map(|p| p.0)
        .min()
        .unwrap_or_default();
    let y = outline
        .iter()
        .flatten()
        .map(|p| p.1)
        .min()
        .unwrap_or_default();
    let mut contours = Vec::new();
    for contour in outline {
        let mut points = Vec::new();
        for index in 0..contour.len() {
            let previous = contour[(index + contour.len() - 1) % contour.len()];
            let current = contour[index];
            let next = contour[(index + 1) % contour.len()];
            if (previous.0 == current.0 && current.0 == next.0)
                || (previous.1 == current.1 && current.1 == next.1)
            {
                continue;
            }
            points.push((current.0 - x, current.1 - y));
        }
        let forward = canonical_rotation(&points);
        points.reverse();
        let reverse = canonical_rotation(&points);
        contours.push(forward.min(reverse));
    }
    contours.sort_unstable();
    let mut signature = Vec::new();
    for contour in contours {
        signature.push((i32::MIN, i32::MIN));
        signature.extend(contour);
    }
    signature
}

// Lexicographically smallest cyclic rotation, in linear time even with repeated
// vertices. Start-point order and winding must not turn clones into evidence.
fn canonical_rotation(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let count = points.len();
    if count == 0 {
        return Vec::new();
    }
    let (mut a, mut b, mut offset) = (0usize, 1usize, 0usize);
    while a < count && b < count && offset < count {
        let left = points[(a + offset) % count];
        let right = points[(b + offset) % count];
        match left.cmp(&right) {
            std::cmp::Ordering::Equal => offset += 1,
            std::cmp::Ordering::Greater => {
                a += offset + 1;
                if a == b {
                    a += 1;
                }
                offset = 0;
            }
            std::cmp::Ordering::Less => {
                b += offset + 1;
                if a == b {
                    b += 1;
                }
                offset = 0;
            }
        }
    }
    let start = a.min(b);
    (0..count)
        .map(|index| points[(start + index) % count])
        .collect()
}

/// Pixel filter used when a bitmap is copied to a different strike size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BitmapScaleFilter {
    /// Take the source sample under each destination pixel's centre.
    #[default]
    Nearest,
    /// Average the source over each destination pixel's exact footprint.
    AreaAverage,
}

/// Largest number of source-by-destination sample visits one rescale may make.
const MAX_SCALE_WORK: usize = 16_000_000;

/// Rescale one bitmap to new dimensions and depth, without touching origins.
///
/// Dimensions, depth and the resulting buffer obey the model's bitmap limits;
/// a zero-sized source or destination yields a correctly shaped empty image.
/// Coverage becomes black with the source alpha when the destination is RGBA,
/// and RGBA becomes its alpha when the destination stores coverage. Origins
/// and glyph metrics belong to the caller, which knows the strike scale.
pub fn scale_bitmap(
    source: &Bitmap,
    width: u16,
    height: u16,
    depth: u8,
    filter: BitmapScaleFilter,
) -> Result<Bitmap, Error> {
    source.validate()?;
    let mut out = Bitmap::new(width, height, depth)?;
    if width == 0 || height == 0 || source.width == 0 || source.height == 0 {
        return Ok(out);
    }
    let work = (usize::from(source.width) + usize::from(width))
        .saturating_mul(usize::from(source.height) + usize::from(height));
    if work > MAX_SCALE_WORK {
        return Err(crate::invalid("bitmap rescale work limit"));
    }
    let max = if depth == 32 {
        255
    } else {
        (1u32 << depth) - 1
    };
    for y in 0..height {
        for x in 0..width {
            let value = match filter {
                BitmapScaleFilter::Nearest => {
                    let sx = nearest(x, width, source.width);
                    let sy = nearest(y, height, source.height);
                    let coverage = u32::from(source.coverage(sx, sy));
                    if depth == 32 {
                        if source.depth == 32 {
                            source.pixel(sx, sy)
                        } else {
                            coverage
                        }
                    } else {
                        (coverage * max + 127) / 255
                    }
                }
                BitmapScaleFilter::AreaAverage => average(source, x, y, width, height, depth, max),
            };
            out.set_pixel(x, y, value)?;
        }
    }
    Ok(out)
}

/// The source index whose pixel contains the destination pixel's centre.
fn nearest(index: u16, count: u16, source: u16) -> u16 {
    (((u32::from(index) * 2 + 1) * u32::from(source)) / (u32::from(count) * 2))
        .min(u32::from(source.saturating_sub(1))) as u16
}

/// Overlap of `[low, high)` with source cell `index`, in source pixels.
fn overlap(index: usize, low: f64, high: f64) -> f64 {
    let start = (index as f64).max(low);
    let end = ((index + 1) as f64).min(high);
    (end - start).max(0.0)
}

/// Exact box filter over the destination pixel's source footprint.
fn average(source: &Bitmap, x: u16, y: u16, width: u16, height: u16, depth: u8, max: u32) -> u32 {
    let sx0 = f64::from(x) * f64::from(source.width) / f64::from(width);
    let sx1 = f64::from(x + 1) * f64::from(source.width) / f64::from(width);
    let sy0 = f64::from(y) * f64::from(source.height) / f64::from(height);
    let sy1 = f64::from(y + 1) * f64::from(source.height) / f64::from(height);
    let columns = (sx0.floor() as usize)..(sx1.ceil() as usize).min(usize::from(source.width));
    let rows = (sy0.floor() as usize)..(sy1.ceil() as usize).min(usize::from(source.height));
    let mut total = 0.0;
    let mut coverage = 0.0;
    let mut channels = [0.0f64; 3];
    for row in rows {
        let weight_y = overlap(row, sy0, sy1);
        if weight_y <= 0.0 {
            continue;
        }
        for column in columns.clone() {
            let weight = overlap(column, sx0, sx1) * weight_y;
            if weight <= 0.0 {
                continue;
            }
            total += weight;
            let alpha = f64::from(source.coverage(column as u16, row as u16));
            coverage += alpha * weight;
            if source.depth == 32 && depth == 32 {
                let rgba = source.pixel(column as u16, row as u16).to_be_bytes();
                for (channel, value) in channels.iter_mut().zip(rgba) {
                    // Premultiply so a transparent neighbour cannot tint the result.
                    *channel += f64::from(value) * alpha * weight;
                }
            }
        }
    }
    if total <= 0.0 {
        return 0;
    }
    let alpha = coverage / total;
    if depth != 32 {
        return (alpha * f64::from(max) / 255.0).round() as u32;
    }
    let out_alpha = alpha.round().clamp(0.0, 255.0) as u8;
    if source.depth != 32 {
        return u32::from(out_alpha);
    }
    if coverage <= 0.0 {
        return 0;
    }
    let mut bytes = [0u8; 4];
    for (byte, channel) in bytes.iter_mut().zip(channels) {
        *byte = (channel / coverage).round().clamp(0.0, 255.0) as u8;
    }
    bytes[3] = out_alpha;
    u32::from_be_bytes(bytes)
}
