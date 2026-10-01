//! Atomic cross-strike transfers and editable pixel construction.
use crate::{Api, ApiError, BitmapPixelComponentsParams, BitmapTransferParams, Change};
use fc_doc::{command::BitmapCommand, Command};
use fc_model::{BitmapGlyph, BitmapStrike, Component, Contour, Glyph, Layer, Point, PointKind};
use std::collections::BTreeSet;

fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::invalid_params(message)
}
fn metric(value: i16, factor: f64) -> Result<i16, ApiError> {
    let scaled = (f64::from(value) * factor).round();
    if !scaled.is_finite() || scaled < f64::from(i16::MIN) || scaled > f64::from(i16::MAX) {
        return Err(invalid("scaled bitmap metric exceeds range"));
    }
    Ok(scaled as i16)
}
/// The glyph-name shape every FontCarver glyph source accepts.
///
/// A new pixel glyph becomes a real glyph in the font and is written to `.fcf`,
/// to UFO and to the compiled `post` table, so a name with a space or a slash
/// in it would only fail much later. The rule matches the Add Glyphs recipe
/// grammar: an ASCII letter, `_` or `.` first, then letters, digits, `.` or `_`.
fn conventional_glyph_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '.')
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_'))
}

fn names(strike: &BitmapStrike, selection: Vec<String>) -> Result<Vec<String>, ApiError> {
    let selected: BTreeSet<String> = if selection.is_empty() {
        strike.glyphs.keys().map(ToString::to_string).collect()
    } else {
        selection.into_iter().collect()
    };
    if selected.is_empty() || selected.len() > 4096 {
        return Err(invalid("select 1–4096 bitmap glyphs"));
    }
    Ok(selected.into_iter().collect())
}

