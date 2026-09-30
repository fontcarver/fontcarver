//! Thin Tauri shell for the FontCarver frontend.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use tauri::Emitter;
use tokio::sync::broadcast::error::RecvError;
use tracing_subscriber::EnvFilter;

mod commands;
pub mod error;
mod windows;

pub use error::Error;

/// Starts the FontCarver desktop application.
///
/// # Errors
///
/// Returns an error when Tauri cannot initialize or run the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Error> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();

    let api = Arc::new(Api::new(Arc::new(Session::new())));
    if let Err(error) = api.start_autosave() {
        tracing::warn!(%error, "could not start crash-recovery autosave");
    }
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
                        if let Err(error) = app_handle.emit("fc:event", &event) {
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
        .invoke_handler(tauri::generate_handler![
            commands::api_call,
            windows::open_workspace_window,
            windows::close_workspace_window,
            windows::rejoin_workspace_window,
        ])
        .run(tauri::generate_context!())
        .map_err(Error::from)
}
