// Niente finestra console su Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    voxlocal_lib::run()
}
