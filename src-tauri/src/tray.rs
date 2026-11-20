//! Icona e menu nella barra di sistema.

use std::sync::Arc;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use crate::models;
use crate::pipeline::{Pipeline, State};
use crate::settings::{self, HotkeyMode};

const TRAY_ID: &str = "main";

struct Items {
    status: MenuItem<Wry>,
    ptt: CheckMenuItem<Wry>,
    toggle: CheckMenuItem<Wry>,
    models: Vec<CheckMenuItem<Wry>>,
    langs: Vec<CheckMenuItem<Wry>>,
}

// ponytail: lista fissa; la scelta libera fra le ~99 lingue di whisper arriva con le impostazioni (Fase 4)
const LANGS: &[(&str, &str)] = &[
    ("auto", "Automatica"),
    ("it", "Italiano"),
    ("en", "English"),
    ("es", "Español"),
    ("fr", "Français"),
    ("de", "Deutsch"),
];

pub fn build(app: &AppHandle, pipeline: &Arc<Pipeline>) -> tauri::Result<()> {
    let s = pipeline.settings();
    let status = MenuItem::with_id(app, "status", "Pronto", false, None::<&str>)?;
    let ptt = CheckMenuItem::with_id(
        app,
        "mode:ptt",
        "Tieni premuto per parlare",
        true,
        s.hotkey_mode == HotkeyMode::Ptt,
        None::<&str>,
    )?;
    let toggle = CheckMenuItem::with_id(
        app,
        "mode:toggle",
        "Premi per avviare/fermare",
        true,
        s.hotkey_mode == HotkeyMode::Toggle,
        None::<&str>,
    )?;
    let model_items = models::MODELS
        .iter()
        .map(|m| {
            CheckMenuItem::with_id(
                app,
                format!("model:{}", m.id),
                m.label,
                true,
                m.id == s.local_model,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let model_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> =
        model_items.iter().map(|i| i as _).collect();
    let model_menu = Submenu::with_items(app, "Modello whisper", true, &model_refs)?;
    let lang_items = LANGS
        .iter()
        .map(|(code, label)| {
            CheckMenuItem::with_id(
                app,
                format!("lang:{code}"),
                *label,
                true,
                *code == s.language,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let lang_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> =
        lang_items.iter().map(|i| i as _).collect();
    let lang_menu = Submenu::with_items(app, "Lingua", true, &lang_refs)?;

    let menu = Menu::with_items(
        app,
        &[
            &status,
            &MenuItem::with_id(
                app,
                "hotkey",
                format!("Scorciatoia: {}", s.hotkey),
                false,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &ptt,
            &toggle,
            &lang_menu,
            &model_menu,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "open", "Apri VoxLocal…", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "Esci", true, None::<&str>)?,
        ],
    )?;
    app.manage(Items {
        status,
        ptt,
        toggle,
        models: model_items,
        langs: lang_items,
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tauri::include_image!("icons/tray.png"))
        .icon_as_template(true)
        .tooltip("VoxLocal")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, ev| on_menu(app, ev.id().as_ref()))
        .build(app)?;
    Ok(())
}

fn on_menu(app: &AppHandle, id: &str) {
    let pipeline = app.state::<Arc<Pipeline>>().inner().clone();
    let items = app.state::<Items>();
    match id {
        "quit" => app.exit(0),
        "open" => open_main(app),
        "mode:ptt" | "mode:toggle" => {
            let mode = if id == "mode:ptt" {
                HotkeyMode::Ptt
            } else {
                HotkeyMode::Toggle
            };
            let _ = items.ptt.set_checked(mode == HotkeyMode::Ptt);
            let _ = items.toggle.set_checked(mode == HotkeyMode::Toggle);
            update(app, &pipeline, |s| s.hotkey_mode = mode);
        }
        _ if id.starts_with("lang:") => {
            let code = &id["lang:".len()..];
            for (item, (c, _)) in items.langs.iter().zip(LANGS) {
                let _ = item.set_checked(*c == code);
            }
            update(app, &pipeline, |s| s.language = code.into());
        }
        _ => {
            let Some(info) = id.strip_prefix("model:").and_then(models::find) else {
                return;
            };
            for (item, m) in items.models.iter().zip(models::MODELS) {
                let _ = item.set_checked(m.id == info.id);
            }
            update(app, &pipeline, |s| s.local_model = info.id.into());
            pipeline.stt.set_model(models::path(info));
            pipeline.ensure_model();
        }
    }
}

fn update(app: &AppHandle, pipeline: &Pipeline, f: impl FnOnce(&mut settings::Settings)) {
    let mut s = pipeline.settings.lock().unwrap();
    f(&mut s);
    settings::save(app, &s);
}

fn open_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_focus();
        return;
    }
    // Creata su richiesta e distrutta alla chiusura: a riposo non occupa RAM (ARCHITECTURE §10).
    let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("VoxLocal")
        .inner_size(720.0, 520.0)
        .build();
}

pub fn set_status(app: &AppHandle, text: &str) {
    if let Some(items) = app.try_state::<Items>() {
        let _ = items.status.set_text(text);
    }
}

pub fn show_state(app: &AppHandle, state: State) {
    let (title, status) = match state {
        State::Idle => (None, "Pronto"),
        State::Recording => (Some("●"), "In ascolto…"),
        State::Transcribing => (Some("…"), "Trascrivo…"),
        State::Pasting => (Some("…"), "Incollo…"),
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(title);
    }
    set_status(app, status);
}
