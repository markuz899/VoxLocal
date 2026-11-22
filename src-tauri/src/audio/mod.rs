//! Cattura dal microfono (cpal) e resampling a 16 kHz mono (rubato).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use rubato::audioadapter_buffers::direct::SequentialSlice;
use rubato::{Fft, FixedSync, Resampler};

pub const WHISPER_RATE: u32 = 16_000;

/// Audio mono f32. Dopo `to_16k` è sempre a 16 kHz.
#[derive(Clone, Debug, Default)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl AudioBuffer {
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        self.samples.len() as u64 * 1000 / self.sample_rate as u64
    }

    pub fn rms(&self) -> f32 {
        rms(&self.samples)
    }

    pub fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()))
    }

    /// Voce bassa o sussurrata: porta il picco verso 0,7 (al massimo ×12), così whisper sente le
    /// parole invece del rumore. Restituisce il guadagno applicato (1 = niente).
    pub fn boost_quiet(&mut self) -> f32 {
        let peak = self.peak();
        if peak >= 0.25 || peak < 0.005 {
            return 1.0;
        }
        let gain = (0.7 / peak).min(12.0);
        for s in &mut self.samples {
            *s *= gain;
        }
        gain
    }
}

fn rms(s: &[f32]) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
}

/// Una registrazione in corso. Il thread dedicato possiede lo stream cpal
/// (che non è `Send` su tutte le piattaforme) e svuota il ring buffer.
pub struct Recorder {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<Result<AudioBuffer>>,
    tap: Tap,
}

/// L'audio registrato finora, leggibile mentre la registrazione continua (dettature a pezzi).
#[derive(Clone)]
pub struct Tap {
    samples: Arc<Mutex<Vec<f32>>>,
    pub rate: u32,
}

impl Tap {
    /// Copia dei campioni da `from` alla fine (vuota se `from` è oltre).
    pub fn from(&self, from: usize) -> Vec<f32> {
        let s = self.samples.lock().unwrap();
        s.get(from..).map(<[f32]>::to_vec).unwrap_or_default()
    }
}

impl Recorder {
    /// Apre il microfono indicato (per nome) o quello di default se manca o non esiste più.
    /// `on_level` riceve l'RMS ~30 volte al secondo.
    pub fn start(device: Option<String>, on_level: impl Fn(f32) + Send + 'static) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32>>();
        let stop_t = stop.clone();
        let shared = Arc::new(Mutex::new(Vec::new()));
        let shared_t = shared.clone();

        let handle = std::thread::Builder::new()
            .name("voxlocal-audio".into())
            .spawn(move || -> Result<AudioBuffer> {
                let opened = open(device.as_deref());
                let (stream, mut cons, rate) = match opened {
                    Ok(v) => {
                        let _ = ready_tx.send(Ok(v.2));
                        v
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(anyhow!("{e:#}")));
                        return Err(e);
                    }
                };
                shared_t.lock().unwrap().reserve(rate as usize * 30);
                let mut fresh = Vec::new();
                let mut drain = |fresh: &mut Vec<f32>| {
                    fresh.clear();
                    let n = cons.slots();
                    if let Ok(chunk) = cons.read_chunk(n) {
                        fresh.extend(chunk);
                    }
                    shared_t.lock().unwrap().extend_from_slice(fresh);
                };
                while !stop_t.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(33));
                    drain(&mut fresh);
                    on_level(rms(&fresh));
                }
                drop(stream);
                drain(&mut fresh);
                Ok(AudioBuffer {
                    samples: std::mem::take(&mut *shared_t.lock().unwrap()),
                    sample_rate: rate,
                })
            })?;

        let rate = ready_rx
            .recv_timeout(Duration::from_secs(2))
            .context("il microfono non risponde")??;
        Ok(Self {
            stop,
            handle,
            tap: Tap {
                samples: shared,
                rate,
            },
        })
    }

    pub fn tap(&self) -> Tap {
        self.tap.clone()
    }

    /// Ferma la cattura e restituisce l'audio alla frequenza nativa del device.
    pub fn stop(self) -> Result<AudioBuffer> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle
            .join()
            .map_err(|_| anyhow!("thread audio andato in panico"))?
    }
}

