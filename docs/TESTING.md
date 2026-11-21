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

## Cronologia, app e modalità per app

- [ ] Detta in Mail e in Slack: nella cronologia ogni riga mostra la sua app. Su Windows il nome è quello dell'eseguibile.
- [ ] Filtri: Oggi, Settimana e Tutto mostrano i conteggi; i filtri per modalità e per app restringono l'elenco.
- [ ] Un dettato elaborato dall'AI: le parti cambiate sono sottolineate e il riquadro "Detto" mostra la frase originale.
- [ ] Mentre detti, con la cronologia aperta, compare in cima la riga "In ascolto", poi "Trascrivo" e "Riscrivo"; a dettato finito la lista si aggiorna.
- [ ] Modalità → Modalità per app: con la regola "Slack usa Messaggio", un dettato in Slack usa Messaggio anche se la modalità attiva è un'altra.
- [ ] `pnpm test` (confronto fra testo detto e scritto) e `cargo test --lib` (filtri, conteggi, regole).
- [ ] Rilevamento dell'app su un desktop vero: `cargo test --lib frontmost -- --ignored --nocapture`.

## Lingua, comando sulla selezione, libreria, traduzione

- [ ] Generale → Lingua dell'interfaccia: passando da English a Italiano cambiano subito finestra, menu del tray e overlay del dettato successivo.
- [ ] **Comando sulla selezione:** seleziona una frase in TextEdit, Mail, Slack e Chrome, tieni ⌥⇧Space e di' "rendilo più formale". La selezione viene sostituita e dopo un ⌘V torna quello che c'era negli appunti.
- [ ] Comando senza selezione: arriva la notifica "Select some text first" e gli appunti restano come prima.
- [ ] Comando con Ollama spento: notifica, selezione intatta, cronologia con stato Errore.
- [ ] Nella cronologia un comando mostra "Command: …" e le parti cambiate rispetto alla selezione.
- [ ] Modalità → Aggiungi dalla libreria: la modalità aggiunta compare nell'elenco e nel tray; il pulsante diventa "Added".
- [ ] Generale → Traduci in: Tedesco. Con la modalità Translate, un dettato in italiano esce in tedesco.

## Statistiche, snippet, correzioni, voce bassa, vocabolario

- [ ] **Statistiche:** dopo qualche dettato i numeri tornano (parole, tempo risparmiato, velocità, dettati), il grafico mostra oggi a destra e il passaggio del mouse dà giorno e parole.
- [ ] **Snippet:** crea "my address" → "Via Roma, Milano"; dettando solo "my address" viene incollato l'indirizzo, mentre in "send my address to Luca" non succede.
- [ ] **Modalità Pulito:**
  - "la riunione è alle 3, anzi alle 4" diventa "La riunione è alle 4.";
  - "primo punto latte a capo secondo punto pane" va a capo invece di scrivere "a capo".
- [ ] **Voce sussurrata** a 30 cm dal microfono: la trascrizione è corretta e nel log compare "voce bassa: guadagno ×…". Il silenzio puro continua a essere scartato.
- [ ] **Vocabolario:** i suggerimenti compaiono dopo che un nome torna in almeno due dettati; "×" lo nasconde per sempre, "+" lo aggiunge. Dalla Cronologia, selezionando una parola, "Add the selection to the vocabulary" la aggiunge.

## Trascrizione di file e backup

- [ ] **Trascrivi un file:** trascina un m4a o un mp4 di qualche minuto. L'avanzamento sale fino a 100% e il testo esce con gli orari. Cambiando scheda e tornando, l'avanzamento o il risultato sono ancora lì.
- [ ] **Annulla** durante la trascrizione: compare "Trascrizione annullata" e se ne può avviare subito un'altra.
- [ ] **Azioni sul risultato:** "Copia" e "Salva come testo" funzionano, con o senza orari. "Riassumi" con un provider LLM configurato dà riassunto, punti chiave e cose da fare; senza provider compare un messaggio che spiega cosa configurare.
- [ ] **File .opus** (vocale WhatsApp): viene rifiutato con un messaggio chiaro.
- [ ] **Backup:** "Esporta" in Generale crea un JSON. "Importa" su un profilo pulito ripristina vocabolario, snippet, modalità e regole per app; reimportare non crea doppioni.
- [ ] **Test lungo automatico:** `VOXLOCAL_LONG_FILE=… VOXLOCAL_MODEL=… cargo test --release --lib long_file -- --ignored --nocapture`.

## Note riunioni

