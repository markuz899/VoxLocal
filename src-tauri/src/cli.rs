//! Sottocomandi da terminale per gestire chiavi, vocabolario e provider prima che esista
//! la finestra delle impostazioni (Fase 4). Senza sottocomando parte l'app normale.

use std::io::{BufRead, IsTerminal, Write};
use std::time::Instant;

use crate::llm::registry::{self, ProviderKind};
use crate::llm::{self, LlmRequest};
use crate::settings::{self, secrets};
use crate::storage::Storage;

const HELP: &str = "\
voxlocal                         avvia l'app nella barra di sistema
voxlocal providers               elenca i provider e lo stato delle chiavi
voxlocal set-key <provider>      salva la chiave API nel portachiavi (letta da stdin)
voxlocal delete-key <provider>   rimuove la chiave dal portachiavi
voxlocal vocab [list]            mostra il vocabolario personale
voxlocal vocab add <termine…>    aggiunge termini (uno per argomento)
voxlocal vocab rm <termine>      rimuove un termine
voxlocal check <provider> [mod]  prova il provider con la modalità Pulito e misura la latenza";

/// `Some(codice di uscita)` se gli argomenti erano un sottocomando, `None` per avviare l'app.
pub fn run(args: &[String]) -> Option<i32> {
    let cmd = args.first()?.as_str();
    let rest = &args[1..];
    let res = match cmd {
        "help" | "--help" | "-h" => {
            println!("{HELP}");
            Ok(())
        }
        "providers" => providers(),
        "set-key" => set_key(rest),
        "delete-key" => {
            arg(rest).and_then(|id| secrets::delete(id).map(|_| println!("chiave di {id} rimossa")))
        }
        "vocab" => vocab(rest),
        "check" => check(rest),
        _ => return None,
    };
    Some(match res {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("errore: {e:#}");
            1
        }
    })
}

fn arg(rest: &[String]) -> anyhow::Result<&str> {
    rest.first()
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("manca il provider (vedi `voxlocal providers`)"))
}

fn providers() -> anyhow::Result<()> {
    let s = settings::read_file();
    for p in registry::all(&s.providers) {
        let key = match (p.kind, p.needs_key, secrets::get(&p.id).is_some()) {
            (ProviderKind::Cli, ..) => "abbonamento via CLI",
            (ProviderKind::None, ..) => "",
            (_, false, _) => "nessuna chiave richiesta",
            (_, true, true) => "chiave ✓",
            (_, true, false) => "chiave mancante",
        };
        let default = if p.id == s.default_llm {
            " (predefinito)"
        } else {
            ""
        };
        println!(
            "{:<12} {:<28} {:<36} {key}{default}",
            p.id, p.label, p.model
        );
    }
    Ok(())
}

fn set_key(rest: &[String]) -> anyhow::Result<()> {
    let id = arg(rest)?;
    let s = settings::read_file();
    registry::find(id, &s.providers)
        .ok_or_else(|| anyhow::anyhow!("provider sconosciuto: {id}"))?;
    if std::io::stdin().is_terminal() {
        print!(
            "Incolla la chiave di {id} e premi Invio (oppure: pbpaste | voxlocal set-key {id}): "
        );
        std::io::stdout().flush()?;
    }
    let mut key = String::new();
    std::io::stdin().lock().read_line(&mut key)?;
    let key = key.trim();
    anyhow::ensure!(!key.is_empty(), "chiave vuota");
    secrets::set(id, key)?;
    println!("chiave di {id} salvata nel portachiavi");
    Ok(())
}

fn vocab(rest: &[String]) -> anyhow::Result<()> {
    let db = Storage::open(&crate::db_path())?;
    match rest.first().map(String::as_str) {
        None | Some("list") => {
            for t in db.vocabulary()? {
                println!("{t}");
            }
        }
        Some("add") => {
            for t in &rest[1..] {
                db.add_term(t)?;
            }
        }
        Some("rm") => {
            let t = rest
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("manca il termine"))?;
            anyhow::ensure!(db.remove_term(t)?, "{t} non è nel vocabolario");
        }
        Some(other) => anyhow::bail!("comando vocab sconosciuto: {other}"),
    }
    Ok(())
}

fn check(rest: &[String]) -> anyhow::Result<()> {
    let id = arg(rest)?;
    let s = settings::read_file();
    let cfg = registry::find(id, &s.providers)
        .ok_or_else(|| anyhow::anyhow!("provider sconosciuto: {id}"))?;
    let Some(provider) = registry::build(&cfg)? else {
        println!("{id}: nessun LLM, si incolla il testo grezzo");
        return Ok(());
    };
    let db = Storage::open(&crate::db_path())?;
    let mode = db
        .mode("clean")?
        .ok_or_else(|| anyhow::anyhow!("modalità clean mancante"))?;
    let vocabulary = db.vocabulary()?;
    let model = rest.get(1).cloned().unwrap_or(cfg.model.clone());
    let text = "ehm allora ciao come stai tutto bene grazie cioè volevo dirti che domani la riunione è alle dieci";
    let timeout = if cfg.kind == ProviderKind::Cli {
        s.cli_timeout_s
    } else {
        s.llm_timeout_s
    };
    let req = LlmRequest {
        system: llm::render(&mode.system_prompt, text, "it", &vocabulary),
        user: llm::render(&mode.user_template, text, "it", &vocabulary),
        model: model.clone(),
        temperature: Some(mode.temperature),
        max_tokens: 512,
        timeout: std::time::Duration::from_secs(timeout),
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let t = Instant::now();
        provider.health_check().await?;
        println!("health check ok in {} ms", t.elapsed().as_millis());
        let t = Instant::now();
        let out = provider.complete(&req).await?;
        println!(
            "{id} / {} in {} ms:\n{}",
            if model.is_empty() { "default" } else { &model },
            t.elapsed().as_millis(),
            out.text
        );
        anyhow::Ok(())
    })
}
