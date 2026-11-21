//! Il guardiano delle riunioni: ogni 2 s guarda se è iniziata una chiamata (microfono in uso da
//! un'altra app per 5 s e un'app da chiamata aperta) e propone di registrare le note; durante una
//! registrazione, se l'audio delle app tace da 2 minuti, propone di fermarla.
//! La proposta è una piccola finestra in alto a destra con due pulsanti (prompt.html).

use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::meeting::detect;
use crate::pipeline::Pipeline;

pub const LABEL: &str = "prompt";
const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 132.0;
/// Il microfono deve restare acceso così a lungo prima di proporre (un vocale veloce non è una chiamata).
const MIC_ON: Duration = Duration::from_secs(5);
/// L'audio delle app deve tacere così a lungo prima di proporre di fermare.
const APPS_QUIET: Duration = Duration::from_secs(120);

pub fn spawn(pipeline: Arc<Pipeline>) {
    std::thread::Builder::new()
        .name("voxlocal-meeting-watch".into())
        .spawn(move || watch(pipeline))
        .ok();
}

fn watch(p: Arc<Pipeline>) {
    // Stato del ciclo: da quando il microfono è acceso, e se per questa accensione si è già chiesto.
    let mut mic_since: Option<Instant> = None;
    let mut asked_start = false;
    let mut asked_end = false;
    loop {
        std::thread::sleep(Duration::from_secs(2));
        if !p.settings().meeting_auto_detect {
            continue;
        }
        if p.meeting_recording() {
            mic_since = None;
            asked_start = true; // dopo la registrazione il microfono resta della chiamata: non richiedere
            let quiet = p.meeting_apps_quiet_for().unwrap_or_default();
            if quiet >= APPS_QUIET && !asked_end {
                asked_end = true;
                show(&p.app_handle(), "end", "", &p.meeting_title_hint());
            } else if quiet < APPS_QUIET {
                asked_end = false;
            }
            continue;
        }
        asked_end = false;
        if !p.dictation_idle() {
            continue; // il microfono acceso è il nostro dettato
        }
        match detect::mic_in_use() {
            Some(true) => {
                let since = *mic_since.get_or_insert_with(Instant::now);
                if !asked_start && since.elapsed() >= MIC_ON {
                    if let Some(app) = detect::call_app() {
                        asked_start = true;
                        log::info!("riunione: chiamata rilevata in {app}");
                        show(&p.app_handle(), "start", &app, &p.meeting_title_hint());
                    }
                }
            }
            _ => {
                mic_since = None;
                asked_start = false;
            }
        }
    }
}

/// Mostra la proposta. `kind` è "start" o "end"; `app` l'app della chiamata; `title` l'evento del calendario.
pub fn show(app: &AppHandle, kind: &str, call_app: &str, title: &str) {
    let enc = |s: &str| {
        s.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect::<String>()
    };
    let lang = app.state::<Arc<Pipeline>>().settings().ui_language;
    let url = format!(
        "prompt.html#kind={kind}&app={}&title={}&lang={lang}",
        enc(call_app),
        enc(title)
    );
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(w) = app.get_webview_window(LABEL) {
            let _ = w.destroy();
        }
        let built = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App(url.into()))
            .title("VoxLocal")
            .inner_size(WIDTH, HEIGHT)
            .decorations(false)
            .transparent(true)
            .shadow(true)
            .resizable(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .focused(false)
            .build();
        match built {
            Ok(w) => {
                if let Ok(Some(m)) = w.primary_monitor() {
                    let scale = m.scale_factor();
                    let size = m.size().to_logical::<f64>(scale);
                    let origin = m.position().to_logical::<f64>(scale);
                    let _ = w.set_position(LogicalPosition::new(
                        origin.x + size.width - WIDTH - 16.0,
                        origin.y + 40.0,
                    ));
                }
            }
            Err(e) => log::warn!("proposta riunione: {e}"),
        }
    });
}

pub fn close(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.destroy();
    }
}
