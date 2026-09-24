//! MATH reads and previews never mutate the document; edits are revision-checked.

use fc_model::{Font, GlyphName, MathConstruction, MathTable, Tag};

use crate::{
    Api, ApiError, Change, DocParams, MathApplyParams, MathDecodeParams, MathPreview,
    MathPreviewGlyph, MathPreviewParams, MathState, MathStatus,
};

const MATH: Tag = Tag(*b"MATH");
const MAX_COMMANDS: usize = 65_536;
const MAX_PREVIEW_PARTS: usize = 1_024;
const MAX_PREVIEW_TARGET: f64 = 1_000_000.0;

/// Decode imported bytes against the glyph order they were written for.
///
/// Legacy documents keep raw MATH with import provenance. Glyph IDs refer to
/// the imported order, so the result must also resolve in the current font.
fn decode_imported(font: &Font) -> Option<Result<MathTable, String>> {
    if font.math_table_owned || font.math.is_some() {
        return None;
    }
    let bytes = font.extra.raw_tables.get(&MATH)?;
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
    let decoded = match fc_sfnt::math::read_table(bytes, order) {
        Ok(math) => math,
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

pub(crate) fn state(api: &Api, params: DocParams) -> Result<MathState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let font = &doc.font;
            let (status, import_error) = if font.math.is_some() {
                (MathStatus::Editable, None)
            } else if font.math_table_owned {
                (MathStatus::Deleted, None)
            } else if let Some(result) = decode_imported(font) {
                (MathStatus::Imported, result.err())
            } else {
                (MathStatus::Absent, None)
            };
            MathState {
                generation: doc.generation,
                status,
                math: font.math.clone(),
                import_error,
                units_per_em: font.info.units_per_em,
            }
        })
        .map_err(ApiError::from)
}

fn stale() -> ApiError {
    ApiError::invalid_params("MATH data changed; reload the editor before applying")
}

pub(crate) fn apply(api: &Api, params: MathApplyParams) -> Result<Change, ApiError> {
    if params.commands.is_empty() || params.commands.len() > MAX_COMMANDS {
        return Err(ApiError::invalid_params(
            "MATH operation count must be between 1 and 65,536",
        ));
    }
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(stale());
            }
            let commands = params
                .commands
                .into_iter()
                .map(fc_doc::Command::Math)
                .collect();
            doc.apply(fc_doc::Command::Batch(commands), params.label)
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn decode(api: &Api, params: MathDecodeParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(stale());
            }
            let math = decode_imported(&doc.font)
                .ok_or_else(|| ApiError::invalid_params("the document has no imported MATH table"))?
                .map_err(|error| {
                    ApiError::invalid_params(format!("imported MATH cannot be edited: {error}"))
                })?;
            doc.apply(
                fc_doc::Command::Math(fc_doc::command::MathCommand::Replace { math: Some(math) }),
                params.label,
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn preview(api: &Api, params: MathPreviewParams) -> Result<MathPreview, ApiError> {
    if !params.target.is_finite() || !(0.0..=MAX_PREVIEW_TARGET).contains(&params.target) {
        return Err(ApiError::invalid_params(
            "preview size must be between 0 and 1,000,000 units",
        ));
    }
    if params
        .min_connector_overlap
        .is_some_and(|overlap| !overlap.is_finite() || overlap < 0.0)
    {
        return Err(ApiError::invalid_params(
            "minimum connector overlap must be a finite non-negative value",
        ));
    }
    api.session
        .with_doc(params.doc, |doc| {
            let font = &doc.font;
            let saved = font.math.as_ref();
            let construction = match params.construction {
                Some(construction) => Some(construction),
                None => saved
                    .and_then(|math| math.glyphs.get(params.glyph.as_str()))
                    .and_then(|info| {
                        if params.vertical {
                            info.vertical.clone()
                        } else {
                            info.horizontal.clone()
                        }
                    }),
            }
            .unwrap_or_default();
            let overlap = params
                .min_connector_overlap
                .or_else(|| saved.map(|math| math.min_connector_overlap))
                .unwrap_or_default();
            let mut preview = choose(&construction, params.target, overlap);
            preview.generation = doc.generation;
            let mut names: Vec<&str> = preview
                .variant
                .iter()
                .chain(&preview.fallback)
                .map(String::as_str)
                .chain(
                    preview
                        .assembly
                        .iter()
                        .flat_map(|layout| layout.parts.iter().map(|part| part.glyph.as_ref())),
                )
                .collect();
            names.sort_unstable();
            names.dedup();
            preview.glyphs = names
                .into_iter()
                .map(|name| outline(font, name))
                .collect::<Result<_, _>>()?;
            Ok(preview)
        })
        .map_err(ApiError::from)?
}

/// Select what a layout engine uses for the requested extent.
fn choose(construction: &MathConstruction, target: f64, overlap: f64) -> MathPreview {
    let mut preview = MathPreview {
        generation: 0,
        variant: None,
        assembly: None,
        fallback: None,
        glyphs: Vec::new(),
    };
    if let Some(variant) = construction
        .variants
        .iter()
        .find(|variant| variant.advance >= target)
    {
        preview.variant = Some(variant.glyph.to_string());
    } else if let Some(layout) = construction
        .assembly
        .as_ref()
        .and_then(|assembly| assembly.layout(target, overlap, MAX_PREVIEW_PARTS))
    {
        preview.assembly = Some(layout);
    } else {
        preview.fallback = construction
            .variants
            .iter()
            .max_by(|a, b| a.advance.total_cmp(&b.advance))
            .map(|variant| variant.glyph.to_string());
    }
    preview
}

fn outline(font: &Font, name: &str) -> Result<MathPreviewGlyph, ApiError> {
    let glyph = font
        .glyph(name)
        .ok_or_else(|| ApiError::not_found(format!("glyph `{name}` does not exist")))?;
    let master = &font.default_master().id;
    let layer = glyph.master_layer(master).ok_or_else(|| {
        ApiError::not_found(format!("glyph `{name}` has no default master layer"))
    })?;
    let outline = fc_model::query::flatten_layer_by_id(font, &glyph.name, &layer.id)
        .map_err(|error| ApiError::invalid_params(error.to_string()))?;
    let bounds = if outline.elements().is_empty() {
        [0.0; 4]
    } else {
        let rect = kurbo::Shape::bounding_box(&outline);
        [rect.x0, rect.y0, rect.x1, rect.y1]
    };
    Ok(MathPreviewGlyph {
        name: name.to_owned(),
        path: super::glyphs::rounded_svg(&outline),
        width: layer.width,
        bounds,
    })
}
