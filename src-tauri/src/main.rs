#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), fontworkshop_lib::Error> {
    fontworkshop_lib::run()
}
