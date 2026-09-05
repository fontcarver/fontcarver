//! Lossless conversion between plist values and JSON-backed model metadata.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use plist::{Dictionary, Value};
use serde_json::{Map, Number};

use crate::Error;

const TYPE_KEY: &str = "$fontcutter.plist.type";
const VALUE_KEY: &str = "value";

pub(crate) fn dict_to_json(dictionary: &Dictionary) -> Result<serde_json::Value, Error> {
    plist_to_json(&Value::Dictionary(dictionary.clone()))
}

pub(crate) fn json_to_dict(value: &serde_json::Value) -> Result<Dictionary, Error> {
    json_to_plist(value)?
        .into_dictionary()
        .ok_or_else(|| Error::UnsupportedPlist("expected a dictionary".to_owned()))
}

pub(crate) fn insert_nested_json(
    root: &mut serde_json::Value,
    key: &str,
    nested: &str,
    value: serde_json::Value,
) {
    if !root.is_object() {
        *root = serde_json::Value::Object(Default::default());
    }
    let Some(object) = root.as_object_mut() else {
        return;
    };
    let nested_object = object
        .entry(key.to_owned())
        .or_insert_with(|| serde_json::Value::Object(Default::default()));
    if let Some(nested_object) = nested_object.as_object_mut() {
        nested_object.insert(nested.to_owned(), value);
    }
}

pub(crate) fn insert_json_value(root: &mut serde_json::Value, key: &str, value: serde_json::Value) {
    if !root.is_object() {
        *root = serde_json::Value::Object(Default::default());
    }
    if let Some(root) = root.as_object_mut() {
        root.insert(key.to_owned(), value);
    }
}

pub(crate) fn plist_to_json(value: &Value) -> Result<serde_json::Value, Error> {
    Ok(match value {
        Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(plist_to_json).collect::<Result<_, _>>()?)
        }
        Value::Dictionary(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| Ok((key.clone(), plist_to_json(value)?)))
                .collect::<Result<_, Error>>()?,
        ),
        Value::Boolean(value) => serde_json::Value::Bool(*value),
        Value::Data(value) => tagged("data", serde_json::Value::String(STANDARD.encode(value))),
        Value::Date(value) => tagged("date", serde_json::Value::String(value.to_xml_format())),
        Value::Real(value) => Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .ok_or_else(|| Error::UnsupportedPlist(format!("non-finite real {value}")))?,
        Value::Integer(value) => {
            if let Some(value) = value.as_signed() {
                serde_json::Value::Number(value.into())
            } else if let Some(value) = value.as_unsigned() {
                serde_json::Value::Number(value.into())
            } else {
                return Err(Error::UnsupportedPlist(format!("integer {value}")));
            }
        }
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Uid(value) => tagged("uid", serde_json::Value::Number(value.get().into())),
        _ => return Err(Error::UnsupportedPlist("unknown plist variant".to_owned())),
    })
}

pub(crate) fn json_to_plist(value: &serde_json::Value) -> Result<Value, Error> {
    Ok(match value {
        serde_json::Value::Null => {
            return Err(Error::UnsupportedPlist("JSON null".to_owned()));
        }
        serde_json::Value::Bool(value) => Value::Boolean(*value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Value::Integer(value.into())
            } else if let Some(value) = value.as_u64() {
                Value::Integer(value.into())
            } else if let Some(value) = value.as_f64() {
                Value::Real(value)
            } else {
                return Err(Error::UnsupportedPlist(format!("number {value}")));
            }
        }
        serde_json::Value::String(value) => Value::String(value.clone()),
        serde_json::Value::Array(values) => {
            Value::Array(values.iter().map(json_to_plist).collect::<Result<_, _>>()?)
        }
        serde_json::Value::Object(values) => {
            if let Some(tag) = tagged_value(values) {
                tag?
            } else {
                Value::Dictionary(
                    values
                        .iter()
                        .filter(|(_, value)| !value.is_null())
                        .map(|(key, value)| Ok((key.clone(), json_to_plist(value)?)))
                        .collect::<Result<_, Error>>()?,
                )
            }
        }
    })
}

fn tagged(kind: &str, value: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ TYPE_KEY: kind, VALUE_KEY: value })
}

fn tagged_value(values: &Map<String, serde_json::Value>) -> Option<Result<Value, Error>> {
    if values.len() != 2 {
        return None;
    }
    let kind = values.get(TYPE_KEY)?.as_str()?;
    let value = values.get(VALUE_KEY)?;
    Some(match kind {
        "data" => value
            .as_str()
            .ok_or_else(|| Error::UnsupportedPlist("non-string data payload".to_owned()))
            .and_then(|value| {
                STANDARD
                    .decode(value)
                    .map(Value::Data)
                    .map_err(|error| Error::UnsupportedPlist(error.to_string()))
            }),
        "date" => value
            .as_str()
            .ok_or_else(|| Error::UnsupportedPlist("non-string date payload".to_owned()))
            .and_then(|value| {
                plist::Date::from_xml_format(value)
                    .map(Value::Date)
                    .map_err(|error| Error::UnsupportedPlist(error.to_string()))
            }),
        "uid" => value
            .as_u64()
            .map(plist::Uid::new)
            .map(Value::Uid)
            .ok_or_else(|| Error::UnsupportedPlist("non-integer UID payload".to_owned())),
        _ => return None,
    })
}
