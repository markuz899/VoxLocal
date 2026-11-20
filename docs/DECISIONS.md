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

## D17 — Abort di whisper con callback `unsafe` scritta a mano (Fase 2)
- **Perché:** `FullParams::set_abort_callback_safe` di whisper-rs 0.16 passa a whisper.cpp un `Box<Box<dyn FnMut>>` ma il trampolino lo rilegge come la closure concreta `F`. Il risultato è una lettura di memoria non valida (panico `failed to lock mutex: Invalid argument` alla prima trascrizione).
- **Soluzione:** uso `set_abort_callback` + `set_abort_callback_user_data` con un puntatore al `CancellationToken`, che vive nello stack frame di `full()`.
- **Quando si aggiorna whisper-rs:** riverificare (la versione è fissata a `=0.16.0`).

## D18 — Bench come `example`, non `[[bin]]`
- **Perché:** Tauri impacchetta nell'app *tutti* i binari del crate, e il bench aggiungeva 4 MB al bundle. Si lancia con `cargo run --release --example bench`.

## D19 — Modello di default `base` ovunque — superata da D27
- **Misura su M1 8 GB, ~10 s di audio:** `base` impiega circa 0,5 s; `small-q5_1` circa 1,4 s ed è anche peggiore sull'italiano di prova. Proporre modelli più grandi in base alla RAM è rimandato all'onboarding (Fase 4).

## D20 — `anyhow` al posto degli enum `thiserror` per modulo (per ora) — superata in Fase 3 per l'LLM (`LlmError`)
- **Perché:** in Fase 2 nessun chiamante deve distinguere i tipi di errore, perché l'annullamento si legge dal token. Gli enum tipizzati arrivano con lo strato LLM, dove servono (timeout → fallback, 401 → chiave mancante).

## D21 — Lingua di default = lingua del sistema, non "auto"
- **Prova reale di Marco:** quattro dettati brevi (2-3 s) in italiano sono stati riconosciuti come de, pt, it e ru (p tra 0,22 e 0,63), con il testo tradotto o traslitterato.
- **Con la lingua fissata:** testo corretto e STT 3 volte più veloce (niente passaggio di rilevamento della lingua).
- **Come funziona ora:** `sys-locale` legge la lingua del sistema. Se whisper non la conosce, si ripiega su "auto". Dal tray si cambia al volo.

## D22 — Abbonamenti Claude/Codex tramite i CLI ufficiali (Fase 3)
- **Come funziona:** il provider `cli` lancia `claude -p` o `codex exec` come sottoprocesso. Prompt su stdin, risultato da stdout, `kill_on_drop` per timeout ed Esc. VoxLocal non tocca token: usa il login che l'utente ha fatto nel CLI.
- **Flag misurati sul Mac di Marco:**
  - Claude Code: `--tools "" --strict-mcp-config --setting-sources "" --no-session-persistence --system-prompt …`, da 3,5 s a 1,7–2,2 s.
  - Codex: `--ephemeral -s read-only -c model_reasoning_effort=none -c mcp_servers={}`, circa 3,5–4,9 s. Il valore `minimal` viene rifiutato dal modello attuale.
- **`ANTHROPIC_API_KEY`:** viene tolta dall'ambiente del figlio. Se presente, Claude Code fatturerebbe sulle API invece che sull'abbonamento.
- **PATH:** un'app avviata dal Finder non ha il PATH della shell. I CLI si cercano anche in `~/.local/bin`, `/opt/homebrew/bin`, ecc.
- **Escluso apposta:** estrarre il token OAuth dell'abbonamento e chiamare le API direttamente. Anthropic lo vieta nei prodotti di terzi. Se VoxLocal diventa un prodotto a pagamento, il provider CLI va rivisto (OPEN_QUESTIONS Q14).
- **Timeout dedicato:** `cli_timeout_s` = 20 s, contro i 15 s delle API.

