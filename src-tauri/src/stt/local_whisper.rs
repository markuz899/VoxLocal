//! whisper.cpp locale: modello caricato in modo lazy e tenuto "caldo" per N minuti.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use std::ffi::c_void;
use tokio_util::sync::CancellationToken;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{SttOptions, SttProvider, Transcript};
use crate::audio::{AudioBuffer, WHISPER_RATE};

#[derive(Clone)]
pub struct LocalWhisper {
    inner: Arc<Inner>,
}

struct Inner {
    model_path: Mutex<PathBuf>,
    ctx: Mutex<Option<(Arc<WhisperContext>, Instant)>>,
    keep_warm: Mutex<Option<Duration>>,
    /// Contesto minimo dell'encoder in frame (0 = sempre 1500, cioè 30 s).
    min_ctx: std::sync::atomic::AtomicI32,
}

impl LocalWhisper {
    /// `keep_warm = None` tiene il modello sempre in memoria.
    pub fn new(model_path: PathBuf, keep_warm: Option<Duration>) -> Self {
        Self {
            inner: Arc::new(Inner {
                model_path: Mutex::new(model_path),
                ctx: Mutex::new(None),
                keep_warm: Mutex::new(keep_warm),
                min_ctx: std::sync::atomic::AtomicI32::new(DEFAULT_MIN_CTX),
            }),
        }
    }

    pub fn set_model(&self, path: PathBuf) {
        *self.inner.model_path.lock().unwrap() = path;
        self.unload();
    }

