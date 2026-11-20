//! Orchestrazione: hotkey → cattura → trascrizione → incolla, con stati, annullamento e metriche.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tokio_util::sync::CancellationToken;

use crate::audio::{self, AudioBuffer, Recorder};
use crate::llm::registry::{self, ProviderKind, NONE_ID};
use crate::llm::{self, LlmError, LlmRequest};
use crate::settings::{secrets, HotkeyMode, Settings};
use crate::storage::{Dictation, Mode, Storage};
use crate::stt::{LocalWhisper, OpenAiCompatibleStt, SttOptions, SttProvider, Transcript};
use crate::{models, overlay, paste, settings, tray};

/// Sotto questa durata è un tocco involontario della hotkey.
const MIN_AUDIO_MS: u64 = 300;
/// RMS sotto ~-54 dBFS: silenzio, inutile mandarlo a whisper (che allucina sul silenzio).
const SILENCE_RMS: f32 = 0.002;
pub const ESC: &str = "Escape";
const LOCAL_STT: &str = "local";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Idle,
    Recording,
    Transcribing,
    PostProcessing,
    Pasting,
}

#[derive(Clone, Serialize)]
struct ModelProgress {
    id: &'static str,
    done: u64,
    total: u64,
}

#[derive(Clone, Serialize)]
struct ModelDone {
    id: &'static str,
    error: Option<String>,
}

#[derive(Clone, Serialize)]
struct StateEvent {
    state: State,
}

pub struct Pipeline {
    app: AppHandle,
    pub stt: LocalWhisper,
    pub settings: Mutex<Settings>,
    pub storage: Arc<Storage>,
    inner: Mutex<Inner>,
    downloading: AtomicBool,
    mic_test: Mutex<Option<Recorder>>,
}

struct Inner {
    state: State,
    recorder: Option<Recorder>,
    cancel: CancellationToken,
    generation: u64,
}

impl Pipeline {
    pub fn new(app: AppHandle, settings: Settings, storage: Arc<Storage>) -> Arc<Self> {
        let model = models::find(&settings.local_model).unwrap_or(&models::MODELS[2]);
        Arc::new(Self {
            stt: LocalWhisper::new(models::path(model), settings.keep_warm()),
            app,
            settings: Mutex::new(settings),
            storage,
            inner: Mutex::new(Inner {
                state: State::Idle,
                recorder: None,
                cancel: CancellationToken::new(),
                generation: 0,
            }),
            downloading: AtomicBool::new(false),
            mic_test: Mutex::new(None),
        })
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn on_hotkey(self: &Arc<Self>, pressed: bool) {
        let mode = self.settings.lock().unwrap().hotkey_mode;
        let state = self.inner.lock().unwrap().state;
        match (state, pressed, mode) {
            (State::Idle, true, _) => self.start(),
            (State::Recording, false, HotkeyMode::Ptt) => self.finish(),
            (State::Recording, true, HotkeyMode::Toggle) => self.finish(),
            // ponytail: hotkey ignorata durante trascrizione/incolla; coda di 1 dettato se serve davvero
            _ => {}
        }
    }

    /// Esc: annulla qualunque cosa sia in corso.
    pub fn cancel(self: &Arc<Self>) {
        let rec = {
            let mut inner = self.inner.lock().unwrap();
            inner.cancel.cancel();
            inner.recorder.take()
        };
        if let Some(rec) = rec {
            std::thread::spawn(move || drop(rec.stop()));
        }
        log::info!("dettato annullato");
        self.set_state(State::Idle);
    }

    fn start(self: &Arc<Self>) {
        let (model, stt_provider) = {
            let s = self.settings.lock().unwrap();
            (s.local_model.clone(), s.stt_provider.clone())
        };
        let Some(info) = models::find(&model) else {
            return self.notify(&format!("Modello sconosciuto: {model}"));
        };
        let installed = models::is_installed(info);
        // Con lo STT remoto il modello locale serve solo come ripiego: non blocca il dettato.
        if stt_provider == LOCAL_STT && !installed {
            self.notify("Il modello di trascrizione non è ancora scaricato: lo scarico ora.");
            return self.ensure_model();
        }

        // Precarica il modello mentre l'utente parla: toglie il caricamento dalla latenza.
        if stt_provider == LOCAL_STT {
            let stt = self.stt.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(e) = stt.preload() {
                    log::error!("{e:#}");
                }
            });
        }

        let app = self.app.clone();
        let device = self.settings.lock().unwrap().audio_device.clone();
        let rec = match Recorder::start(device, move |level| {
            let _ = app.emit("audio://level", level);
        }) {
            Ok(r) => r,
            Err(e) => return self.notify(&format!("Microfono non disponibile: {e:#}")),
        };

