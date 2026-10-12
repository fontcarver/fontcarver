//! Validated, undoable named glyph inventories in font metadata.

use std::collections::HashSet;

use fc_doc::Document;

use crate::{
    Api, ApiError, DocParams, GlyphList, GlyphListView, GlyphListsResult, SetGlyphListsParams,
};

use super::inventory::{read_collection, write_collection};

const COLLECTION_KEY: &str = "glyph_lists";
const MAX_LISTS: usize = 128;
const MAX_GLYPHS_PER_LIST: usize = 8192;
const MAX_TOTAL_GLYPHS: usize = 65_536;
const MAX_COLLECTION_BYTES: usize = 2 * 1024 * 1024;

/// Return named lists with coverage resolved against the current document.
pub(crate) fn get(api: &Api, params: DocParams) -> Result<GlyphListsResult, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            Ok(views(document, stored_lists(document)?))
        })
        .map_err(ApiError::from)?
}

/// Validate and replace the collection in one reversible document edit.
pub(crate) fn set(api: &Api, params: SetGlyphListsParams) -> Result<GlyphListsResult, ApiError> {
    let lists = normalize(params.lists)?;
    let value = serde_json::to_value(&lists)
        .map_err(|error| ApiError::invalid_params(error.to_string()))?;
    api.session
        .with_doc_mut(params.doc, |document| {
            // Reject malformed existing collections instead of silently discarding data.
            stored_lists(document)?;
            write_collection(document, COLLECTION_KEY, value, "Edit glyph lists")?;
            Ok(views(document, lists))
        })
        .map_err(ApiError::from)?
}

fn stored_lists(document: &Document) -> Result<Vec<GlyphList>, ApiError> {
    let Some(value) = read_collection(&document.font.info.user_data, COLLECTION_KEY)? else {
        return Ok(Vec::new());
    };
    let lists = serde_json::from_value(value.clone()).map_err(|error| {
        ApiError::invalid_params(format!("invalid stored glyph lists: {error}"))
    })?;
    normalize(lists)
}

fn normalize(mut lists: Vec<GlyphList>) -> Result<Vec<GlyphList>, ApiError> {
    if lists.len() > MAX_LISTS {
        return Err(ApiError::invalid_params(
            "at most 128 glyph lists are supported",
        ));
    }
    let mut ids = HashSet::new();
    let mut total_glyphs = 0usize;
    for list in &mut lists {
        if list.id.is_empty()
            || list.id.len() > 80
            || !list
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ApiError::invalid_params(
                "glyph list IDs must contain 1–80 ASCII letters, digits, underscores, or hyphens",
            ));
        }
        if !ids.insert(list.id.clone()) {
            return Err(ApiError::invalid_params("glyph list IDs must be unique"));
        }
        if list.name.chars().any(char::is_control) {
            return Err(ApiError::invalid_params(
                "glyph list names cannot contain control characters",
            ));
        }
        list.name = list.name.trim().to_owned();
        if list.name.is_empty() || list.name.chars().count() > 128 {
            return Err(ApiError::invalid_params(
                "glyph list names must contain 1–128 characters",
            ));
        }
        if list.glyphs.len() > MAX_GLYPHS_PER_LIST {
            return Err(ApiError::invalid_params(
                "a glyph list cannot exceed 8192 names",
            ));
        }
        total_glyphs += list.glyphs.len();
        if total_glyphs > MAX_TOTAL_GLYPHS {
            return Err(ApiError::invalid_params(
                "glyph lists cannot exceed 65536 total names",
            ));
        }
        if list.glyphs.iter().any(|name| !valid_glyph_name(name)) {
            return Err(ApiError::invalid_params(
                "glyph lists require exact glyph names of at most 127 bytes, using an ASCII letter, dot, or underscore first, followed by ASCII letters, digits, dots, or underscores",
            ));
        }
        let mut names = HashSet::new();
        list.glyphs.retain(|name| names.insert(name.clone()));
    }
    let serialized =
        serde_json::to_vec(&lists).map_err(|error| ApiError::invalid_params(error.to_string()))?;
    if serialized.len() > MAX_COLLECTION_BYTES {
        return Err(ApiError::invalid_params(
            "glyph list metadata cannot exceed 2 MiB",
        ));
    }
    Ok(lists)
}

fn valid_glyph_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 127
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'.' | b'_'))
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
}

fn views(document: &Document, lists: Vec<GlyphList>) -> GlyphListsResult {
    GlyphListsResult {
        lists: lists
            .into_iter()
            .map(|list| {
                let (present, missing) = list
                    .glyphs
                    .iter()
                    .cloned()
                    .partition(|name| document.font.glyph(name).is_some());
                GlyphListView {
                    id: list.id,
                    name: list.name,
                    glyphs: list.glyphs,
                    present,
                    missing,
                }
            })
            .collect(),
    }
}
