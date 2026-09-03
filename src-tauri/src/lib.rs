//! Thin Tauri shell for the FontWorkshop frontend.

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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .run(tauri::generate_context!())
        .map_err(Error::from)
}
