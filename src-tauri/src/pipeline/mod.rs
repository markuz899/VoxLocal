//! Orchestrazione: hotkey → cattura → trascrizione → incolla, con stati, annullamento e metriche.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tokio_util::sync::CancellationToken;

use crate::audio::{self, Recorder};
use crate::settings::{HotkeyMode, Settings};
use crate::stt::{LocalWhisper, SttOptions, SttProvider};
use crate::{models, paste, tray};

/// Sotto questa durata è un tocco involontario della hotkey.
const MIN_AUDIO_MS: u64 = 300;
/// RMS sotto ~-54 dBFS: silenzio, inutile mandarlo a whisper (che allucina sul silenzio).
const SILENCE_RMS: f32 = 0.002;
pub const ESC: &str = "Escape";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Idle,
    Recording,
    Transcribing,
    Pasting,
}

#[derive(Clone, Serialize)]
struct StateEvent {
    state: State,
}

pub struct Pipeline {
    app: AppHandle,
    pub stt: LocalWhisper,
    pub settings: Mutex<Settings>,
    inner: Mutex<Inner>,
    downloading: AtomicBool,
}

struct Inner {
    state: State,
    recorder: Option<Recorder>,
    cancel: CancellationToken,
    generation: u64,
}

impl Pipeline {
    pub fn new(app: AppHandle, settings: Settings) -> Arc<Self> {
        let model = models::find(&settings.local_model).unwrap_or(&models::MODELS[2]);
        Arc::new(Self {
            stt: LocalWhisper::new(models::path(model), settings.keep_warm()),
            app,
            settings: Mutex::new(settings),
            inner: Mutex::new(Inner {
                state: State::Idle,
                recorder: None,
                cancel: CancellationToken::new(),
                generation: 0,
            }),
            downloading: AtomicBool::new(false),
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
        let model = self.settings.lock().unwrap().local_model.clone();
        let Some(info) = models::find(&model) else {
            return self.notify(&format!("Modello sconosciuto: {model}"));
        };
        if !models::is_installed(info) {
            self.notify("Il modello di trascrizione non è ancora scaricato: lo scarico ora.");
            return self.ensure_model();
        }

        // Precarica il modello mentre l'utente parla: toglie il caricamento dalla latenza.
        let stt = self.stt.clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(e) = stt.preload() {
                log::error!("{e:#}");
            }
        });

        let app = self.app.clone();
        let rec = match Recorder::start(move |level| {
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
        let opts = SttOptions {
            language: settings.language(),
            initial_prompt: None,
            cancel: cancel.clone(),
        };
        let t = Instant::now();
        let transcript = self.stt.transcribe(&audio, &opts).await?;
        let stt_ms = t.elapsed().as_millis();
        if cancel.is_cancelled() || transcript.text.is_empty() {
            return Ok(());
        }

        self.set_state(State::Pasting);
        let t = Instant::now();
        let pasted = self.paste(transcript.text.clone(), &settings).await;
        let paste_ms = t.elapsed().as_millis();

        log::info!(
            "dettato ok: audio={audio_ms}ms resample={resample_ms}ms stt={stt_ms}ms paste={paste_ms}ms \
             totale={}ms lingua={:?} caratteri={}",
            released.elapsed().as_millis(),
            transcript.language,
            transcript.text.chars().count()
        );
        if let Err(e) = pasted {
            log::warn!("incolla automatico non riuscito: {e:#}");
            let key = if cfg!(target_os = "macos") {
                "⌘V"
            } else {
                "Ctrl+V"
            };
            self.notify(&format!(
                "Testo copiato negli appunti: premi {key} per incollarlo."
            ));
        }
        Ok(())
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
    pub fn ensure_model(self: &Arc<Self>) {
        let id = self.settings.lock().unwrap().local_model.clone();
        let Some(info) = models::find(&id) else {
            return;
        };
        if models::is_installed(info) || self.downloading.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let app = me.app.clone();
            let mut last = 0;
            let res = models::download(info, |done, total| {
                let pct = done * 100 / total.max(1);
                if pct != last {
                    last = pct;
                    let _ = app.emit("models://progress", (info.id, done, total));
                    tray::set_status(&app, &format!("Scarico {}… {pct}%", info.id));
                }
            })
            .await;
            me.downloading.store(false, Ordering::SeqCst);
            match res {
                Ok(_) => {
                    tray::set_status(&me.app, "Pronto");
                    me.notify(&format!("Modello {} pronto.", info.id));
                }
                Err(e) => {
                    log::error!("download modello: {e:#}");
                    tray::set_status(&me.app, "Download non riuscito");
                    me.notify(&format!("Download del modello non riuscito: {e:#}"));
                }
            }
        });
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
