//! Configurazione dei provider e costruzione dell'implementazione giusta.
//! Nuovo provider = un file in `llm/` + un ramo in `build`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{Anthropic, CliProvider, Gemini, LlmError, LlmProvider, OpenAiCompatible, Result};
use crate::settings::secrets;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    OpenaiCompat,
    Anthropic,
    Gemini,
    Cli,
    /// Nessun post-processing: si incolla la trascrizione grezza.
    None,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    pub id: String,
    pub label: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub model: String,
    pub needs_key: bool,
    /// Solo openai_compat: false per i modelli che rifiutano temperature/max_tokens.
    pub send_sampling: bool,
    /// Solo cli.
    pub command: Vec<String>,
    pub system_flag: Option<String>,
    pub model_flag: Option<String>,
    pub clear_env: Vec<String>,
    /// Se presente, il provider espone anche `/audio/transcriptions` con questo modello.
    pub stt_model: Option<String>,
}

pub const NONE_ID: &str = "none";

fn oa(id: &str, label: &str, base_url: &str, model: &str, needs_key: bool) -> ProviderConfig {
    ProviderConfig {
        id: id.into(),
        label: label.into(),
        kind: ProviderKind::OpenaiCompat,
        base_url: base_url.into(),
        model: model.into(),
        needs_key,
        send_sampling: true,
        ..Default::default()
    }
}

