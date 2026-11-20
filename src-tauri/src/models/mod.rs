//! Modelli ggml di whisper.cpp: manifest con SHA-256, download riprendibile e verificato.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

pub struct ModelInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub file: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

/// Hash e dimensioni dai metadati LFS di Hugging Face (presi il 2026-11-20).
pub const MODELS: &[ModelInfo] = &[
    m(
        "tiny",
        "Tiny (75 MB)",
        "ggml-tiny.bin",
        77691713,
        "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
    ),
    m(
        "tiny-q5_1",
        "Tiny q5 (31 MB)",
        "ggml-tiny-q5_1.bin",
        32152673,
        "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7",
    ),
    m(
        "base",
        "Base (142 MB)",
        "ggml-base.bin",
        147951465,
        "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    ),
    m(
        "base-q5_1",
        "Base q5 (57 MB)",
        "ggml-base-q5_1.bin",
        59707625,
        "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
    ),
    m(
        "small",
        "Small (466 MB)",
        "ggml-small.bin",
        487601967,
        "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    ),
    m(
        "small-q5_1",
        "Small q5 (181 MB)",
        "ggml-small-q5_1.bin",
        190085487,
        "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
    ),
    m(
        "medium-q5_0",
        "Medium q5 (514 MB)",
        "ggml-medium-q5_0.bin",
        539212467,
        "19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f",
    ),
    m(
        "large-v3-turbo-q5_0",
        "Large v3 Turbo q5 (547 MB)",
        "ggml-large-v3-turbo-q5_0.bin",
        574041195,
        "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    ),
    m(
        "large-v3-turbo",
        "Large v3 Turbo (1,5 GB)",
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
        label,
        file,
        size,
        sha256,
    }
}

pub fn find(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.id == id)
}

/// Default suggerito per l'hardware. Bench su M1 8 GB, 10 s di audio: base ≈ 0,5 s,
/// small-q5_1 ≈ 1,4 s (e peggiore sull'italiano), quindi base ovunque.
// ponytail: niente rilevamento RAM/GPU; large-v3-turbo-q5_0 come proposta per macchine ≥16 GB arriva con l'onboarding
pub fn recommended() -> &'static str {
    "base"
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
    let part = dest.with_extension("bin.part");

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
    let mut req = client.get(format!("{BASE_URL}/{}", m.file));
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
