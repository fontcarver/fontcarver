#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), fontcarver_lib::Error> {
    fontcarver_lib::run()
}