/// Microfono a flusso: i campioni (mono, frequenza nativa) vanno a `on_chunk` man mano che arrivano,
/// senza tenerli in memoria. Serve alle registrazioni lunghe (riunioni).
pub struct MicStream {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<Result<()>>,
    pub rate: u32,
}

impl MicStream {
    pub fn start(
        device: Option<String>,
        mut on_chunk: impl FnMut(&[f32]) + Send + 'static,
    ) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32>>();
        let stop_t = stop.clone();
        let handle = std::thread::Builder::new()
            .name("voxlocal-mic-stream".into())
            .spawn(move || -> Result<()> {
                let (stream, mut cons, rate) = match open(device.as_deref()) {
                    Ok(v) => {
                        let _ = ready_tx.send(Ok(v.2));
                        v
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(anyhow!("{e:#}")));
                        return Err(e);
                    }
                };
                let _ = rate;
                let mut drain = |on_chunk: &mut dyn FnMut(&[f32])| {
                    let n = cons.slots();
                    if let Ok(chunk) = cons.read_chunk(n) {
                        let (a, b) = chunk.as_slices();
                        on_chunk(a);
                        on_chunk(b);
                        chunk.commit_all();
                    }
                };
                while !stop_t.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(100));
                    drain(&mut on_chunk);
                }
                drop(stream);
                drain(&mut on_chunk);
                Ok(())
            })?;
        let rate = ready_rx
            .recv_timeout(Duration::from_secs(2))
            .context("il microfono non risponde")??;
        Ok(Self { stop, handle, rate })
    }

    pub fn stop(self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle
            .join()
            .map_err(|_| anyhow!("thread del microfono andato in panico"))?
    }
}

type Opened = (cpal::Stream, rtrb::Consumer<f32>, u32);

/// Nomi dei microfoni disponibili, nell'ordine del sistema.
pub fn input_devices() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|it| {
            it.filter_map(|d| d.description().ok().map(|x| x.name().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn open(name: Option<&str>) -> Result<Opened> {
    let host = cpal::default_host();
    let chosen = name.and_then(|n| {
        host.input_devices()
            .ok()?
            .find(|d| d.description().is_ok_and(|x| x.name() == n))
    });
    if name.is_some() && chosen.is_none() {
        log::warn!("microfono {name:?} non trovato, uso quello di default");
    }
    let device = chosen
        .or_else(|| host.default_input_device())
        .context("nessun microfono disponibile")?;
    let config = device.default_input_config()?;
    let rate = config.sample_rate();
    // ponytail: 4 s di margine nel ring buffer, il thread lo svuota ogni 33 ms
    let (prod, cons) = rtrb::RingBuffer::new(rate as usize * 4);
    let stream = match config.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, config.into(), prod)?,
        SampleFormat::I16 => build::<i16>(&device, config.into(), prod)?,
        SampleFormat::I32 => build::<i32>(&device, config.into(), prod)?,
        SampleFormat::U16 => build::<u16>(&device, config.into(), prod)?,
        f => return Err(anyhow!("formato audio non supportato: {f}")),
    };
    stream.play()?;
    Ok((stream, cons, rate))
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut prod: rtrb::Producer<f32>,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let ch = config.channels as usize;
    // Nel callback niente allocazioni né lock: downmix e push nel ring buffer.
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            for frame in data.chunks_exact(ch) {
                let s = frame.iter().map(|&x| x.to_sample::<f32>()).sum::<f32>() / ch as f32;
                let _ = prod.push(s);
            }
        },
        |e| log::error!("errore stream audio: {e}"),
        None,
    )?;
    Ok(stream)
}

