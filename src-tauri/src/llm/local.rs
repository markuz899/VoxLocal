//! Modello di pulizia locale: un `llama-server` (llama.cpp) incluso nell'app come sidecar, avviato quando
//! serve con il GGUF scaricato e fermato dopo un po' di inattività. Niente Ollama né altro da installare.
//!
//! Processo separato e non libreria: whisper-rs e llama.cpp portano ciascuno la propria copia di ggml,
//! che nello stesso eseguibile andrebbero in conflitto; e se il server cade, l'app resta in piedi.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::json;

use super::{http, non_empty, send_json, LlmError, LlmProvider, LlmRequest, LlmResponse, Result};
use crate::models;

struct Server {
    child: Child,
    port: u16,
    model: PathBuf,
    last_used: Instant,
    /// Ha risposto al controllo di salute: il modello è caricato.
    ready: bool,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

static SERVER: Mutex<Option<Server>> = Mutex::new(None);

pub struct LocalLlm {
    pub id: String,
}

/// Il template di Qwen3 senza ragionamento, identico a quello usato in addestramento
/// (`cleanup-model/train/train.py`, `chat`).
fn chat_prompt(user: &str) -> String {
    format!("<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n")
}

fn server_exe() -> Result<PathBuf> {
    // Tauri mette i sidecar accanto all'eseguibile dell'app (anche in sviluppo, in target/debug).
    let name = if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    };
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(name)))
        .filter(|p| p.exists())
        .ok_or_else(|| {
            LlmError::Cli("motore locale (llama-server) mancante nell'installazione".into())
        })?;
    Ok(exe)
}

fn spawn(model: &Path) -> Result<Server> {
    // ponytail: porta libera chiesta al sistema e rilasciata subito; la corsa con un altro programma
    // è teorica, e in quel caso il controllo di salute fallisce e il dettato ripiega sul testo grezzo
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map_err(|e| LlmError::Cli(format!("nessuna porta libera: {e}")))?
        .port();
    let mut cmd = Command::new(server_exe()?);
    cmd.arg("-m")
        .arg(model)
        .args(["--host", "127.0.0.1", "--port", &port.to_string()])
        // un dettato alla volta, testi brevi; -ngl 99 = tutto sulla GPU dove c'è (Metal), ignorato altrove
        .args(["-c", "2048", "-np", "1", "-ngl", "99"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: niente finestra della console
    }
    let child = cmd
        .spawn()
        .map_err(|e| LlmError::Cli(format!("avvio del motore locale: {e}")))?;
    log::info!(
        "llama-server avviato sulla porta {port} con {}",
        model.display()
    );
    Ok(Server {
        child,
        port,
        model: model.to_path_buf(),
        last_used: Instant::now(),
        ready: false,
    })
}

/// Porta di un server con questo modello: riusa quello acceso, altrimenti lo (ri)avvia.
/// Il bool dice se è già pronto; se no, va aspettato con `wait_ready`.
fn port_for(model: &Path) -> Result<(u16, bool)> {
    let mut slot = SERVER.lock().unwrap();
    if let Some(s) = slot.as_mut() {
        let alive = matches!(s.child.try_wait(), Ok(None));
        if alive && s.model == model {
            s.last_used = Instant::now();
            return Ok((s.port, s.ready));
        }
    }
    *slot = None; // ferma quello vecchio (Drop) prima di avviarne un altro
    let s = spawn(model)?;
    let port = s.port;
    *slot = Some(s);
    Ok((port, false))
}

async fn wait_ready(port: u16, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        let ok = http()
            .get(format!("http://127.0.0.1:{port}/health"))
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success());
        if ok {
            if let Some(s) = SERVER.lock().unwrap().as_mut().filter(|s| s.port == port) {
                s.ready = true;
            }
            return Ok(());
        }
        let dead = SERVER
            .lock()
            .unwrap()
            .as_mut()
            .is_none_or(|s| s.port != port || !matches!(s.child.try_wait(), Ok(None)));
        if dead || start.elapsed() > timeout {
            *SERVER.lock().unwrap() = None;
            return Err(LlmError::Cli(if dead {
                "il motore locale si è chiuso durante l'avvio".into()
            } else {
                "il motore locale non si è avviato in tempo".into()
            }));
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

/// Accende il server in background, se il modello è scaricato (all'inizio di un dettato).
pub fn warm(model_id: &str) {
    let Some(info) = models::LLM_MODELS.iter().find(|m| m.id == model_id) else {
        return;
    };
    if !models::is_installed(info) {
        return;
    }
    match port_for(&models::path(info)) {
        Ok((port, false)) => {
            tauri::async_runtime::spawn(async move {
                if let Err(e) = wait_ready(port, Duration::from_secs(60)).await {
                    log::warn!("preavvio di llama-server: {e}");
                }
            });
        }
        Ok(_) => {}
        Err(e) => log::warn!("preavvio di llama-server: {e}"),
    }
}

/// Ferma il server se non viene usato da `idle` (chiamata periodica, come per Whisper).
pub fn release_if_idle(idle: Option<Duration>) {
    let Some(idle) = idle else { return };
    let mut slot = SERVER.lock().unwrap();
    if slot.as_ref().is_some_and(|s| s.last_used.elapsed() > idle) {
        log::info!(
            "llama-server fermato dopo {} s di inattività",
            idle.as_secs()
        );
        *slot = None;
    }
}

/// Alla chiusura dell'app: il server non deve restare in memoria da solo.
pub fn shutdown() {
    *SERVER.lock().unwrap() = None;
}

#[async_trait]
impl LlmProvider for LocalLlm {
    fn id(&self) -> &str {
        &self.id
    }

    async fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let info = models::LLM_MODELS
            .iter()
            .find(|m| m.id == req.model)
            .ok_or_else(|| LlmError::Cli(format!("modello locale sconosciuto: {}", req.model)))?;
        if !models::is_installed(info) {
            return Err(LlmError::Cli(format!(
                "il modello «{}» non è ancora scaricato: scaricalo in Provider LLM",
                info.label
            )));
        }
        let (port, ready) = port_for(&models::path(info))?;
        if !ready {
            wait_ready(port, Duration::from_secs(60)).await?;
        }
        let body = json!({
            "prompt": chat_prompt(&req.user),
            "n_predict": 512,
            "temperature": 0.0,
            "stop": ["<|im_end|>"],
            "cache_prompt": true,
        });
        let v = send_json(
            http()
                .post(format!("http://127.0.0.1:{port}/completion"))
                .timeout(req.timeout)
                .json(&body),
        )
        .await?;
        if let Some(s) = SERVER.lock().unwrap().as_mut() {
            s.last_used = Instant::now();
        }
        let text = v["content"]
            .as_str()
            .ok_or_else(|| LlmError::Parse("manca content".into()))?;
        non_empty(text.to_string())
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        Ok(models::LLM_MODELS
            .iter()
            .map(|m| m.id.to_string())
            .collect())
    }

    async fn health_check(&self) -> Result<()> {
        server_exe().map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_matches_training_template() {
        assert_eq!(
            chat_prompt("<mode>clean</mode><lang>it</lang>\nciao"),
            "<|im_start|>user\n<mode>clean</mode><lang>it</lang>\nciao<|im_end|>\n\
             <|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
    }
}