pub(crate) fn transfer(api: &Api, params: BitmapTransferParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(invalid("document changed; refresh before copying pixels"));
            }
            if params.target_ppem == 0
                || params.target_ppem > 4096
                || !matches!(params.target_depth, 1 | 2 | 4 | 8 | 32)
            {
                return Err(invalid("invalid target size or depth"));
            }
            let source = doc
                .font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (params.source_ppem, params.source_depth))
                .ok_or_else(|| invalid("source strike does not exist"))?;
            if (source.ppem, source.depth) == (params.target_ppem, params.target_depth) {
                return Err(invalid("choose a different target strike"));
            }
            let factor = if params.scale {
                f64::from(params.target_ppem) / f64::from(source.ppem)
            } else {
                1.0
            };
            let mut target = doc
                .font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (params.target_ppem, params.target_depth))
                .cloned()
                .unwrap_or(BitmapStrike {
                    ppem: params.target_ppem,
                    depth: params.target_depth,
                    ascent: metric(source.ascent, factor)?,
                    descent: metric(source.descent, factor)?,
                    ..Default::default()
                });
            let filter = fc_raster::BitmapScaleFilter::from(params.filter.unwrap_or_default());
            let mut work = 0usize;
            for name in names(source, params.glyphs)? {
                let value = source
                    .flattened(&name.as_str().into())
                    .map_err(|e| invalid(e.to_string()))?;
                let src = &value.bitmap;
                let dimension = |n: u16| -> Result<u16, ApiError> {
                    let scaled = if n == 0 {
                        0.0
                    } else {
                        (f64::from(n) * factor).round().max(1.0)
                    };
                    if scaled > 4096.0 {
                        Err(invalid("scaled bitmap dimensions exceed limit"))
                    } else {
                        Ok(scaled as u16)
                    }
                };
                let width = dimension(src.width)?;
                let height = dimension(src.height)?;
                work = work.saturating_add(usize::from(width) * usize::from(height));
                if work > 16_000_000 {
                    return Err(invalid("cross-strike pixel work limit"));
                }
                let mut bitmap =
                    fc_raster::scale_bitmap(src, width, height, params.target_depth, filter)
                        .map_err(|e| invalid(e.to_string()))?;
                bitmap.origin_x = metric(src.origin_x, factor)?;
                bitmap.origin_y = metric(src.origin_y, factor)?;
                target.glyphs.insert(
                    name.into(),
                    BitmapGlyph {
                        bitmap,
                        advance: metric(value.advance, factor)?,
                        vertical_advance: value
                            .vertical_advance
                            .map(|v| metric(v, factor))
                            .transpose()?,
                        vertical_origin: value
                            .vertical_origin
                            .map(|(x, y)| {
                                Ok::<_, ApiError>((metric(x, factor)?, metric(y, factor)?))
                            })
                            .transpose()?,
                        refs: vec![],
                    },
                );
            }
            doc.apply(
                Command::Bitmap(BitmapCommand::SetStrike { strike: target }),
                "Copy bitmap strike pixels",
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn pixel_components(
    api: &Api,
    params: BitmapPixelComponentsParams,
) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(invalid(
                    "document changed; refresh before building components",
                ));
            }
            if params.threshold == 0
                || params.pixel_glyph.is_empty()
                || params.pixel_glyph.len() > 255
            {
                return Err(invalid("invalid pixel glyph name or threshold"));
            }
            let master = params
                .master
                .unwrap_or_else(|| doc.font.default_master().id.clone());
            if doc.font.master(&master).is_none() {
                return Err(invalid("master does not exist"));
            }
            let source = doc
                .font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (params.ppem, params.depth))
                .ok_or_else(|| invalid("source strike does not exist"))?;
            let selected = names(source, params.glyphs)?;
            if selected.contains(&params.pixel_glyph) {
                return Err(invalid("pixel glyph cannot reference itself"));
            }
            let upm = f64::from(doc.font.info.units_per_em);
            if upm == 0.0 {
                return Err(invalid("units per em must be nonzero"));
            }
            let mut commands = Vec::new();
            if doc.font.glyph(&params.pixel_glyph).is_none()
                && !conventional_glyph_name(&params.pixel_glyph)
            {
                return Err(invalid(
                    "a new pixel glyph name must start with a letter, `_` or `.` and use only letters, digits, `.` and `_`",
                ));
            }
            if let Some(pixel) = doc.font.glyph(&params.pixel_glyph) {
                let layer = pixel
                    .master_layer(&master)
                    .ok_or_else(|| invalid("pixel glyph has no selected master layer"))?;
                if layer.contours.is_empty() || !layer.components.is_empty() {
                    return Err(invalid(
                        "pixel glyph must contain a nonempty direct outline",
                    ));
                }
            } else {
                let mut pixel = Glyph::new(params.pixel_glyph.as_str());
                for m in &doc.font.masters {
                    let mut layer = Layer::new_master(m.id.clone());
                    layer.width = upm;
                    layer.contours.push(Contour {
                        closed: true,
                        points: [(0.0, 0.0), (upm, 0.0), (upm, upm), (0.0, upm)]
                            .into_iter()
                            .map(|(x, y)| Point {
                                x,
                                y,
                                kind: PointKind::Line,
                                ..Default::default()
                            })
                            .collect(),
                        ..Default::default()
                    });
                    pixel.layers.push(layer);
                }
                commands.push(Command::AddGlyph { glyph: pixel });
            }
            let mut work = 0usize;
            let mut count = 0usize;
            let unit = upm / f64::from(params.ppem);
            for name in selected {
                let value = source
                    .flattened(&name.as_str().into())
                    .map_err(|e| invalid(e.to_string()))?;
                let bitmap = &value.bitmap;
                work = work.saturating_add(usize::from(bitmap.width) * usize::from(bitmap.height));
                if work > 4_000_000 {
                    return Err(invalid("pixel component work limit"));
                }
                let layer = doc
                    .font
                    .glyph(&name)
                    .and_then(|g| g.master_layer(&master))
                    .ok_or_else(|| invalid("destination outline layer does not exist"))?;
                let mut components = Vec::new();
                for y in 0..bitmap.height {
                    for x in 0..bitmap.width {
                        if bitmap.coverage(x, y) < params.threshold {
                            continue;
                        }
                        count += 1;
                        if count > 100_000 || components.len() >= 16_384 {
                            return Err(invalid("pixel component count limit"));
                        }
                        let ox = (f64::from(bitmap.origin_x) + f64::from(x)) * unit;
                        let oy =
                            (f64::from(bitmap.origin_y) + f64::from(bitmap.height - y - 1)) * unit;
                        components.push(Component {
                            base: params.pixel_glyph.as_str().into(),
                            transform: kurbo::Affine::new([
                                1.0 / f64::from(params.ppem),
                                0.0,
                                0.0,
                                1.0 / f64::from(params.ppem),
                                ox,
                                oy,
                            ]),
                            ..Default::default()
                        });
                    }
                }
                commands.push(Command::ReplaceLayerOutline {
                    glyph: name.as_str().into(),
                    layer: layer.id.clone(),
                    contours: vec![],
                    components,
                    anchors: layer.anchors.clone(),
                });
                commands.push(Command::SetWidth {
                    glyph: name.into(),
                    layer: layer.id.clone(),
                    width: f64::from(value.advance) * unit,
                });
            }
            doc.apply(Command::Batch(commands), "Build editable pixel components")
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn detect(
    api: &Api,
    params: crate::BitmapDetectParams,
) -> Result<crate::BitmapPixelDetection, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let master = params
                .master
                .unwrap_or_else(|| doc.font.default_master().id.clone());
            let detection = fc_raster::detect_pixel_font(&doc.font, &master)
                .map_err(|e| invalid(e.to_string()))?;
            Ok(crate::BitmapPixelDetection {
                generation: doc.generation,
                ppem: detection.map(|d| d.ppem),
                units_per_pixel: detection.map(|d| d.grid_size),
                evidence_glyphs: detection.map_or(0, |d| d.distinctive_glyphs as u32),
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn reconstruct(
    api: &Api,
    params: crate::BitmapReconstructParams,
) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(invalid("document changed; detect the grid again"));
            }
            let master = params
                .master
                .unwrap_or_else(|| doc.font.default_master().id.clone());
            let selected = if params.glyphs.is_empty() {
                doc.font
                    .glyph_order()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            } else {
                params.glyphs
            };
            if selected.len() > 4096 {
                return Err(invalid("reconstruct at most 4096 glyphs per operation"));
            }
            let mut strike = fc_raster::reconstruct_pixel_font(&doc.font, &master, &selected)
                .map_err(|e| invalid(e.to_string()))?
                .ok_or_else(|| invalid("no supported exact pixel grid detected"))?;
            if let Some(existing) = doc
                .font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (strike.ppem, strike.depth))
            {
                let mut merged = existing.clone();
                merged.glyphs.extend(strike.glyphs);
                strike = merged;
            }
            doc.apply(
                Command::Bitmap(BitmapCommand::SetStrike { strike }),
                "Reconstruct pixel-font strike",
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}
