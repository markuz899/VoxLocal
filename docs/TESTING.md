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
cargo test --lib                              # unit test (resampling)
cargo fmt --check && cargo clippy --all-targets
cd .. && pnpm format:check && pnpm build      # tsc + vite
```

## Bench di trascrizione

```sh
cd src-tauri
cargo run --release --example bench -- --model base            # tutti i WAV in fixtures/
cargo run --release --example bench -- --model base --lang it --runs 5 ../fixtures/it_alice_48k.wav
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

## Metriche misurate

| Metrica | Target | Fase 2 (M1 8 GB, macOS 14.0) |
|---|---|---|
| `.app` / DMG compresso (senza modelli) | < 15 MB | 7,7 MB / 3,9 MB |
| RAM a riposo, modello non caricato | < 80 MB | 37,8 MB RSS (nessuna finestra webview aperta) |
| STT `base`, ~10 s di audio, modello caldo | < 1,5 s fine parlato → incolla | 0,49 s (jfk 11 s), 0,61 s (it 8,4 s); resample < 20 ms |
| Avvio a freddo fino al tray | < 1 s | 485 ms il primo avvio, ~115 ms i successivi |

Il dato "fine parlato → incolla" completo (incluso incolla) si legge nel log alla voce `totale=`. Va misurato con la prova manuale, perché da qui non posso premere la scorciatoia (vedi il report della Fase 2).
