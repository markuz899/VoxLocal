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
    slow: bool,
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
            slow: models::slow_here(m.id),
        })
        .collect()
}

/// I modelli di pulizia locali, per la scheda Provider LLM.
#[tauri::command]
pub fn list_llm_models() -> Vec<ModelView> {
    models::LLM_MODELS
        .iter()
        .map(|m| ModelView {
            id: m.id,
            label: m.label,
            size: m.size,
            installed: models::is_installed(m),
            recommended: m.id == models::LLM_RECOMMENDED,
            // ponytail: misurato solo sul Mac M1 (0,9 s); sui PC senza GPU l'1,7B va provato
            slow: m.id != models::LLM_RECOMMENDED
                && !cfg!(all(target_os = "macos", target_arch = "aarch64")),
        })
        .collect()
}

#[tauri::command]
pub fn download_model(p: P, id: String) -> R<()> {
    p.download_model(&id); // se un altro download è in corso, questo aspetta il suo turno
    Ok(())
}

#[tauri::command]
pub fn delete_model(p: P, id: String) -> R<()> {
    let info = models::find(&id).ok_or("modello sconosciuto")?;
    let s = p.settings();
    let llm_in_use =
        crate::llm::registry::find(crate::llm::LOCAL_ID, &s.providers).map(|c| c.model);
    if s.local_model == id || llm_in_use.as_deref() == Some(id.as_str()) {
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
pub fn list_snippets(p: P) -> R<Vec<crate::storage::Snippet>> {
    p.storage.snippets().map_err(err)
}

#[tauri::command]
pub fn save_snippet(p: P, trigger: String, text: String) -> R<()> {
    p.storage
        .save_snippet(&trigger, &text)
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn delete_snippet(p: P, id: i64) -> R<()> {
    p.storage.delete_snippet(id).map_err(err)
}

#[tauri::command]
pub fn get_stats(p: P, since: i64, utc_offset_min: i64, days: usize) -> R<crate::storage::Stats> {
    p.storage
        .stats(since, utc_offset_min, days.min(366))
        .map_err(err)
}

#[tauri::command]
pub fn vocabulary_suggestions(p: P) -> R<Vec<(String, i64)>> {
    let dismissed = p.settings().vocab_dismissed;
    p.storage
        .vocabulary_suggestions(&dismissed, 20)
        .map_err(err)
}

/// Scarta un suggerimento del vocabolario: non verrà più proposto.
#[tauri::command]
pub fn dismiss_suggestion(p: P, term: String) -> R<()> {
    let mut s = p.settings();
    if !s
        .vocab_dismissed
        .iter()
        .any(|d| d.eq_ignore_ascii_case(&term))
    {
        s.vocab_dismissed.push(term);
    }
    p.apply_settings(s).map_err(|e| format!("{e:#}"))
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

/// Estensioni che `audio::decode_file` sa leggere.
const MEDIA_EXT: &[&str] = &[
    "mp3", "m4a", "aac", "mp4", "mov", "m4v", "mkv", "webm", "wav", "flac", "ogg", "oga", "caf",
    "aif", "aiff",
];

fn pick(app: &AppHandle, name: &str, ext: &[&str]) -> Option<std::path::PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .add_filter(name, ext)
        .blocking_pick_file()
        .and_then(|f| f.into_path().ok())
}

fn save_to(app: &AppHandle, default_name: &str, ext: &str) -> Option<std::path::PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .set_file_name(default_name)
        .add_filter(ext, &[ext])
        .blocking_save_file()
        .and_then(|f| f.into_path().ok())
}

/// Finestra di scelta di un file audio o video; `None` se l'utente annulla.
#[tauri::command]
pub async fn pick_media_file(app: AppHandle) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || pick(&app, "Audio / video", MEDIA_EXT))
        .await
        .ok()
        .flatten()
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn transcribe_file(p: P, path: String) -> R<()> {
    let path = std::path::PathBuf::from(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();
    if ext == "opus" {
        return Err("Opus files (WhatsApp voice notes) are not supported yet.".into());
    }
    p.transcribe_file(path).map_err(err)
}

#[tauri::command]
pub fn cancel_file(p: P) {
    p.cancel_file();
}

#[tauri::command]
pub async fn summarize(p: P<'_>, text: String) -> R<String> {
    p.summarize(&text).await.map_err(|e| format!("{e:#}"))
}

/// Salva un testo in un file scelto dall'utente; restituisce il percorso, `None` se annulla.
#[tauri::command]
pub async fn save_text_file(app: AppHandle, name: String, text: String) -> R<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = save_to(&app, &name, "txt") else {
            return Ok(None);
        };
        std::fs::write(&path, text).map_err(err)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(err)?
}

/// Esporta vocabolario, snippet, modalità personali e regole per app in un file JSON.
#[tauri::command]
pub async fn export_settings(app: AppHandle, p: P<'_>) -> R<Option<String>> {
    let mut bundle = p.storage.export_bundle().map_err(err)?;
    bundle.app_modes = p
        .settings()
        .app_modes
        .into_iter()
        .map(|r| (r.app, r.mode))
        .collect();
    let json = serde_json::to_string_pretty(&bundle).map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = save_to(&app, "voxlocal-settings.json", "json") else {
            return Ok(None);
        };
        std::fs::write(&path, json).map_err(err)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(err)?
}

#[derive(Serialize)]
pub struct ImportResult {
    terms: usize,
    snippets: usize,
    modes: usize,
    app_rules: usize,
}

/// Importa un file esportato, unendolo a quello che c'è. `None` se l'utente annulla.
#[tauri::command]
pub async fn import_settings(app: AppHandle, p: P<'_>) -> R<Option<ImportResult>> {
    let Some(path) =
        tauri::async_runtime::spawn_blocking(move || pick(&app, "VoxLocal", &["json"]))
            .await
            .map_err(err)?
    else {
        return Ok(None);
    };
    let bundle: crate::storage::Bundle =
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(err)?)
            .map_err(|_| "This is not a VoxLocal settings file.".to_string())?;
    let [terms, snippets, modes] = p.storage.import_bundle(&bundle).map_err(err)?;
    // Regole per app: quelle del file sostituiscono quelle con lo stesso nome di app.
    let mut s = p.settings();
    for (app, mode) in &bundle.app_modes {
        s.app_modes.retain(|r| !r.app.eq_ignore_ascii_case(app));
        s.app_modes.push(crate::settings::AppMode {
            app: app.clone(),
            mode: mode.clone(),
        });
    }
    p.apply_settings(s).map_err(|e| format!("{e:#}"))?;
    Ok(Some(ImportResult {
        terms,
        snippets,
        modes,
        app_rules: bundle.app_modes.len(),
    }))
}

