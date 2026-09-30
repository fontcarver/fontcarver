//! Versioned overlay editing and revision-checked glyph metadata refresh.
use std::collections::BTreeSet;

use fc_doc::{command::glyph_data_overlay, Command};
use fc_glyphdata::{parse_overlay, resolve_info};
use serde_json::Value;

use crate::{
    Api, ApiError, Change, GlyphDataState, GlyphDataStateParams, SetGlyphDataOverlayParams,
    UpdateGlyphInfoParams,
};

pub(crate) fn state(api: &Api, params: GlyphDataStateParams) -> Result<GlyphDataState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let name = doc
                .glyph_name(params.gid)
                .ok_or_else(|| ApiError::not_found("glyph handle is no longer present"))?;
            let glyph = doc
                .font
                .glyph(name)
                .ok_or_else(|| ApiError::not_found("glyph is missing"))?;
            let overlay = glyph_data_overlay(&doc.font.info)?;
            let info = resolve_info(name.as_ref(), &glyph.unicodes, &overlay);
            let raw_name = name.as_ref();
            let base = raw_name
                .char_indices()
                .find(|(index, character)| *index > 0 && *character == '.')
                .map_or(raw_name, |(index, _)| &raw_name[..index]);
            let custom = overlay.glyphs.contains_key(name.as_ref())
                || overlay.glyphs.contains_key(base)
                || glyph.unicodes != info.unicodes
                || glyph
                    .category
                    .as_ref()
                    .is_some_and(|value| Some(value) != info.category.as_ref())
                || glyph
                    .sub_category
                    .as_ref()
                    .is_some_and(|value| Some(value) != info.sub_category.as_ref())
                || glyph
                    .script
                    .as_ref()
                    .is_some_and(|value| Some(value) != info.script.as_ref())
                || glyph
                    .production_name
                    .as_ref()
                    .is_some_and(|value| Some(value) != info.production_name.as_ref());
            Ok(GlyphDataState {
                generation: doc.generation,
                overlay_json: serde_json::to_string_pretty(&overlay)
                    .map_err(|error| ApiError::internal(error.to_string()))?,
                info: info.into(),
                custom,
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn set_overlay(
    api: &Api,
    params: SetGlyphDataOverlayParams,
) -> Result<Change, ApiError> {
    let text = if params.overlay_json.trim().is_empty() {
        r#"{"version":1,"glyphs":{}}"#
    } else {
        &params.overlay_json
    };
    let overlay = parse_overlay(text).map_err(ApiError::invalid_params)?;
    let value =
        serde_json::to_value(&overlay).map_err(|error| ApiError::internal(error.to_string()))?;
    api.session
        .with_doc_mut(params.doc, |doc| {
            check_generation(doc.generation, params.expected_generation)?;
            check_label(&params.label)?;
            let mut info = doc.font.info.clone();
            if info.user_data.is_null() {
                info.user_data = Value::Object(Default::default());
            }
            let data = info.user_data.as_object_mut().ok_or_else(|| {
                ApiError::invalid_params(
                    "font user data must be an object to store a glyph database overlay",
                )
            })?;
            data.insert(fc_doc::command::GLYPH_DATA_OVERLAY_KEY.into(), value);
            doc.apply(
                Command::SetFontInfo {
                    info: Box::new(info),
                },
                params.label,
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn update(api: &Api, params: UpdateGlyphInfoParams) -> Result<Change, ApiError> {
    if params.glyphs.is_empty() || params.glyphs.len() > 65_536 {
        return Err(ApiError::invalid_params(
            "select between one and 65536 glyphs",
        ));
    }
    api.session
        .with_doc_mut(params.doc, |doc| {
            check_generation(doc.generation, params.expected_generation)?;
            check_label(&params.label)?;
            let glyphs = params
                .glyphs
                .iter()
                .map(|gid| {
                    doc.glyph_name(*gid)
                        .cloned()
                        .ok_or_else(|| ApiError::not_found("selected glyph no longer exists"))
                })
                .collect::<Result<BTreeSet<_>, _>>()?
                .into_iter()
                .collect();
            doc.apply(Command::UpdateGlyphInfo { glyphs }, params.label)
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

fn check_generation(current: u64, expected: u64) -> Result<(), ApiError> {
    if current != expected {
        return Err(ApiError::invalid_params(
            "glyph information changed; reload before applying edits",
        ));
    }
    Ok(())
}

fn check_label(label: &str) -> Result<(), ApiError> {
    if label.len() > 512 {
        return Err(ApiError::invalid_params("undo label exceeds 512 bytes"));
    }
    Ok(())
}
