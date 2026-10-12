//! Serde helpers for fields where absence and explicit `null` differ.

use serde::{Deserialize, Deserializer};

/// Deserializes a present field into `Some`, preserving JSON `null` as `Some(None)`.
///
/// Pair this with `#[serde(default, deserialize_with = "double_option::deserialize")]`:
/// serde supplies `None` for an absent field, while this function handles only
/// fields that are present in the input.
pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Reads a possibly absent JSON field with double-option semantics.
pub fn from_value<T>(value: Option<&serde_json::Value>) -> serde_json::Result<Option<Option<T>>>
where
    T: serde::de::DeserializeOwned,
{
    value
        .map(|value| serde_json::from_value::<Option<T>>(value.clone()))
        .transpose()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Example {
        #[serde(default, deserialize_with = "super::deserialize")]
        value: Option<Option<u8>>,
    }

    #[test]
    fn distinguishes_absent_null_and_present_values() {
        assert_eq!(
            serde_json::from_value::<Example>(json!({})).unwrap().value,
            None
        );
        assert_eq!(
            serde_json::from_value::<Example>(json!({ "value": null }))
                .unwrap()
                .value,
            Some(None)
        );
        assert_eq!(
            serde_json::from_value::<Example>(json!({ "value": 7 }))
                .unwrap()
                .value,
            Some(Some(7))
        );
    }
}
