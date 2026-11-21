pub mod audio;
pub mod cli;
mod commands;
mod context;
mod i18n;
pub mod llm;
pub mod models;
mod overlay;
pub mod paste;
mod permissions;
pub mod pipeline;
pub mod settings;
pub mod storage;
pub mod stt;
mod tray;
mod updater;

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

use pipeline::Pipeline;

pub const IDENTIFIER: &str = "app.voxlocal.desktop";

/// Stessa cartella di `app_data_dir()` di Tauri: così bench e sottocomandi la condividono con l'app.
pub fn data_dir() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(IDENTIFIER)
}

pub fn db_path() -> std::path::PathBuf {
    data_dir().join("voxlocal.db")
}

/// Apre (o porta davanti) la finestra principale, opzionalmente su una sezione (`#history`, …).
/// Creata su richiesta e distrutta alla chiusura: a riposo non occupa RAM (ARCHITECTURE §10).
pub(crate) fn open_main(app: &AppHandle, section: Option<&str>) {
    if let Some(w) = app.get_webview_window("main") {
        if let Some(s) = section {
            let _ = w.emit("navigate", s);
        }
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let url = format!("index.html#{}", section.unwrap_or(""));
    let res = WebviewWindowBuilder::new(app, "main", WebviewUrl::App(url.into()))
        .title("VoxLocal")
        .inner_size(880.0, 620.0)
        .min_inner_size(720.0, 520.0)
        .center()
        .build();
    match res {
        Ok(w) => {
            let _ = w.set_focus();
        }
        Err(e) => log::error!("finestra principale: {e}"),
    }
}

pub fn run() {
    let shortcuts = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            let p = app.state::<Arc<Pipeline>>();
            let pressed = event.state() == ShortcutState::Pressed;
            if shortcut.matches(Modifiers::empty(), Code::Escape) {
                if pressed {
                    p.cancel();
                }
                return;
            }
            // La scorciatoia del comando sulla selezione, altrimenti quella del dettato.
            let command = p.settings().command_hotkey;
            let is_command = command
                .parse::<tauri_plugin_global_shortcut::Shortcut>()
                .is_ok_and(|c| c.id() == shortcut.id());
            let trigger = if is_command {
                pipeline::Trigger::Command
            } else {
                pipeline::Trigger::Dictation
            };
            p.on_hotkey(pressed, trigger);
        })
        .build();

    let app = tauri::Builder::default()
        // single-instance deve essere il primo plugin registrato.
        // Riaprire l'app (doppio clic, Spotlight) porta davanti le impostazioni.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            open_main(app, None)
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(5_000_000)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(shortcuts)
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updater::Pending::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_input_devices,
            commands::mic_test,
            commands::list_models,
            commands::download_model,
            commands::delete_model,
            commands::list_providers,
            commands::save_provider,
            commands::set_api_key,
            commands::delete_api_key,
            commands::check_provider,
            commands::list_provider_models,
            commands::list_modes,
            commands::update_mode,
            commands::create_mode,
            commands::create_mode_with_task,
            commands::delete_mode,
            commands::reset_mode,
            commands::get_vocabulary,
            commands::add_term,
            commands::remove_term,
            commands::get_history,
            commands::history_facets,
            commands::list_snippets,
            commands::save_snippet,
            commands::delete_snippet,
            commands::get_stats,
            commands::vocabulary_suggestions,
            commands::dismiss_suggestion,
            commands::delete_dictation,
            commands::clear_history,
            commands::copy_text,
            commands::permissions_status,
            commands::request_microphone,
            commands::request_accessibility,
            commands::open_permission_settings,
            commands::get_autostart,
            commands::set_autostart,
            commands::finish_onboarding,
            commands::app_info,
            commands::check_update,
            commands::install_update,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            whisper_rs::install_logging_hooks();

            let handle = app.handle().clone();
            let settings = settings::load(&handle);
            let storage = Arc::new(storage::Storage::open(&db_path())?);
            match storage.purge(settings.retention_days) {
                Ok(n) if n > 0 => log::info!("cronologia: rimossi {n} dettati oltre la retention"),
                Err(e) => log::error!("pulizia cronologia: {e:#}"),
                _ => {}
            }
            let pipeline = Pipeline::new(handle.clone(), settings.clone(), storage);
            app.manage(pipeline.clone());
            tray::build(&handle, &pipeline)?;

            for hk in [&settings.hotkey, &settings.command_hotkey] {
                if hk.is_empty() {
                    continue;
                }
                if let Err(e) = handle.global_shortcut().register(hk.as_str()) {
                    pipeline.notify(&format!(
                        "{} {hk}: {e}",
                        i18n::tr(
                            &settings.ui_language,
                            "Shortcut not available",
                            "Scorciatoia non disponibile"
                        )
                    ));
                }
            }
            // Primo avvio: l'onboarding guida permessi e download del modello (centinaia di MB,
            // mai scaricati senza che l'utente lo sappia). Dopo, il modello mancante si riscarica.
            if settings.onboarded {
                pipeline.ensure_model();
            } else {
                open_main(&handle, Some("onboarding"));
            }

            // Rilascia il modello whisper dopo `keep_warm_minutes` di inattività.
            let stt = pipeline.stt.clone();
            tauri::async_runtime::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_secs(30));
                loop {
                    tick.tick().await;
                    stt.release_if_idle();
                }
            });
            updater::spawn_background_checks(handle.clone());
            log::info!("VoxLocal pronto, scorciatoia {}", settings.hotkey);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("avvio di VoxLocal fallito");

    app.run(|_, event| {
        // App da barra di sistema: chiudere l'ultima finestra non deve terminarla.
        if let RunEvent::ExitRequested {
            api, code: None, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}
