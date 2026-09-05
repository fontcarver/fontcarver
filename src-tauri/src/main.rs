#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), fontcutter_lib::Error> {
    fontcutter_lib::run()
}
