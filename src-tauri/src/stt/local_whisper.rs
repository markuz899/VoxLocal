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
}

impl LocalWhisper {
    /// `keep_warm = None` tiene il modello sempre in memoria.
    pub fn new(model_path: PathBuf, keep_warm: Option<Duration>) -> Self {
        Self {
            inner: Arc::new(Inner {
                model_path: Mutex::new(model_path),
                ctx: Mutex::new(None),
                keep_warm: Mutex::new(keep_warm),
            }),
        }
    }

    pub fn set_model(&self, path: PathBuf) {
        *self.inner.model_path.lock().unwrap() = path;
        self.unload();
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
        let ctx = WhisperContext::new_with_params(
            path.to_str().ok_or_else(|| anyhow!("percorso non UTF-8"))?,
            WhisperContextParameters::default(),
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
        p.set_no_timestamps(true);
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

        let text = state
            .as_iter()
            .map(|s| s.to_str_lossy().map(|c| c.into_owned()).unwrap_or_default())
            .collect::<String>()
            .trim()
            .to_string();
        let language =
            whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_string);

        if let Some((_, used)) = self.inner.ctx.lock().unwrap().as_mut() {
            *used = Instant::now();
        }
        if *self.inner.keep_warm.lock().unwrap() == Some(Duration::ZERO) {
            self.unload();
        }
        Ok(Transcript { text, language })
    }
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
