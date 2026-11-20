# Testing

## Prerequisiti (macOS)

```sh
. "$HOME/.cargo/env"
export PATH="$HOME/.local/opt/cmake-3.31.6-macos-universal/CMake.app/Contents/bin:$PATH"   # cmake arm64
nvm use            # Node 22 da .nvmrc
pnpm install
```

## Test automatici

```sh
cd src-tauri
cargo test --lib                              # 16 test: resampling, provider (wiremock), CLI finto, registry, SQLite
cargo fmt --check && cargo clippy --all-targets
cd .. && pnpm format:check && pnpm build      # tsc + vite
```

## Bench di trascrizione

```sh
cd src-tauri
cargo run --release --example bench -- --model base            # tutti i WAV in fixtures/
cargo run --release --example bench -- --model base --lang it --runs 5 ../fixtures/it_alice_48k.wav
cargo run --release --example bench -- --model large-v3-turbo-q5_0 --min-ctx 0   # finestra fissa di 30 s, per confronto
```

Il primo avvio scarica il modello in `~/Library/Application Support/app.voxlocal.desktop/models/` e ne verifica lo SHA-256. Anche la prima esecuzione dopo una build compila gli shader Metal, quindi il caricamento richiede circa 10 s; dalla seconda in poi circa 0,2 s.

Fixture: `en_jfk.wav` (campione di whisper.cpp, licenza MIT), `it_alice_48k.wav` ed `en_samantha_44k.wav` (sintetizzati con `say` di macOS; frequenze diverse apposta per esercitare il resampling).

## Prova manuale end-to-end (Fase 2)

1. `pnpm tauri dev`, oppure apri `src-tauri/target/release/bundle/macos/VoxLocal.app` dopo `pnpm tauri build --bundles app`.
2. Compare l'icona del microfono nella barra dei menu.
3. Apri TextEdit, tieni premuto **⌥Space**, parla, rilascia.
   - Al primo dettato macOS chiede i permessi per il **Microfono** e per l'**Accessibilità**. In dev vengono attribuiti al terminale che ha lanciato l'app.
   - Durante la registrazione il tray mostra ●, durante la trascrizione mostra ….
4. Il testo viene incollato in TextEdit e la clipboard precedente torna com'era dopo circa 250 ms.
5. **Esc** durante la registrazione o la trascrizione annulla senza incollare.
6. Dal tray: "Premi per avviare/fermare" passa alla modalità toggle; "Modello whisper" cambia modello (scaricandolo, se serve).

Log: `~/Library/Logs/app.voxlocal.desktop/VoxLocal.log`. Ogni dettato scrive una riga del tipo
`dettato ok: audio=… resample=… stt=… paste=… totale=…`.

## Fase 3 — LLM, modalità, chiavi

I provider in abbonamento (Claude Code, Codex) esistono solo nella build personale: `pnpm dev:personal`.

Sottocomandi del binario (in dev: `src-tauri/target/debug/voxlocal`):

```sh
voxlocal providers                       # elenco provider + stato chiavi
pbpaste | voxlocal set-key groq          # chiave nel portachiavi (mai su disco)
voxlocal vocab add VoxLocal Tauri        # vocabolario → whisper initial_prompt + LLM
voxlocal check claude-cli                # prova reale con la modalità Pulito + latenza
```

Nell'app, dal tray:
- **Modalità:** Grezzo, Pulito, Email, Messaggio, Elenco puntato, Traduci in EN.
- **Provider LLM predefinito:** quello usato dalle modalità che non ne fissano uno.
- **Trascrizione:** whisper locale oppure Groq/OpenAI cloud, con ripiego su whisper locale in caso di errore.

Fallback da verificare:
- **LLM spento o senza chiave:** scegli Ollama non avviato e detta in "Pulito". Deve incollare il testo grezzo, mandare la notifica "Incollo il testo non elaborato" e salvare in cronologia con `status = llm_fallback`.
- **Esc durante "Elaboro con l'LLM…"** (✎ nel tray): non incolla nulla e il dettato va in cronologia come `cancelled`.

