//! Cattura dal microfono (cpal) e resampling a 16 kHz mono (rubato).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
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
}

impl Recorder {
    /// Apre il microfono indicato (per nome) o quello di default se manca o non esiste più.
    /// `on_level` riceve l'RMS ~30 volte al secondo.
    pub fn start(device: Option<String>, on_level: impl Fn(f32) + Send + 'static) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();
        let stop_t = stop.clone();

        let handle = std::thread::Builder::new()
            .name("voxlocal-audio".into())
            .spawn(move || -> Result<AudioBuffer> {
                let opened = open(device.as_deref());
                let (stream, mut cons, rate) = match opened {
                    Ok(v) => {
                        let _ = ready_tx.send(Ok(()));
                        v
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(anyhow!("{e:#}")));
                        return Err(e);
                    }
                };
                let mut samples = Vec::with_capacity(rate as usize * 30);
                let mut drain = |samples: &mut Vec<f32>| -> usize {
                    let n = cons.slots();
                    if let Ok(chunk) = cons.read_chunk(n) {
                        samples.extend(chunk);
                    }
                    n
                };
                while !stop_t.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(33));
                    let start = samples.len();
                    drain(&mut samples);
                    on_level(rms(&samples[start..]));
                }
                drop(stream);
                drain(&mut samples);
                Ok(AudioBuffer {
                    samples,
                    sample_rate: rate,
                })
            })?;

        ready_rx
            .recv_timeout(Duration::from_secs(2))
            .context("il microfono non risponde")??;
        Ok(Self { stop, handle })
    }

    /// Ferma la cattura e restituisce l'audio alla frequenza nativa del device.
    pub fn stop(self) -> Result<AudioBuffer> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle
            .join()
            .map_err(|_| anyhow!("thread audio andato in panico"))?
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