#[tauri::command]
pub fn meeting_start(p: P) -> R<crate::pipeline::MeetingState> {
    p.meeting_start().map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn meeting_stop(p: P<'_>) -> R<i64> {
    p.meeting_stop().await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn meeting_state(p: P) -> crate::pipeline::MeetingState {
    p.meeting_state()
}

#[tauri::command]
pub fn list_meetings(p: P) -> R<Vec<crate::storage::Meeting>> {
    p.storage.meetings().map_err(err)
}

#[tauri::command]
pub fn get_meeting(p: P, id: i64) -> R<Option<crate::storage::Meeting>> {
    p.storage.meeting(id).map_err(err)
}

#[tauri::command]
pub async fn summarize_meeting(p: P<'_>, id: i64) -> R<crate::pipeline::MeetingSummary> {
    p.summarize_meeting(id).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn rename_meeting(p: P, id: i64, title: String) -> R<()> {
    p.storage.rename_meeting(id, &title).map_err(err)
}

#[tauri::command]
pub fn delete_meeting(p: P, id: i64) -> R<()> {
    p.delete_meeting(id).map_err(|e| format!("{e:#}"))
}

/// Il permesso "Registrazione schermo" (audio delle app su macOS); `request` apre la richiesta di sistema.
#[tauri::command]
pub fn screen_permission(request: bool) -> bool {
    crate::meeting::screen_permission(request)
}

/// Chiude la proposta di registrare o fermare una riunione.
#[tauri::command]
pub fn prompt_close(app: AppHandle) {
    crate::meeting_watch::close(&app);
}

/// Salva le cose da fare di una riunione dopo una modifica.
#[tauri::command]
pub fn save_meeting_actions(p: P, id: i64, actions: Vec<crate::storage::Action>) -> R<()> {
    p.save_meeting_actions(id, &actions)
        .map_err(|e| format!("{e:#}"))
}

/// Manda in Promemoria le cose da fare ancora aperte.
#[tauri::command]
pub async fn add_meeting_reminders(p: P<'_>, id: i64) -> R<usize> {
    p.add_meeting_reminders(id)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// La barra dei comandi: parole selezionate e comandi recenti.
#[tauri::command]
pub fn palette_data(p: P) -> crate::pipeline::PaletteData {
    p.palette_data()
}

/// Applica alla selezione il comando scritto nella barra.
#[tauri::command]
pub fn palette_run(p: P, instruction: String) {
    p.palette_run(instruction);
}

/// Dalla barra si passa a dettare il comando.
#[tauri::command]
pub fn palette_speak(p: P) {
    p.palette_speak();
}

/// Adatta l'altezza della barra ai comandi mostrati.
#[tauri::command]
pub fn palette_resize(app: AppHandle, height: f64) {
    crate::palette::resize(&app, height);
}

/// Chiude la barra senza comando.
#[tauri::command]
pub fn palette_cancel(p: P) {
    p.palette_cancel(true);
}

/// Apre la finestra principale su una sezione (dalla proposta riunione).
#[tauri::command]
pub fn open_section(app: AppHandle, section: String) {
    crate::open_main(&app, Some(&section));
}

/// Il permesso Calendari: "granted", "denied" o "undetermined"; `request` apre la richiesta di sistema.
#[tauri::command]
pub async fn calendar_access(request: bool) -> String {
    tauri::async_runtime::spawn_blocking(move || {
        crate::meeting::calendar::access(request).to_string()
    })
    .await
    .unwrap_or_else(|_| "denied".into())
}

#[tauri::command]
pub async fn ask(p: P<'_>, question: String) -> R<crate::pipeline::Answer> {
    p.ask(&question).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn ask_target(p: P) -> crate::pipeline::AskTarget {
    p.ask_target()
}
