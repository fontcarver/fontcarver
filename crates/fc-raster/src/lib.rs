//! Bounded outline rasterization and exact pixel-edge tracing.
#![warn(missing_docs)]
mod hint;
mod trace;
use fc_model::{Bitmap, BitmapGlyph, BitmapStrike, Font, MasterId};
pub use hint::hinted_paths;
use kurbo::{Affine, PathEl, Shape};
pub use trace::trace_bitmap;

/// A rasterization or tracing input cannot be represented safely.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid parameters, metrics or resource budget.
    #[error("bitmap operation: {0}")]
    Invalid(String),
    /// Invalid model geometry or bitmap data.
    #[error(transparent)]
    Model(#[from] fc_model::Error),
}
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}

/// Coverage rasterization settings shared by model and hinted paths.
#[derive(Clone, Copy, Debug)]
pub struct RasterOptions {
    /// Pixels per em.
    pub ppem: u16,
    /// Coverage bit depth.
    pub depth: u8,
    /// Minimum coverage for monochrome output, 1..255.
    pub threshold: u8,
}

/// Rasterize a selected master with analytical antialiasing, without hinting.
pub fn rasterize(
    font: &Font,
    master: &MasterId,
    ppem: u16,
    depth: u8,
    names: &[String],
) -> Result<BitmapStrike, Error> {
    rasterize_options(
        font,
        master,
        names,
        RasterOptions {
            ppem,
            depth,
            threshold: 128,
        },
        None,
    )
}

