//! Overlay: piccola finestra trasparente in basso al centro, sempre in primo piano, che non prende
//! mai il focus (altrimenti ⌘V finirebbe qui invece che nell'app di destinazione, DECISIONS D7).
//!
//! Creata al primo dettato e distrutta dopo `IDLE_CLOSE` di inattività: una webview sempre viva
//! costava ~70 MB fra i processi WebKit e sforava il target di RAM a riposo (DECISIONS D29).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::pipeline::State;

pub const LABEL: &str = "overlay";
const WIDTH: f64 = 248.0;
const HEIGHT: f64 = 56.0;
/// Distanza dal bordo inferiore dello schermo, sopra il Dock/la barra delle applicazioni.
const BOTTOM_MARGIN: f64 = 96.0;
const IDLE_CLOSE: Duration = Duration::from_secs(30);

/// Incrementato a ogni cambio di stato: un timer di chiusura vecchio non chiude un overlay riusato.
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn create(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("VoxLocal")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .focusable(false)
        .focused(false)
        .visible(false)
        .build()?;
    // Click-through: non intercetta il mouse destinato alle finestre sotto.
    let _ = w.set_ignore_cursor_events(true);
    Ok(w)
}

fn place(w: &WebviewWindow) {
    let Ok(Some(m)) = w.primary_monitor() else {
        return;
    };
    let scale = m.scale_factor();
    let size = m.size().to_logical::<f64>(scale);
    let origin = m.position().to_logical::<f64>(scale);
    let _ = w.set_position(LogicalPosition::new(
        origin.x + (size.width - WIDTH) / 2.0,
        origin.y + size.height - HEIGHT - BOTTOM_MARGIN,
    ));
}

pub fn show_state(app: &AppHandle, state: State) {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let existing = app.get_webview_window(LABEL);

    if state == State::Idle {
        let Some(w) = existing else {
            return;
        };
        let _ = w.hide();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(IDLE_CLOSE).await;
            if GENERATION.load(Ordering::SeqCst) == generation {
                let _ = w.destroy();
            }
        });
        return;
    }

    let w = match existing {
        Some(w) => w,
        None => match create(app) {
            Ok(w) => w,
            Err(e) => return log::warn!("overlay: {e}"),
        },
    };
    if !w.is_visible().unwrap_or(false) {
        place(&w);
        let _ = w.show();
    }
}
