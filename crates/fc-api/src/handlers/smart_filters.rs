//! Named project filters with validated expressions and ordinary undo history.

use std::collections::HashSet;

use crate::{
    Api, ApiError, SavedSmartFilter, SmartFilterParseParams, SmartFilterParseResult,
    SmartFiltersGetParams, SmartFiltersResult, SmartFiltersSetParams,
};

use super::{inventory, smart_filter_parser};

const COLLECTION: &str = "smart_filters";
const MAX_FILTERS: usize = 256;
const MAX_NAME_BYTES: usize = 256;
const MAX_COLLECTION_BYTES: usize = 1024 * 1024;

pub(crate) fn get(
    api: &Api,
    params: SmartFiltersGetParams,
) -> Result<SmartFiltersResult, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            read_filters(&document.font.info.user_data)
                .map(|filters| SmartFiltersResult { filters })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn set(
    api: &Api,
    params: SmartFiltersSetParams,
) -> Result<SmartFiltersResult, ApiError> {
    let filters = validate_filters(params.filters)?;
    let value =
        serde_json::to_value(&filters).map_err(|error| ApiError::internal(error.to_string()))?;
    api.session
        .with_doc_mut(params.doc, |document| {
            // A malformed saved collection must not be silently discarded by an edit.
            read_filters(&document.font.info.user_data)?;
            inventory::write_collection(document, COLLECTION, value, "Edit smart filters")?;
            Ok(SmartFiltersResult { filters })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn parse(
    _api: &Api,
    params: SmartFilterParseParams,
) -> Result<SmartFilterParseResult, ApiError> {
    let root = smart_filter_parser::parse(&params.expression)?;
    let expression = smart_filter_parser::format(&root);
    // Canonical text must remain valid when sent back by the visual editor.
    smart_filter_parser::parse(&expression)?;
    Ok(SmartFilterParseResult { expression, root })
}

fn read_filters(user_data: &serde_json::Value) -> Result<Vec<SavedSmartFilter>, ApiError> {
    let Some(value) = inventory::read_collection(user_data, COLLECTION)? else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_array()
        .ok_or_else(|| ApiError::invalid_params("saved smart filters must be an array"))?;
    if entries.len() > MAX_FILTERS {
        return Err(ApiError::invalid_params(
            "at most 256 smart filters may be saved",
        ));
    }
    let filters =
        <Vec<SavedSmartFilter> as serde::Deserialize>::deserialize(value).map_err(|error| {
            ApiError::invalid_params(format!("invalid saved smart filters: {error}"))
        })?;
    validate_filters(filters)
}

fn validate_filters(mut filters: Vec<SavedSmartFilter>) -> Result<Vec<SavedSmartFilter>, ApiError> {
    if filters.len() > MAX_FILTERS {
        return Err(ApiError::invalid_params(
            "at most 256 smart filters may be saved",
        ));
    }
    let mut names = HashSet::new();
    let mut bytes = 0usize;
    let mut canonical_bytes = 0usize;
    for filter in &mut filters {
        filter.name = filter.name.trim().to_owned();
        if filter.name.is_empty()
            || filter.name.len() > MAX_NAME_BYTES
            || filter.name.chars().any(char::is_control)
        {
            return Err(ApiError::invalid_params(
                "smart filter names must contain 1 to 256 UTF-8 bytes without control characters",
            ));
        }
        if !names.insert(filter.name.clone()) {
            return Err(ApiError::invalid_params(
                "smart filter names must be unique",
            ));
        }
        bytes = bytes
            .saturating_add(filter.name.len())
            .saturating_add(filter.expression.len());
        if bytes > MAX_COLLECTION_BYTES {
            return Err(ApiError::invalid_params(
                "saved smart filters exceed the 1 MiB text limit",
            ));
        }
        let parsed = smart_filter_parser::parse(&filter.expression)?;
        filter.expression = smart_filter_parser::format(&parsed);
        smart_filter_parser::parse(&filter.expression)?;
        canonical_bytes = canonical_bytes
            .saturating_add(filter.name.len())
            .saturating_add(filter.expression.len());
        if canonical_bytes > MAX_COLLECTION_BYTES {
            return Err(ApiError::invalid_params(
                "saved smart filters exceed the 1 MiB text limit",
            ));
        }
    }
    Ok(filters)
}
