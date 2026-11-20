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
| Q9 | Firma macOS | Serve un **Developer ID Application** (non Apple Development) per notarizzare. Lo gestiamo in Fase 5; in dev firmo con Apple Development. |
| Q10 | Endpoint e chiavi dell'updater | Release su GitHub (`latest.json`) e coppia di chiavi minisign generata in Fase 5. Repo GitHub da definire. |
| Q11 | Lingua della UI | Italiano e inglese, con i18n minimale (un JSON per lingua, niente libreria). |
| Q12 | PATH della shell | Rust e cmake universal sono installati, ma `~/.zshrc` non è aggiornato (vedi sotto). Gli script di build impostano il PATH da soli. |
| Q13 | DMG con layout grafico | `bundle_dmg.sh` di Tauri pilota il Finder via AppleScript e qui va in timeout (manca il permesso Automazione). In locale creo il DMG con `hdiutil`; in CI (Fase 5) il problema non si pone. |

## Da fare a mano (Marco)

Aggiungere a `~/.zshrc`:

```sh
. "$HOME/.cargo/env"
export PATH="$HOME/.local/opt/cmake-3.31.6-macos-universal/CMake.app/Contents/bin:$PATH"
```
