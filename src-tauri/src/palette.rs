//! La barra dei comandi sulla selezione: si apre con un tocco della scorciatoia del comando (senza
//! parlare) e lascia scrivere l'istruzione o sceglierne una recente (palette.html).

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "palette";
const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 340.0;

pub fn show(app: &AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(w) = app.get_webview_window(LABEL) {
            let _ = w.destroy();
        }
        let built = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("palette.html".into()))
            .title("VoxLocal")
            .inner_size(WIDTH, HEIGHT)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .center()
            .focused(true)
            .build();
        match built {
            Ok(w) => {
                let _ = w.set_focus();
            }
            Err(e) => log::warn!("barra dei comandi: {e}"),
        }
    });
}

/// La barra alta quanto i comandi mostrati (la pagina misura la scheda e la chiede).
pub fn resize(app: &AppHandle, height: f64) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_size(tauri::LogicalSize::new(WIDTH, height.clamp(80.0, HEIGHT)));
    }
}

pub fn close(app: &AppHandle) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = app2.get_webview_window(LABEL) {
            let _ = w.destroy();
        }
    });
}
