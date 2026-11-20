//! Superficie IPC tipizzata: l'unico modo in cui il webview parla al backend.
//! Le chiavi API si possono scrivere e cancellare, mai rileggere.

use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

use crate::llm::registry::{self, ProviderConfig, ProviderKind};
use crate::pipeline::Pipeline;
use crate::settings::{secrets, Settings};
use crate::storage::{HistoryFacets, HistoryFilter, HistoryItem, Mode, ModeUpdate};
use crate::{audio, models, permissions};

type P<'a> = State<'a, Arc<Pipeline>>;
type R<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub fn get_settings(p: P) -> Settings {
    p.settings()
}

#[tauri::command]
pub fn save_settings(p: P, settings: Settings) -> R<Settings> {
    p.apply_settings(settings).map_err(|e| format!("{e:#}"))?;
    Ok(p.settings())
}

#[tauri::command]
pub async fn list_input_devices() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(audio::input_devices)
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn mic_test(p: P<'_>, on: bool) -> R<()> {
    let p = p.inner().clone();
    tauri::async_runtime::spawn_blocking(move || p.mic_test(on))
        .await
        .map_err(err)?
        .map_err(|e| format!("{e:#}"))
}

#[derive(Serialize)]
pub struct ModelView {
    id: &'static str,
    label: &'static str,
    size: u64,
    installed: bool,
    recommended: bool,
}

#[tauri::command]
pub fn list_models() -> Vec<ModelView> {
    models::MODELS
        .iter()
        .map(|m| ModelView {
            id: m.id,
            label: m.label,
            size: m.size,
            installed: models::is_installed(m),
            recommended: m.id == models::recommended(),
        })
        .collect()
}

#[tauri::command]
pub fn download_model(p: P, id: String) -> R<()> {
    if p.is_downloading() {
        return Err("c'è già un download in corso".into());
    }
    p.download_model(&id);
    Ok(())
}

#[tauri::command]
pub fn delete_model(p: P, id: String) -> R<()> {
    let info = models::find(&id).ok_or("modello sconosciuto")?;
    if p.settings().local_model == id {
        return Err("è il modello in uso: scegline un altro prima di eliminarlo".into());
    }
    std::fs::remove_file(models::path(info)).map_err(err)
}

#[derive(Serialize)]
pub struct ProviderView {
    #[serde(flatten)]
    config: ProviderConfig,
    has_key: bool,
}

#[tauri::command]
pub fn list_providers(p: P) -> Vec<ProviderView> {
    registry::all(&p.settings().providers)
        .into_iter()
        .map(|c| ProviderView {
            has_key: secrets::get(&c.id).is_some(),
            config: c,
        })
        .collect()
}

