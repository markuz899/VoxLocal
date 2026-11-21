//! Note delle riunioni: due tracce registrate insieme (il microfono = "io", l'audio delle app = "gli altri"),
//! scritte su disco a 16 kHz mentre arrivano, poi trascritte e unite in ordine di tempo.

#[cfg(target_os = "macos")]
mod system_audio;

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::audio::{AudioBuffer, MicStream, WHISPER_RATE};
use crate::stt::Segment;

/// Una traccia scritta su disco: mono, 16 kHz, 16 bit, con un ricampionatore lineare a flusso.
/// ponytail: interpolazione lineare senza filtro anti-alias; per la voce basta, rubato se serve qualità musicale.
pub struct TrackWriter {
    wav: Option<hound::WavWriter<BufWriter<File>>>,
    step: f64,
    pos: f64,
    prev: f32,
    samples: u64,
}

impl TrackWriter {
    pub fn create(path: &Path, in_rate: u32) -> Result<Self> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: WHISPER_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        Ok(Self {
            wav: Some(
                hound::WavWriter::create(path, spec)
                    .with_context(|| format!("creazione di {}", path.display()))?,
            ),
            step: in_rate as f64 / WHISPER_RATE as f64,
            pos: 0.0,
            prev: 0.0,
            samples: 0,
        })
    }

    /// Aggiunge campioni alla frequenza d'ingresso.
    pub fn push(&mut self, input: &[f32]) {
        let Some(wav) = self.wav.as_mut() else { return };
        // `pos` è la posizione del prossimo campione d'uscita, misurata dall'inizio di `input`
        // (−1 indica `prev`, l'ultimo campione del blocco precedente).
        let n = input.len() as f64;
        while self.pos < n - 1.0 + 1e-9 && !input.is_empty() {
            let i = self.pos.floor();
            let f = (self.pos - i) as f32;
            let a = if i < 0.0 {
                self.prev
            } else {
                input[i as usize]
            };
            // sull'ultimo campione del blocco f è 0: b non conta, ma l'indice resta nel blocco
            let b = input[((i + 1.0).max(0.0) as usize).min(input.len() - 1)];
            let v = (a + (b - a) * f).clamp(-1.0, 1.0);
            let _ = wav.write_sample((v * i16::MAX as f32) as i16);
            self.samples += 1;
            self.pos += self.step;
        }
        if let Some(&last) = input.last() {
            self.prev = last;
            self.pos -= n;
        }
    }

    pub fn duration_ms(&self) -> u64 {
        self.samples * 1000 / WHISPER_RATE as u64
    }

    pub fn finish(&mut self) -> Result<()> {
        if let Some(w) = self.wav.take() {
            w.finalize()?;
        }
        Ok(())
    }
}

