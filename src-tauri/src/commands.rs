//! Tauri command registration.

use std::sync::Arc;

use fw_api::{Api, ApiError};
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub(crate) fn api_call(
    method: String,
    params: Value,
    api: State<'_, Arc<Api>>,
) -> Result<Value, ApiError> {
    api.call(&method, params)
}
