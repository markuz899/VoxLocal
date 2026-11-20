# VoxLocal — Architettura

Stato: Fase 1 (design). Ogni scelta qui è motivata in [DECISIONS.md](DECISIONS.md);
quello che non è deciso sta in [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md).

## 1. Vista d'insieme

```
┌──────────────────────────── Webview (React/TS) ────────────────────────────┐
│  overlay.html        settings.html (impostazioni · cronologia · onboarding) │
│  sola UI: niente rete, niente segreti, niente SQL                           │
└───────────────▲───────────────────────────────────┬────────────────────────┘
                │ eventi (pipeline://state, audio://level)│ invoke(command)
┌───────────────┴───────────────────────────────────▼────────────────────────┐
│ commands.rs  — superficie IPC tipizzata (unico punto d'ingresso dal webview)│
├─────────────────────────────────────────────────────────────────────────────┤
│ hotkey ──► pipeline (state machine, CancellationToken, metriche latenza)    │
│              │          │                 │                │                │
│              ▼          ▼                 ▼                ▼                │
│           audio       stt               llm              paste              │
│     cpal → rubato  trait SttProvider  trait LlmProvider  clipboard + enigo  │
│     → AudioBuffer  ├ LocalWhisper     ├ OpenAiCompatible  (per piattaforma) │
│                    └ OpenAiCompatStt  ├ Anthropic                            │
│                         │             ├ Gemini                               │
│                         ▼             └ None                                 │
│                      models (download HF, SHA-256, gestione file)           │
├─────────────────────────────────────────────────────────────────────────────┤
│ storage (rusqlite: cronologia, vocabolario, modalità, FTS5, migrazioni)     │
│ settings (tauri-plugin-store JSON  +  keyring per le chiavi API)            │
│ tray · single-instance · autostart · updater · notification · log           │
└─────────────────────────────────────────────────────────────────────────────┘
```

Regole di dipendenza tra moduli:

- `pipeline` conosce i trait, non le implementazioni. Le riceve dal `registry`.
- `stt`, `llm`, `storage` e `paste` non sanno nulla di Tauri, quindi si testano con `cargo test` senza app.
- Solo `commands.rs` e `pipeline` emettono eventi verso il webview.
- Il webview non fa rete. La CSP di Tauri blocca `connect-src` verso l'esterno: la difesa è tecnica, non solo una convenzione.

### Thread e runtime

| Lavoro | Dove gira |
|---|---|
| Callback di cpal (cattura) | thread audio di cpal: copia i campioni in un `rtrb` (ring buffer lock-free) e basta |
| Raccolta campioni + livello RMS per l'overlay | task tokio; emette `audio://level` a ~30 Hz |
| Resampling (rubato) | alla fine della registrazione, in un'unica passata (con 10 s di audio servono < 5 ms) |
| Inferenza whisper | `spawn_blocking` (CPU/Metal, non deve bloccare il runtime) |
| HTTP verso STT/LLM | async su tokio, `reqwest` con rustls |
| Simulazione dei tasti (enigo) | **main thread** su macOS (le API TIS/CGEvent lo richiedono), tramite `run_on_main_thread` |

## 2. Macchina a stati della pipeline

```
                 hotkey down (PTT) / press (toggle)
       ┌──────┐ ─────────────────────────────────► ┌───────────┐
       │ Idle │                                     │ Recording │──┐ Esc / durata < 300 ms
       └──────┘ ◄──────────────┐                    └─────┬─────┘  │ (tocco involontario)
          ▲  ▲                 │   hotkey up / 2ª press   │        ▼
          │  │                 │                          ▼     Cancelled ──► Idle
          │  │            ┌────┴────┐  ok      ┌──────────────┐
          │  │            │ Pasting │◄─────────│PostProcessing│  (saltato se modalità = None)
          │  │            └────┬────┘          └──────┬───────┘
          │  │                 │   ▲  errore/timeout   │
          │  │                 │   └── testo grezzo ◄──┘   (fallback: mai perdere il dettato)
          │  │                 ▼                       ▲
          │  └──────────── Done (salvato in cronologia) │ ok
          │                                     ┌──────┴──────┐
          └───── Error (notifica + cronologia) ◄│ Transcribing│◄── Recording terminata
                                                └─────────────┘
```

