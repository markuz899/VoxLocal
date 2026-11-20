//! Post-processing con LLM: trait comune, errori tipizzati, template delle modalità.

mod anthropic;
mod cli;
mod gemini;
mod openai_compat;
pub mod registry;

pub use anthropic::Anthropic;
pub use cli::CliProvider;
pub use gemini::Gemini;
pub use openai_compat::OpenAiCompatible;

use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct LlmRequest {
    pub system: String,
    pub user: String,
    pub model: String,
    pub temperature: Option<f64>,
    pub max_tokens: u32,
    pub timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct LlmResponse {
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("tempo scaduto")]
    Timeout,
    #[error("chiave API mancante per {0}")]
    MissingKey(String),
    #[error("chiave API rifiutata (HTTP {0})")]
    Auth(u16),
    #[error("HTTP {status}: {body}")]
    Http { status: u16, body: String },
    #[error("rete: {0}")]
    Network(String),
    #[error("risposta non valida: {0}")]
    Parse(String),
    #[error("{0}")]
    Cli(String),
    #[error("annullato")]
    Cancelled,
}

impl From<reqwest::Error> for LlmError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout
        } else if e.is_decode() {
            Self::Parse(e.to_string())
        } else {
            Self::Network(e.to_string())
        }
    }
}

pub type Result<T> = std::result::Result<T, LlmError>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> &str;
    async fn complete(&self, req: &LlmRequest) -> Result<LlmResponse>;
    async fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }
    async fn health_check(&self) -> Result<()>;
}

/// Client HTTP condiviso (pool di connessioni riusato tra i dettati: niente handshake TLS ogni volta).
pub(crate) fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("client HTTP")
    })
}

/// Invia, mappa 401/403 su `Auth` e gli altri non-2xx su `Http`, poi decodifica il JSON.
pub(crate) async fn send_json(rb: reqwest::RequestBuilder) -> Result<serde_json::Value> {
    let resp = rb.send().await?;
    let status = resp.status();
    if status == 401 || status == 403 {
        return Err(LlmError::Auth(status.as_u16()));
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(LlmError::Http {
            status: status.as_u16(),
            body: body.chars().take(300).collect(),
        });
    }
    Ok(resp.json().await?)
}

pub(crate) fn non_empty(text: String) -> Result<LlmResponse> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err(LlmError::Parse("testo vuoto".into()));
    }
    Ok(LlmResponse { text })
}

/// Sostituisce `{{text}}`, `{{language}}` e `{{vocabulary}}` (nessun motore di template).
pub fn render(template: &str, text: &str, language: &str, vocabulary: &[String]) -> String {
    let vocab = if vocabulary.is_empty() {
        "(nessuno)".to_string()
    } else {
        vocabulary.join(", ")
    };
    template
        .replace("{{language}}", language)
        .replace("{{vocabulary}}", &vocab)
        // Per ultimo: il testo dettato può contenere "{{…}}" e non va reinterpretato.
        .replace("{{text}}", text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_fills_variables_and_leaves_dictation_untouched() {
        let out = render(
            "[{{language}}] {{vocabulary}} <d>{{text}}</d>",
            "scrivi {{language}}",
            "it",
            &["VoxLocal".into(), "Tauri".into()],
        );
        assert_eq!(out, "[it] VoxLocal, Tauri <d>scrivi {{language}}</d>");
        assert_eq!(render("{{vocabulary}}", "", "en", &[]), "(nessuno)");
    }
}
