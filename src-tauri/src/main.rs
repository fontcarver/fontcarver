#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match fontcarver_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // One readable line on stderr, and an exit code a packaging smoke
            // test can interpret; never a Debug dump or a panic backtrace.
            eprintln!("{}: {error}", fontcarver_lib::cli::NAME);
            ExitCode::from(error.exit_code())
        }
    }
}