Cronologia: `sqlite3 ~/Library/Application\ Support/app.voxlocal.desktop/voxlocal.db "select status, raw_text, final_text, llm_provider, llm_ms, total_ms from dictations order by id desc limit 5"`.

Misure del 2026-11-20 (`voxlocal check`, modalità Pulito, frase di 18 parole):

| Provider | Latenza |
|---|---|
| claude-cli (Haiku, abbonamento) | 2,2 s |
| codex-cli (abbonamento) | 4,9 s |

## Fase 4 — Interfaccia

Al primo avvio si apre l'onboarding. Per rivederlo: Impostazioni → Informazioni → "riapri la guida". Per ripartire da zero, cancella `settings.json` in `~/Library/Application Support/app.voxlocal.desktop/`.

Checklist manuale:
- [ ] **Onboarding:**
  - [ ] il microfono mostra le barre che si muovono;
  - [ ] il passo Accessibilità si aggiorna da solo dopo aver dato il permesso;
  - [ ] il modello si scarica con la barra di avanzamento;
  - [ ] il dettato di prova finisce nel riquadro;
  - [ ] "Inizia a usare VoxLocal" chiude l'onboarding.
- [ ] **Overlay:** compare in basso al centro mentre detti, con le barre che seguono la voce. Mostra "Trascrivo" ed "Elaboro" con l'onda, poi sparisce. **Il testo deve finire nell'app di destinazione, non nell'overlay** (Q18).
- [ ] **Esc** durante l'overlay annulla.
- [ ] **Generale:**
  - [ ] cambiare scorciatoia: la nuova funziona subito, la vecchia no;
  - [ ] il misuratore della prova del microfono si muove;
  - [ ] l'interruttore "Apri all'accesso" sopravvive a un riavvio dell'app.
- [ ] **Trascrizione:** scaricare, usare ed eliminare un modello (quello in uso non si può eliminare).
- [ ] **Modalità:** modificare e salvare; "Ripristina originale"; creare ed eliminare una modalità personalizzata. Il menu del tray si aggiorna da solo.
- [ ] **Provider:** salvare una chiave (poi `voxlocal providers` deve dire "chiave ✓"); con "Verifica" compare la latenza; rimuovere la chiave.
- [ ] **Cronologia:** la ricerca trova anche parole senza accento ("perche" trova "perché"); "Mostra originale" sui dettati elaborati; "Svuota" chiede conferma nella pagina.
- [ ] Chiudere la finestra non chiude l'app; riaprire VoxLocal (doppio clic) porta davanti le impostazioni.

L'anteprima della UI fuori da Tauri si fa con `mockIPC` di `@tauri-apps/api/mocks` (pagina temporanea, non nel repo).

## Metriche misurate

| Metrica | Target | Fase 2 (M1 8 GB, macOS 14.0) |
|---|---|---|
| `.app` / DMG compresso (senza modelli) | < 15 MB | Fase 4: 9,1 MB / 4,7 MB |
| RAM a riposo, modello non caricato | < 80 MB | Fase 4: RSS 43 MB, footprint 17 MB, nessun processo WebKit (overlay creato al bisogno, D29) |
| STT `base`, ~10 s di audio, modello caldo | < 1,5 s fine parlato → incolla | 0,30 s (it 8,4 s) con flash attention + contesto proporzionale |
| STT `large-v3-turbo-q5_0` (default Apple Silicon) | — | 0,6-0,9 s per 1-8 s di parlato, 2,4 s per 21 s; picco RAM 469 MB |
| Avvio a freddo fino al tray | < 1 s | 485 ms il primo avvio, ~115 ms i successivi |

Il dato "fine parlato → incolla" completo (incluso incolla) si legge nel log alla voce `totale=`. Va misurato con la prova manuale, perché da qui non posso premere la scorciatoia (vedi il report della Fase 2).