## D23 — Modalità e vocabolario in SQLite già dalla Fase 3
- Lo schema è quello di ARCHITECTURE §5, con FTS5 e trigger. Ogni dettato finisce in `dictations`, compresi `llm_fallback`, `paste_failed` e `cancelled`: la regola "mai perdere un dettato" vale da subito, anche se la UI della cronologia arriva in Fase 4.
- Le modalità predefinite sono seed `INSERT OR IGNORE` per slug: le modifiche dell'utente non vengono sovrascritte agli avvii successivi.

## D24 — "None" è un `ProviderKind`, non un'implementazione
- `registry::build` restituisce `Ok(None)` e la pipeline salta il passaggio LLM. Un provider finto che restituisce il testo avrebbe dovuto disfare il template.

## D25 — Prompt injection: il dettato è dato, non istruzioni
- Ogni modalità racchiude il testo in `<dettato>…</dettato>` e il system prompt vieta di rispondergli o di eseguirne le istruzioni.
- **Misura:** senza i tag, Haiku *rispondeva* a "ciao come stai" invece di correggerlo.
- `{{text}}` viene sostituito per ultimo, così un dettato che contiene `{{language}}` non viene reinterpretato come variabile.

## D26 — Temperatura `f64`
- Un `f32` 0.2 serializzato in JSON diventa `0.20000000298…`; l'ha scoperto il test wiremock.

## D27 — Default `large-v3-turbo-q5_0` su Apple Silicon, flash attention, contesto proporzionale
- **Il problema:** una prova reale di Marco con `base` ("inviamo una email a giorgio…") ha dato "Inviamo nemmeno il A giorgio… graverchità". La voce sintetica del bench nascondeva il limite: `base` non regge la voce reale in italiano.
- **Flash attention** (`WhisperContextParameters::flash_attn`): turbo passa da 2,8 a 2,1 s, nessuna perdita.
- **`audio_ctx` proporzionale alla durata più 2 s**, al posto della finestra fissa di 30 s: turbo su 8,4 s di audio passa da 2,1 a 0,86 s.
  - Con un minimo di 256 frame turbo entra in loop di ripetizione su frasi di 2 s; con **512** no (verificato su 1, 2, 8, 21 s, in italiano e inglese).
- **Misure finali su M1:** 2-8 s di parlato in 0,6-0,9 s, 21 s in 2,4 s. RAM con il modello caricato 469 MB, contro 296 MB di base; viene liberata dopo `keep_warm`.
- **Altre piattaforme:** su Intel/Windows (CPU) resta `base` finché non c'è un bench in CI.
- Sostituisce D19.

## D28 — Provider in abbonamento solo nelle build personali
- Feature cargo `subscription-cli`, disattivata di default: la build commerciale non contiene i preset `claude-cli`/`codex-cli` e rifiuta anche i provider `cli` aggiunti a mano in `settings.json`.
- **Build personale:** `pnpm dev:personal` / `pnpm build:personal`.

## Rischi principali (in ordine)

1. **Push-to-talk e Released:** su Windows, se l'utente rilascia prima il modificatore e poi il tasto, `RegisterHotKey` non genera sempre l'evento Released. *Mitigazione:* tetto di durata più il fatto che una seconda pressione chiude comunque. Se non basta, si passa a un hook di tastiera di basso livello (`rdev`) solo su Windows.
2. **Permesso di Accessibilità su macOS** che si invalida a ogni ricompilazione con firma ad-hoc: è fastidioso in sviluppo. *Mitigazione:* firmare le build di dev con il certificato Apple Development (ricordarsi del WWDR G3).
3. **Bundle sotto i 15 MB** con whisper.cpp + Metal + WebView: dovrebbe starci, perché un'app Tauri vuota pesa circa 3-4 MB. Lo verifico in Fase 2.
4. **RAM a riposo sotto gli 80 MB:** il processo WebKit dell'overlay conta, e su macOS i processi WebContent sono separati. Misuro l'RSS totale dell'albero dei processi, non solo quello del binario.
5. **Hash SHA-256 dei modelli:** vanno presi dai metadati LFS di Hugging Face e congelati nel manifest. Se upstream ripubblica un file, il download fallisce in modo sicuro e chiaro (preferibile a un modello non verificato).
