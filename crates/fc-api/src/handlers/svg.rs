//! SVG reads and previews never mutate the document; edits are revision-checked.

use fc_model::{Font, GlyphName, SvgTable, Tag};

use crate::{
    Api, ApiError, Change, DocParams, SvgApplyParams, SvgDecodeParams, SvgState, SvgStatus,
};

const SVG: Tag = Tag(*b"SVG ");

/// Decode imported bytes against the glyph order they were written for.
///
/// Legacy documents keep raw SVG with import provenance. Glyph IDs refer to
/// the imported order, so the result must also resolve in the current font.
fn decode_imported(font: &Font) -> Option<Result<SvgTable, String>> {
    if font.svg_table_owned || font.svg.is_some() {
        return None;
    }
    let bytes = font.extra.raw_tables.get(&SVG)?;
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
    let decoded = match fc_sfnt::svg::read_table(bytes, order) {
        Ok(svg) => svg,
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

pub(crate) fn state(api: &Api, params: DocParams) -> Result<SvgState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let font = &doc.font;
            let (status, import_error) = if font.svg.is_some() {
                (SvgStatus::Editable, None)
            } else if font.svg_table_owned {
                (SvgStatus::Deleted, None)
            } else if let Some(result) = decode_imported(font) {
                (SvgStatus::Imported, result.err())
            } else {
                (SvgStatus::Absent, None)
            };
            SvgState {
                generation: doc.generation,
                status,
                svg: font.svg.clone(),
                import_error,
                units_per_em: font.info.units_per_em,
            }
        })
        .map_err(ApiError::from)
}

fn stale() -> ApiError {
    ApiError::invalid_params("SVG data changed; reload the editor before applying")
}

pub(crate) fn apply(api: &Api, params: SvgApplyParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(stale());
            }
            doc.apply(
                fc_doc::Command::Svg(fc_doc::command::SvgCommand::Replace { svg: params.svg }),
                params.label,
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn decode(api: &Api, params: SvgDecodeParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(stale());
            }
            let svg = decode_imported(&doc.font)
                .ok_or_else(|| ApiError::invalid_params("the document has no imported SVG table"))?
                .map_err(|error| {
                    ApiError::invalid_params(format!("imported SVG cannot be edited: {error}"))
                })?;
            doc.apply(
                fc_doc::Command::Svg(fc_doc::command::SvgCommand::Replace { svg: Some(svg) }),
                params.label,
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}