/// Salva URL e modello di un provider (personalizzazione di un preset o provider nuovo).
#[tauri::command]
pub fn save_provider(p: P, provider: ProviderConfig) -> R<()> {
    if provider.id.trim().is_empty() {
        return Err("l'id del provider è obbligatorio".into());
    }
    if provider.kind == ProviderKind::Cli && !cfg!(feature = "subscription-cli") {
        return Err("i provider via CLI non sono disponibili in questa build".into());
    }
    let mut s = p.settings();
    s.providers.retain(|c| c.id != provider.id);
    s.providers.push(provider);
    p.apply_settings(s).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn set_api_key(id: String, key: String) -> R<()> {
    if key.trim().is_empty() {
        return Err("la chiave è vuota".into());
    }
    secrets::set(&id, &key).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn delete_api_key(id: String) -> R<()> {
    secrets::delete(&id).map_err(|e| format!("{e:#}"))
}

/// Verifica raggiungibilità e chiave. Restituisce la latenza in ms.
#[tauri::command]
pub async fn check_provider(p: P<'_>, id: String) -> R<u64> {
    let cfg = registry::find(&id, &p.settings().providers).ok_or("provider sconosciuto")?;
    let Some(provider) = registry::build(&cfg).map_err(err)? else {
        return Ok(0);
    };
    let t = Instant::now();
    provider.health_check().await.map_err(err)?;
    Ok(t.elapsed().as_millis() as u64)
}

#[tauri::command]
pub async fn list_provider_models(p: P<'_>, id: String) -> R<Vec<String>> {
    let cfg = registry::find(&id, &p.settings().providers).ok_or("provider sconosciuto")?;
    match registry::build(&cfg).map_err(err)? {
        Some(provider) => provider.list_models().await.map_err(err),
        None => Ok(vec![]),
    }
}

#[tauri::command]
pub fn list_modes(p: P) -> R<Vec<Mode>> {
    p.storage.modes().map_err(err)
}

#[tauri::command]
pub fn update_mode(app: AppHandle, p: P, mode: ModeUpdate) -> R<()> {
    p.storage.update_mode(&mode).map_err(err)?;
    crate::tray::refresh(&app);
    Ok(())
}

#[tauri::command]
pub fn create_mode(app: AppHandle, p: P, name: String) -> R<String> {
    let slug = p.storage.create_mode(&name).map_err(err)?;
    crate::tray::refresh(&app);
    Ok(slug)
}

/// Aggiunge una modalità dalla libreria di modalità pronte.
#[tauri::command]
pub fn create_mode_with_task(app: AppHandle, p: P, name: String, task: String) -> R<String> {
    let slug = p.storage.create_mode_with_task(&name, &task).map_err(err)?;
    crate::tray::refresh(&app);
    Ok(slug)
}

#[tauri::command]
pub fn delete_mode(p: P, slug: String) -> R<()> {
    p.storage.delete_mode(&slug).map_err(err)?;
    let mut s = p.settings();
    if s.active_mode == slug {
        s.active_mode = crate::storage::RAW_SLUG.into();
    }
    // apply_settings ricostruisce anche il tray.
    p.apply_settings(s).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn reset_mode(app: AppHandle, p: P, slug: String) -> R<()> {
    p.storage.reset_mode(&slug).map_err(err)?;
    crate::tray::refresh(&app);
    Ok(())
}

#[tauri::command]
pub fn get_vocabulary(p: P) -> R<Vec<String>> {
    p.storage.vocabulary().map_err(err)
}

#[tauri::command]
pub fn add_term(p: P, term: String) -> R<()> {
    if term.trim().is_empty() {
        return Err("il termine è vuoto".into());
    }
    p.storage.add_term(&term).map_err(err)
}

#[tauri::command]
pub fn remove_term(p: P, term: String) -> R<()> {
    p.storage.remove_term(&term).map(|_| ()).map_err(err)
}

#[tauri::command]
pub fn get_history(
    p: P,
    filter: HistoryFilter,
    limit: usize,
    offset: usize,
) -> R<Vec<HistoryItem>> {
    p.storage
        .history(&filter, limit.min(200), offset)
        .map_err(err)
}

#[tauri::command]
pub fn history_facets(p: P, today: i64, week: i64) -> R<HistoryFacets> {
    p.storage.history_facets(today, week).map_err(err)
}

#[tauri::command]
pub fn delete_dictation(p: P, id: i64) -> R<()> {
    p.storage.delete_dictation(id).map_err(err)
}

#[tauri::command]
pub fn clear_history(p: P) -> R<usize> {
    p.storage.clear_history().map_err(err)
}

#[tauri::command]
pub fn copy_text(text: String) -> R<()> {
    crate::paste::set_clipboard(&text)
        .map(|_| ())
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn permissions_status() -> permissions::Status {
    permissions::status()
}

#[tauri::command]
pub async fn request_microphone() -> permissions::Access {
    tauri::async_runtime::spawn_blocking(permissions::request_microphone)
        .await
        .unwrap_or(permissions::Access::Unknown)
}

#[tauri::command]
pub fn request_accessibility() {
    permissions::request_accessibility();
}

#[tauri::command]
pub fn open_permission_settings(app: AppHandle, which: String) -> R<()> {
    let url = permissions::settings_url(&which).ok_or("nessuna pagina di impostazioni")?;
    app.opener().open_url(url, None::<&str>).map_err(err)
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> R<()> {
    let al = app.autolaunch();
    if enabled { al.enable() } else { al.disable() }.map_err(err)
}

#[tauri::command]
pub fn finish_onboarding(p: P) -> R<()> {
    let mut s = p.settings();
    s.onboarded = true;
    p.apply_settings(s).map_err(|e| format!("{e:#}"))
}

/// Cerca un aggiornamento adesso. `None` = già all'ultima versione.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> R<Option<String>> {
    crate::updater::check(&app)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> R<()> {
    crate::updater::install(&app)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    pending_update: Option<String>,
    data_dir: String,
    log_dir: String,
    personal_build: bool,
    platform: &'static str,
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    use tauri::Manager;
    AppInfo {
        version: app.package_info().version.to_string(),
        pending_update: crate::updater::pending_version(&app),
        data_dir: crate::data_dir().display().to_string(),
        log_dir: app
            .path()
            .app_log_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        personal_build: cfg!(feature = "subscription-cli"),
        platform: std::env::consts::OS,
    }
}