- Gli stati sono un `enum PipelineState`. Ogni transizione emette `pipeline://state { state, dictation_id, elapsed_ms }`, che pilota l'overlay.
- **Annullamento:** un solo `CancellationToken` (tokio-util) per dettato. Esc lo attiva. Whisper controlla il token tramite `abort_callback`, l'HTTP con `tokio::select!`. Un dettato annullato *dopo* la trascrizione viene salvato comunque in cronologia con `status = cancelled`: il testo non si perde.
- **Concorrenza:** un dettato alla volta. Se si preme la hotkey mentre la pipeline è in Transcribing o PostProcessing, si apre in coda un secondo dettato (max 1 in coda); l'incolla avviene in ordine.
- **Metriche:** ogni dettato registra `audio_ms`, `resample_ms`, `stt_ms`, `llm_ms`, `paste_ms` e `total_ms` (dal rilascio della hotkey all'incolla). Finiscono sia nel log sia nella riga della cronologia.
- **Tetto alla registrazione:** 10 minuti, configurabile. Oltre, si ferma da sola con una notifica (protegge RAM e tempi di trascrizione).

## 3. Trait dei provider e registry

```rust
pub struct AudioBuffer { pub samples: Vec<f32>, pub sample_rate: u32 } // sempre 16 kHz mono
pub struct SttOptions  { pub language: Option<String>, pub initial_prompt: Option<String>,
                         pub cancel: CancellationToken }
pub struct Transcript  { pub text: String, pub language: Option<String>, pub segments: Vec<Segment> }

pub struct LlmRequest  { pub system: String, pub user: String, pub model: String,
                         pub temperature: f32, pub max_tokens: u32, pub timeout: Duration,
                         pub cancel: CancellationToken }
pub struct LlmResponse { pub text: String, pub usage: Option<Usage> }

#[async_trait] pub trait SttProvider: Send + Sync { /* come da requisito */ }
#[async_trait] pub trait LlmProvider: Send + Sync { /* come da requisito */ }
```

**Registry:** la configurazione di un provider è `ProviderConfig { id, label, kind, base_url, model, needs_key, send_sampling, command, system_flag, model_flag, clear_env, stt_model }`. I preset stanno nel codice; `settings.providers` contiene solo personalizzazioni e aggiunte, che vincono sui preset con lo stesso id. Il registry fa un `match kind` e restituisce un `Arc<dyn …Provider>`, tenuto in cache finché la sua configurazione non cambia. Per aggiungere un provider bastano un file (`llm/mistral_native.rs`, per esempio) e un ramo nel `match`.

**Preset OpenAI-compatibili:** sono dati, non codice. Sono una tabella statica `(id, label, base_url, needs_key, default_model)`: OpenAI, Groq, OpenRouter, Together, Mistral, DeepSeek, Ollama `:11434/v1`, LM Studio `:1234/v1` e llama.cpp server `:8080/v1`.

**`LocalWhisper`:**
- Tiene `Mutex<Option<(WhisperContext, Instant)>>` e carica il modello in modo lazy al primo dettato. Per ridurre la latenza percepita, il precaricamento parte già alla *pressione* della hotkey.
- Un timer tokio rilascia il contesto dopo `keep_warm_minutes` (default 10; 0 = rilascio subito; −1 = sempre caldo).
- Usa un solo `WhisperState`. Parametri: `greedy best_of=1` e `no_context=true`; `language` è "auto" oppure fisso; `initial_prompt` è costruito dal vocabolario (troncato a ~200 token).

## 4. Modalità e template

Una modalità è una riga di `modes`: `{name, system_prompt, user_template, provider_id|null, model|null, temperature}`.

Variabili supportate: `{{text}}`, `{{language}}`, `{{vocabulary}}`. La sostituzione è testuale e fatta a mano (non serve un motore di template).

Modalità predefinite: Grezzo (provider `none`), Pulito, Email, Messaggio, Elenco puntato, Traduci in EN. Si possono modificare; "ripristina predefinita" le rigenera dal seed. Ogni modalità ha un'hotkey opzionale. La modalità attiva si sceglie anche dal menu del tray.

## 5. Schema SQLite

File: `<app_data_dir>/voxlocal.db`, in modalità WAL. Le migrazioni sono incluse nel binario e versionate con `PRAGMA user_version`.

```sql
CREATE TABLE dictations (
  id            INTEGER PRIMARY KEY,
  created_at    INTEGER NOT NULL,              -- unix ms
  mode_id       INTEGER REFERENCES modes(id) ON DELETE SET NULL,
  language      TEXT,
  raw_text      TEXT NOT NULL,
  final_text    TEXT NOT NULL,                 -- = raw_text se LLM assente/fallito
  stt_provider  TEXT NOT NULL,
  stt_model     TEXT,
  llm_provider  TEXT,
  llm_model     TEXT,
  status        TEXT NOT NULL CHECK (status IN ('ok','llm_fallback','cancelled','paste_failed','error')),
  error         TEXT,
  audio_ms      INTEGER NOT NULL,
  stt_ms        INTEGER, llm_ms INTEGER, paste_ms INTEGER, total_ms INTEGER,
  target_app    TEXT                           -- hook: contesto app attiva (post-MVP)
);
CREATE INDEX dictations_created ON dictations(created_at);

CREATE VIRTUAL TABLE dictations_fts USING fts5(
  raw_text, final_text, content='dictations', content_rowid='id', tokenize='unicode61 remove_diacritics 2'
);
-- trigger AFTER INSERT/UPDATE/DELETE che tengono allineato dictations_fts

CREATE TABLE vocabulary (
  id INTEGER PRIMARY KEY, term TEXT NOT NULL UNIQUE COLLATE NOCASE,
  replacement TEXT,                            -- opz.: "vox local" → "VoxLocal"
  created_at INTEGER NOT NULL
);

CREATE TABLE modes (
  id INTEGER PRIMARY KEY, slug TEXT UNIQUE, name TEXT NOT NULL,
  system_prompt TEXT NOT NULL, user_template TEXT NOT NULL,
  provider_id TEXT, model TEXT, temperature REAL NOT NULL DEFAULT 0.2,
  hotkey TEXT, builtin INTEGER NOT NULL DEFAULT 0, sort INTEGER NOT NULL DEFAULT 0
);
```

- **Retention:** all'avvio e poi ogni 6 ore gira `DELETE FROM dictations WHERE created_at < ?`, con N giorni configurabili (0 = per sempre).
- **Audio:** non viene salvato di default. Un'opzione tiene gli ultimi N WAV in `<app_data_dir>/audio/` per poterli ritrascrivere.

## 6. Impostazioni

Formato obiettivo (in Fase 2 lo struct `Settings` è ancora piatto, con solo i campi già usati; diventa annidato in Fase 4). Le impostazioni stanno in `tauri-plugin-store` (`settings.json` in `app_config_dir`). È JSON leggibile e versionato con `"version"`. Lato Rust c'è uno struct `Settings` serde con `#[serde(default)]` su tutto: un file vecchio o parziale non rompe l'avvio.

```jsonc
{
  "version": 1,
  "app_name": "VoxLocal",
  "hotkeys": { "dictate": "Alt+Space", "mode": "ptt" },        // "ptt" | "toggle"
  "audio":   { "device": null, "max_seconds": 600, "keep_audio": 0 },
  "language": "auto",                                          // "auto" | "it" | "en" | …
  "stt":     { "provider": "local", "local_model": "base", "keep_warm_minutes": 10,
               "remote": { "preset": "groq", "base_url": "https://api.groq.com/openai/v1",
                           "model": "whisper-large-v3-turbo", "timeout_s": 30 } },
  "llm_providers": [
    { "id": "ollama", "kind": "openai_compat", "base_url": "http://localhost:11434/v1", "model": "llama3.2" },
    { "id": "anthropic", "kind": "anthropic", "model": "claude-haiku-5-5" }
  ],
  "llm":     { "timeout_s": 15, "connect_timeout_s": 5 },
  "active_mode": "clean",
  "paste":   { "method": "auto", "restore_clipboard": true, "restore_delay_ms": 200 },
  "history": { "retention_days": 90 },
  "startup": { "autostart": false, "check_updates": true },
  "log_level": "info"
}
```

**Segreti:** le chiavi API vanno nel keyring del sistema (Keychain, Windows Credential Manager, Secret Service) con `service = "<bundle id>"` e `user = "<provider id>"`. Nel JSON c'è solo il riferimento implicito tramite l'id. Il webview può *scrivere* una chiave (`set_api_key`) e sapere *se* esiste (`has_api_key`), ma non può mai rileggerla.

## 7. Errori e timeout

Errori: `thiserror`, un enum per modulo (`AudioError`, `SttError`, `LlmError`, `PasteError`), raccolti in `AppError`. Verso il webview viaggiano come `{ code, message }` serializzati: niente stack trace né segreti.

| Fase | Timeout di default | In caso di errore |
|---|---|---|
| Apertura del microfono | 2 s | notifica "microfono non disponibile", torna a Idle |
| Whisper locale | nessuno (annullabile) | stato Error, il dettato resta in cronologia con status `error` |
| STT remoto | connessione 5 s, totale 30 s | **fallback automatico a LocalWhisper** se un modello è installato, altrimenti Error |
| LLM | connessione 5 s, totale 15 s | **incolla il testo grezzo**, status `llm_fallback`, notifica discreta |
| Incolla | — | il testo resta nella clipboard (niente ripristino), status `paste_failed`, notifica "premi ⌘V" |

- **Retry:** nessuno sull'LLM, perché la latenza conta più dell'affidabilità e c'è già il fallback. Un solo retry sugli errori di *connessione* dell'STT remoto.
- **Log:** `tauri-plugin-log` su file, con rotazione a 5 MB × 3. I testi dettati non finiscono nei log, a meno di `log_level = trace`.

## 8. Incolla

1. Salva il contenuto testuale della clipboard (con `arboard`, già dipendenza di clipboard-manager).
2. Scrive il testo.
3. Simula ⌘V o Ctrl+V con enigo.
4. Dopo `restore_delay_ms` ripristina il testo precedente, ma solo se la clipboard contiene ancora il nostro testo (così non sovrascrive una copia fatta nel frattempo dall'utente).

- **Linux/Wayland** (`WAYLAND_DISPLAY` presente e niente XWayland per la simulazione): si salta il passo 3 e si lascia il testo nella clipboard con una notifica.
- **Windows:** se l'app in primo piano gira come amministratore, UIPI blocca SendInput. In quel caso si fa lo stesso fallback ("testo copiato").

## 9. Permessi per piattaforma

| | macOS | Windows | Linux |
|---|---|---|---|
| Microfono | `NSMicrophoneUsageDescription` in Info.plist; la richiesta TCC parte alla prima cattura. Lo stato si legge con `AVCaptureDevice.authorizationStatus` (objc2) per l'onboarding | Impostazioni → Privacy → Microfono ("consenti alle app desktop"); se cpal fallisce si rimanda a `ms-settings:privacy-microphone` | PulseAudio/PipeWire, niente prompt |
| Simulare i tasti | **Accessibilità**: `AXIsProcessTrustedWithOptions` (prompt) e link a `x-apple.systempreferences:…Privacy_Accessibility`. Va rifatta a ogni cambio di firma, quindi le build di sviluppo la perdono | non serve | X11 sì; Wayland no (fallback clipboard) |
| Hotkey globale | Carbon `RegisterEventHotKey` tramite il plugin: **non** serve "Monitoraggio input" | `RegisterHotKey` | X11 sì; Wayland limitato |

L'onboarding controlla ogni permesso, mostra lo stato e ricontrolla a intervalli (polling) dopo che l'utente torna da Impostazioni di Sistema.

## 10. Finestre e overlay

- **overlay:**
  - Finestra separata da 220×56 px, trasparente, senza decorazioni, `always_on_top`, `focus: false`, `skip_taskbar`. Su macOS ha il livello "floating panel" e `visibleOnAllWorkspaces`.
  - Non deve mai rubare il focus all'app di destinazione, altrimenti l'incolla finisce nell'overlay. Si crea all'avvio nascosta: costa poco e appare in < 16 ms.
  - **Esc:** l'overlay non ha il focus, quindi Esc è un'altra scorciatoia globale, registrata solo durante Recording, Transcribing e PostProcessing e rimossa subito dopo.
- **settings:** si crea su richiesta dal tray e si **distrugge** alla chiusura, non si nasconde. Questo libera la RAM del webview ed è necessario per stare sotto gli 80 MB a riposo.
- Su macOS l'app non compare nel Dock (`ActivationPolicy::Accessory`): si usa dal tray.

## 11. Modelli Whisper

- Manifest statico nel codice: nome, file, dimensione e SHA-256 per `tiny`, `base`, `small`, `medium`, `large-v3-turbo` e le varianti `-q5_0`/`-q8_0`.
- Sorgente: `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/<file>`.
- Download in streaming su `<file>.part` con supporto `Range` per riprendere, SHA-256 calcolato al volo, `rename` atomico solo se l'hash coincide. L'avanzamento arriva al webview con l'evento `models://progress`.
- **Default:** `base` su tutte le piattaforme (misurato in Fase 2, DECISIONS D19). L'onboarding proporrà `large-v3-turbo-q5_0` sulle macchine con ≥ 16 GB.

## 12. Hook post-MVP (già previsti, implementazione vuota)

- `license::LicenseGate` (trait): `check() -> LicenseState`. Nell'MVP è `AlwaysValid`; poi arriverà Lemon Squeezy (`/v1/licenses/activate` e `/validate`, con cache offline).
- `context::ActiveAppProbe` (trait): `frontmost_app() -> Option<AppInfo>`. Nell'MVP restituisce `None`, ma la colonna `target_app` esiste già. Dopo: regola "app → modalità".
- `audio::Vad` (trait) collegato al loop di cattura: nell'MVP non fa nulla; poi Silero o webrtc-vad per lo stop automatico.

## 13. Performance: come stare nei target

| Target | Strategia |
|---|---|
| Installer < 15 MB | niente modelli inclusi; `opt-level="s"`, `lto`, `strip`, `panic=abort` nel profilo release; whisper.cpp statico (~2-3 MB) |
| RAM a riposo < 80 MB | finestra delle impostazioni distrutta, modello rilasciato, overlay minimo (HTML/CSS senza React, se serve) |
| Latenza < 1,5 s (base, 10 s, M1) | modello già caldo, resampling in una passata, Metal, greedy, `no_timestamps` |
| Avvio < 1 s fino al tray | tray creato prima di tutto il resto; DB e store aperti dopo, in un task |

Misure: `cargo run --release --example bench -- --model base` (in `src-tauri`) stampa caricamento, resample, stt mediano/minimo e RTF per ogni WAV in `fixtures/`. In app, il comando `debug_metrics` restituisce RSS e le ultime N latenze.
