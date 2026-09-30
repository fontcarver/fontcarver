//! Atomic inventory collection edits using the existing font-info undo snapshot.

use fc_doc::{Command, Document};
use serde_json::{Map, Value};

use crate::ApiError;

const INVENTORY_KEY: &str = "com.fontcarver.inventory";

/// Read one collection without treating incompatible metadata as an empty value.
pub(crate) fn read_collection<'a>(
    user_data: &'a Value,
    key: &str,
) -> Result<Option<&'a Value>, ApiError> {
    let Some(root) = object_or_absent(user_data, "font info user data")? else {
        return Ok(None);
    };
    let Some(inventory) = root.get(INVENTORY_KEY) else {
        return Ok(None);
    };
    Ok(object_or_absent(inventory, "font inventory metadata")?
        .and_then(|inventory| inventory.get(key)))
}

/// Replace one validated collection while preserving its sibling metadata.
pub(crate) fn write_collection(
    document: &mut Document,
    key: &str,
    value: Value,
    label: &str,
) -> Result<(), ApiError> {
    if read_collection(&document.font.info.user_data, key)? == Some(&value) {
        return Ok(());
    }
    let mut info = document.font.info.clone();
    let mut root = object_or_absent(&info.user_data, "font info user data")?
        .cloned()
        .unwrap_or_default();
    let mut inventory = root
        .get(INVENTORY_KEY)
        .map(|value| object_or_absent(value, "font inventory metadata"))
        .transpose()?
        .flatten()
        .cloned()
        .unwrap_or_default();
    inventory.insert(key.to_owned(), value);
    root.insert(INVENTORY_KEY.to_owned(), Value::Object(inventory));
    info.user_data = Value::Object(root);
    document
        .apply(
            Command::SetFontInfo {
                info: Box::new(info),
            },
            label,
        )
        .map_err(ApiError::from)?;
    Ok(())
}

fn object_or_absent<'a>(
    value: &'a Value,
    description: &str,
) -> Result<Option<&'a Map<String, Value>>, ApiError> {
    match value {
        Value::Null => Ok(None),
        Value::Object(object) => Ok(Some(object)),
        _ => Err(ApiError::invalid_params(format!(
            "{description} must be an object or null; existing metadata was preserved"
        ))),
    }
}