/// Dove tagliare l'audio non ancora trascritto di una dettatura in corso (`pending`, alla frequenza
/// `rate`): nell'ultima pausa di almeno 0,4 s dopo i primi 12 s, oppure, oltre i 30 s senza pause,
/// nel punto più silenzioso degli ultimi secondi. `None` = aspetta ancora.
pub fn next_cut(pending: &[f32], rate: u32) -> Option<usize> {
    const MIN_S: usize = 12;
    const MAX_S: usize = 30;
    const QUIET_FRAMES: usize = 4;
    let frame = (rate / 10).max(1) as usize; // 100 ms
    if pending.len() < MIN_S * rate as usize {
        return None;
    }
    let levels: Vec<f32> = pending.chunks_exact(frame).map(rms).collect();
    let avg = levels.iter().sum::<f32>() / levels.len() as f32;
    // una pausa è molto sotto il volume medio della voce: niente soglia fissa, il microfono cambia
    let quiet = |l: f32| l < avg * 0.2;
    let first = MIN_S * 10;
    // l'ultima pausa: il pezzo più lungo possibile, meno spezzature
    for end in (first + QUIET_FRAMES..levels.len()).rev() {
        if levels[end - QUIET_FRAMES..end].iter().all(|&l| quiet(l)) {
            return Some((end - QUIET_FRAMES / 2) * frame);
        }
    }
    if levels.len() < MAX_S * 10 {
        return None;
    }
    let from = (MAX_S - 10) * 10;
    let (i, _) = levels[from..]
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(b.1))?;
    Some((from + i) * frame + frame / 2)
}

/// Porta l'audio a 16 kHz in un'unica passata (vedi DECISIONS D6).
pub fn to_16k(buf: AudioBuffer) -> Result<AudioBuffer> {
    if buf.sample_rate == WHISPER_RATE || buf.samples.is_empty() {
        return Ok(AudioBuffer {
            sample_rate: WHISPER_RATE,
            ..buf
        });
    }
    let len = buf.samples.len();
    let mut r = Fft::<f32>::new(
        buf.sample_rate as usize,
        WHISPER_RATE as usize,
        1024,
        1,
        FixedSync::Input,
    )?;
    let input = SequentialSlice::new(&buf.samples, 1, len)?;
    let out = r.process_all(&input, len, None)?;
    Ok(AudioBuffer {
        samples: out.take_data(),
        sample_rate: WHISPER_RATE,
    })
}

/// Legge un WAV qualsiasi (rate, canali, int/float) come mono f32.
pub fn read_wav(path: &std::path::Path) -> Result<AudioBuffer> {
    let mut r = hound::WavReader::open(path)?;
    let spec = r.spec();
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<Result<_, _>>()?
        }
    };
    let ch = spec.channels as usize;
    let samples = raw
        .chunks_exact(ch)
        .map(|f| f.iter().sum::<f32>() / ch as f32)
        .collect();
    Ok(AudioBuffer {
        samples,
        sample_rate: spec.sample_rate,
    })
}

