//! LLM tramite un CLI installato dall'utente (Claude Code, Codex…): usa il *suo* login e il suo
//! abbonamento. VoxLocal non vede né salva token: lancia il programma e legge stdout.

use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::{non_empty, LlmError, LlmProvider, LlmRequest, LlmResponse, Result};

pub struct CliProvider {
    pub id: String,
    /// Programma + argomenti fissi, es. `["claude", "-p", "--tools", ""]`.
    pub command: Vec<String>,
    /// Se presente, il system prompt va in questo flag; altrimenti finisce in testa allo stdin.
    pub system_flag: Option<String>,
    /// Flag per il modello (`--model`, `-m`); il modello vuoto lascia il default del CLI.
    pub model_flag: Option<String>,
    /// Variabili tolte dall'ambiente del figlio. Per Claude Code: con `ANTHROPIC_API_KEY`
    /// presente fatturerebbe sulle API invece che sull'abbonamento.
    pub clear_env: Vec<String>,
}

impl CliProvider {
    fn command(&self) -> Result<Command> {
        let (prog, args) = self
            .command
            .split_first()
            .ok_or_else(|| LlmError::Cli("comando vuoto".into()))?;
        let path = resolve(prog).ok_or_else(|| LlmError::Cli(format!("{prog} non trovato")))?;
        let mut cmd = Command::new(path);
        cmd.args(args)
            .env("PATH", search_path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for k in &self.clear_env {
            cmd.env_remove(k);
        }
        Ok(cmd)
    }
}

#[async_trait]
impl LlmProvider for CliProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn complete(&self, req: &LlmRequest) -> Result<LlmResponse> {
        let mut cmd = self.command()?;
        if let (Some(flag), false) = (&self.model_flag, req.model.is_empty()) {
            cmd.args([flag, &req.model]);
        }
        let input = match &self.system_flag {
            Some(flag) => {
                cmd.args([flag, &req.system]);
                req.user.clone()
            }
            None => format!("{}\n\n{}", req.system, req.user),
        };

        let run = async {
            let mut child = cmd.spawn().map_err(|e| LlmError::Cli(e.to_string()))?;
            let mut stdin = child.stdin.take().expect("stdin piped");
            stdin
                .write_all(input.as_bytes())
                .await
                .map_err(|e| LlmError::Cli(e.to_string()))?;
            drop(stdin);
            child
                .wait_with_output()
                .await
                .map_err(|e| LlmError::Cli(e.to_string()))
        };
        // Il drop del future (timeout o annullamento) uccide il processo: kill_on_drop.
        let out = tokio::time::timeout(req.timeout, run)
            .await
            .map_err(|_| LlmError::Timeout)??;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            let tail: Vec<&str> = err.lines().rev().take(3).collect();
            return Err(LlmError::Cli(format!(
                "uscita {}: {}",
                out.status,
                tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
            )));
        }
        non_empty(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    async fn health_check(&self) -> Result<()> {
        let prog = self.command.first().map(String::as_str).unwrap_or("");
        let path = resolve(prog).ok_or_else(|| LlmError::Cli(format!("{prog} non trovato")))?;
        let ok = Command::new(path)
            .arg("--version")
            .env("PATH", search_path())
            .output()
            .await
            .map_err(|e| LlmError::Cli(e.to_string()))?
            .status
            .success();
        ok.then_some(())
            .ok_or_else(|| LlmError::Cli(format!("{prog} --version è fallito")))
    }
}

/// Un'app avviata dal Finder non eredita il PATH della shell: aggiungo le cartelle
/// dove di solito finiscono i CLI installati dall'utente.
fn search_path() -> String {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if let Some(home) = dirs::home_dir() {
        for d in [".local/bin", ".npm-global/bin", ".bun/bin", ".cargo/bin"] {
            dirs.push(home.join(d));
        }
    }
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        dirs.push(d.into());
    }
    std::env::join_paths(dirs)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn resolve(prog: &str) -> Option<PathBuf> {
    let p = PathBuf::from(prog);
    if p.is_absolute() {
        return p.exists().then_some(p);
    }
    let exts: &[&str] = if cfg!(windows) {
        &["", ".exe", ".cmd"]
    } else {
        &[""]
    };
    std::env::split_paths(&search_path())
        .flat_map(|d| exts.iter().map(move |e| d.join(format!("{prog}{e}"))))
        .find(|c| c.is_file())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sh(script: &str) -> CliProvider {
        CliProvider {
            id: "fake".into(),
            command: vec!["sh".into(), "-c".into(), script.into()],
            system_flag: None,
            model_flag: None,
            clear_env: vec!["VOXLOCAL_TEST_SECRET".into()],
        }
    }

    fn req(timeout_ms: u64) -> LlmRequest {
        LlmRequest {
            system: "SYS".into(),
            user: "ciao".into(),
            model: String::new(),
            temperature: None,
            max_tokens: 10,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    #[tokio::test]
    async fn pipes_prompt_and_clears_env() {
        std::env::set_var("VOXLOCAL_TEST_SECRET", "leak");
        let p = sh(r#"cat; printf ' [%s]' "${VOXLOCAL_TEST_SECRET:-pulito}""#);
        let out = p.complete(&req(5000)).await.unwrap();
        assert_eq!(out.text, "SYS\n\nciao [pulito]");
    }

    #[tokio::test]
    async fn times_out_and_reports_failures() {
        let err = sh("sleep 5").complete(&req(200)).await.unwrap_err();
        assert!(matches!(err, LlmError::Timeout), "{err:?}");
        let err = sh("echo 'login scaduto' >&2; exit 1")
            .complete(&req(5000))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("login scaduto"), "{err}");
    }
}
