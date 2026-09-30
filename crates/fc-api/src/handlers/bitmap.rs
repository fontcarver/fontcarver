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
            let glyph = strike
                .flattened(&params.glyph.as_str().into())
                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
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
            let master = params
                .master
                .unwrap_or_else(|| doc.font.default_master().id.clone());
            let layer = doc
                .font
                .glyph(&params.glyph)
                .and_then(|g| g.master_layer(&master))
                .ok_or_else(|| ApiError::not_found("outline layer does not exist"))?;
            let command = fc_doc::Command::Batch(vec![
                fc_doc::Command::ReplaceLayerOutline {
                    glyph: params.glyph.as_str().into(),
                    layer: layer.id.clone(),
                    contours,
                    components: Vec::new(),
                    anchors: layer.anchors.clone(),
                },
                fc_doc::Command::SetWidth {
                    glyph: params.glyph.into(),
                    layer: layer.id.clone(),
                    width: f64::from(glyph.advance) * f64::from(doc.font.info.units_per_em)
                        / f64::from(params.ppem),
                },
            ]);
            doc.apply(command, "Trace bitmap outlines")
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn preview(
    api: &Api,
    params: crate::BitmapPreviewParams,
) -> Result<Vec<crate::BitmapPreviewLine>, ApiError> {
    if params.text.chars().count() > 128 {
        return Err(ApiError::invalid_params(
            "waterfall text exceeds 128 characters",
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
            doc.font
                .bitmap_strikes
                .iter()
                .map(|strike| {
                    let glyphs = params
                        .text
                        .chars()
                        .map(|ch| {
                            let name = names.get(&(ch as u32)).copied();
                            let Some(name) = name.filter(|n| strike.glyphs.contains_key(*n)) else {
                                return Ok(None);
                            };
                            let glyph = strike
                                .flattened(name)
                                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
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
