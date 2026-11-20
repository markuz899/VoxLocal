# Decisioni, rischi e alternative

Formato breve: decisione → perché → alternative scartate. Le dipendenze "pesanti" vanno motivate qui.

## D1 — Persistenza con `rusqlite` (feature `bundled`), non `tauri-plugin-sql`
- **Perché:** il plugin espone l'SQL al webview, mentre per requisito dati e logica stanno nel backend. Con rusqlite le migrazioni restano in Rust e si testano con `cargo test`. Inoltre `bundled` porta FTS5 su tutte le piattaforme.
- **Costo:** circa 1 MB nel binario.
- **Alternativa:** `tauri-plugin-sql` (sqlx). Scartata: compilazione più pesante e superficie IPC più larga.

## D2 — `async-trait` per i trait dei provider
- **Perché:** il registry restituisce `Arc<dyn LlmProvider>`, e gli `async fn` nativi nei trait non sono ancora dyn-compatible senza boxing manuale.
- **Alternativa:** `Pin<Box<dyn Future>>` scritto a mano. Fa la stessa cosa con più rumore.

## D3 — Un solo `OpenAiCompatible` + preset come dati
- **Perché:** copre 9+ provider con un solo file da testare. Un preset è una riga in una tabella.
- **Rischio:** differenze sottili tra provider (DeepSeek non accetta `max_tokens` oltre una certa soglia, Ollama ignora alcuni parametri). Mitigazione: invio solo `model`, `messages`, `temperature` e `max_tokens`; ogni preset può avere un campo `quirks` se servirà.

## D4 — Cancellazione con `tokio-util::CancellationToken`
- **Perché:** un solo primitivo attraversa audio, whisper (`abort_callback`) e HTTP (`select!`). tokio-util è già nell'albero delle dipendenze di Tauri.

## D5 — Ring buffer `rtrb` tra il callback di cpal e il resto
- **Perché:** nel callback audio non si deve allocare né prendere lock. `rtrb` è piccolo, senza dipendenze, wait-free.
- **Alternativa:** `crossbeam-channel` con `Vec` per callback. Alloca nel callback: scartata.

## D6 — Resampling in una passata a fine registrazione
- **Perché:** whisper ha comunque bisogno del buffer completo, e lavorare per blocchi in tempo reale aggiunge complessità senza ridurre la latenza finale (< 5 ms per 10 s di audio).
- **Rivedere** quando arriverà il VAD/streaming (post-MVP).

## D7 — Overlay come finestra non focalizzabile + Esc come scorciatoia globale temporanea
- **Perché:** se l'overlay prendesse il focus, l'incolla finirebbe al posto sbagliato.
- **Rischio:** su alcune app Esc è "rubato" durante la registrazione. È accettabile: la registrazione dura pochi secondi e Esc viene rilasciato subito dopo.
- **Alternativa:** NSPanel nativo `nonactivatingPanel` su macOS. È più corretto ma richiede codice objc2; lo tengo come piano B se `focus:false` si rivelasse inaffidabile.

## D8 — Incolla con salvataggio e ripristino della clipboard *testuale*
- **Rischio:** se l'utente aveva in clipboard un'immagine o un file, viene persa. Vedi OPEN_QUESTIONS Q4.
- **Alternativa:** digitare il testo carattere per carattere (`enigo.text()`). Non tocca la clipboard, ma è lento sui testi lunghi e sbaglia con IME e layout non US. Lo tengo come metodo `paste.method = "type"` selezionabile.

## D9 — `enigo` eseguito sul main thread su macOS
- **Perché:** su macOS enigo usa API (TIS per il layout di tastiera) che vanno chiamate dal main thread, altrimenti crash o tasti sbagliati. Si risolve con `app.run_on_main_thread`.

## D10 — `whisper-rs` con feature per piattaforma
- **Configurazione:**
  - macOS arm64: `metal`
  - macOS x86_64: CPU + Accelerate (Metal su GPU Intel/AMD è lento o instabile, vedi Q6)
  - Windows: CPU, con le feature cargo `vulkan` e `cuda` opzionali
- **Rischio di build:** whisper-rs-sys compila whisper.cpp con cmake e bindgen (richiede libclang). Su questo Mac il cmake di Homebrew è x86_64 sotto Rosetta, e ho installato un cmake universal in `~/.local/opt`. In CI non è un problema.
- **Rischio di versione:** l'API di whisper-rs cambia spesso tra le minor. La fisso a una versione esatta.

## D11 — `keyring` v3
- **Perché:** usa Keychain, Credential Manager e Secret Service tramite un'unica API.
- **Rischio:** su Linux senza Secret Service attivo fallisce. In quel caso si mostra un errore chiaro e non si salva in chiaro (vedi Q8).

## D12 — `reqwest` con `rustls-tls`, senza `default-features`
- **Perché:** niente OpenSSL da linkare su Windows o Linux, binario prevedibile. Timeout per richiesta (`.timeout()`) più `connect_timeout` sul client.

## D13 — `wiremock` (dev-dependency) per i test dei provider
- Requisito esplicito. Non entra nel binario.

## D14 — `hound` per leggere e scrivere WAV (bench, fixture, audio opzionale)
- Piccolo, senza dipendenze.

## D15 — Overlay in HTML/CSS + TS puro; React solo nella finestra delle impostazioni
- **Perché:** l'overlay resta sempre in memoria, e meno JS significa meno RAM a riposo. Vite gestisce due entry (`overlay.html`, `settings.html`).
- Le impostazioni usano React + Tailwind come da stack.

## D16 — pnpm con Node 22 (`.nvmrc`)
- **Perché:** il pnpm installato richiede Node ≥ 22.13. La Tauri CLI sta tra le devDependency (`@tauri-apps/cli`), non è installata a livello globale.

## Rischi principali (in ordine)

1. **Push-to-talk e Released:** su Windows, se l'utente rilascia prima il modificatore e poi il tasto, `RegisterHotKey` non genera sempre l'evento Released. *Mitigazione:* tetto di durata più il fatto che una seconda pressione chiude comunque. Se non basta, si passa a un hook di tastiera di basso livello (`rdev`) solo su Windows.
2. **Permesso di Accessibilità su macOS** che si invalida a ogni ricompilazione con firma ad-hoc: è fastidioso in sviluppo. *Mitigazione:* firmare le build di dev con il certificato Apple Development (ricordarsi del WWDR G3).
3. **Bundle sotto i 15 MB** con whisper.cpp + Metal + WebView: dovrebbe starci, perché un'app Tauri vuota pesa circa 3-4 MB. Lo verifico in Fase 2.
4. **RAM a riposo sotto gli 80 MB:** il processo WebKit dell'overlay conta, e su macOS i processi WebContent sono separati. Misuro l'RSS totale dell'albero dei processi, non solo quello del binario.
5. **Hash SHA-256 dei modelli:** vanno presi dai metadati LFS di Hugging Face e congelati nel manifest. Se upstream ripubblica un file, il download fallisce in modo sicuro e chiaro (preferibile a un modello non verificato).