/// Decodifica un file audio o video (mp3, m4a/aac, mp4/mov, mkv, wav, flac, ogg vorbis, caf, aiff)
/// in mono, alla frequenza originale. `on_progress` riceve la frazione letta (0..1).
// ponytail: niente Opus (i vocali WhatsApp .opus): symphonia non lo decodifica; aggiungere
// un decoder Opus se i clienti lo chiedono.
pub fn decode_file(path: &std::path::Path) -> Result<AudioBuffer> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    let file =
        std::fs::File::open(path).with_context(|| format!("apertura di {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| anyhow!("formato non supportato: {e}"))?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| anyhow!("il file non ha una traccia audio"))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| anyhow!("traccia audio non leggibile"))?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| anyhow!("codec non supportato: {e}"))?;

    let mut out = Vec::new();
    let mut rate = 0;
    let mut frame: Vec<f32> = Vec::new();
    while let Some(packet) = format.next_packet().map_err(|e| anyhow!("lettura: {e}"))? {
        if packet.track_id != track_id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(anyhow!("decodifica: {e}")),
        };
        rate = buf.spec().rate();
        let channels = buf.spec().channels().count().max(1);
        frame.resize(buf.samples_interleaved(), 0.0);
        buf.copy_to_slice_interleaved(&mut frame);
        // Mono: media dei canali.
        out.extend(
            frame
                .chunks_exact(channels)
                .map(|c| c.iter().sum::<f32>() / channels as f32),
        );
    }
    anyhow::ensure!(!out.is_empty() && rate > 0, "il file non contiene audio");
    Ok(AudioBuffer {
        samples: out,
        sample_rate: rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Voce finta: un tono a 0,3 di ampiezza; le pause sono silenzio.
    fn speech(secs: &[(f32, bool)], rate: u32) -> Vec<f32> {
        let mut out = vec![];
        for &(s, voiced) in secs {
            let n = (s * rate as f32) as usize;
            out.extend((0..n).map(|i| {
                if voiced {
                    0.3 * (i as f32 * 0.07).sin()
                } else {
                    0.0
                }
            }));
        }
        out
    }

    #[test]
    fn cuts_in_the_last_pause_after_twelve_seconds() {
        let rate = 16_000;
        // pausa a 5 s (troppo presto), pausa a 14 s, pausa a 18 s: si taglia in quella a 18 s
        let a = speech(
            &[
                (5.0, true),
                (0.6, false),
                (8.4, true),
                (0.6, false),
                (3.4, true),
                (0.6, false),
                (2.0, true),
            ],
            rate,
        );
        let cut = next_cut(&a, rate).expect("c'è una pausa dopo 12 s");
        let at = cut as f32 / rate as f32;
        assert!((18.0..18.7).contains(&at), "taglio a {at} s");
        // prima dei 12 s non si taglia mai
        assert_eq!(next_cut(&a[..11 * rate as usize], rate), None);
    }

    #[test]
    fn without_pauses_waits_then_cuts_at_the_quietest_point() {
        let rate = 16_000;
        let mut a = speech(&[(31.0, true)], rate);
        assert_eq!(next_cut(&a[..25 * rate as usize], rate), None);
        // un calo di volume a 24 s, non abbastanza per una pausa vera: è il punto più basso
        let dip = 24 * rate as usize;
        for s in &mut a[dip..dip + rate as usize / 10] {
            *s *= 0.5;
        }
        let at = next_cut(&a, rate).unwrap() as f32 / rate as f32;
        assert!((24.0..24.2).contains(&at), "taglio a {at} s");
    }

    #[test]
    fn boosts_only_quiet_audio() {
        let mut quiet = AudioBuffer {
            samples: vec![0.0, 0.05, -0.1, 0.02],
            sample_rate: 16_000,
        };
        let g = quiet.boost_quiet();
        assert!((g - 7.0).abs() < 1e-4, "{g}");
        assert!((quiet.peak() - 0.7).abs() < 1e-4);

        let mut loud = AudioBuffer {
            samples: vec![0.6, -0.4],
            sample_rate: 16_000,
        };
        assert_eq!(loud.boost_quiet(), 1.0);
        assert_eq!(loud.samples, vec![0.6, -0.4]);

        let mut silence = AudioBuffer {
            samples: vec![0.001; 10],
            sample_rate: 16_000,
        };
        assert_eq!(silence.boost_quiet(), 1.0);
    }

    #[test]
    fn resample_48k_to_16k_keeps_duration() {
        let one_sec: Vec<f32> = (0..48_000)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin())
            .collect();
        let out = to_16k(AudioBuffer {
            samples: one_sec,
            sample_rate: 48_000,
        })
        .unwrap();
        assert_eq!(out.sample_rate, 16_000);
        assert!(
            (out.samples.len() as i64 - 16_000).abs() <= 16,
            "{}",
            out.samples.len()
        );
        assert!((out.rms() - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.05);
    }

    #[test]
    fn decodes_a_stereo_file_to_mono() {
        let path = std::env::temp_dir().join(format!("voxlocal-decode-{}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..44_100 {
            w.write_sample(i16::MAX / 2).unwrap();
            w.write_sample(0i16).unwrap();
        }
        w.finalize().unwrap();
        let buf = decode_file(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(buf.sample_rate, 44_100);
        assert_eq!(buf.samples.len(), 44_100);
        assert!(
            (buf.samples[100] - 0.25).abs() < 0.01,
            "{}",
            buf.samples[100]
        );
        assert!(decode_file(std::path::Path::new("/nonexistent.mp3")).is_err());
    }
}
