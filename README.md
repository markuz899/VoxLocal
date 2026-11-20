# VoxLocal

App desktop di dettatura vocale per macOS e Windows (Linux in parte). Tieni premuta una scorciatoia, parli, rilasci: il testo compare nell'app che stai usando, che sia email, chat, editor o browser.

La voce viene trascritta **sul computer** con whisper.cpp: funziona offline e l'audio non lascia il dispositivo. Se vuoi, un LLM a tua scelta ripulisce o riscrive il testo prima dell'incolla.

## Come funziona

```
scorciatoia ─► microfono ─► 16 kHz ─► whisper.cpp ─► [LLM opzionale] ─► incolla ─► cronologia
  (⌥Space)      (cpal)     (rubato)   (sul computer)   (modalità)       (⌘V/Ctrl+V)  (SQLite)
```

1. **Scorciatoia globale.** Di default ⌥Space su Mac e Alt+Space su Windows: tieni premuto per parlare, oppure premi per iniziare e premi di nuovo per finire. Esc annulla in qualsiasi momento.
2. **Cattura.** L'audio arriva dal microfono scelto in un buffer senza lock. Quando rilasci, viene convertito a 16 kHz mono in un'unica passata. Le pressioni sotto i 300 ms e il silenzio vengono scartati.
3. **Trascrizione locale.** whisper.cpp (tramite `whisper-rs`) usa Metal su Apple Silicon. Il modello predefinito è Large v3 Turbo compresso: con flash attention e una finestra di contesto proporzionale alla durata dell'audio, trascrive una frase in 0,6–0,9 s su un M1.
   - Il modello viene caricato già alla pressione del tasto, resta in memoria 10 minuti e poi viene liberato.
   - La trascrizione può anche passare da Groq o OpenAI. Se il servizio non risponde, si torna a quella locale.
4. **Modalità (opzionale).** Il testo può passare da un LLM con un template di prompt: Pulito, Email, Messaggio, Elenco puntato, Traduci in EN, oppure modalità create da te. Il vocabolario personale (nomi, sigle) viene passato sia a whisper sia all'LLM. Se l'LLM sbaglia o va in timeout, viene incollato il testo grezzo: un dettato non si perde mai.
5. **Incolla.** VoxLocal salva gli appunti, ci scrive il testo, simula ⌘V o Ctrl+V e poi rimette negli appunti quello che c'era prima. Se l'incolla automatico non riesce, il testo resta negli appunti e arriva una notifica.
6. **Cronologia.** Ogni dettato finisce in SQLite locale, con testo grezzo e finale, tempi e stato, ricerca full-text e tempo di conservazione configurabile.

### Provider LLM

Un solo client compatibile con le API di OpenAI copre OpenAI, Groq, OpenRouter, Together, Mistral, DeepSeek, Ollama, LM Studio e llama.cpp server. Anthropic e Gemini hanno client nativi; "Nessuno" incolla la trascrizione grezza. Aggiungere un provider vuol dire un file in `src-tauri/src/llm/` più una riga nel registry.

Le chiavi API stanno nel portachiavi del sistema (Portachiavi, Gestione credenziali, Secret Service), mai su disco in chiaro. La parte web dell'app non fa chiamate di rete e non vede segreti.

## Stack

- **Tauri v2**: backend in Rust, interfaccia in React + TypeScript + Vite + Tailwind. L'overlay è in TypeScript puro.
- **Audio:** `cpal`, `rubato`, `rtrb`.
- **Trascrizione:** `whisper-rs` (whisper.cpp), modelli ggml da Hugging Face verificati con SHA-256.
- **Rete:** `reqwest` (rustls) per gli LLM; `wiremock` per i test.
- **Dati e sistema:** `rusqlite` (FTS5), `keyring`, `enigo` + `arboard` per l'incolla.
- **Distribuzione:** aggiornamenti firmati (minisign) da GitHub Releases.

## Struttura

```
src-tauri/src
  audio/      cattura, resampling, lettura WAV
  stt/        trait SttProvider, whisper locale, STT compatibile OpenAI
  llm/        trait LlmProvider, registry, OpenAI-compat, Anthropic, Gemini, CLI
  pipeline/   macchina a stati, annullamento, metriche, fallback
  paste/      clipboard + tasti simulati
  storage/    SQLite: cronologia, vocabolario, modalità
  settings/   impostazioni (store) + chiavi (keyring)
  models/     manifest, download verificato
  commands.rs superficie IPC verso la UI
src/          impostazioni, onboarding, cronologia, overlay (React/TS)
site/         landing page statica (GitHub Pages)
docs/         ARCHITECTURE, DECISIONS, OPEN_QUESTIONS, TESTING, RELEASE, INSTALL
```

## Sviluppo

Requisiti: Rust stabile, Node 22 (`.nvmrc`), pnpm, cmake. Su macOS servono Xcode e i Command Line Tools.

```sh
pnpm install
pnpm tauri dev                 # build commerciale
pnpm dev:personal              # con i provider in abbonamento Claude Code / Codex (solo uso personale)

cd src-tauri
cargo test --lib               # test, compresi i provider con server finti (wiremock)
cargo run --release --example bench -- --model large-v3-turbo-q5_0   # benchmark di trascrizione
```

Sottocomandi del binario: `voxlocal providers`, `voxlocal set-key <provider>`, `voxlocal vocab add <termine>`, `voxlocal check <provider>`.

## Prestazioni misurate (Mac M1, 8 GB)

| | Obiettivo | Misurato |
|---|---|---|
| Installer senza modelli | < 15 MB | DMG universal 9,6 MB |
| RAM a riposo | < 80 MB | 43 MB di RSS (17 MB di footprint) |
| Fine del parlato → incolla, 10 s, `base` | < 1,5 s | ~0,5–0,8 s |
| Avvio fino al tray | < 1 s | 220–510 ms |

## Rilascio e installazione

- Pubblicare una versione e firmare il codice: [docs/RELEASE.md](docs/RELEASE.md)
- Installare l'app: [docs/INSTALL.md](docs/INSTALL.md)
- Architettura e decisioni: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/DECISIONS.md](docs/DECISIONS.md)
