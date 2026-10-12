use crate::{
    Api, ApiError, BitmapApplyParams, BitmapGlyphParams, BitmapGlyphView, BitmapState,
    BitmapStrikeInfo, Change, DocParams,
};

pub(crate) fn state(api: &Api, params: DocParams) -> Result<BitmapState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| BitmapState {
            generation: doc.generation,
            strikes: doc
                .font
                .bitmap_strikes
                .iter()
                .map(|s| BitmapStrikeInfo {
                    ppem: s.ppem,
                    depth: s.depth,
                    ascent: s.ascent,
                    descent: s.descent,
                    properties: s.properties.clone(),
                    glyph_count: s.glyphs.len() as u32,
                })
                .collect(),
        })
        .map_err(ApiError::from)
}

pub(crate) fn glyph(api: &Api, params: BitmapGlyphParams) -> Result<BitmapGlyphView, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            if doc.font.glyph(&params.glyph).is_none() {
                return Err(ApiError::not_found("glyph does not exist"));
            }
            let strike = doc
                .font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (params.ppem, params.depth))
                .ok_or_else(|| ApiError::not_found("bitmap strike does not exist"))?;
            let value = strike.glyphs.get(params.glyph.as_str()).cloned();
            let resolved = if value.is_some() {
                Some(
                    strike
                        .flattened(&params.glyph.into())
                        .map_err(|e| ApiError::invalid_params(e.to_string()))?,
                )
            } else {
                None
            };
            Ok(BitmapGlyphView {
                generation: doc.generation,
                value,
                resolved,
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn apply(api: &Api, params: BitmapApplyParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(ApiError::invalid_params(
                    "bitmap document changed; refresh before editing",
                ));
            }
            doc.apply(fc_doc::Command::Bitmap(params.command), params.label)
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn rasterize(api: &Api, params: crate::BitmapRasterParams) -> Result<Change, ApiError> {
    let font = api
        .session
        .with_doc(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(ApiError::invalid_params(
                    "document changed; refresh before rasterizing",
                ));
            }
            Ok(doc.font.clone())
        })
        .map_err(ApiError::from)??;
    if let Some(blocker) = fc_doc::problems::compilation_blocker(&font) {
        return Err(ApiError::invalid_params(blocker.message));
    }
    let master = params
        .master
        .unwrap_or_else(|| font.default_master().id.clone());
    let names = if params.glyphs.is_empty() {
        font.glyph_order().map(ToString::to_string).collect()
    } else {
        params.glyphs
    };
    let paths = match params.hinting {
        None | Some(crate::BitmapHinting::None) => None,
        Some(mode) => {
            let generations = api
                .session
                .with_doc(params.doc, |doc| {
                    doc.font
                        .glyph_order()
                        .map(|n| {
                            (
                                n.clone(),
                                doc.glyph_id(n).map_or(0, |id| doc.glyph_generation(id)),
                            )
                        })
                        .collect::<std::collections::HashMap<_, _>>()
                })
                .map_err(ApiError::from)?;
            let mut outlines = font.clone();
            outlines.bitmap_strikes.clear();
            outlines.bitmap_tables_owned = true;
            let report = fc_compile::export(
                &outlines,
                &|name| generations.get(name).copied().unwrap_or(0),
                &fc_compile::ExportOptions {
                    master: Some(master.clone()),
                    ..Default::default()
                },
            )?;
            let decoded = fc_sfnt::read_bytes(
                &report.bytes,
                &fc_sfnt::ReadOptions {
                    decode_bitmaps: false,
                    keep_original_bytes: false,
                    ..Default::default()
                },
            )?;
            let compiled = decoded
                .fonts
                .first()
                .ok_or_else(|| ApiError::invalid_params("compiled font is empty"))?;
            let glyphs = names
                .iter()
                .map(|name| {
                    compiled
                        .glyphs
                        .get_index_of(name.as_str())
                        .map(|gid| (name.clone(), gid as u32))
                        .ok_or_else(|| {
                            ApiError::not_found(format!("{name} is not in the compiled font"))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Some(
                fc_raster::hinted_paths(
                    &report.bytes,
                    &glyphs,
                    params.ppem,
                    params.depth == 1,
                    matches!(mode, crate::BitmapHinting::Auto),
                )
                .map_err(|e| ApiError::invalid_params(e.to_string()))?,
            )
        }
    };
    let mut strike = fc_raster::rasterize_options(
        &font,
        &master,
        &names,
        fc_raster::RasterOptions {
            ppem: params.ppem,
            depth: params.depth,
            threshold: params.threshold.unwrap_or(128),
        },
        paths.as_ref(),
    )
    .map_err(|e| ApiError::invalid_params(e.to_string()))?;
    if let Some(existing) = font
        .bitmap_strikes
        .iter()
        .find(|s| (s.ppem, s.depth) == (params.ppem, params.depth))
    {
        let mut merged = existing.clone();
        merged.glyphs.extend(strike.glyphs);
        strike = merged;
    }
    apply(
        api,
        crate::BitmapApplyParams {
            doc: params.doc,
            expected_generation: params.expected_generation,
            command: fc_doc::command::BitmapCommand::SetStrike { strike },
            label: "Rasterize bitmap strike".into(),
        },
    )
}

/// Smallest pixel size offered as a grid preset.
///
/// Four pixels per em is also the floor `detect_pixel_font` accepts, and below
/// it a "grid" has no shape to speak of. The ceiling is the units per em
/// itself, bounded by the largest strike the model stores: one font unit per
/// pixel is the finest grid that means anything, since a pixel smaller than a
/// unit can no longer land on integer coordinates.
const GRID_MIN_PPEM: u16 = 4;

/// Exact pixel grids for the document's units per em, with strike metrics.
///
/// A grid is exact when one pixel is a whole number of font units, so pixel
/// edges land on integer coordinates and a traced outline needs no rounding.
/// A requested size is reported with its own verdict and, when it is not
/// exact, the nearest size that is.
pub(crate) fn grid(
    api: &Api,
    params: crate::BitmapGridParams,
) -> Result<crate::BitmapGridInfo, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let upm = doc.font.info.units_per_em;
            if upm == 0 {
                return Err(ApiError::invalid_params("units per em must be nonzero"));
            }
            let master = match &params.master {
                Some(id) => doc
                    .font
                    .master(id)
                    .ok_or_else(|| ApiError::not_found("master does not exist"))?,
                None => doc.font.default_master(),
            };
            let (ascender, descender) = (master.metrics.ascender, master.metrics.descender);
            let choice = |ppem: u16| -> crate::BitmapGridChoice {
                let scale = f64::from(ppem) / f64::from(upm);
                let metric = |value: f64| -> i16 {
                    let scaled = (value * scale).round();
                    if scaled.is_finite() {
                        scaled.clamp(0.0, f64::from(i16::MAX)) as i16
                    } else {
                        0
                    }
                };
                crate::BitmapGridChoice {
                    ppem,
                    units_per_pixel: f64::from(upm) / f64::from(ppem),
                    exact: upm % ppem == 0,
                    ascent: metric(ascender),
                    descent: metric(-descender),
                }
            };
            let presets = (GRID_MIN_PPEM..=upm.min(fc_model::MAX_BITMAP_SIZE))
                .filter(|ppem| upm % ppem == 0)
                .map(choice)
                .collect::<Vec<_>>();
            let requested = params.ppem.filter(|ppem| *ppem > 0).map(choice);
            // The nearest exact size, preferring the smaller one on a tie so
            // the suggestion never silently grows the strike.
            let nearest = requested.filter(|choice| !choice.exact).and_then(|choice| {
                presets
                    .iter()
                    .map(|preset| preset.ppem)
                    .min_by_key(|ppem| (ppem.abs_diff(choice.ppem), *ppem))
            });
            Ok(crate::BitmapGridInfo {
                generation: doc.generation,
                units_per_em: upm,
                presets,
                requested,
                nearest,
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn trace(api: &Api, params: crate::BitmapTraceParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(ApiError::invalid_params(
                    "document changed; refresh before tracing",
                ));
            }
            let strike = doc
                .font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (params.ppem, params.depth))
                .ok_or_else(|| ApiError::not_found("strike does not exist"))?;
            // One undo step covers the whole selection: the single `glyph` plus
            // any extra names, in a stable order and without duplicates.
            let mut selected = Vec::new();
            let all = if params.all_images {
                strike.glyphs.keys().map(ToString::to_string).collect()
            } else {
                Vec::new()
            };
            for name in std::iter::once(&params.glyph)
                .chain(params.glyphs.iter())
                .chain(all.iter())
            {
                if !selected.contains(name) {
                    selected.push(name.clone());
                }
            }
            if selected.len() > 4096 {
                return Err(ApiError::invalid_params(
                    "trace at most 4096 glyphs per operation",
                ));
            }
            let mode = match params.mode.unwrap_or_default() {
                crate::BitmapTraceMode::PixelEdges => fc_raster::BitmapTraceMode::PixelEdges,
                crate::BitmapTraceMode::ChamferedCorners => {
                    fc_raster::BitmapTraceMode::ChamferedCorners
                }
                crate::BitmapTraceMode::RoundedCorners => {
                    fc_raster::BitmapTraceMode::RoundedCorners
                }
                crate::BitmapTraceMode::DiagonalSmoothing => {
                    fc_raster::BitmapTraceMode::DiagonalSmoothing
                }
            };
            let master = params
                .master
                .unwrap_or_else(|| doc.font.default_master().id.clone());
            let mut commands = Vec::with_capacity(selected.len() * 2);
            for name in &selected {
                let glyph = strike
                    .flattened(&name.as_str().into())
                    .map_err(|e| ApiError::invalid_params(e.to_string()))?;
                let contours = fc_raster::trace_bitmap_with_options(
                    &glyph.bitmap,
                    params.ppem,
                    doc.font.info.units_per_em,
                    params.threshold,
                    fc_raster::BitmapTraceOptions {
                        mode,
                        join_diagonals: params.join_diagonals.unwrap_or(false),
                    },
                )
                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
                let layer = doc
                    .font
                    .glyph(name)
                    .and_then(|g| g.master_layer(&master))
                    .ok_or_else(|| ApiError::not_found("outline layer does not exist"))?;
                commands.push(fc_doc::Command::ReplaceLayerOutline {
                    glyph: name.as_str().into(),
                    layer: layer.id.clone(),
                    contours,
                    components: Vec::new(),
                    anchors: layer.anchors.clone(),
                });
                commands.push(fc_doc::Command::SetWidth {
                    glyph: name.as_str().into(),
                    layer: layer.id.clone(),
                    width: f64::from(glyph.advance) * f64::from(doc.font.info.units_per_em)
                        / f64::from(params.ppem),
                });
            }
            doc.apply(fc_doc::Command::Batch(commands), "Trace bitmap outlines")
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn preview(
    api: &Api,
    params: crate::BitmapPreviewParams,
) -> Result<Vec<crate::BitmapPreviewLine>, ApiError> {
    if params.text.chars().count() > 128 || params.glyphs.len() > 128 {
        return Err(ApiError::invalid_params(
            "waterfall sample exceeds 128 entries",
        ));
    }
    api.session
        .with_doc(params.doc, |doc| {
            let mut bytes = 0usize;
            let requested = params
                .text
                .chars()
                .map(|c| c as u32)
                .collect::<std::collections::HashSet<_>>();
            let mut names = std::collections::HashMap::new();
            for glyph in doc.font.glyphs.values() {
                for code in &glyph.unicodes {
                    if requested.contains(code) {
                        names.entry(*code).or_insert(&glyph.name);
                    }
                }
            }
            // Named glyphs win over the text, so a glyph with no Unicode value
            // — `.notdef`, a stylistic alternate, the reusable pixel glyph —
            // can still be compared across the strikes.
            let selection: Vec<Option<fc_model::GlyphName>> = if params.glyphs.is_empty() {
                params
                    .text
                    .chars()
                    .map(|ch| names.get(&(ch as u32)).map(|name| (*name).clone()))
                    .collect()
            } else {
                params
                    .glyphs
                    .iter()
                    .map(|name| {
                        doc.font
                            .glyph(name)
                            .map(|glyph| glyph.name.clone())
                            .ok_or_else(|| {
                                ApiError::not_found(format!("glyph `{name}` does not exist"))
                            })
                    })
                    .map(|found| found.map(Some))
                    .collect::<Result<_, ApiError>>()?
            };
            doc.font
                .bitmap_strikes
                .iter()
                .map(|strike| {
                    let glyphs = selection
                        .iter()
                        .map(|entry| {
                            let name = entry.as_ref();
                            let Some(name) = name.filter(|n| strike.glyphs.contains_key(*n)) else {
                                return Ok(None);
                            };
                            let mut glyph = strike
                                .flattened(name)
                                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
                            // Match glyph-cell vertical inheritance, but keep an
                            // explicit strike metric ahead of outline/VORG data.
                            // Resolving this cloned preview never edits the font.
                            let master = doc.font.default_master();
                            let layer = doc
                                .font
                                .glyph(name)
                                .and_then(|g| g.master_layer(&master.id));
                            let scale =
                                f64::from(strike.ppem) / f64::from(doc.font.info.units_per_em);
                            let pixel_metric = |value: f64| {
                                let value = value.round();
                                if !value.is_finite()
                                    || !(f64::from(i16::MIN)..=f64::from(i16::MAX)).contains(&value)
                                {
                                    return Err(ApiError::invalid_params(
                                        "vertical preview metric exceeds pixel range",
                                    ));
                                }
                                Ok(value as i16)
                            };
                            if glyph.vertical_advance.is_none() {
                                let advance = layer
                                    .and_then(|l| l.vertical_width)
                                    .or(master.metrics.default_vertical_advance)
                                    .unwrap_or(f64::from(doc.font.info.units_per_em));
                                glyph.vertical_advance = Some(pixel_metric(advance * scale)?);
                            }
                            if glyph.vertical_origin.is_none() {
                                let origin = layer
                                    .and_then(|l| l.vertical_origin)
                                    .or(master.metrics.default_vertical_origin)
                                    .unwrap_or(master.metrics.ascender);
                                glyph.vertical_origin = Some((
                                    pixel_metric(f64::from(glyph.advance) / 2.0)?,
                                    pixel_metric(origin * scale)?,
                                ));
                            }
                            bytes += glyph.bitmap.rows.len();
                            if bytes > 16 * 1024 * 1024 {
                                return Err(ApiError::invalid_params(
                                    "waterfall response exceeds pixel budget",
                                ));
                            }
                            Ok(Some(glyph))
                        })
                        .collect::<Result<_, ApiError>>()?;
                    Ok(crate::BitmapPreviewLine {
                        ppem: strike.ppem,
                        depth: strike.depth,
                        ascent: strike.ascent,
                        descent: strike.descent,
                        glyphs,
                    })
                })
                .collect()
        })
        .map_err(ApiError::from)?
}

pub(crate) fn configure(
    api: &Api,
    params: crate::BitmapConfigureParams,
) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(ApiError::invalid_params(
                    "document changed; reload strike metadata",
                ));
            }
            let mut strike = doc
                .font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (params.ppem, params.depth))
                .cloned()
                .ok_or_else(|| ApiError::not_found("strike does not exist"))?;
            strike.ascent = params.ascent;
            strike.descent = params.descent;
            strike.properties = params.properties;
            doc.apply(
                fc_doc::Command::Bitmap(fc_doc::command::BitmapCommand::SetStrike { strike }),
                "Edit bitmap strike properties",
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}
