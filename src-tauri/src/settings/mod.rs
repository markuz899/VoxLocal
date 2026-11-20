//! Impostazioni persistite con tauri-plugin-store (`settings.json`).

use serde::{Deserialize, Serialize};
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

// ponytail: solo i campi della Fase 2; provider, modalità e cronologia arrivano nelle fasi 3-4
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