    /// Contesto minimo dell'encoder in frame (50 al secondo); 0 = finestra fissa di 30 s.
    pub fn set_min_ctx(&self, frames: i32) {
        self.inner
            .min_ctx
            .store(frames, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_keep_warm(&self, keep_warm: Option<Duration>) {
        *self.inner.keep_warm.lock().unwrap() = keep_warm;
    }

    pub fn unload(&self) {
        if self.inner.ctx.lock().unwrap().take().is_some() {
            log::info!("modello whisper rilasciato");
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.inner.ctx.lock().unwrap().is_some()
    }

    /// Rilascia il modello se è inattivo da più di `keep_warm`.
    pub fn release_if_idle(&self) {
        let Some(keep) = *self.inner.keep_warm.lock().unwrap() else {
            return;
        };
        let mut ctx = self.inner.ctx.lock().unwrap();
        if ctx.as_ref().is_some_and(|(_, used)| used.elapsed() >= keep) {
            *ctx = None;
            log::info!("modello whisper rilasciato dopo {keep:?} di inattività");
        }
    }

    /// Carica il modello se serve (bloccante). Restituisce i ms di caricamento.
    pub fn preload(&self) -> Result<u64> {
        let t = Instant::now();
        self.context()?;
        Ok(t.elapsed().as_millis() as u64)
    }

    fn context(&self) -> Result<Arc<WhisperContext>> {
        // Il lock resta preso durante il caricamento: chi arriva dopo aspetta invece di caricare due volte.
        let mut slot = self.inner.ctx.lock().unwrap();
        if let Some((ctx, used)) = slot.as_mut() {
            *used = Instant::now();
            return Ok(ctx.clone());
        }
        let path = self.inner.model_path.lock().unwrap().clone();
        if !path.exists() {
            bail!("modello non trovato: {}", path.display());
        }
        let t = Instant::now();
        // Flash attention: encoder più veloce su Metal/CPU, nessuna perdita di qualità.
        let mut cp = WhisperContextParameters::default();
        cp.flash_attn(true);
        let ctx = WhisperContext::new_with_params(
            path.to_str().ok_or_else(|| anyhow!("percorso non UTF-8"))?,
            cp,
        )
        .map_err(|e| anyhow!("caricamento modello fallito: {e}"))?;
        log::info!("modello {} caricato in {:?}", path.display(), t.elapsed());
        let ctx = Arc::new(ctx);
        *slot = Some((ctx.clone(), Instant::now()));
        Ok(ctx)
    }

    /// Trascrizione bloccante: chiamarla da `spawn_blocking` o da un thread dedicato.
    pub fn transcribe_blocking(
        &self,
        audio: &AudioBuffer,
        opts: &SttOptions,
    ) -> Result<Transcript> {
        let (segments, language) = self.run(audio, opts, None)?;
        let text = segments
            .into_iter()
            .map(|s| s.text)
            .collect::<Vec<_>>()
            .join(" ");
        Ok(Transcript {
            text: drop_repeated_sentences(&text),
            language,
        })
    }

    /// Trascrizione lunga (file, riunioni) a segmenti con l'orario; `on_progress` riceve 0..100.
    pub fn transcribe_segments(
        &self,
        audio: &AudioBuffer,
        opts: &SttOptions,
        on_progress: impl FnMut(i32) + 'static,
    ) -> Result<(Vec<Segment>, Option<String>)> {
        let (mut segments, language) = self.run(audio, opts, Some(Box::new(on_progress)))?;
        for s in &mut segments {
            s.text = drop_repeated_sentences(&s.text);
        }
        segments.dedup_by(|b, a| a.text.len() > 20 && a.text == b.text);
        Ok((segments, language))
    }

    fn run(
        &self,
        audio: &AudioBuffer,
        opts: &SttOptions,
        progress: Option<Box<dyn FnMut(i32)>>,
    ) -> Result<(Vec<Segment>, Option<String>)> {
        if audio.sample_rate != WHISPER_RATE {
            bail!(
                "whisper vuole audio a 16 kHz, ricevuto {} Hz",
                audio.sample_rate
            );
        }
        let ctx = self.context()?;
        let mut state = ctx.create_state().map_err(|e| anyhow!("{e}"))?;

        let mut p = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        p.set_language(Some(opts.language.as_deref().unwrap_or("auto")));
        if let Some(prompt) = opts.initial_prompt.as_deref().filter(|s| !s.is_empty()) {
            p.set_initial_prompt(prompt);
        }
        p.set_no_context(true);
        // I segmenti con l'orario servono solo alla trascrizione lunga.
        p.set_no_timestamps(progress.is_none());
        if let Some(cb) = progress {
            p.set_progress_callback_safe(cb);
        }
        let min_ctx = self
            .inner
            .min_ctx
            .load(std::sync::atomic::Ordering::Relaxed);
        if min_ctx > 0 {
            p.set_audio_ctx(audio_ctx(audio.samples.len(), min_ctx));
        }
        p.set_print_special(false);
        p.set_print_progress(false);
        p.set_print_realtime(false);
        p.set_print_timestamps(false);
        // Non uso `set_abort_callback_safe`: in whisper-rs 0.16 confonde i tipi del puntatore e
        // la callback legge memoria non valida (panico "failed to lock mutex").
        let cancel = opts.cancel.clone();
        // SAFETY: `cancel` vive fino alla fine della funzione, whisper lo legge solo dentro `full`.
        unsafe {
            p.set_abort_callback(Some(abort_if_cancelled));
            p.set_abort_callback_user_data(&cancel as *const CancellationToken as *mut c_void);
        }

        state.full(p, &audio.samples).map_err(|e| {
            if opts.cancel.is_cancelled() {
                anyhow!("annullato")
            } else {
                anyhow!("trascrizione fallita: {e}")
            }
        })?;

        // whisper misura il tempo in centesimi di secondo.
        let segments: Vec<Segment> = state
            .as_iter()
            .filter_map(|s| {
                let text = s.to_str_lossy().ok()?.trim().to_string();
                (!text.is_empty()).then(|| Segment {
                    start_ms: s.start_timestamp().max(0) as u64 * 10,
                    end_ms: s.end_timestamp().max(0) as u64 * 10,
                    text,
                })
            })
            .collect();
        let language =
            whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_string);

        if let Some((_, used)) = self.inner.ctx.lock().unwrap().as_mut() {
            *used = Instant::now();
        }
        if *self.inner.keep_warm.lock().unwrap() == Some(Duration::ZERO) {
            self.unload();
        }
        Ok((segments, language))
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Whisper elabora sempre 30 s (1500 frame dell'encoder, 50 al secondo) anche per 2 s di parlato.
/// Riducendo il contesto alla durata reale + margine il costo scende in proporzione.
// Sotto ~10 s di contesto turbo entra in loop di ripetizione (misurato con 256 frame): minimo 512.
// ponytail: margine fisso di 2 s; se compaiono troncamenti sul finale, alzarlo o usare set_min_ctx(0)
fn audio_ctx(samples: usize, min_ctx: i32) -> i32 {
    let secs = samples as f32 / WHISPER_RATE as f32;
    (((secs + 2.0) * 50.0).ceil() as i32).clamp(min_ctx.min(1500), 1500)
}

const DEFAULT_MIN_CTX: i32 = 512;

/// Whisper a volte riscrive la stessa frase due volte di fila ("…sala grande. La riunione è…
/// sala grande."): si tiene la prima. Solo frasi di almeno 4 parole, così "Sì. Sì." resta.
// ponytail: confronto esatto senza punteggiatura; ripetizioni parziali non le prende.
fn drop_repeated_sentences(text: &str) -> String {
    let key = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut out: Vec<&str> = vec![];
    for sentence in text.split_inclusive(['.', '!', '?']) {
        let k = key(sentence);
        let repeat = out.last().is_some_and(|prev| key(prev) == k);
        if !(repeat && k.split(' ').count() >= 4) {
            out.push(sentence);
        }
    }
    out.concat().trim().to_string()
}

unsafe extern "C" fn abort_if_cancelled(user_data: *mut c_void) -> bool {
    (*(user_data as *const CancellationToken)).is_cancelled()
}

#[async_trait]
impl SttProvider for LocalWhisper {
    fn id(&self) -> &str {
        "local"
    }

    async fn transcribe(&self, audio: &AudioBuffer, opts: &SttOptions) -> Result<Transcript> {
        let (me, audio, opts) = (self.clone(), audio.clone(), opts.clone());
        tokio::task::spawn_blocking(move || me.transcribe_blocking(&audio, &opts)).await?
    }
}

#[cfg(test)]
mod repeat_tests {
    use super::drop_repeated_sentences as d;

    #[test]
    fn drops_whole_sentence_repeated_by_whisper() {
        assert_eq!(
            d("La riunione è alle 3, anzi alle 4, nella sala grande. La riunione è alle 3, anzi alle 4, nella sala grande."),
            "La riunione è alle 3, anzi alle 4, nella sala grande."
        );
        assert_eq!(d("Sì. Sì. Va bene."), "Sì. Sì. Va bene.");
        assert_eq!(
            d("Prima frase qui ok. Seconda frase diversa ora."),
            "Prima frase qui ok. Seconda frase diversa ora."
        );
    }

    /// Trascrizione lunga su un file vero: `VOXLOCAL_LONG_FILE=… VOXLOCAL_MODEL=… cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn long_file_gives_timed_segments_and_progress() {
        let (Ok(file), Ok(model)) = (
            std::env::var("VOXLOCAL_LONG_FILE"),
            std::env::var("VOXLOCAL_MODEL"),
        ) else {
            return;
        };
        let audio =
            crate::audio::to_16k(crate::audio::decode_file(file.as_ref()).unwrap()).unwrap();
        let stt = super::LocalWhisper::new(model.into(), None);
        let seen = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        let s2 = seen.clone();
        let t = std::time::Instant::now();
        let (segments, lang) = stt
            .transcribe_segments(
                &audio,
                &super::SttOptions {
                    language: Some("it".into()),
                    ..Default::default()
                },
                move |p| s2.lock().unwrap().push(p),
            )
            .unwrap();
        println!(
            "{} s di audio in {:?}, lingua {lang:?}, avanzamento {:?}",
            audio.duration_ms() / 1000,
            t.elapsed(),
            seen.lock().unwrap()
        );
        for s in &segments {
            println!("[{:>6} - {:>6}] {}", s.start_ms, s.end_ms, s.text);
        }
        assert!(segments.len() > 3);
        assert!(segments.windows(2).all(|w| w[0].start_ms <= w[1].start_ms));
        assert!(segments.last().unwrap().end_ms > audio.duration_ms() / 2);
        assert!(!seen.lock().unwrap().is_empty());
    }
}
