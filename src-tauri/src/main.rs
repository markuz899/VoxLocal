// Niente finestra console su Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = voxlocal_lib::cli::run(&args) {
        std::process::exit(code);
    }
    voxlocal_lib::run()
}
