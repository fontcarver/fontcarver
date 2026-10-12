//! Tauri command registration.

use std::sync::Arc;

use fc_api::{Api, ApiError};
use serde_json::Value;
use tauri::State;

/// Dispatches synchronous kernel work on Tauri's blocking command threadpool so
/// font operations never stall the main or IPC thread.
#[tauri::command(async)]
pub(crate) fn api_call(
    method: String,
    params: Value,
    api: State<'_, Arc<Api>>,
) -> Result<Value, ApiError> {
    api.call(&method, params)
}
