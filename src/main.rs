#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    warehouse::run_app().unwrap();
}
