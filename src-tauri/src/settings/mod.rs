//! Impostazioni persistite con tauri-plugin-store (`settings.json`) e chiavi nel keyring.

pub mod secrets;

use serde::{Deserialize, Serialize};

use crate::llm::registry::ProviderConfig;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

const FILE: &str = "settings.json";
const KEY: &str = "settings";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HotkeyMode {
    Ptt,
    Toggle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub hotkey: String,
    pub hotkey_mode: HotkeyMode,
    /// "auto" oppure codice ISO ("it", "en", …).
    pub language: String,
    pub local_model: String,
    /// 0 = rilascia subito, negativo = sempre caldo.
    pub keep_warm_minutes: i64,
    pub restore_clipboard: bool,
    pub restore_delay_ms: u64,
    pub max_seconds: u64,
    /// Slug della modalità attiva (tabella `modes`).
    pub active_mode: String,
    /// Provider LLM per le modalità che non ne indicano uno.
    pub default_llm: String,
    /// Personalizzazioni dei preset o provider aggiuntivi (vedi `llm::registry`).
    pub providers: Vec<ProviderConfig>,
    /// "local" oppure l'id di un provider con `stt_model` (es. "groq").
    pub stt_provider: String,
    pub llm_timeout_s: u64,
    /// I CLI in abbonamento partono a freddo a ogni dettato: serve più margine.
    pub cli_timeout_s: u64,
    pub retention_days: u32,
    /// Nome del microfono; `None` = quello di default del sistema.
    pub audio_device: Option<String>,
    /// false finché l'onboarding non è completato.
    pub onboarded: bool,
    /// Controllo automatico degli aggiornamenti (all'avvio e ogni 6 ore).
    pub check_updates: bool,
    /// Regole "app → modalità": vincono sulla modalità attiva quando il dettato parte in quell'app.
    pub app_modes: Vec<AppMode>,
    /// Lingua dell'interfaccia: "en" (default) o "it".
    pub ui_language: String,
    /// Seconda scorciatoia: comando vocale sul testo selezionato.
    pub command_hotkey: String,
    /// Lingua d'arrivo della modalità di traduzione ({{target_language}} nei template).
    pub translate_to: String,
    /// Parole suggerite per il vocabolario che l'utente ha scartato: non si ripropongono.
    pub vocab_dismissed: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppMode {
    /// Nome (o parte del nome) dell'app, senza distinzione di maiuscole: "Slack", "Mail".
    pub app: String,
    /// Slug della modalità (tabella `modes`).
    pub mode: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            hotkey: "Alt+Space".into(),
            hotkey_mode: HotkeyMode::Ptt,
            language: system_language(),
            local_model: crate::models::recommended().into(),
            keep_warm_minutes: 10,
            restore_clipboard: true,
            restore_delay_ms: 250,
            max_seconds: 600,
            active_mode: crate::storage::RAW_SLUG.into(),
            default_llm: "ollama".into(),
            providers: vec![],
            stt_provider: "local".into(),
            llm_timeout_s: 15,
            cli_timeout_s: 20,
            retention_days: 90,
            audio_device: None,
            onboarded: false,
            check_updates: true,
            app_modes: vec![],
            ui_language: "en".into(),
            command_hotkey: "Alt+Shift+Space".into(),
            translate_to: "en".into(),
            vocab_dismissed: vec![],
        }
    }
}

impl Settings {
    pub fn keep_warm(&self) -> Option<std::time::Duration> {
        (self.keep_warm_minutes >= 0)
            .then(|| std::time::Duration::from_secs(self.keep_warm_minutes as u64 * 60))
    }

    pub fn language(&self) -> Option<String> {
        (self.language != "auto").then(|| self.language.clone())
    }
}

/// Lingua del sistema se whisper la conosce, altrimenti "auto".
/// Il rilevamento automatico su frasi di 2-3 s con `base` è inaffidabile (it scambiato per ru/pt/de).
fn system_language() -> String {
    sys_locale::get_locale()
        .and_then(|l| l.split(['-', '_']).next().map(str::to_lowercase))
        .filter(|code| whisper_rs::get_lang_id(code).is_some())
        .unwrap_or_else(|| "auto".into())
}

/// Lettura diretta del file, per i sottocomandi da terminale (l'app usa `load`).
pub fn read_file() -> Settings {
    std::fs::read(crate::data_dir().join(FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| serde_json::from_value(v[KEY].clone()).ok())
        .unwrap_or_default()
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    app.store(FILE)
        .ok()
        .and_then(|s| s.get(KEY))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn save<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    match app.store(FILE) {
        Ok(store) => {
            store.set(KEY, serde_json::to_value(settings).unwrap_or_default());
            if let Err(e) = store.save() {
                log::error!("salvataggio impostazioni fallito: {e}");
            }
        }
        Err(e) => log::error!("store impostazioni non disponibile: {e}"),
    }
}
