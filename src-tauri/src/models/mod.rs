//! Modelli scaricabili: ggml di whisper.cpp e GGUF del modello di pulizia locale.
//! Manifest con SHA-256, download riprendibile e verificato.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
const LLM_URL: &str = "https://huggingface.co/markuz89/voxclean/resolve/main";

pub struct ModelInfo {
    pub id: &'static str,
    pub base: &'static str,
    pub label: &'static str,
    pub file: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

/// Hash e dimensioni dai metadati LFS di Hugging Face (presi il 2026-11-20).
pub const MODELS: &[ModelInfo] = &[
    m(
        "tiny",
        "Tiny",
        "ggml-tiny.bin",
        77691713,
        "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
    ),
    m(
        "tiny-q5_1",
        "Tiny compresso",
        "ggml-tiny-q5_1.bin",
        32152673,
        "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7",
    ),
    m(
        "base",
        "Base",
        "ggml-base.bin",
        147951465,
        "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    ),
    m(
        "base-q5_1",
        "Base compresso",
        "ggml-base-q5_1.bin",
        59707625,
        "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
    ),
    m(
        "small",
        "Small",
        "ggml-small.bin",
        487601967,
        "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    ),
    m(
        "small-q5_1",
        "Small compresso",
        "ggml-small-q5_1.bin",
        190085487,
        "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
    ),
    m(
        "medium-q5_0",
        "Medium compresso",
        "ggml-medium-q5_0.bin",
        539212467,
        "19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f",
    ),
    m(
        "large-v3-turbo-q5_0",
        "Large v3 Turbo compresso",
        "ggml-large-v3-turbo-q5_0.bin",
        574041195,
        "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    ),
    m(
        "large-v3-turbo",
        "Large v3 Turbo",
        "ggml-large-v3-turbo.bin",
        1624555275,
        "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
    ),
];

const fn m(
    id: &'static str,
    label: &'static str,
    file: &'static str,
    size: u64,
    sha256: &'static str,
) -> ModelInfo {
    ModelInfo {
        id,
        base: BASE_URL,
        label,
        file,
        size,
        sha256,
    }
}

/// Modelli di pulizia addestrati per VoxLocal (cleanup-model/), Qwen3 rifinito, licenza Apache 2.0.
/// Hash e dimensioni dei file pubblicati.
pub const LLM_MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "voxclean-0.6b",
        base: LLM_URL,
        label: "Veloce (0,6B)",
        file: "voxclean-0.6b-q4_k_m.gguf",
        size: 396704576,
        sha256: "5651bbfb95621b887e2927e36df63ddd4d5e6ba1a6bbaf3781ed207ad55af408",
    },
    ModelInfo {
        id: "voxclean-1.7b",
        base: LLM_URL,
        label: "Preciso (1,7B)",
        file: "voxclean-1.7b-q4_k_m.gguf",
        size: 1107408704,
        sha256: "dd70d3d4cc1d4b5bb377eaacd9d539794b74bef3962b2b972ffd4c48d2edaa9a",
    },
];

/// Il modello di pulizia proposto di serie: sul Mac M1 lo 0,6B risponde in 0,2 s, l'1,7B in 0,9 s
/// (95% contro 97% all'esame, cleanup-model/PIANO.md).
pub const LLM_RECOMMENDED: &str = "voxclean-0.6b";

/// Cerca sia tra i modelli di Whisper sia tra quelli di pulizia (stesso download, stessa cartella).
pub fn find(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().chain(LLM_MODELS).find(|m| m.id == id)
}

/// Default suggerito per l'hardware (DECISIONS D27). Su M1 8 GB, con flash attention e
/// contesto proporzionale, large-v3-turbo-q5_0 trascrive 2-8 s di parlato in 0,6-0,9 s ed è
/// l'unico che regge la voce reale in italiano; base sbaglia ("una e meia" per "un'email").
// ponytail: su CPU (Intel/Windows) turbo non è misurato; resta base finché il bench in CI non dice altro
/// Senza GPU (tutto tranne i Mac Apple Silicon) medium e large trascrivono una frase in minuti.
pub fn slow_here(id: &str) -> bool {
    !cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && (id.starts_with("medium") || id.starts_with("large"))
}

pub fn recommended() -> &'static str {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "large-v3-turbo-q5_0"
    } else {
        "base"
    }
}

/// Stessa cartella di `app_data_dir()` di Tauri, così bench e app condividono i modelli.
pub fn models_dir() -> PathBuf {
    crate::data_dir().join("models")
}

pub fn path(m: &ModelInfo) -> PathBuf {
    models_dir().join(m.file)
}

/// Controllo rapido per dimensione: l'hash è già stato verificato al download.
pub fn is_installed(m: &ModelInfo) -> bool {
    std::fs::metadata(path(m)).is_ok_and(|md| md.len() == m.size)
}

/// Scarica il modello su `<file>.part`, riprende da dove era rimasto,
/// verifica lo SHA-256 e rinomina in modo atomico solo se l'hash coincide.
pub async fn download(m: &ModelInfo, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf> {
    let dest = path(m);
    if is_installed(m) {
        return Ok(dest);
    }
    tokio::fs::create_dir_all(models_dir()).await?;
    let mut part = dest.clone().into_os_string();
    part.push(".part"); // "ggml-base.bin.part", "voxclean-0.6b-q4_k_m.gguf.part"
    let part = PathBuf::from(part);

    let mut hasher = Sha256::new();
    let mut have = 0u64;
    if let Ok(mut f) = tokio::fs::File::open(&part).await {
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            have += n as u64;
        }
    }

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;
    let mut req = client.get(format!("{}/{}", m.base, m.file));
    if have > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
    }
    let resp = req.send().await?.error_for_status()?;
    let mut file = if have > 0 && resp.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&part)
            .await?
    } else {
        hasher = Sha256::new();
        have = 0;
        tokio::fs::File::create(&part).await?
    };

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("download interrotto")?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        have += chunk.len() as u64;
        progress(have, m.size);
    }
    file.flush().await?;
    drop(file);

    let got = hex::encode(hasher.finalize());
    if got != m.sha256 {
        let _ = tokio::fs::remove_file(&part).await;
        bail!(
            "SHA-256 non valido per {}: atteso {}, ottenuto {got}",
            m.file,
            m.sha256
        );
    }
    tokio::fs::rename(&part, &dest).await?;
    Ok(dest)
}
