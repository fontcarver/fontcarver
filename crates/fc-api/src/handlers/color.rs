//! COLR/CPAL reads and previews never mutate the document; edits are revision-checked.

use fc_model::{ColorTable, Font, GlyphName, Tag};

use crate::{
    Api, ApiError, Change, ColorApplyParams, ColorDecodeParams, ColorPreview, ColorPreviewLayer,
    ColorPreviewParams, ColorState, ColorStatus, DocParams,
};

const COLR: Tag = Tag(*b"COLR");
const CPAL: Tag = Tag(*b"CPAL");
const MAX_PREVIEW_PARTS: usize = 1_024;

/// Decode imported bytes against the glyph order they were written for.
///
/// Legacy documents keep raw COLR/CPAL with import provenance. Glyph IDs refer to
/// the imported order, so the result must also resolve in the current font.
fn decode_imported(font: &Font) -> Option<Result<ColorTable, String>> {
    if font.color_tables_owned || font.color.is_some() {
        return None;
    }
    let colr = font.extra.raw_tables.get(&COLR);
    let cpal = font.extra.raw_tables.get(&CPAL);
    if colr.is_none() && cpal.is_none() {
        return None;
    }
    let Some(cpal) = cpal else {
        return Some(Err("missing CPAL table".into()));
    };
    let Some(order) = font
        .extra
        .source
        .as_ref()
        .map(|source| &source.glyph_order)
        .filter(|order| !order.is_empty())
    else {
        return Some(Err(
            "the original glyph order of the imported table is unknown".into(),
        ));
    };
    let decoded = match fc_sfnt::color::read_tables(colr.map(Vec::as_slice), cpal, order) {
        Ok(color) => color,
        Err(error) => return Some(Err(error.to_string())),
    };
    let current: Vec<GlyphName> = font.glyph_order().cloned().collect();
    Some(
        decoded
            .validate(&current)
            .map(|()| decoded)
            .map_err(|error| format!("{error} in the current glyph set")),
    )
}

pub(crate) fn state(api: &Api, params: DocParams) -> Result<ColorState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let font = &doc.font;
            let (status, import_error) = if font.color.is_some() {
                (ColorStatus::Editable, None)
            } else if font.color_tables_owned {
                (ColorStatus::Deleted, None)
            } else if let Some(result) = decode_imported(font) {
                (ColorStatus::Imported, result.err())
            } else {
                (ColorStatus::Absent, None)
            };
            ColorState {
                generation: doc.generation,
                status,
                color: font.color.clone(),
                import_error,
                units_per_em: font.info.units_per_em,
            }
        })
        .map_err(ApiError::from)
}

fn stale() -> ApiError {
    ApiError::invalid_params("COLR/CPAL data changed; reload the editor before applying")
}

pub(crate) fn apply(api: &Api, params: ColorApplyParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(stale());
            }
            doc.apply(
                fc_doc::Command::Color(fc_doc::command::ColorCommand::Replace {
                    color: params.color,
                }),
                params.label,
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn decode(api: &Api, params: ColorDecodeParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(stale());
            }
            let color = decode_imported(&doc.font)
                .ok_or_else(|| {
                    ApiError::invalid_params("the document has no imported COLR/CPAL table")
                })?
                .map_err(|error| {
                    ApiError::invalid_params(format!(
                        "imported COLR/CPAL cannot be edited: {error}"
                    ))
                })?;
            doc.apply(
                fc_doc::Command::Color(fc_doc::command::ColorCommand::Replace {
                    color: Some(color),
                }),
                params.label,
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn preview(api: &Api, params: ColorPreviewParams) -> Result<ColorPreview, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let font = &doc.font;
            let master = params.master.as_ref().unwrap_or(&font.default_master().id);
            if font.master(master).is_none() {
                return Err(ApiError::not_found("preview master does not exist"));
            }
            let glyph = font
                .glyph(&params.glyph)
                .ok_or_else(|| ApiError::not_found("preview base glyph does not exist"))?;
            let base = glyph
                .master_layer(master)
                .ok_or_else(|| ApiError::not_found("preview base has no master layer"))?;
            let saved = font.color.as_ref();
            let layers = params
                .layers
                .as_deref()
                .or_else(|| {
                    saved.and_then(|c| c.glyphs.get(params.glyph.as_str()).map(Vec::as_slice))
                })
                .unwrap_or(&[]);
            let palette = params.palette.as_deref().or_else(|| {
                saved.and_then(|c| {
                    c.palettes
                        .get(usize::from(params.palette_index))
                        .map(|p| p.colors.as_slice())
                })
            });
            if layers.len() > MAX_PREVIEW_PARTS {
                return Err(ApiError::invalid_params(
                    "preview supports at most 1024 layers",
                ));
            }
            let mut output = ColorPreview {
                generation: doc.generation,
                width: base.width,
                units_per_em: font.info.units_per_em,
                layers: Vec::new(),
            };
            for layer in layers {
                let color = layer
                    .palette_index
                    .map(|i| {
                        palette
                            .and_then(|p| p.get(usize::from(i)))
                            .copied()
                            .ok_or_else(|| {
                                ApiError::invalid_params("preview palette entry is missing")
                            })
                    })
                    .transpose()?;
                let glyph = font.glyph(&layer.glyph).ok_or_else(|| {
                    ApiError::not_found(format!("outline glyph {} does not exist", layer.glyph))
                })?;
                let source = glyph
                    .master_layer(master)
                    .ok_or_else(|| ApiError::not_found("outline glyph has no master layer"))?;
                let outline = fc_geom::flatten_layer_by_id(font, &glyph.name, &source.id)
                    .map_err(|e| ApiError::invalid_params(e.to_string()))?;
                let bounds = if outline.elements().is_empty() {
                    [0.; 4]
                } else {
                    let r = kurbo::Shape::bounding_box(&outline);
                    [r.x0, r.y0, r.x1, r.y1]
                };
                output.layers.push(ColorPreviewLayer {
                    glyph: layer.glyph.to_string(),
                    path: super::glyphs::rounded_svg(&outline),
                    color,
                    bounds,
                });
            }
            Ok(output)
        })
        .map_err(ApiError::from)?
}
