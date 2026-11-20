# Domande aperte

Ogni domanda ha un **default**: se non arriva una risposta, procedo con quello.

| # | Domanda | Default proposto |
|---|---|---|
| Q1 | Scorciatoia di default | **⌥Space** su macOS (attenzione: è anche "spazio unificatore" in alcuni layout), **Ctrl+Alt+Space** su Windows e Linux. Push-to-talk come modalità predefinita. Configurabile nell'onboarding. |
| Q2 | Modello di default su un M1 da 8 GB | ✅ **Risolta in Fase 2:** `base`. Il bench la dà 3 volte più veloce di `small-q5_1` e non peggiore (DECISIONS D19). |
| Q3 | Retention della cronologia | 90 giorni |
| Q4 | Clipboard con contenuto non testuale (immagini, file) | Nell'MVP la ripristino solo se era testo. Se era altro, non la ripristino e lo segnalo nei log. Dopo: snapshot multi-formato tramite NSPasteboard/Win32. |
| Q5 | Bundle id | `app.voxlocal.desktop`: serve stabile per keyring, updater e permessi TCC. |
| Q6 | Mac Intel: Metal sì o no? | No, CPU + Accelerate. Si riapre se qualcuno con un Mac Intel lo misura. |
| Q7 | Salvare l'audio dei dettati | Disattivato (0 file). L'opzione esiste. |
| Q8 | Linux senza Secret Service | Errore esplicito, nessun salvataggio in chiaro. Le chiavi si possono passare per variabile d'ambiente (`VOXLOCAL_<PROVIDER>_KEY`), lette solo dal backend. |
| Q9 | Firma macOS e Windows | Il workflow è pronto (RELEASE.md). Mancano le decisioni commerciali: iscrizione all'Apple Developer Program (99 USD/anno) e il tipo di firma per Windows (consigliato Azure Trusted Signing). Finché mancano, le build escono non notarizzate o non firmate. |
| Q10 | Repository GitHub | **Da creare** (`gh` non è autenticato su questo Mac). Le chiavi dell'updater sono già generate. Le release devono essere pubbliche: codice privato più repository pubblico solo per le release, se serve (RELEASE.md). |
| Q11 | Lingua della UI | Per ora **solo italiano**, con i testi raccolti nei componenti. L'inglese serve prima della vendita fuori dall'Italia: estrarre le stringhe in un JSON per lingua, senza libreria. |
| Q12 | PATH della shell | Rust e cmake universal sono installati, ma `~/.zshrc` non è aggiornato (vedi sotto). Gli script di build impostano il PATH da soli. |
| Q13 | DMG con layout grafico | `bundle_dmg.sh` di Tauri pilota il Finder via AppleScript e qui va in timeout (manca il permesso Automazione). In locale creo il DMG con `hdiutil`; in CI (Fase 5) il problema non si pone. |
| Q14 | ✅ Decisa: esclusi dalla build commerciale (D28). Provider CLI in abbonamento in un prodotto distribuito | Per uso personale va bene. Se VoxLocal si vende, Anthropic richiede un'approvazione per offrire l'uso dell'abbonamento claude.ai; i termini di OpenAI/Codex sono da verificare. Default: il provider resta, documentato come "usa il CLI che hai installato tu". |
| Q15 | Provider LLM predefinito | `ollama` (locale, coerente con "locale di default"). La modalità attiva di default è però **Grezzo**, quindi nessun LLM finché l'utente non sceglie una modalità. |
| Q16 | Nomi dei modelli nei preset | Invecchiano (gpt-5-mini, llama-3.3…). Si cambiano in `settings.json` → `providers`, senza ricompilare. In Fase 4 l'elenco arriverà da `list_models`. |
| Q17 | Portachiavi e binari diversi | Una chiave salvata dal binario di dev (`target/debug/voxlocal`) fa comparire il prompt "consenti accesso" alla `.app` di release. Basta scegliere "Consenti sempre"; con la firma Developer ID (Fase 5) il problema si riduce. |
| Q18 | L'overlay "non focalizzabile" funziona davvero? | `focusable(false)` più click-through. Va verificato a mano che, con l'overlay visibile, ⌘V arrivi all'app di destinazione (TextEdit, Slack, browser). Piano B: NSPanel non attivante (DECISIONS D7). |
| Q19 | Microfono scelto e poi scollegato | Si ripiega sul microfono di sistema, con un avviso nel log. Dopo la vendita: notificarlo. |

## Da fare a mano (Marco)

Aggiungere a `~/.zshrc`:

```sh
. "$HOME/.cargo/env"
export PATH="$HOME/.local/opt/cmake-3.31.6-macos-universal/CMake.app/Contents/bin:$PATH"
```