/// Chi parla in un segmento della trascrizione.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speaker {
    Me,
    Others,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Line {
    pub speaker: Speaker,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

type MicWriter = Arc<Mutex<Option<TrackWriter>>>;

/// Una registrazione in corso.
pub struct Recording {
    pub dir: PathBuf,
    pub started: Instant,
    pub started_at: i64,
    mic: Option<(MicStream, MicWriter)>,
    /// Almeno un campione diverso da zero dal microfono. Uno zero perfetto per secondi è macOS che
    /// non concede il microfono (un microfono vero ha sempre un po' di rumore).
    mic_heard: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    system: Option<(system_audio::SystemAudio, Arc<Mutex<TrackWriter>>)>,
    pub system_error: Option<String>,
}

/// Le tracce scritte su disco a fine registrazione.
pub struct Tracks {
    pub dir: PathBuf,
    pub me: Option<PathBuf>,
    pub others: Option<PathBuf>,
    pub duration_ms: u64,
    pub started_at: i64,
}

impl Recording {
    /// Apre il microfono e (su macOS) l'audio delle app. Se l'audio delle app non parte, registra
    /// comunque il microfono e lo dice in `system_error`.
    pub fn start(root: &Path, mic_device: Option<String>) -> Result<Self> {
        let started_at = now_ms();
        let dir = root.join(started_at.to_string());
        std::fs::create_dir_all(&dir)?;

        let mic_heard = Arc::new(AtomicBool::new(false));
        // la frequenza del microfono si conosce solo dopo l'apertura: lo scrittore nasce al primo blocco
        // (i campioni dei primi ~100 ms, prima che la frequenza sia nota, si perdono)
        let mic = {
            let path = dir.join("me.wav");
            let writer: MicWriter = Arc::new(Mutex::new(None));
            let rate = Arc::new(AtomicU32::new(0));
            let (w, r, heard) = (writer.clone(), rate.clone(), mic_heard.clone());
            let stream = MicStream::start(mic_device, move |chunk| {
                let mut g = w.lock().unwrap();
                if g.is_none() {
                    match r.load(Ordering::Relaxed) {
                        0 => return,
                        hz => match TrackWriter::create(&path, hz) {
                            Ok(t) => *g = Some(t),
                            Err(e) => {
                                log::error!("traccia del microfono: {e:#}");
                                return;
                            }
                        },
                    }
                }
                if chunk.iter().any(|v| *v != 0.0) {
                    heard.store(true, Ordering::Relaxed);
                }
                if let Some(t) = g.as_mut() {
                    t.push(chunk);
                }
            })?;
            rate.store(stream.rate, Ordering::Relaxed);
            Some((stream, writer))
        };

        #[cfg(target_os = "macos")]
        let (system, system_error) = {
            let path = dir.join("others.wav");
            let writer = Arc::new(Mutex::new(TrackWriter::create(&path, system_audio::RATE)?));
            let w = writer.clone();
            match system_audio::SystemAudio::start(move |chunk| w.lock().unwrap().push(chunk)) {
                Ok(s) => (Some((s, writer)), None),
                Err(e) => {
                    let _ = std::fs::remove_file(&path);
                    (None, Some(format!("{e:#}")))
                }
            }
        };
        #[cfg(not(target_os = "macos"))]
        let system_error = Some("l'audio delle app per ora si registra solo su macOS".to_string());

        Ok(Self {
            dir,
            started: Instant::now(),
            started_at,
            mic,
            mic_heard,
            #[cfg(target_os = "macos")]
            system,
            system_error,
        })
    }

    /// Dopo 3 s il microfono non ha mandato nemmeno rumore: il permesso Microfono manca.
    pub fn mic_silent(&self) -> bool {
        self.mic.is_some()
            && self.started.elapsed().as_secs() >= 3
            && !self.mic_heard.load(Ordering::Relaxed)
    }

    pub fn stop(self) -> Result<Tracks> {
        let mut duration_ms = 0;
        let mut me = None;
        if let Some((stream, writer)) = self.mic {
            stream.stop()?;
            if let Some(mut w) = writer.lock().unwrap().take() {
                duration_ms = duration_ms.max(w.duration_ms());
                w.finish()?;
                me = Some(self.dir.join("me.wav"));
            }
        }
        #[allow(unused_mut)]
        let mut others = None;
        #[cfg(target_os = "macos")]
        if let Some((sys, writer)) = self.system {
            sys.stop();
            let mut w = writer.lock().unwrap();
            duration_ms = duration_ms.max(w.duration_ms());
            w.finish()?;
            others = Some(self.dir.join("others.wav"));
        }
        Ok(Tracks {
            dir: self.dir,
            me,
            others,
            duration_ms: duration_ms.max(self.started.elapsed().as_millis() as u64),
            started_at: self.started_at,
        })
    }
}

/// Legge una traccia scritta da `TrackWriter`.
pub fn read_track(path: &Path) -> Result<AudioBuffer> {
    let buf = crate::audio::read_wav(path)?;
    anyhow::ensure!(
        buf.sample_rate == WHISPER_RATE,
        "traccia a {} Hz",
        buf.sample_rate
    );
    Ok(buf)
}

/// RMS di un tratto della traccia, in ms.
fn window_rms(audio: &AudioBuffer, start_ms: u64, end_ms: u64) -> f32 {
    let r = WHISPER_RATE as u64;
    let a = ((start_ms * r / 1000) as usize).min(audio.samples.len());
    let b = ((end_ms * r / 1000) as usize).clamp(a, audio.samples.len());
    let s = &audio.samples[a..b];
    if s.is_empty() {
        return 0.0;
    }
    (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
}

/// Toglie i segmenti inventati sul silenzio: Whisper scrive frasi tipiche ("Grazie a tutti.") dove nessuno parla.
pub fn keep_spoken(segments: Vec<Segment>, audio: &AudioBuffer) -> Vec<Segment> {
    segments
        .into_iter()
        .filter(|s| window_rms(audio, s.start_ms, s.end_ms.max(s.start_ms + 200)) >= SILENCE_RMS)
        .collect()
}

/// Sotto questa energia il tratto è silenzio (o rumore di fondo del microfono).
// ponytail: soglia fissa; se ci sono microfoni molto rumorosi, renderla relativa al rumore misurato.
const SILENCE_RMS: f32 = 0.004;

fn words(t: &str) -> std::collections::HashSet<String> {
    t.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() > 2)
        .map(str::to_lowercase)
        .collect()
}

/// Inviluppo di energia a passi di 20 ms (50 al secondo) di un tratto della traccia.
fn envelope(audio: &AudioBuffer, start_ms: i64, end_ms: i64) -> Vec<f32> {
    let step = (WHISPER_RATE / 50) as usize;
    let a = (start_ms.max(0) as usize * WHISPER_RATE as usize / 1000).min(audio.samples.len());
    let b = (end_ms.max(0) as usize * WHISPER_RATE as usize / 1000).clamp(a, audio.samples.len());
    audio.samples[a..b]
        .chunks(step)
        .map(|c| (c.iter().map(|x| x * x).sum::<f32>() / c.len() as f32).sqrt())
        .collect()
}

fn correlation(x: &[f32], y: &[f32]) -> f32 {
    let n = x.len().min(y.len());
    if n < 10 {
        return 0.0;
    }
    let (x, y) = (&x[..n], &y[..n]);
    let mx = x.iter().sum::<f32>() / n as f32;
    let my = y.iter().sum::<f32>() / n as f32;
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (dx, dy) = (x[i] - mx, y[i] - my);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx <= f32::EPSILON || syy <= f32::EPSILON {
        return 0.0;
    }
    sxy / (sxx * syy).sqrt()
}

/// Toglie dal microfono l'eco delle casse confrontando il suono, non le parole: se nel tratto di un
/// segmento il volume del microfono segue quello dell'audio delle app (con un ritardo fino a 400 ms),
/// è la voce delle app ripresa dal microfono, anche quando Whisper l'ha trascritta in modo diverso.
// ponytail: correlazione degli inviluppi a 20 ms; un vero cancellatore d'eco (AEC) se servisse di più.
pub fn drop_acoustic_echo(me: Vec<Segment>, mic: &AudioBuffer, apps: &AudioBuffer) -> Vec<Segment> {
    me.into_iter()
        .filter(|s| {
            let (a, b) = (s.start_ms as i64, s.end_ms.max(s.start_ms + 400) as i64);
            let m = envelope(mic, a, b);
            if window_rms(apps, s.start_ms, b as u64) < SILENCE_RMS {
                return true; // le app tacciono: è la tua voce
            }
            let best = (0..=20)
                .map(|lag| correlation(&m, &envelope(apps, a - lag * 20, b - lag * 20)))
                .fold(f32::MIN, f32::max);
            best < ECHO_CORRELATION
        })
        .collect()
}

/// Oltre questa correlazione degli inviluppi il tratto del microfono è eco delle app.
const ECHO_CORRELATION: f32 = 0.6;

/// Unisce le due trascrizioni in ordine di tempo. Con le casse la voce degli altri rientra nel microfono:
/// un segmento del microfono che si sovrappone nel tempo a uno degli altri e ne ripete le parole è un'eco, e si toglie.
pub fn merge(me: Vec<Segment>, others: Vec<Segment>) -> Vec<Line> {
    let echo = |m: &Segment| {
        let mw = words(&m.text);
        if mw.is_empty() {
            return false;
        }
        others.iter().any(|o| {
            let overlap = m
                .end_ms
                .min(o.end_ms)
                .saturating_sub(m.start_ms.max(o.start_ms));
            let span = (m.end_ms - m.start_ms).max(1);
            if overlap * 2 < span {
                return false;
            }
            let ow = words(&o.text);
            let common = mw.intersection(&ow).count();
            common * 2 >= mw.len()
        })
    };
    let mut lines: Vec<Line> = me
        .iter()
        .filter(|m| !echo(m))
        .map(|s| Line {
            speaker: Speaker::Me,
            start_ms: s.start_ms,
            end_ms: s.end_ms,
            text: s.text.clone(),
        })
        .chain(others.iter().map(|s| Line {
            speaker: Speaker::Others,
            start_ms: s.start_ms,
            end_ms: s.end_ms,
            text: s.text.clone(),
        }))
        .collect();
    lines.sort_by_key(|l| (l.start_ms, l.speaker == Speaker::Others));
    lines
}

/// La trascrizione come testo, per il riassunto e per "copia".
pub fn as_text(lines: &[Line], me: &str, others: &str) -> String {
    lines
        .iter()
        .map(|l| {
            format!(
                "{}: {}",
                if l.speaker == Speaker::Me { me } else { others },
                l.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn root_dir() -> PathBuf {
    crate::data_dir().join("meetings")
}

pub fn remove_dir(dir: &Path) -> Result<()> {
    if dir.starts_with(root_dir()) && dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| anyhow!("{e}"))?;
    }
    Ok(())
}

/// Il permesso "Registrazione schermo" (macOS): serve all'audio delle app.
pub fn screen_permission(request: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        system_audio::permission(request)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request;
        true
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(a: u64, b: u64, t: &str) -> Segment {
        Segment {
            start_ms: a,
            end_ms: b,
            text: t.into(),
        }
    }

    #[test]
    fn merge_orders_by_time_and_drops_the_echo() {
        let me = vec![
            seg(0, 2000, "Buongiorno a tutti, iniziamo."),
            // la voce degli altri rientrata nel microfono dalle casse
            seg(3100, 5900, "Il rilascio slitta a venerdì per i test."),
            seg(7000, 8000, "Va bene, ci sentiamo venerdì."),
        ];
        let others = vec![seg(
            3000,
            6000,
            "Il rilascio slitta a venerdì per via dei test.",
        )];
        let lines = merge(me, others);
        let who: Vec<_> = lines.iter().map(|l| (l.speaker, l.start_ms)).collect();
        assert_eq!(
            who,
            vec![
                (Speaker::Me, 0),
                (Speaker::Others, 3000),
                (Speaker::Me, 7000)
            ]
        );
        assert_eq!(
            as_text(&lines[..1], "Io", "Altri"),
            "Io: Buongiorno a tutti, iniziamo."
        );
    }

    #[test]
    fn writer_resamples_to_16k_on_the_fly() {
        let path = std::env::temp_dir().join(format!("voxlocal-track-{}.wav", std::process::id()));
        let mut w = TrackWriter::create(&path, 48_000).unwrap();
        // 1 s di sinusoide a 440 Hz in blocchi di 960 campioni, come arriva da ScreenCaptureKit
        let tone: Vec<f32> = (0..48_000)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.5)
            .collect();
        for c in tone.chunks(960) {
            w.push(c);
        }
        w.finish().unwrap();
        let buf = read_track(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(
            (buf.samples.len() as i64 - 16_000).abs() <= 2,
            "{}",
            buf.samples.len()
        );
        let peak = buf.samples.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.5).abs() < 0.02, "{peak}");
    }

    #[test]
    fn writer_survives_any_block_size_and_rate() {
        let path =
            std::env::temp_dir().join(format!("voxlocal-track-blocks-{}.wav", std::process::id()));
        for rate in [44_100, 48_000, 16_000, 96_000] {
            let mut w = TrackWriter::create(&path, rate).unwrap();
            let mut fed = 0;
            for size in [1, 2, 3, 5120, 441, 960, 7, 4096, 1] {
                w.push(&vec![0.1; size]);
                fed += size;
            }
            w.finish().unwrap();
            let got = read_track(&path).unwrap().samples.len() as i64;
            let want = fed as i64 * 16_000 / rate as i64;
            assert!((got - want).abs() <= 2, "{rate}: {got} vs {want}");
        }
        let _ = std::fs::remove_file(&path);
    }

    /// Una voce finta: rumore modulato da sillabe irregolari.
    fn speechy(seed: u32, secs: usize, gain: f32) -> Vec<f32> {
        let mut x = seed;
        let mut rnd = move || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x as f32 / u32::MAX as f32) * 2.0 - 1.0
        };
        let n = secs * 16_000;
        let mut env = 0.0f32;
        (0..n)
            .map(|i| {
                if i % 2400 == 0 {
                    env = if rnd() > -0.2 { rnd().abs() } else { 0.0 };
                }
                rnd() * env * gain
            })
            .collect()
    }

    #[test]
    fn acoustic_echo_is_dropped_and_own_voice_kept() {
        let apps = AudioBuffer {
            samples: speechy(7, 6, 0.4),
            sample_rate: WHISPER_RATE,
        };
        // l'eco: la voce delle app, 120 ms dopo e più bassa, con il rumore della stanza
        let mut echo = vec![0f32; 1920];
        echo.extend(apps.samples.iter().map(|v| v * 0.15));
        echo.truncate(apps.samples.len());
        let room = speechy(99, 6, 0.01);
        let mic_echo = AudioBuffer {
            samples: echo.iter().zip(&room).map(|(a, b)| a + b).collect(),
            sample_rate: WHISPER_RATE,
        };
        // la tua voce, indipendente dalle app
        let mic_me = AudioBuffer {
            samples: speechy(1234, 6, 0.3),
            sample_rate: WHISPER_RATE,
        };
        let segs = || vec![seg(500, 3500, "una frase"), seg(3500, 5800, "un'altra")];
        assert!(drop_acoustic_echo(segs(), &mic_echo, &apps).is_empty());
        assert_eq!(drop_acoustic_echo(segs(), &mic_me, &apps).len(), 2);
    }

    #[test]
    fn silence_segments_are_dropped() {
        let mut audio = AudioBuffer {
            samples: vec![0.0; 32_000],
            sample_rate: WHISPER_RATE,
        };
        for s in &mut audio.samples[16_000..] {
            *s = 0.2;
        }
        let kept = keep_spoken(
            vec![
                seg(0, 900, "Grazie a tutti."),
                seg(1000, 1900, "Parlo davvero."),
            ],
            &audio,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].text, "Parlo davvero.");
    }

    /// Prova vera sul Mac: `VOXLOCAL_MODEL=… cargo test --release --lib meeting_end_to_end -- --ignored --nocapture`.
    /// Riproduce una frase dalle casse, registra le due tracce, le trascrive e le unisce.
    #[test]
    #[ignore]
    fn meeting_end_to_end() {
        let Ok(model) = std::env::var("VOXLOCAL_MODEL") else {
            return;
        };
        let tmp = std::env::temp_dir().join(format!("voxlocal-meeting-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let aiff = tmp.join("frase.aiff");
        std::process::Command::new("say")
            .args(["-v", "Alice", "-o"])
            .arg(&aiff)
            .arg("[[slnc 1500]] Buongiorno a tutti, oggi parliamo del rilascio della versione due e dei test su Windows.")
            .status()
            .unwrap();
        // l'app che suona parte prima della registrazione, come Zoom in una riunione vera
        let mut player = std::process::Command::new("afplay")
            .arg(&aiff)
            .spawn()
            .unwrap();
        let rec = Recording::start(&tmp, None).unwrap();
        println!("audio delle app: {:?}", rec.system_error);
        // oltre i 10 s: il filtro delle app si rinnova almeno una volta (lì c'era un crash)
        std::thread::sleep(std::time::Duration::from_secs(14));
        let tracks = rec.stop().unwrap();
        let _ = player.wait();
        let stt = crate::stt::LocalWhisper::new(model.into(), None);
        let opts = crate::stt::SttOptions {
            language: Some("it".into()),
            ..Default::default()
        };
        let mut out = vec![];
        for path in [tracks.me.clone(), tracks.others.clone()] {
            let Some(path) = path else {
                out.push(vec![]);
                continue;
            };
            let audio = read_track(&path).unwrap();
            println!(
                "{}: {} s, rms {:.4}",
                path.display(),
                audio.duration_ms() / 1000,
                audio.rms()
            );
            let (segs, _) = stt.transcribe_segments(&audio, &opts, |_| {}).unwrap();
            out.push(keep_spoken(segs, &audio));
        }
        let others_out = out.pop().unwrap();
        let me_out = out.pop().unwrap();
        let lines = merge(me_out, others_out);
        println!("{}", as_text(&lines, "Io", "Altri"));
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(lines
            .iter()
            .any(|l| l.speaker == Speaker::Others && l.text.to_lowercase().contains("rilascio")));
    }

    /// Riapplica il filtro anti-eco a una riunione salvata: `VOXLOCAL_MEETING_DIR=… VOXLOCAL_MODEL=… cargo test --release --lib echo_on_saved -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn echo_on_saved_meeting() {
        let (Ok(dir), Ok(model)) = (
            std::env::var("VOXLOCAL_MEETING_DIR"),
            std::env::var("VOXLOCAL_MODEL"),
        ) else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let mic = read_track(&dir.join("me.wav")).unwrap();
        let apps = read_track(&dir.join("others.wav")).unwrap();
        let stt = crate::stt::LocalWhisper::new(model.into(), None);
        let opts = crate::stt::SttOptions {
            language: Some("it".into()),
            ..Default::default()
        };
        let (segs, _) = stt.transcribe_segments(&mic, &opts, |_| {}).unwrap();
        let segs = keep_spoken(segs, &mic);
        let kept = drop_acoustic_echo(segs.clone(), &mic, &apps);
        for s in &segs {
            let keep = kept.iter().any(|k| k.start_ms == s.start_ms);
            println!(
                "{} [{:>6}] {}",
                if keep { "TIENE" } else { "ECO  " },
                s.start_ms,
                s.text
            );
        }
    }
}
