//! Aggiornamenti firmati (minisign) da `latest.json` pubblicato a ogni release su GitHub.
//! La firma si verifica con la chiave pubblica in tauri.conf.json: un pacchetto alterato
//! viene rifiutato anche se il server è compromesso.

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Aggiornamento trovato e non ancora installato (mostrato nel tray e in Informazioni).
#[derive(Default)]
pub struct Pending(pub Mutex<Option<Update>>);

/// Primo controllo dopo l'avvio: non rallenta l'apertura e lascia stabilizzare la rete.
const STARTUP_DELAY: Duration = Duration::from_secs(20);
const INTERVAL: Duration = Duration::from_secs(6 * 3600);

pub fn pending_version(app: &AppHandle) -> Option<String> {
    app.state::<Pending>()
        .0
        .lock()
        .unwrap()
        .as_ref()
        .map(|u| u.version.clone())
}

/// Cerca un aggiornamento; se c'è lo ricorda, aggiorna il tray e restituisce la versione.
pub async fn check(app: &AppHandle) -> anyhow::Result<Option<String>> {
    let update = app.updater()?.check().await?;
    let version = update.as_ref().map(|u| u.version.clone());
    *app.state::<Pending>().0.lock().unwrap() = update;
    crate::tray::refresh(app);
    let _ = app.emit("update://available", &version);
    Ok(version)
}

/// Scarica, verifica la firma, installa e riavvia.
pub async fn install(app: &AppHandle) -> anyhow::Result<()> {
    let update = app
        .state::<Pending>()
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| anyhow::anyhow!("nessun aggiornamento da installare"))?;
    crate::tray::set_status(app, &format!("Scarico la versione {}…", update.version));
    let mut done = 0u64;
    let handle = app.clone();
    update
        .download_and_install(
            |chunk, total| {
                done += chunk as u64;
                if let Some(total) = total {
                    let _ = handle.emit("update://progress", (done, total));
                }
            },
            || {},
        )
        .await?;
    log::info!("aggiornamento {} installato, riavvio", update.version);
    app.restart();
}

/// Controlli periodici, se l'utente non li ha disattivati.
pub fn spawn_background_checks(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        loop {
            let enabled = app
                .state::<std::sync::Arc<crate::pipeline::Pipeline>>()
                .settings()
                .check_updates;
            if enabled && pending_version(&app).is_none() {
                match check(&app).await {
                    Ok(Some(v)) => app
                        .state::<std::sync::Arc<crate::pipeline::Pipeline>>()
                        .notify(&format!(
                            "È disponibile VoxLocal {v}. Installalo dal menu nella barra."
                        )),
                    Ok(None) => log::info!("nessun aggiornamento disponibile"),
                    Err(e) => log::warn!("controllo aggiornamenti: {e:#}"),
                }
            }
            tokio::time::sleep(INTERVAL).await;
        }
    });
}
