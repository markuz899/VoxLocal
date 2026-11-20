//! Icona e menu nella barra di sistema.

use std::collections::HashMap;
use std::sync::Arc;

use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Wry};

use crate::llm::registry::{self, ProviderKind};
use crate::models;
use crate::pipeline::{Pipeline, State};
use crate::settings::{self, HotkeyMode, Settings};

const TRAY_ID: &str = "main";

// ponytail: lista fissa; la scelta libera fra le ~99 lingue di whisper arriva con le impostazioni (Fase 4)
const LANGS: &[(&str, &str)] = &[
    ("auto", "Automatica"),
    ("it", "Italiano"),
    ("en", "English"),
    ("es", "Español"),
    ("fr", "Français"),
    ("de", "Deutsch"),
];

/// Gruppi di voci a scelta singola, indicizzati per prefisso dell'id ("lang", "mode", …).
type Radios = HashMap<&'static str, Vec<(String, CheckMenuItem<Wry>)>>;

struct Items {
    status: MenuItem<Wry>,
    radios: Radios,
}

fn radio(
    app: &AppHandle,
    radios: &mut Radios,
    group: &'static str,
    title: &str,
    options: Vec<(String, String)>,
    selected: &str,
) -> tauri::Result<Submenu<Wry>> {
    let items = options
        .into_iter()
        .map(|(value, label)| {
            let item = CheckMenuItem::with_id(
                app,
                format!("{group}:{value}"),
                label,
                true,
                value == selected,
                None::<&str>,
            )?;
            Ok((value, item))
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|(_, i)| i as _).collect();
    let sub = Submenu::with_items(app, title, true, &refs)?;
    radios.insert(group, items);
    Ok(sub)
}

fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

pub fn build(app: &AppHandle, pipeline: &Arc<Pipeline>) -> tauri::Result<()> {
    let s = pipeline.settings();
    let mut radios = Radios::new();

    let modes = pipeline.storage.modes().unwrap_or_default();
    let mode_menu = radio(
        app,
        &mut radios,
        "mode",
        "Modalità",
        modes
            .iter()
            .map(|m| (m.slug.clone(), m.name.clone()))
            .collect(),
        &s.active_mode,
    )?;
    let providers = registry::all(&s.providers);
    let llm_menu = radio(
        app,
        &mut radios,
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
    let stt_menu = radio(
        app,
        &mut radios,
        "stt",
        "Trascrizione",
        stt_options,
        &s.stt_provider,
    )?;
    let lang_menu = radio(
        app,
        &mut radios,
        "lang",
        "Lingua",
        pairs(LANGS),
        &s.language,
    )?;
    let model_menu = radio(
        app,
        &mut radios,
        "model",
        "Modello whisper",
        models::MODELS
            .iter()
            .map(|m| (m.id.to_string(), m.label.to_string()))
            .collect(),
        &s.local_model,
    )?;
    let hk_menu = radio(
        app,
        &mut radios,
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
            &model_menu,
            &hk_menu,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "repaste", "Copia l'ultimo dettato", true, None::<&str>)?,
            &MenuItem::with_id(app, "open", "Apri VoxLocal…", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "Esci", true, None::<&str>)?,
        ],
    )?;
    app.manage(Items { status, radios });

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
    match id {
        "quit" => return app.exit(0),
        "open" => return open_main(app),
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
    if let Some(list) = app.state::<Items>().radios.get(group) {
        for (v, item) in list {
            let _ = item.set_checked(v == value);
        }
    }
    let value = value.to_string();
    match group {
        "mode" => update(app, &pipeline, |s| s.active_mode = value),
        "llm" => update(app, &pipeline, |s| s.default_llm = value),
        "stt" => update(app, &pipeline, |s| s.stt_provider = value),
        "lang" => update(app, &pipeline, |s| s.language = value),
        "hk" => update(app, &pipeline, |s| {
            s.hotkey_mode = if value == "ptt" {
                HotkeyMode::Ptt
            } else {
                HotkeyMode::Toggle
            }
        }),
        "model" => {
            let Some(info) = models::find(&value) else {
                return;
            };
            update(app, &pipeline, |s| s.local_model = value);
            pipeline.stt.set_model(models::path(info));
            pipeline.ensure_model();
        }
        _ => {}
    }
}

fn update(app: &AppHandle, pipeline: &Pipeline, f: impl FnOnce(&mut Settings)) {
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
        State::PostProcessing => (Some("✎"), "Elaboro con l'LLM…"),
        State::Pasting => (Some("…"), "Incollo…"),
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(title);
    }
    set_status(app, status);
}