        let (generation, max) = {
            let mut inner = self.inner.lock().unwrap();
            inner.recorder = Some(rec);
            inner.cancel = CancellationToken::new();
            inner.generation += 1;
            (inner.generation, self.settings.lock().unwrap().max_seconds)
        };
        self.set_state(State::Recording);

        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(max)).await;
            let still = {
                let inner = me.inner.lock().unwrap();
                inner.generation == generation && inner.state == State::Recording
            };
            if still {
                me.notify("Durata massima raggiunta: registrazione fermata.");
                me.finish();
            }
        });
    }

    fn finish(self: &Arc<Self>) {
        let released = Instant::now();
        let (rec, cancel) = {
            let mut inner = self.inner.lock().unwrap();
            (inner.recorder.take(), inner.cancel.clone())
        };
        let Some(rec) = rec else { return };
        self.set_state(State::Transcribing);
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = me.run(rec, &cancel, released).await {
                if !cancel.is_cancelled() {
                    log::error!("dettato fallito: {e:#}");
                    me.notify(&format!("Dettato non riuscito: {e:#}"));
                }
            }
            if !cancel.is_cancelled() {
                me.set_state(State::Idle);
            }
        });
    }

    async fn run(
        &self,
        rec: Recorder,
        cancel: &CancellationToken,
        released: Instant,
    ) -> Result<()> {
        let raw = tokio::task::spawn_blocking(move || rec.stop()).await??;
        let audio_ms = raw.duration_ms();
        if audio_ms < MIN_AUDIO_MS || raw.rms() < SILENCE_RMS {
            log::info!("scartato: {audio_ms} ms, rms {:.4}", raw.rms());
            return Ok(());
        }

        let t = Instant::now();
        let audio = audio::to_16k(raw)?;
        let resample_ms = t.elapsed().as_millis();

        let settings = self.settings();
        let vocabulary = self.storage.vocabulary().unwrap_or_default();
        let opts = SttOptions {
            language: settings.language(),
            initial_prompt: (!vocabulary.is_empty()).then(|| vocabulary.join(", ")),
            cancel: cancel.clone(),
        };
        let t = Instant::now();
        let (transcript, stt_id) = self.transcribe(&audio, &opts, &settings).await?;
        let stt_ms = t.elapsed().as_millis() as u64;
        if cancel.is_cancelled() || transcript.text.is_empty() {
            return Ok(());
        }

        let mut d = Dictation {
            language: transcript.language.clone(),
            raw_text: transcript.text.clone(),
            final_text: transcript.text.clone(),
            stt_model: (stt_id == "local").then(|| settings.local_model.clone()),
            stt_provider: stt_id,
            status: "ok",
            audio_ms,
            stt_ms: Some(stt_ms),
            ..Default::default()
        };

        // Post-processing LLM: qualunque errore → si incolla il testo grezzo (mai perdere il dettato).
        if let Some(mode) = self.storage.mode(&settings.active_mode).ok().flatten() {
            d.mode_id = Some(mode.id);
            let provider = mode
                .provider_id
                .clone()
                .unwrap_or_else(|| settings.default_llm.clone());
            if provider != NONE_ID {
                self.set_state(State::PostProcessing);
                let language = transcript
                    .language
                    .clone()
                    .unwrap_or_else(|| "rilevata automaticamente".into());
                let t = Instant::now();
                let res = self
                    .post_process(
                        &mode,
                        &provider,
                        &d.raw_text,
                        &language,
                        &vocabulary,
                        &settings,
                        cancel,
                    )
                    .await;
                d.llm_ms = Some(t.elapsed().as_millis() as u64);
                d.llm_provider = Some(provider.clone());
                match res {
                    Ok((text, model)) => {
                        d.final_text = text;
                        d.llm_model = Some(model);
                    }
                    Err(LlmError::Cancelled) => {}
                    Err(e) => {
                        log::warn!("LLM {provider} fallito, incollo il grezzo: {e}");
                        d.status = "llm_fallback";
                        d.error = Some(e.to_string());
                        self.notify(&format!("{provider}: {e}. Incollo il testo non elaborato."));
                    }
                }
            }
        }
        if cancel.is_cancelled() {
            d.status = "cancelled";
            self.save(&d);
            return Ok(());
        }

        self.set_state(State::Pasting);
        let t = Instant::now();
        let pasted = self.paste(d.final_text.clone(), &settings).await;
        d.paste_ms = Some(t.elapsed().as_millis() as u64);
        d.total_ms = Some(released.elapsed().as_millis() as u64);

        log::info!(
            "dettato {}: audio={audio_ms}ms resample={resample_ms}ms stt={stt_ms}ms ({}) llm={:?}ms ({:?}) \
             paste={:?}ms totale={:?}ms lingua={:?} caratteri={}",
            d.status,
            d.stt_provider,
            d.llm_ms,
            d.llm_provider,
            d.paste_ms,
            d.total_ms,
            d.language,
            d.final_text.chars().count()
        );
        if let Err(e) = pasted {
            log::warn!("incolla automatico non riuscito: {e:#}");
            d.status = "paste_failed";
            d.error = Some(format!("{e:#}"));
            let key = if cfg!(target_os = "macos") {
                "⌘V"
            } else {
                "Ctrl+V"
            };
            self.notify(&format!(
                "Testo copiato negli appunti: premi {key} per incollarlo."
            ));
        }
        self.save(&d);
        Ok(())
    }

    fn save(&self, d: &Dictation) {
        if let Err(e) = self.storage.insert_dictation(d) {
            log::error!("salvataggio in cronologia fallito: {e:#}");
        }
    }

    /// STT remoto se configurato, con ripiego su whisper locale se fallisce.
    async fn transcribe(
        &self,
        audio: &AudioBuffer,
        opts: &SttOptions,
        s: &Settings,
    ) -> Result<(Transcript, String)> {
        if s.stt_provider != LOCAL_STT {
            match self.remote_stt(s) {
                Ok(remote) => match remote.transcribe(audio, opts).await {
                    Ok(t) => return Ok((t, s.stt_provider.clone())),
                    Err(e) if opts.cancel.is_cancelled() => return Err(e),
                    Err(e) => log::warn!("STT remoto fallito, uso whisper locale: {e:#}"),
                },
                Err(e) => log::warn!("STT remoto non configurato, uso whisper locale: {e:#}"),
            }
        }
        Ok((self.stt.transcribe(audio, opts).await?, LOCAL_STT.into()))
    }

    fn remote_stt(&self, s: &Settings) -> Result<OpenAiCompatibleStt> {
        let cfg = registry::find(&s.stt_provider, &s.providers)
            .ok_or_else(|| anyhow!("provider STT sconosciuto: {}", s.stt_provider))?;
        let model = cfg
            .stt_model
            .clone()
            .ok_or_else(|| anyhow!("{} non offre trascrizione", cfg.id))?;
        let api_key = secrets::get(&cfg.id);
        if cfg.needs_key && api_key.is_none() {
            return Err(anyhow!("chiave API mancante per {}", cfg.id));
        }
        Ok(OpenAiCompatibleStt {
            id: cfg.id,
            base_url: cfg.base_url,
            api_key,
            model,
            timeout: Duration::from_secs(30),
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn post_process(
        &self,
        mode: &Mode,
        provider_id: &str,
        text: &str,
        language: &str,
        vocabulary: &[String],
        s: &Settings,
        cancel: &CancellationToken,
    ) -> std::result::Result<(String, String), LlmError> {
        let cfg = registry::find(provider_id, &s.providers)
            .ok_or_else(|| LlmError::Cli(format!("provider sconosciuto: {provider_id}")))?;
        let Some(provider) = registry::build(&cfg)? else {
            return Ok((text.to_string(), String::new()));
        };
        let model = mode
            .model
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| cfg.model.clone());
        let timeout = if cfg.kind == ProviderKind::Cli {
            s.cli_timeout_s
        } else {
            s.llm_timeout_s
        };
        let req = LlmRequest {
            system: llm::render(&mode.system_prompt, text, language, vocabulary),
            user: llm::render(&mode.user_template, text, language, vocabulary),
            model: model.clone(),
            temperature: Some(mode.temperature),
            max_tokens: 2048,
            timeout: Duration::from_secs(timeout),
        };
        tokio::select! {
            r = provider.complete(&req) => r.map(|r| (r.text, model)),
            _ = cancel.cancelled() => Err(LlmError::Cancelled),
        }
    }

    async fn paste(&self, text: String, settings: &Settings) -> Result<()> {
        let previous = paste::set_clipboard(&text)?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.app.run_on_main_thread(move || {
            let _ = tx.send(paste::send_paste_keys());
        })?;
        // Se i tasti falliscono, il testo resta negli appunti apposta (niente ripristino).
        rx.await??;
        if settings.restore_clipboard {
            paste::restore_later(
                previous,
                text,
                Duration::from_millis(settings.restore_delay_ms),
            );
        }
        Ok(())
    }

    fn set_state(&self, state: State) {
        let prev = std::mem::replace(&mut self.inner.lock().unwrap().state, state);
        if prev == state {
            return;
        }
        let _ = self.app.emit("pipeline://state", StateEvent { state });
        tray::show_state(&self.app, state);
        overlay::show_state(&self.app, state);
        // Esc globale solo mentre serve (DECISIONS D7). Fuori dal lock: register tocca il main thread.
        let gs = self.app.global_shortcut();
        let res = match (prev, state) {
            (State::Idle, _) => gs.register(ESC),
            (_, State::Idle) => gs.unregister(ESC),
            _ => Ok(()),
        };
        if let Err(e) = res {
            log::warn!("Esc globale: {e}");
        }
    }

    /// Scarica il modello configurato se manca (una sola volta per volta).
    /// Scarica il modello configurato se manca.
    pub fn ensure_model(self: &Arc<Self>) {
        let id = self.settings.lock().unwrap().local_model.clone();
        self.download_model(&id);
    }

    /// Scarica un modello (uno alla volta). Eventi: `models://progress` e `models://done`.
    pub fn download_model(self: &Arc<Self>, id: &str) {
        let Some(info) = models::find(id) else {
            return;
        };
        if models::is_installed(info) || self.downloading.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let app = me.app.clone();
            let mut last = u64::MAX;
            let res = models::download(info, |done, total| {
                let pct = done * 100 / total.max(1);
                if pct != last {
                    last = pct;
                    let _ = app.emit(
                        "models://progress",
                        ModelProgress {
                            id: info.id,
                            done,
                            total,
                        },
                    );
                    tray::set_status(&app, &format!("Scarico {}… {pct}%", info.id));
                }
            })
            .await;
            me.downloading.store(false, Ordering::SeqCst);
            let error = res.as_ref().err().map(|e| format!("{e:#}"));
            let _ = me.app.emit(
                "models://done",
                ModelDone {
                    id: info.id,
                    error: error.clone(),
                },
            );
            match error {
                None => {
                    tray::set_status(&me.app, "Pronto");
                    me.notify(&format!("Modello {} pronto.", info.label));
                }
                Some(e) => {
                    log::error!("download modello: {e}");
                    tray::set_status(&me.app, "Download non riuscito");
                    me.notify(&format!("Download del modello non riuscito: {e}"));
                }
            }
        });
    }

    /// Prova del microfono per le impostazioni: emette `audio://level` finché non viene fermata.
    pub fn mic_test(&self, on: bool) -> Result<()> {
        let mut slot = self.mic_test.lock().unwrap();
        if let Some(rec) = slot.take() {
            let _ = rec.stop();
        }
        if on {
            let app = self.app.clone();
            let device = self.settings.lock().unwrap().audio_device.clone();
            *slot = Some(Recorder::start(device, move |level| {
                let _ = app.emit("audio://level", level);
            })?);
        }
        Ok(())
    }

    pub fn is_downloading(&self) -> bool {
        self.downloading.load(Ordering::SeqCst)
    }

    /// Unico punto in cui cambiano le impostazioni (tray e finestra): applica gli effetti
    /// collaterali, salva, aggiorna il tray e avvisa le finestre aperte.
    pub fn apply_settings(self: &Arc<Self>, new: Settings) -> Result<()> {
        let old = self.settings();
        if new.hotkey != old.hotkey {
            let gs = self.app.global_shortcut();
            gs.register(new.hotkey.as_str())
                .map_err(|e| anyhow!("la scorciatoia {} non è disponibile: {e}", new.hotkey))?;
            let _ = gs.unregister(old.hotkey.as_str());
        }
        if new.local_model != old.local_model {
            let info = models::find(&new.local_model)
                .ok_or_else(|| anyhow!("modello sconosciuto: {}", new.local_model))?;
            self.stt.set_model(models::path(info));
        }
        if new.keep_warm_minutes != old.keep_warm_minutes {
            self.stt.set_keep_warm(new.keep_warm());
        }
        *self.settings.lock().unwrap() = new.clone();
        settings::save(&self.app, &new);
        tray::refresh(&self.app);
        let _ = self.app.emit("settings://changed", &new);
        Ok(())
    }

    pub fn notify(&self, body: &str) {
        use tauri_plugin_notification::NotificationExt;
        log::info!("notifica: {body}");
        let _ = self
            .app
            .notification()
            .builder()
            .title("VoxLocal")
            .body(body)
            .show();
    }
}
