//! Dettature lunghe: quanto si aspetta dopo il rilascio, tutto alla fine contro a pezzi.
//!
//! cargo run --release --example chunked -- [--model base] [--lang it] file.wav
//! Simula il tempo reale: un pezzo si può trascrivere solo dopo che è stato detto, e uno alla volta.

use std::time::Instant;

use anyhow::{Context, Result};
use voxlocal_lib::{audio, models, stt::LocalWhisper, stt::SttOptions, stt::SttProvider};

#[tokio::main]
async fn main() -> Result<()> {
    let mut model = "base".to_string();
    let mut lang = Some("it".to_string());
    let mut file = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--model" => model = args.next().context("--model vuole un valore")?,
            "--lang" => lang = args.next().filter(|l| l != "auto"),
            f => file = Some(f.to_string()),
        }
    }
    let file = file.context("serve un file WAV")?;
    let info = models::find(&model).context("modello sconosciuto")?;
    let stt = LocalWhisper::new(models::path(info), None);
    let opts = SttOptions {
        language: lang,
        initial_prompt: None,
        cancel: Default::default(),
    };
    let raw = audio::read_wav(std::path::Path::new(&file))?;
    let rate = raw.sample_rate;
    let secs = |n: usize| n as f64 / rate as f64;
    stt.preload()?;

    // tutto alla fine
    let t = Instant::now();
    let whole = stt.transcribe(&audio::to_16k(raw.clone())?, &opts).await?;
    let whole_s = t.elapsed().as_secs_f64();

    // a pezzi, con l'orologio del parlato
    let (mut committed, mut busy_until, mut texts) = (0usize, 0f64, vec![]);
    let mut now = 0usize;
    while now < raw.samples.len() {
        now = (now + rate as usize / 2).min(raw.samples.len());
        let Some(cut) = audio::next_cut(&raw.samples[committed..now], rate) else {
            continue;
        };
        let part = audio::AudioBuffer {
            samples: raw.samples[committed..committed + cut].to_vec(),
            sample_rate: rate,
        };
        committed += cut;
        let t = Instant::now();
        texts.push(stt.transcribe(&audio::to_16k(part)?, &opts).await?.text);
        busy_until = busy_until.max(secs(now)) + t.elapsed().as_secs_f64();
        println!("pezzo fino a {:.1} s", secs(committed));
    }
    let end = secs(raw.samples.len());
    let tail = audio::AudioBuffer {
        samples: raw.samples[committed..].to_vec(),
        sample_rate: rate,
    };
    let t = Instant::now();
    texts.push(stt.transcribe(&audio::to_16k(tail)?, &opts).await?.text);
    let wait = (busy_until - end).max(0.0) + t.elapsed().as_secs_f64();

    println!("\naudio {end:.0} s, modello {model}");
    println!("tutto alla fine: {whole_s:.1} s di attesa dopo il rilascio");
    println!(
        "a pezzi ({} pezzi): {wait:.1} s di attesa dopo il rilascio",
        texts.len()
    );
    println!("\n-- tutto --\n{}", whole.text.trim());
    println!("\n-- a pezzi --\n{}", texts.join(" ").trim());
    Ok(())
}
