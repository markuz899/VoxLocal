//! Speech-to-text: trait comune e implementazioni.

mod local_whisper;
mod openai_compat;

pub use local_whisper::{LocalWhisper, Segment};
pub use openai_compat::OpenAiCompatibleStt;

use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::audio::AudioBuffer;

#[derive(Clone, Debug, Default)]
pub struct SttOptions {
    /// `None` = rilevamento automatico.
    pub language: Option<String>,
    /// Vocabolario personale, passato a whisper come `initial_prompt`.
    pub initial_prompt: Option<String>,
    pub cancel: CancellationToken,
}

#[derive(Clone, Debug, Default)]
pub struct Transcript {
    pub text: String,
    pub language: Option<String>,
}

#[async_trait]
pub trait SttProvider: Send + Sync {
    fn id(&self) -> &str;
    async fn transcribe(&self, audio: &AudioBuffer, opts: &SttOptions) -> Result<Transcript>;
}