- [ ] **Primo avvio:** "Avvia le note della riunione" chiede il permesso Registrazione schermo. Senza permesso registra solo il microfono e lo dice; dopo averlo concesso (e riaperto l'app) registra anche le app.
- [ ] **Chiamata con le cuffie** (Meet o Zoom, 5 minuti): le righe sono attribuite a "Io" e "Altri" in ordine giusto. "Riassumi" dà decisioni e cose da fare.
- [ ] **Stessa chiamata con le casse:** le frasi degli altri non compaiono doppie come "Io".
- [ ] **Dalla barra dei menu:** avvio e stop funzionano; a fine registrazione si apre la pagina Note riunioni con l'avanzamento della trascrizione.
- [ ] **App chiusa durante la trascrizione:** alla riapertura la riunione risulta "Trascrizione non riuscita", non "in corso" per sempre.
- [ ] **Elimina:** toglie la riunione e la sua cartella audio in `meetings/`.
- [ ] **Test automatico:** `VOXLOCAL_MODEL=… cargo test --release --lib meeting_end_to_end -- --ignored --nocapture`.

## Avvio automatico e calendario

- [ ] **Avvio della chiamata:** apri una chiamata di prova in Meet ("Verifica audio e video") o FaceTime. Entro circa 7 s compare in alto a destra "Chiamata in …"; "Registra le note" avvia la registrazione.
- [ ] **"Non ora":** non ripropone finché il microfono resta acceso; alla chiamata successiva propone di nuovo.
- [ ] **Durante un dettato:** il microfono di VoxLocal non fa partire la proposta.
- [ ] **Fine chiamata:** chiudi la chiamata lasciando la registrazione attiva. Dopo circa 2 minuti compare "La chiamata sembra finita"; "Ferma e trascrivi" ferma e apre Note riunioni.
- [ ] **Calendario:**
  - attivando "Nome e partecipanti dal calendario" macOS chiede l'accesso;
  - con un evento in corso, la proposta mostra il titolo e la riunione salvata ha titolo e partecipanti;
  - negando l'accesso, la pagina dice dove riattivarlo.

## Chiedi alle tue riunioni

- [ ] **Domanda con parole chiave** ("Cosa abbiamo deciso sul rilascio?"): la risposta cita [n]; cliccando un numero si vede la frase, "Apri la riunione" apre quella riunione.
- [ ] **Domanda senza parole chiave** ("Cosa devo fare questa settimana?"): risponde dalle riunioni recenti.
- [ ] **Domanda su qualcosa mai detto:** dice che non ha trovato nulla, senza inventare.
- [ ] **Avviso privacy:** con un provider cloud compare l'avviso arancione; con Ollama "restano su questo computer".
- [ ] **Riunione eliminata:** non compare più tra le fonti.

## Comando scritto nella barra

- [ ] **Tocco:** seleziona un testo in Note, tocca ⌥⇧ Spazio senza parlare: si apre la barra con il numero di parole.
- [ ] **Scritto:** scrivi "traduci in inglese" e premi Invio: la selezione in Note viene sostituita.
- [ ] **Recente:** riapri la barra: il comando appena usato è in cima; frecce più Invio lo applicano.
- [ ] **Esc e clic fuori:** la barra si chiude, la selezione resta, gli appunti tornano quelli di prima.
- [ ] **Voce:** tenendo premuto e parlando il comando funziona come prima, senza barra.
- [ ] **Senza selezione:** un tocco senza testo selezionato mostra l'avviso, nessuna barra.

## Cose da fare dalla riunione

- [ ] **Riassumi** una riunione in cui qualcuno promette qualcosa ("preparo le note entro giovedì"): compare "Cose da fare" con chi, cosa e scadenza.
- [ ] **Spunta e togli** una voce, cambia pagina e torna: la modifica è rimasta.
- [ ] **Aggiungi a Promemoria:** la prima volta macOS chiede il permesso; le voci aperte compaiono in Promemoria con la data e la riunione nelle note.
- [ ] **Copia elenco** incolla una lista Markdown a caselle.
- [ ] **Riunione senza impegni:** nessuna sezione "Cose da fare".

## Checklist di rilascio

Prima di pubblicare la bozza creata dal workflow Release (vedi [RELEASE.md](RELEASE.md)):

**Build**
- [ ] CI verde su macOS, Windows e Linux.
- [ ] `version` aggiornata in `package.json`; il tag è `v` + la stessa versione.
- [ ] La bozza contiene `.dmg` universal, `.app.tar.gz` con `.sig`, `-setup.exe` con `.sig`, `.AppImage`/`.deb` e `latest.json`.
- [ ] `latest.json` punta agli URL di questa release e ha una `signature` per ogni piattaforma.
- [ ] Nessun provider `claude-cli`/`codex-cli` nella build pubblicata (è commerciale, D28).

**macOS** (su un Mac pulito o un altro account utente)
- [ ] Il `.dmg` si apre senza avvisi di Gatekeeper (`spctl -a -vv /Applications/VoxLocal.app` → "accepted, source=Notarized Developer ID"). Senza segreti Apple: niente notarizzazione, va bene solo per beta privata.
- [ ] Onboarding completo: permessi, download del modello, dettato di prova.
- [ ] Dettato in TextEdit, Mail, Slack e Chrome: il testo arriva lì, non nell'overlay.
- [ ] La parte Intel parte: `arch -x86_64 /Applications/VoxLocal.app/Contents/MacOS/voxlocal help`.

**Windows** (10 e 11)
- [ ] L'installer parte; si sa quale avviso SmartScreen compare (non firmato) o nessuno (firmato).
- [ ] Icona nell'area di notifica; onboarding; dettato in Blocco note, Word e Chrome.
- [ ] Push-to-talk: rilasciando prima il modificatore e poi Spazio la registrazione si ferma comunque (rischio 1 in DECISIONS).
- [ ] Chiave API salvata in Gestione credenziali e riletta dopo un riavvio.

**Aggiornamento**
- [ ] Installa la versione precedente, pubblica la nuova: entro un controllo compare la notifica; "Installa e riavvia" porta alla nuova versione e conserva impostazioni, modelli e cronologia.

**Prestazioni** (confronto con la tabella "Metriche misurate")
- [ ] `cargo run --release --example bench -- --model large-v3-turbo-q5_0`
- [ ] RAM a riposo e avvio a freddo misurati sulla build pubblicata.