#[cfg(feature = "subscription-cli")]
fn cli(id: &str, label: &str, command: &[&str], model: &str) -> ProviderConfig {
    ProviderConfig {
        id: id.into(),
        label: label.into(),
        kind: ProviderKind::Cli,
        model: model.into(),
        command: command.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

/// Preset precompilati. I nomi dei modelli invecchiano: si cambiano in `settings.json`
/// (voce `providers`) senza toccare il codice.
pub fn presets() -> Vec<ProviderConfig> {
    #[allow(unused_mut)]
    let mut v = vec![
        ProviderConfig {
            id: NONE_ID.into(),
            label: "Nessuno (testo grezzo)".into(),
            kind: ProviderKind::None,
            ..Default::default()
        },
        // Il modello di pulizia addestrato per VoxLocal. Per ora gira in Ollama (`ollama create voxclean`);
        // quando sarà incluso nell'app questo provider resterà con lo stesso id.
        oa(
            super::LOCAL_ID,
            "VoxLocal (locale)",
            "http://localhost:11434/v1",
            "voxclean",
            false,
        ),
        ProviderConfig {
            send_sampling: false,
            stt_model: Some("gpt-4o-mini-transcribe".into()),
            ..oa(
                "openai",
                "OpenAI",
                "https://api.openai.com/v1",
                "gpt-5-mini",
                true,
            )
        },
        ProviderConfig {
            stt_model: Some("whisper-large-v3-turbo".into()),
            ..oa(
                "groq",
                "Groq",
                "https://api.groq.com/openai/v1",
                "llama-3.3-70b-versatile",
                true,
            )
        },
        oa(
            "openrouter",
            "OpenRouter",
            "https://openrouter.ai/api/v1",
            "meta-llama/llama-3.3-70b-instruct",
            true,
        ),
        oa(
            "together",
            "Together",
            "https://api.together.xyz/v1",
            "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            true,
        ),
        oa(
            "mistral",
            "Mistral",
            "https://api.mistral.ai/v1",
            "mistral-small-latest",
            true,
        ),
        oa(
            "deepseek",
            "DeepSeek",
            "https://api.deepseek.com/v1",
            "deepseek-chat",
            true,
        ),
        oa(
            "ollama",
            "Ollama (locale)",
            "http://localhost:11434/v1",
            "llama3.2",
            false,
        ),
        oa(
            "lmstudio",
            "LM Studio (locale)",
            "http://localhost:1234/v1",
            "local-model",
            false,
        ),
        oa(
            "llamacpp",
            "llama.cpp server (locale)",
            "http://localhost:8080/v1",
            "local-model",
            false,
        ),
        ProviderConfig {
            id: "anthropic".into(),
            label: "Anthropic API".into(),
            kind: ProviderKind::Anthropic,
            base_url: "https://api.anthropic.com".into(),
            model: "claude-haiku-5-5".into(),
            needs_key: true,
            ..Default::default()
        },
        ProviderConfig {
            id: "gemini".into(),
            label: "Google Gemini".into(),
            kind: ProviderKind::Gemini,
            base_url: "https://generativelanguage.googleapis.com".into(),
            model: "gemini-flash-latest".into(),
            needs_key: true,
            ..Default::default()
        },
    ];
    // Abbonamenti via CLI ufficiale, solo nelle build personali. Flag misurati il 2026-11-20 (D22).
    #[cfg(feature = "subscription-cli")]
    v.extend([
        ProviderConfig {
            system_flag: Some("--system-prompt".into()),
            model_flag: Some("--model".into()),
            clear_env: vec!["ANTHROPIC_API_KEY".into(), "ANTHROPIC_AUTH_TOKEN".into()],
            ..cli(
                "claude-cli",
                "Claude Code (abbonamento)",
                &[
                    "claude",
                    "-p",
                    "--tools",
                    "",
                    "--strict-mcp-config",
                    "--setting-sources",
                    "",
                    "--no-session-persistence",
                ],
                "haiku",
            )
        },
        ProviderConfig {
            model_flag: Some("-m".into()),
            ..cli(
                "codex-cli",
                "Codex (abbonamento)",
                &[
                    "codex",
                    "exec",
                    "--skip-git-repo-check",
                    "--ephemeral",
                    "-s",
                    "read-only",
                    "-c",
                    "model_reasoning_effort=none",
                    "-c",
                    "mcp_servers={}",
                ],
                "",
            )
        },
    ]);
    v
}

/// Provider per id: prima le personalizzazioni dell'utente, poi i preset.
pub fn find(id: &str, custom: &[ProviderConfig]) -> Option<ProviderConfig> {
    custom
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .or_else(|| presets().into_iter().find(|p| p.id == id))
}

/// Tutti i provider selezionabili (preset sovrascritti dalle personalizzazioni, più quelli nuovi).
pub fn all(custom: &[ProviderConfig]) -> Vec<ProviderConfig> {
    let mut out: Vec<ProviderConfig> = presets()
        .into_iter()
        .map(|p| custom.iter().find(|c| c.id == p.id).cloned().unwrap_or(p))
        .collect();
    let extra: Vec<ProviderConfig> = custom
        .iter()
        .filter(|c| !out.iter().any(|p| p.id == c.id))
        .cloned()
        .collect();
    out.extend(extra);
    out
}

/// `Ok(None)` = provider "none", nessun LLM.
pub fn build(cfg: &ProviderConfig) -> Result<Option<Arc<dyn LlmProvider>>> {
    let key = secrets::get(&cfg.id);
    let need_key = || {
        key.clone()
            .ok_or_else(|| LlmError::MissingKey(cfg.id.clone()))
    };
    if cfg.needs_key && key.is_none() {
        return Err(LlmError::MissingKey(cfg.id.clone()));
    }
    let p: Arc<dyn LlmProvider> = match cfg.kind {
        ProviderKind::None => return Ok(None),
        ProviderKind::OpenaiCompat => Arc::new(OpenAiCompatible {
            id: cfg.id.clone(),
            base_url: cfg.base_url.clone(),
            api_key: key,
            send_sampling: cfg.send_sampling,
        }),
        ProviderKind::Anthropic => Arc::new(Anthropic {
            id: cfg.id.clone(),
            base_url: cfg.base_url.clone(),
            api_key: need_key()?,
        }),
        ProviderKind::Gemini => Arc::new(Gemini {
            id: cfg.id.clone(),
            base_url: cfg.base_url.clone(),
            api_key: need_key()?,
        }),
        // Blocca anche i provider cli aggiunti a mano in settings.json.
        ProviderKind::Cli if !cfg!(feature = "subscription-cli") => {
            return Err(LlmError::Cli(
                "i provider via CLI non sono disponibili in questa build".into(),
            ))
        }
        ProviderKind::Cli => Arc::new(CliProvider {
            id: cfg.id.clone(),
            command: cfg.command.clone(),
            system_flag: cfg.system_flag.clone(),
            model_flag: cfg.model_flag.clone(),
            clear_env: cfg.clear_env.clone(),
        }),
    };
    Ok(Some(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_overrides_preset_and_extends_list() {
        let custom = vec![
            ProviderConfig {
                model: "qwen3".into(),
                ..find("ollama", &[]).unwrap()
            },
            oa("mio", "Mio server", "http://10.0.0.2/v1", "x", false),
        ];
        assert_eq!(find("ollama", &custom).unwrap().model, "qwen3");
        let all = all(&custom);
        assert_eq!(all.iter().filter(|p| p.id == "ollama").count(), 1);
        assert!(all.iter().any(|p| p.id == "mio"));
        assert!(find("nope", &custom).is_none());
    }

    #[test]
    fn builds_every_keyless_preset() {
        for p in presets().iter().filter(|p| !p.needs_key) {
            let built = build(p).unwrap();
            assert_eq!(built.is_none(), p.kind == ProviderKind::None, "{}", p.id);
        }
    }
}
