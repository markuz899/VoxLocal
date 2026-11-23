//! Misura la trascrizione locale sui WAV di prova.
//!
//! cargo run --release --example bench -- [--model base] [--lang it|en|auto] [--runs 3] [file.wav …]
//! Senza file usa `fixtures/*.wav`. Scarica il modello se manca.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use voxlocal_lib::{audio, models, stt::LocalWhisper, stt::SttOptions};

#[tokio::main]
async fn main() -> Result<()> {
    let mut model = "base".to_string();
    let mut lang: Option<String> = None;
    let mut runs = 3usize;
    let mut min_ctx: Option<i32> = None;
    let mut files: Vec<PathBuf> = vec![];
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--model" => model = args.next().context("--model vuole un valore")?,
            "--lang" => lang = args.next().filter(|l| l != "auto"),
            "--min-ctx" => {
                min_ctx = Some(args.next().context("--min-ctx vuole un valore")?.parse()?)
            }
            "--runs" => runs = args.next().context("--runs vuole un valore")?.parse()?,
            "-h" | "--help" => {
                println!("bench [--model ID] [--lang it|en|auto] [--runs N] [--min-ctx FRAME, 0 = 30 s fissi] [file.wav …]");
                println!(
                    "modelli: {}",
                    models::MODELS
                        .iter()
                        .map(|m| m.id)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                return Ok(());
            }
            f => files.push(f.into()),
        }
    }
    if files.is_empty() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures");
        files = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "wav"))
            .collect();
        files.sort();
    }
    if files.is_empty() || runs == 0 {
        bail!("nessun file WAV da misurare");
    }

    let info = models::find(&model).with_context(|| format!("modello sconosciuto: {model}"))?;
    if !models::is_installed(info) {
        eprintln!("scarico {}…", info.file);
        let mut last = u64::MAX;
        models::download(info, |d, t| {
            if d * 100 / t != last {
                last = d * 100 / t;
                eprint!("\r{last:>3}%");
            }
        })
        .await?;
        eprintln!();
    }

    whisper_rs::install_logging_hooks();
    let whisper = LocalWhisper::new(models::path(info), None);
    if let Some(m) = min_ctx {
        whisper.set_min_ctx(m);
    }
    let load_ms = whisper.preload()?;
    println!("{}", whisper_rs::print_system_info());
    println!("modello {model}  caricamento {load_ms} ms  run per file {runs}\n");
    println!(
        "{:<28} {:>7} {:>9} {:>8} {:>8} {:>6}  testo",
        "file", "audio s", "resample", "stt med", "stt min", "RTF"
    );

    let opts = SttOptions {
        language: lang,
        ..Default::default()
    };
    for f in &files {
        let raw = audio::decode_file(f)?;
        let secs = raw.duration_ms() as f64 / 1000.0;
        let t = Instant::now();
        let pcm = audio::to_16k(raw)?;
        let resample_ms = t.elapsed().as_secs_f64() * 1000.0;

        let mut times = vec![];
        let mut text = String::new();
        for _ in 0..runs {
            let t = Instant::now();
            let tr = whisper.transcribe_blocking(&pcm, &opts)?;
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            text = format!("[{}] {}", tr.language.unwrap_or_default(), tr.text);
        }
        times.sort_by(f64::total_cmp);
        let med = times[times.len() / 2];
        println!(
            "{:<28} {:>7.1} {:>7.1}ms {:>6.0}ms {:>6.0}ms {:>6.3}  {}",
            f.file_name().unwrap().to_string_lossy(),
            secs,
            resample_ms,
            med,
            times[0],
            med / 1000.0 / secs,
            text.chars().take(90).collect::<String>()
        );
    }
    Ok(())
}
