//! Thin Tauri shell for the FontWorkshop frontend.

use std::sync::Arc;

use fw_api::Api;
use fw_doc::Session;
use tauri::Emitter;
use tokio::sync::broadcast::error::RecvError;

mod commands;
pub mod error;

pub use error::Error;

/// Starts the FontWorkshop desktop application.
///
/// # Errors
///
/// Returns an error when Tauri cannot initialize or run the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Error> {
    let api = Arc::new(Api::new(Arc::new(Session::new())));
    let mut events = api.subscribe();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(api)
        .setup(move |app| {
            let app_handle = app.handle().clone();
            drop(std::thread::spawn(move || loop {
                match events.blocking_recv() {
                    Ok(event) => {
                        if let Err(error) = app_handle.emit("fw:event", &event) {
                            tracing::warn!(%error, "failed to emit document event");
                            break;
                        }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        tracing::warn!(skipped, "Tauri event subscriber lagged");
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                }
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::api_call])
        .run(tauri::generate_context!())
        .map_err(Error::from)
}
