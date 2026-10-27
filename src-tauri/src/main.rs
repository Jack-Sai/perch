// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if perch_lib::try_run_cli() {
        return;
    }
    perch_lib::run()
}
