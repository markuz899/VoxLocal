//! STT remoto su `/audio/transcriptions` (OpenAI, Groq e server locali compatibili).

use std::io::Cursor;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};

use super::{SttOptions, SttProvider, Transcript};
use crate::audio::AudioBuffer;

pub struct OpenAiCompatibleStt {
    pub id: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub timeout: Duration,
}

/// WAV PCM 16 bit in memoria: metà dei byte del f32 e accettato da tutti gli endpoint.
fn wav_bytes(audio: &AudioBuffer) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cur = Cursor::new(Vec::with_capacity(audio.samples.len() * 2 + 44));
    let mut w = hound::WavWriter::new(&mut cur, spec)?;
    for s in &audio.samples {
        w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
    }
    w.finalize()?;
    Ok(cur.into_inner())
}

#[async_trait]
impl SttProvider for OpenAiCompatibleStt {
    fn id(&self) -> &str {
        &self.id
    }

    async fn transcribe(&self, audio: &AudioBuffer, opts: &SttOptions) -> Result<Transcript> {
        let file = Part::bytes(wav_bytes(audio)?)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;
        let mut form = Form::new()
            .part("file", file)
            .text("model", self.model.clone())
            .text("response_format", "json");
        if let Some(l) = &opts.language {
            form = form.text("language", l.clone());
        }
        if let Some(p) = opts.initial_prompt.as_ref().filter(|p| !p.is_empty()) {
            form = form.text("prompt", p.clone());
        }

        let url = format!(
            "{}/audio/transcriptions",
            self.base_url.trim_end_matches('/')
        );
        let mut rb = crate::llm::http()
            .post(url)
            .timeout(self.timeout)
            .multipart(form);
        if let Some(k) = &self.api_key {
            rb = rb.bearer_auth(k);
        }
        let send = rb.send();
        let resp = tokio::select! {
            r = send => r.context("STT remoto non raggiungibile")?,
            _ = opts.cancel.cancelled() => return Err(anyhow!("annullato")),
        };
        let status = resp.status();
        if !status.is_success() {
            let body: String = resp
                .text()
                .await
                .unwrap_or_default()
                .chars()
                .take(300)
                .collect();
            return Err(anyhow!("STT remoto HTTP {status}: {body}"));
        }
        let v: serde_json::Value = resp.json().await?;
        let text = v["text"]
            .as_str()
            .ok_or_else(|| anyhow!("risposta STT senza campo text"))?
            .trim()
            .to_string();
        Ok(Transcript {
            text,
            language: v["language"]
                .as_str()
                .map(str::to_string)
                .or(opts.language.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn uploads_wav_with_model_and_language() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/audio/transcriptions"))
            .and(header("authorization", "Bearer gk"))
            .respond_with(|r: &wiremock::Request| {
                let body = String::from_utf8_lossy(&r.body);
                let ok = body.contains("whisper-large-v3-turbo")
                    && body.contains("name=\"language\"")
                    && body.contains("RIFF")
                    && body.contains("Tauri");
                ResponseTemplate::new(if ok { 200 } else { 400 })
                    .set_body_json(serde_json::json!({"text": " Ciao Tauri. "}))
            })
            .mount(&server)
            .await;

        let stt = OpenAiCompatibleStt {
            id: "groq".into(),
            base_url: format!("{}/v1", server.uri()),
            api_key: Some("gk".into()),
            model: "whisper-large-v3-turbo".into(),
            timeout: Duration::from_secs(2),
        };
        let audio = AudioBuffer {
            samples: vec![0.0; 16_000],
            sample_rate: 16_000,
        };
        let opts = SttOptions {
            language: Some("it".into()),
            initial_prompt: Some("Tauri".into()),
            ..Default::default()
        };
        let t = stt.transcribe(&audio, &opts).await.unwrap();
        assert_eq!(t.text, "Ciao Tauri.");
        assert_eq!(t.language.as_deref(), Some("it"));
    }
}
