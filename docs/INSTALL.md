# Installare VoxLocal

VoxLocal è un'app **desktop**: macOS e Windows sono supportati, Linux in parte. Non esistono versioni per iPhone, iPad o Android. I file si scaricano dalla pagina delle release su GitHub.

Requisiti: macOS 11 o successivo (Apple Silicon o Intel), oppure Windows 10/11 a 64 bit. Servono circa 600 MB liberi per il modello di trascrizione consigliato.

## macOS

1. Scarica `VoxLocal_x.y.z_universal.dmg`. Lo stesso file va bene per i Mac Apple Silicon e Intel.
2. Aprilo e trascina **VoxLocal** nella cartella **Applicazioni**.
3. Apri VoxLocal da Applicazioni o da Spotlight. Compare l'icona del microfono nella barra dei menu e si apre la guida iniziale:
   - **Microfono:** macOS chiede il permesso, rispondi "Consenti";
   - **Accessibilità:** serve per incollare il testo. Impostazioni di Sistema → Privacy e sicurezza → Accessibilità → attiva VoxLocal;
   - **Modello:** il download parte da qui, una volta sola;
   - **Prova:** detta una frase nel riquadro.

Se compare **"VoxLocal non può essere aperto perché proviene da uno sviluppatore non identificato"**, la versione non è ancora firmata e notarizzata da Apple. Fai clic destro sull'app → **Apri** → **Apri**. Serve solo la prima volta. Su macOS 15 il percorso è Impostazioni di Sistema → Privacy e sicurezza → "Apri comunque".

## Windows

1. Scarica `VoxLocal_x.y.z_x64-setup.exe` e aprilo. L'installazione è per l'utente corrente e non chiede la password di amministratore.
2. Se compare **"Windows ha protetto il PC"** (SmartScreen, finché l'installer non è firmato): **Ulteriori informazioni** → **Esegui comunque**.
3. VoxLocal parte nell'area di notifica (in basso a destra, a volte sotto la freccia ^) e apre la guida iniziale.
4. Se non sente il microfono: Impostazioni → Privacy e sicurezza → Microfono → attiva "Consenti alle app desktop di accedere al microfono".

Limite noto: Windows non permette di incollare nelle finestre aperte come amministratore. In quel caso il testo resta negli appunti e basta premere Ctrl+V.

## Linux (in parte supportato)

- `.AppImage`: `chmod +x VoxLocal_*.AppImage` e poi aprilo.
- `.deb` (Ubuntu, Debian): `sudo apt install ./VoxLocal_*.deb`.

Su **X11** funziona tutto. Su **Wayland** la scorciatoia globale e l'incolla automatico possono non funzionare: il testo resta negli appunti. Per le chiavi API serve un portachiavi attivo (GNOME Keyring o KWallet).

## Aggiornamenti

VoxLocal cerca gli aggiornamenti all'avvio e ogni 6 ore. Quando ne trova uno, manda una notifica; dal menu nella barra (o da Impostazioni → Informazioni) scegli **Installa e riavvia**. Gli aggiornamenti sono firmati: un file alterato viene rifiutato.

## Disinstallare

- **macOS:** trascina VoxLocal da Applicazioni al Cestino. Per togliere anche modelli, cronologia e impostazioni, cancella `~/Library/Application Support/app.voxlocal.desktop` e `~/Library/Logs/app.voxlocal.desktop`.
- **Windows:** Impostazioni → App → VoxLocal → Disinstalla. I dati restano in `%APPDATA%\app.voxlocal.desktop`.
- **Chiavi API:** si tolgono da Impostazioni → Provider LLM → Rimuovi chiave, prima di disinstallare. Altrimenti restano in Portachiavi (macOS) o Gestione credenziali (Windows) sotto `app.voxlocal.desktop`.