/// Rasterize model outlines, or supplied grid-fitted paths in pixel coordinates.
pub fn rasterize_options(
    font: &Font,
    master: &MasterId,
    names: &[String],
    options: RasterOptions,
    paths: Option<&std::collections::HashMap<String, kurbo::BezPath>>,
) -> Result<BitmapStrike, Error> {
    let RasterOptions {
        ppem,
        depth,
        threshold,
    } = options;
    if threshold == 0 {
        return Err(invalid("monochrome threshold must be at least 1"));
    }
    if ppem == 0 || ppem > 4096 || !matches!(depth, 1 | 2 | 4 | 8) || font.info.units_per_em == 0 {
        return Err(invalid("invalid size, depth or units per em"));
    }
    let source = font
        .master(master)
        .ok_or_else(|| invalid("missing master"))?;
    let scale = f64::from(ppem) / f64::from(font.info.units_per_em);
    let metric = |v: f64| -> Result<i16, Error> {
        let n = (v * scale).round();
        if !n.is_finite() || n < i16::MIN as f64 || n > i16::MAX as f64 {
            Err(invalid("pixel metric overflow"))
        } else {
            Ok(n as i16)
        }
    };
    let mut strike = BitmapStrike {
        ppem,
        depth,
        ascent: metric(source.metrics.ascender)?.max(0),
        descent: metric(-source.metrics.descender)?.max(0),
        ..Default::default()
    };
    let mut total = 0usize;
    for name in names {
        let glyph = font.glyph(name).ok_or_else(|| invalid("missing glyph"))?;
        let layer = glyph
            .master_layer(master)
            .ok_or_else(|| invalid("missing master layer"))?;
        check_work(
            font,
            name,
            master,
            &mut std::collections::HashSet::new(),
            &mut std::collections::HashMap::new(),
        )?;
        let path = if let Some(paths) = paths {
            paths
                .get(name)
                .cloned()
                .ok_or_else(|| invalid("missing hinted glyph path"))?
        } else {
            Affine::scale(scale) * fc_model::flatten_layer(font, name, master)?
        };
        if path.elements().len() > 100_000 {
            return Err(invalid("glyph rasterization work limit"));
        }
        let bitmap = if path.elements().is_empty() {
            Bitmap::new(0, 0, depth)?
        } else {
            let bounds = path.bounding_box();
            let x = bounds.x0.floor();
            let y = bounds.y0.floor();
            let right = bounds.x1.ceil();
            let top = bounds.y1.ceil();
            if [x, y, right, top]
                .iter()
                .any(|v| !v.is_finite() || *v < i16::MIN as f64 || *v > i16::MAX as f64)
            {
                return Err(invalid("raster bounds overflow"));
            }
            let width = right - x;
            let height = top - y;
            if width > 4096.0 || height > 4096.0 || width * height > 4_000_000.0 {
                return Err(invalid("raster dimensions exceed limit"));
            }
            let mut bitmap = Bitmap::new(width as u16, height as u16, depth)?;
            bitmap.origin_x = x as i16;
            bitmap.origin_y = y as i16;
            if width > 0.0 && height > 0.0 {
                let mut builder = tiny_skia::PathBuilder::new();
                let point = |p: kurbo::Point| ((p.x - x) as f32, (top - p.y) as f32);
                for element in path.elements() {
                    match element {
                        PathEl::MoveTo(p) => {
                            let (x, y) = point(*p);
                            builder.move_to(x, y);
                        }
                        PathEl::LineTo(p) => {
                            let (x, y) = point(*p);
                            builder.line_to(x, y);
                        }
                        PathEl::QuadTo(a, b) => {
                            let (ax, ay) = point(*a);
                            let (bx, by) = point(*b);
                            builder.quad_to(ax, ay, bx, by);
                        }
                        PathEl::CurveTo(a, b, c) => {
                            let (ax, ay) = point(*a);
                            let (bx, by) = point(*b);
                            let (cx, cy) = point(*c);
                            builder.cubic_to(ax, ay, bx, by, cx, cy);
                        }
                        PathEl::ClosePath => builder.close(),
                    }
                }
                if let Some(path) = builder.finish() {
                    let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32)
                        .ok_or_else(|| invalid("raster allocation failed"))?;
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(0, 0, 0, 255);
                    paint.anti_alias = true;
                    pixmap.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                    let max = (1u32 << depth) - 1;
                    for row in 0..bitmap.height {
                        for col in 0..bitmap.width {
                            let alpha = u32::from(
                                pixmap.data()[(usize::from(row) * usize::from(bitmap.width)
                                    + usize::from(col))
                                    * 4
                                    + 3],
                            );
                            bitmap.set_pixel(
                                col,
                                row,
                                if depth == 1 {
                                    u32::from(alpha >= u32::from(threshold))
                                } else {
                                    (alpha * max + 127) / 255
                                },
                            )?;
                        }
                    }
                }
            }
            bitmap
        };
        total += bitmap.rows.len();
        if total > fc_model::MAX_FONT_BITMAP_BYTES {
            return Err(invalid("strike raster memory limit"));
        }
        strike.glyphs.insert(
            name.as_str().into(),
            BitmapGlyph {
                bitmap,
                advance: metric(layer.width)?,
                vertical_advance: layer.vertical_width.map(metric).transpose()?,
                vertical_origin: layer
                    .vertical_origin
                    .map(|y| Ok::<_, Error>((metric(layer.width / 2.0)?, metric(y)?)))
                    .transpose()?,
                refs: Vec::new(),
            },
        );
    }
    strike.validate()?;
    Ok(strike)
}

// Count expanded work on the DAG before recursive flattening allocates paths.
fn check_work(
    font: &Font,
    name: &str,
    master: &MasterId,
    active: &mut std::collections::HashSet<String>,
    memo: &mut std::collections::HashMap<String, usize>,
) -> Result<usize, Error> {
    if let Some(count) = memo.get(name) {
        return Ok(*count);
    }
    if active.len() >= 64 || !active.insert(name.into()) {
        return Err(invalid("component cycle/depth limit"));
    }
    let layer = font
        .glyph(name)
        .and_then(|g| g.master_layer(master))
        .ok_or_else(|| invalid("missing component layer"))?;
    let mut count = 1 + layer
        .contours
        .iter()
        .map(|c| c.points.len() + 1)
        .sum::<usize>();
    for component in &layer.components {
        count = count.saturating_add(check_work(font, &component.base, master, active, memo)?);
        if count > 100_000 {
            return Err(invalid("expanded outline work limit"));
        }
    }
    if count > 100_000 {
        return Err(invalid("outline work limit"));
    }
    active.remove(name);
    memo.insert(name.into(), count);
    Ok(count)
}
