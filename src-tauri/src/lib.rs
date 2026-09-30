//! Thin Tauri shell for the FontCarver frontend.

use std::sync::Arc;

use fc_api::Api;
use fc_doc::Session;
use tauri::Emitter;
use tokio::sync::broadcast::error::RecvError;
use tracing_subscriber::EnvFilter;

pub mod cli;
mod commands;
pub mod error;
mod windows;

pub use error::Error;

/// Runs the desktop shell as its command line asks.
///
/// `--version` and `--help` print and return; anything else opens the window.
/// A missing or unreachable graphical session becomes a described error rather
/// than a panic, because the packaging smoke tests run this binary on machines
/// that have no session at all.
///
/// # Errors
///
/// Returns [`Error::Usage`] for an argument the shell does not accept,
/// [`Error::NoDisplay`] when no graphical session can be reached,
/// [`Error::Startup`] when the window system fails while starting, and
/// [`Error::Tauri`] when Tauri itself reports a failure.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Error> {
    match cli::parse(std::env::args_os().skip(1)).map_err(Error::Usage)? {
        cli::Request::Version => {
            println!("{} {}", cli::NAME, cli::VERSION);
            return Ok(());
        }
        cli::Request::Help => {
            println!("{}", cli::usage());
            return Ok(());
        }
        cli::Request::Launch => {}
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    if !cli::display_configured(
        std::env::var("DISPLAY").ok().as_deref(),
        std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
    ) {
        return Err(Error::NoDisplay(
            "neither DISPLAY nor WAYLAND_DISPLAY is set".to_owned(),
        ));
    }
    launch()
}

/// Runs one startup step, turning an unwinding panic into a described error.
///
/// The guard and its panic hook cover *only* the step they are given, which is
/// creating the application. A guard left in place for the application's whole
/// life would swallow every later panic and mislabel it as a startup failure, so
/// the default hook is back in place before the event loop runs.
fn guarded<T>(step: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
    use std::sync::{Arc, Mutex};

    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&captured);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // The default hook would print the panic and a backtrace; the payload is
        // kept instead and reported as an ordinary error by `main`. The payload
        // is the part that names the cause — GTK's own "Failed to initialize gtk
        // backend!" — while the location only names a file inside a dependency.
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| info.to_string());
        if let Ok(mut slot) = sink.lock() {
            *slot = Some(message);
        }
    }));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(step));
    std::panic::set_hook(previous);
    match result {
        Ok(result) => result,
        Err(_) => {
            let message = captured
                .lock()
                .ok()
                .and_then(|slot| slot.clone())
                .unwrap_or_else(|| "the window system failed during startup".to_owned());
            // GTK's own wording is the useful part, and it is what tells a
            // headless runner that the display, not FontCarver, is the problem.
            Err(if mentions_display(&message) {
                Error::NoDisplay(first_line(&message))
            } else {
                Error::Startup(first_line(&message))
            })
        }
    }
}

/// Whether a panic message is about the window system rather than FontCarver.
fn mentions_display(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    ["gtk", "display", "wayland", "x11", "xopendisplay"]
        .iter()
        .any(|needle| message.contains(needle))
}

/// The first line of a panic message, which carries its wording.
fn first_line(message: &str) -> String {
    message
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(message)
        .trim()
        .to_owned()
}

fn launch() -> Result<(), Error> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();

    let api = Arc::new(Api::new(Arc::new(Session::new())));
    if let Err(error) = api.start_autosave() {
        tracing::warn!(%error, "could not start crash-recovery autosave");
    }
    let mut events = api.subscribe();

    let builder = tauri::Builder::default()
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
        ]);

    // The window system is reached while the application is built, and tao
    // panics rather than returning when GTK cannot open the display it was
    // pointed at. Only that step is guarded, so the process exits with a message
    // and code 1 instead of a backtrace and code 101 — and a panic once the
    // application is running still reaches the default hook.
    let app = guarded(|| {
        builder
            .build(tauri::generate_context!())
            .map_err(Error::from)
    })?;
    app.run(|_handle, _event| {});
    Ok(())
}
