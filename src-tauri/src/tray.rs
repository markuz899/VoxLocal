//! Icona e menu nella barra di sistema. Il menu si ricostruisce da zero a ogni cambio di
//! impostazioni o modalità (`refresh`): niente stato duplicato da tenere allineato.

use std::sync::{Arc, Mutex};

use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::llm::registry::{self, ProviderKind};
use crate::pipeline::{Pipeline, State};
use crate::settings::HotkeyMode;

const TRAY_ID: &str = "main";

// ponytail: lista breve nel tray; la finestra delle impostazioni offre tutte le lingue
pub const LANGS: &[(&str, &str)] = &[
    ("auto", "Automatica"),
    ("it", "Italiano"),
    ("en", "English"),
    ("es", "Español"),
    ("fr", "Français"),
    ("de", "Deutsch"),
];

/// Voce di stato del menu corrente (cambia a ogni `refresh`).
struct Status(Mutex<Option<MenuItem<Wry>>>);

fn radio(
    app: &AppHandle,
    group: &str,
    title: &str,
    options: Vec<(String, String)>,
    selected: &str,
) -> tauri::Result<Submenu<Wry>> {
    let items = options
        .into_iter()
        .map(|(value, label)| {
            CheckMenuItem::with_id(
                app,
                format!("{group}:{value}"),
                label,
                true,
                value == selected,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|i| i as _).collect();
    Submenu::with_items(app, title, true, &refs)
}

fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn menu(app: &AppHandle, pipeline: &Pipeline) -> tauri::Result<Menu<Wry>> {
    let s = pipeline.settings();
    let modes = pipeline.storage.modes().unwrap_or_default();
    let providers = registry::all(&s.providers);

    let mode_menu = radio(
        app,
        "mode",
        "Modalità",
        modes
            .iter()
            .map(|m| (m.slug.clone(), m.name.clone()))
            .collect(),
        &s.active_mode,
    )?;
    let lang_menu = radio(app, "lang", "Lingua", pairs(LANGS), &s.language)?;
    let llm_menu = radio(
        app,
        "llm",
        "Provider LLM predefinito",
        providers
            .iter()
            .filter(|p| p.kind != ProviderKind::None)
            .map(|p| (p.id.clone(), p.label.clone()))
            .collect(),
        &s.default_llm,
    )?;
    let mut stt_options = vec![("local".to_string(), "Whisper locale".to_string())];
    stt_options.extend(
        providers
            .iter()
            .filter(|p| p.stt_model.is_some())
            .map(|p| (p.id.clone(), format!("{} (cloud)", p.label))),
    );
    let stt_menu = radio(app, "stt", "Trascrizione", stt_options, &s.stt_provider)?;
    let hk_menu = radio(
        app,
        "hk",
        &format!("Scorciatoia {}", s.hotkey),
        pairs(&[
            ("ptt", "Tieni premuto per parlare"),
            ("toggle", "Premi per avviare/fermare"),
        ]),
        if s.hotkey_mode == HotkeyMode::Ptt {
            "ptt"
        } else {
            "toggle"
        },
    )?;

    let status = MenuItem::with_id(app, "status", "Pronto", false, None::<&str>)?;
    let update = match crate::updater::pending_version(app) {
        Some(v) => MenuItem::with_id(
            app,
            "update",
            format!("Installa VoxLocal {v} e riavvia"),
            true,
            None::<&str>,
        )?,
        None => MenuItem::with_id(
            app,
            "check-update",
            "Cerca aggiornamenti",
            true,
            None::<&str>,
        )?,
    };
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &mode_menu,
            &lang_menu,
            &PredefinedMenuItem::separator(app)?,
            &llm_menu,
            &stt_menu,
            &hk_menu,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "repaste", "Copia l'ultimo dettato", true, None::<&str>)?,
            &MenuItem::with_id(app, "history", "Cronologia…", true, None::<&str>)?,
            &MenuItem::with_id(app, "open", "Impostazioni…", true, Some("CmdOrCtrl+,"))?,
            &update,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Esci da VoxLocal", true, Some("CmdOrCtrl+Q"))?,
        ],
    )?;
    *app.state::<Status>().0.lock().unwrap() = Some(status);
    Ok(menu)
}

pub fn build(app: &AppHandle, pipeline: &Arc<Pipeline>) -> tauri::Result<()> {
    app.manage(Status(Mutex::new(None)));
    let menu = menu(app, pipeline)?;
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

/// Ricostruisce il menu (dopo un cambio di impostazioni o di modalità).
pub fn refresh(app: &AppHandle) {
    let pipeline = app.state::<Arc<Pipeline>>().inner().clone();
    match (menu(app, &pipeline), app.tray_by_id(TRAY_ID)) {
        (Ok(m), Some(tray)) => {
            let _ = tray.set_menu(Some(m));
        }
        (Err(e), _) => log::error!("menu del tray: {e}"),
        _ => {}
    }
}

fn on_menu(app: &AppHandle, id: &str) {
    let pipeline = app.state::<Arc<Pipeline>>().inner().clone();
    match id {
        "quit" => return app.exit(0),
        "open" => return crate::open_main(app, None),
        "history" => return crate::open_main(app, Some("history")),
        "check-update" | "update" => {
            let app = app.clone();
            let install = id == "update";
            tauri::async_runtime::spawn(async move {
                let res = if install {
                    crate::updater::install(&app).await.map(|_| None)
                } else {
                    crate::updater::check(&app).await
                };
                let p = app.state::<Arc<Pipeline>>();
                match res {
                    Ok(Some(v)) => p.notify(&format!("È disponibile VoxLocal {v}.")),
                    Ok(None) => set_status(&app, "VoxLocal è aggiornato"),
                    Err(e) => p.notify(&format!("Aggiornamento non riuscito: {e:#}")),
                }
            });
            return;
        }
        "repaste" => {
            if let Ok(Some(text)) = pipeline.storage.last_dictation() {
                let _ = crate::paste::set_clipboard(&text);
                set_status(app, "Ultimo dettato copiato negli appunti");
            }
            return;
        }
        _ => {}
    }
    let Some((group, value)) = id.split_once(':') else {
        return;
    };
    let mut s = pipeline.settings();
    let value = value.to_string();
    match group {
        "mode" => s.active_mode = value,
        "llm" => s.default_llm = value,
        "stt" => s.stt_provider = value,
        "lang" => s.language = value,
        "hk" => {
            s.hotkey_mode = if value == "ptt" {
                HotkeyMode::Ptt
            } else {
                HotkeyMode::Toggle
            }
        }
        _ => return,
    }
    if let Err(e) = pipeline.apply_settings(s) {
        pipeline.notify(&format!("{e:#}"));
    }
}

pub fn set_status(app: &AppHandle, text: &str) {
    if let Some(item) = app.state::<Status>().0.lock().unwrap().as_ref() {
        let _ = item.set_text(text);
    }
}

pub fn show_state(app: &AppHandle, state: State) {
    let (title, status) = match state {
        State::Idle => (None, "Pronto"),
        State::Recording => (Some("●"), "In ascolto…"),
        State::Transcribing => (Some("…"), "Trascrivo…"),
        State::PostProcessing => (Some("✎"), "Elaboro con l'LLM…"),
        State::Pasting => (Some("…"), "Incollo…"),
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(title);
    }
    set_status(app, status);
}
