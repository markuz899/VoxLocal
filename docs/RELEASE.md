# Rilascio e firma del codice

Qui non ci sono segreti: solo dove vanno e come si ottengono.

## Come esce una versione

1. Aggiorna `version` in `package.json` (è l'unica fonte: `tauri.conf.json` la legge da lì).
2. Completa la checklist di rilascio in [TESTING.md](TESTING.md).
3. `git tag v0.2.0 && git push origin v0.2.0`
4. Il workflow **Release** costruisce:
   - macOS universal (Apple Silicon + Intel): `.dmg` per l'installazione e `.app.tar.gz` per gli aggiornamenti;
   - Windows: installer NSIS `.exe` (in `bundle.targets` c'è solo `nsis`, niente MSI);
   - Linux, best-effort: `.AppImage` e `.deb`;
   - `latest.json` con le firme dell'updater.
5. La release nasce **in bozza**. Prova gli installer dalla bozza, poi premi "Publish": da quel momento le app installate vedono l'aggiornamento.

## Segreti di GitHub (Settings → Secrets and variables → Actions)

| Segreto | Obbligatorio | Cos'è |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | sì | Contenuto di `~/.tauri/voxlocal-updater.key` sul Mac di Marco |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | sì | Nel Portachiavi: `security find-generic-password -a voxlocal -s voxlocal-updater-key-password -w` |
| `APPLE_CERTIFICATE` | per vendere su macOS | Certificato *Developer ID Application* esportato in `.p12`, in base64 su una riga (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | con il precedente | Password scelta all'esportazione del `.p12` |
| `APPLE_SIGNING_IDENTITY` | con il precedente | Es. `Developer ID Application: Nome Cognome (TEAMID)` |
| `APPLE_ID` | per la notarizzazione | Email dell'account Apple Developer |
| `APPLE_PASSWORD` | per la notarizzazione | *Password specifica per app* creata su account.apple.com, non quella dell'account |
| `APPLE_TEAM_ID` | per la notarizzazione | ID del team (10 caratteri) |

Senza i segreti Apple la build macOS esce firmata solo ad-hoc: funziona, ma chi la scarica deve aprirla con clic destro → Apri (vedi [INSTALL.md](INSTALL.md)).

**La chiave dell'updater non va mai persa né cambiata.** Le app già installate accettano solo aggiornamenti firmati con quella chiave. Se la perdi, gli utenti dovranno reinstallare a mano. Fanne una copia di sicurezza (gestore di password, chiavetta cifrata).

## macOS: Developer ID e notarizzazione

1. Iscriviti all'**Apple Developer Program** (99 USD/anno). Il certificato *Apple Development* che hai già serve solo per lo sviluppo.
2. Xcode → Settings → Accounts → Manage Certificates → **+** → *Developer ID Application*.
3. In Accesso Portachiavi esporta certificato e chiave privata come `.p12`, con una password.
4. Crea una password specifica per app su account.apple.com → Accesso e sicurezza.
5. Carica i segreti della tabella. Il workflow firma, invia ad Apple per la notarizzazione (qualche minuto) e "graffa" il ticket al `.dmg`.

L'overlay usa l'API privata per la trasparenza (`macOSPrivateApi`, DECISIONS D30), quindi **niente Mac App Store**. La distribuzione con Developer ID non ha questo limite.

## Windows: firma del codice

Un installer non firmato funziona, ma SmartScreen mostra "Windows ha protetto il PC" e molti utenti si fermano lì. Le opzioni, dalla più semplice:

| Opzione | Costo indicativo | Note |
|---|---|---|
| **Azure Trusted Signing** | ~10 USD/mese | Il più pratico per chi vende software. Richiede un'azienda o una persona verificabile; in Tauri si usa `bundle.windows.signCommand` con `trusted-signing-cli` |
| Certificato OV | 200–400 USD/anno | Il certificato è su token hardware o HSM cloud: va integrato con lo strumento del fornitore |
| Certificato EV | 300–600 USD/anno | Reputazione SmartScreen immediata, ma stessi vincoli hardware |

Quando scegli l'opzione, si aggiungono a `tauri.conf.json` la configurazione `bundle.windows` e i relativi segreti nel workflow. Finché non lo fai, l'installer esce non firmato.

## L'updater e il repository

L'endpoint in `tauri.conf.json` contiene il segnaposto `GITHUB_REPOSITORY`; il workflow lo sostituisce con il repository che fa la build. Due conseguenze:

- **Gli asset delle release devono essere pubblici.** Con un repository privato l'updater non riesce a scaricarli. Se vuoi tenere privato il codice, crea un repository pubblico vuoto solo per le release (es. `voxlocal-releases`) e pubblica lì gli asset, usando gli input `owner`/`repo` di `tauri-action` e un token con permesso di scrittura su quel repository.
- Le build fatte in locale hanno ancora il segnaposto: il controllo degli aggiornamenti fallisce, scrive un avviso nel log e non disturba l'utente.

## Build personale

La build con i provider in abbonamento (Claude Code, Codex) non va mai pubblicata (DECISIONS D28). Si costruisce solo in locale con `pnpm build:personal`.
