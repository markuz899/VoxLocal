pub mod audio;
pub mod models;
pub mod paste;
pub mod pipeline;
pub mod settings;
pub mod stt;
mod tray;

use std::sync::Arc;
use std::time::Duration;

use tauri::{Manager, RunEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

use pipeline::Pipeline;

pub fn run() {
    let shortcuts = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            let p = app.state::<Arc<Pipeline>>();
            let pressed = event.state() == ShortcutState::Pressed;
            if shortcut.matches(Modifiers::empty(), Code::Escape) {
                if pressed {
                    p.cancel();
                }
            } else {
                p.on_hotkey(pressed);
            }
        })
        .build();

    let app = tauri::Builder::default()
        // single-instance deve essere il primo plugin registrato.
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
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
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            whisper_rs::install_logging_hooks();

            let handle = app.handle().clone();
            let settings = settings::load(&handle);
            let pipeline = Pipeline::new(handle.clone(), settings.clone());
            app.manage(pipeline.clone());
            tray::build(&handle, &pipeline)?;

            if let Err(e) = handle.global_shortcut().register(settings.hotkey.as_str()) {
                pipeline.notify(&format!(
                    "Scorciatoia {} non disponibile: {e}",
                    settings.hotkey
                ));
            }
            pipeline.ensure_model();

            // Rilascia il modello whisper dopo `keep_warm_minutes` di inattività.
            let stt = pipeline.stt.clone();
            tauri::async_runtime::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_secs(30));
                loop {
                    tick.tick().await;
                    stt.release_if_idle();
                }
            });
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
