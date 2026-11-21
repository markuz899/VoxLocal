// Testi dell'interfaccia: inglese (default) e italiano. Una chiave per testo; {nome} viene sostituito.
// ponytail: due dizionari in un file; un JSON per lingua quando ne arriva una terza
import { useSettings } from "./App";

const en = {
  // shell
  "nav.general": "General",
  "nav.transcription": "Transcription",
  "nav.modes": "Modes",
  "nav.providers": "LLM providers",
  "nav.vocabulary": "Vocabulary",
  "nav.history": "History",
  "nav.faq": "FAQ",
  "nav.about": "About",
  "nav.sections": "Sections",
  "app.loadError": "Could not read the settings: {error}",

  // common
  "common.save": "Save",
  "common.saved": "Saved.",
  "common.cancel": "Cancel",
  "common.delete": "Delete",
  "common.remove": "Remove",
  "common.close": "Close",
  "common.continue": "Continue",
  "common.back": "Back",
  "common.stop": "Stop",
  "common.test": "Test",
  "common.download": "Download",
  "common.use": "Use",
  "common.inUse": "In use",
  "common.ready": "Ready",
  "common.recommended": "recommended",
  "common.systemMic": "System microphone",
  "common.pressKeys": "Press the keys…",
  "common.checking": "Checking…",

  // general
  "general.title": "General",
  "general.dictation": "Dictation",
  "general.uiLanguage": "Interface language",
  "general.shortcut": "Shortcut",
  "general.shortcutHint": "Works in any app. Esc cancels.",
  "general.commandShortcut": "Selection command shortcut",
  "general.commandShortcutHint":
    "Select text, hold it and say what to do: “make it more formal”.",
  "general.activation": "How it starts",
  "general.ptt": "Hold to talk",
  "general.toggle": "Press to start and stop",
  "general.language": "Dictation language",
  "general.languageHint":
    "Fixing it makes transcription more accurate and faster.",
  "general.translateTo": "Translate into",
  "general.translateToHint":
    "Used by the Translate mode and by modes with {{target_language}}.",
  "general.microphone": "Microphone",
  "general.device": "Device",
  "general.testMic": "Test the microphone",
  "general.system": "System",
  "general.autostart": "Open at login",
  "general.restoreClipboard": "Restore the clipboard after pasting",
  "general.restoreClipboardHint":
    "Puts back what was in the clipboard before the dictation.",
  "general.retention": "Keep the history for",
  "general.days": "{n} days",
  "general.year": "1 year",
  "general.forever": "Forever",

  // transcription
  "stt.title": "Transcription",
  "stt.description":
    "The local model works offline and your voice never leaves the computer. Cloud services are lighter on the computer but send the audio to the provider.",
  "stt.engine": "Engine",
  "stt.local": "On this computer",
  "stt.cloud": "{name} (cloud)",
  "stt.missingKey": "The {name} key is missing",
  "stt.missingKeyHint": "Until it is added, VoxLocal uses the local model.",
  "stt.addKey": "Add the key",
  "stt.keepWarm": "Keep the model in memory",
  "stt.keepWarmHint":
    "The longer it stays loaded, the faster the next dictation.",
  "stt.releaseNow": "Release right away",
  "stt.minutes": "{n} minutes",
  "stt.always": "Always",
  "stt.models": "Local models",
  "stt.downloading": "Downloading… {pct}%",
  "model.tiny": "The fastest, often wrong",
  "model.tiny-q5_1": "The lightest, often wrong",
  "model.base": "Fast, inaccurate with real voices",
  "model.base-q5_1": "Fast and light, inaccurate",
  "model.small": "Balanced",
  "model.small-q5_1": "Balanced and light",
  "model.medium-q5_0": "Accurate, slower",
  "model.large-v3-turbo-q5_0": "The most accurate at dictation speed",
  "model.large-v3-turbo": "The most accurate, needs more memory",

  // modes
  "modes.title": "Modes",
  "modes.description":
    "Each mode decides what happens to the text after transcription. You can also pick it from the menu bar.",
  "modes.new": "New mode",
  "modes.newName": "New mode",
  "modes.fromLibrary": "Add from the library",
  "modes.name": "Name",
  "modes.processing": "Processing",
  "modes.default": "Default: {name}",
  "modes.none": "None: paste the transcription",
  "modes.model": "Model",
  "modes.modelPlaceholder": "the provider's",
  "modes.instructions":
    "Instructions for the LLM. Variables: {{language}}, {{vocabulary}}, {{target_language}}",
  "modes.message": "Message sent. {{text}} is the dictated text.",
  "modes.useThis": "Use this mode",
  "modes.isActive": "{name} is the active mode.",
  "modes.reset": "Restore the original",
  "modes.restored": "Restored.",
  "modes.deleted": "Mode deleted.",
  "modes.created": "Mode created.",
  "modes.active": "Active",
  "modes.translateTo": "Translates into",
  "library.title": "Mode library",
  "library.description": "Ready-made modes: add one and edit it as you like.",
  "library.add": "Add",
  "library.added": "Added",
  "rules.title": "Modes per app",
  "rules.description":
    "When you dictate in an app on this list, VoxLocal uses its mode instead of the active one. Part of the name is enough: “Mail” also matches “Microsoft Outlook Mail”.",
  "rules.empty": "No rules: every dictation uses the active mode.",
  "rules.app": "App",
  "rules.appPlaceholder": "App name",
  "rules.uses": "uses",
  "rules.add": "Add a rule",

  // providers
  "providers.title": "LLM providers",
  "providers.description":
    "The provider that rewrites the text in modes like Clean and Email, and runs selection commands. Keys stay in the system keychain.",
  "providers.default": "Default provider",
  "providers.defaultHint":
    "Used by modes that do not name one, and by selection commands.",
  "providers.list": "Providers",
  "providers.configure": "Configure",
  "providers.cli": "Subscription via CLI",
  "providers.local": "Local, no key needed",
  "providers.keySaved": "Key saved",
  "providers.keyMissing": "Key missing",
  "providers.apiUrl": "API address",
  "providers.cliModel": "the CLI's default",
  "providers.replaceKey": "Replace the API key",
  "providers.apiKey": "API key",
  "providers.keyStored": "stored in the keychain",
  "providers.pasteKey": "paste the key",
  "providers.verify": "Check",
  "providers.reachable": "Reachable in {ms}.",
  "providers.removeKey": "Remove the key",
  "providers.keyRemoved": "Key removed.",

  // vocabulary
  "vocab.title": "Vocabulary",
  "vocab.description":
    "Names, acronyms and technical terms written the way you want. They help transcription and are passed to the LLM.",
  "vocab.placeholder": "E.g. Kubernetes, Jane Doe, ACME Inc.",
  "vocab.add": "Add",
  "vocab.empty":
    "No terms yet. Add the words transcription gets wrong most often.",
  "vocab.removeTerm": "Remove {term}",

  // history
  "history.title": "History",
  "history.clear": "Clear the history",
  "history.clearConfirm": "Delete {n} dictations",
  "history.period": "Period",
  "history.today": "Today",
  "history.week": "Week",
  "history.all": "All",
  "history.allModes": "All modes",
  "history.allApps": "All apps",
  "history.search": "Search dictations",
  "history.list": "Dictations",
  "history.now": "Now",
  "history.inProgress": "Dictation in progress",
  "history.noResults": "No dictations match these filters.",
  "history.empty": "Your dictations will appear here.",
  "history.unknownApp": "Unknown app",
  "history.more": "Show more",
  "history.pick": "Pick a dictation to see its details.",
  "history.deleteRow": "Delete the dictation",
  "history.deleteRowConfirm": "Confirm: delete the dictation",
  "history.clickAgain": "Click again to delete",
  "history.command": "Command",
  "history.said": "Said",
  "history.selectionChanged": "Original selection",
  "history.hoverHint": "Hover the underlined parts to see what you said.",
  "history.added": "Added by the rewrite: you did not say it.",
  "history.copyText": "Copy the text",
  "history.copySaid": "Copy what was said",
  "history.showSaid": "Show what was said",
  "history.showWritten": "Show the written text",
  "history.copied": "{what} copied to the clipboard.",
  "history.textWord": "Text",
  "history.saidWord": "Said text",
  "meta.app": "App",
  "meta.mode": "Mode",
  "meta.language": "Language",
  "meta.microphone": "Microphone",
  "meta.transcription": "Transcription",
  "meta.processing": "Processing",
  "meta.none": "None",
  "meta.local": "Local",
  "time.audio": "Audio",
  "time.transcription": "Transcription",
  "time.processing": "Processing",
  "time.paste": "Paste",
  "time.total": "Total",
  "status.ok": "Pasted",
  "status.llm_fallback": "Not rewritten",
  "status.cancelled": "Cancelled",
  "status.paste_failed": "In the clipboard",
  "status.error": "Error",
  "live.recording": "Listening",
  "live.transcribing": "Transcribing",
  "live.postprocessing": "Rewriting",
  "live.pasting": "Pasting",

  // about
  "about.title": "About",
  "about.version": "Version",
  "about.personal": " (personal build)",
  "about.data": "Data",
  "about.log": "Logs",
  "about.updates": "Updates",
  "about.updateAvailable": "Version {v} is available",
  "about.autoUpdates": "Check for updates automatically",
  "about.restartHint": "VoxLocal restarts after installing.",
  "about.updatesHint": "At launch and every 6 hours. Updates are signed.",
  "about.install": "Install and restart",
  "about.installing": "Downloading and installing…",
  "about.checkNow": "Check now",
  "about.found": "Found version {v}.",
  "about.upToDate": "You have the latest version.",
  "about.privacy": "Privacy",
  "about.noTelemetry": "No telemetry",
  "about.noTelemetryHint":
    "VoxLocal sends no statistics or usage data. Audio and text leave the computer only if you choose a cloud service for transcription or rewriting.",
  "about.reopenGuide": "To go through the first setup again, {link}.",
  "about.reopenGuideLink": "reopen the guide",

  // onboarding
  "onb.step": "Step {n} of {total}",
  "onb.mic.title": "I only listen when you want",
  "onb.mic.lead":
    "VoxLocal uses the microphone only while you hold the shortcut. Your voice is transcribed on this computer and is not saved.",
  "onb.mic.denied": "Microphone access was denied.",
  "onb.mic.openSettings": "Open System Settings",
  "onb.mic.granted": "I granted it",
  "onb.mic.speak": "Speak: the bars should move.",
  "onb.mic.allow": "Allow the microphone",
  "onb.ax.title": "Typing for you",
  "onb.ax.lead":
    "To paste the text into the app you are using, macOS asks for the Accessibility permission. VoxLocal only uses it to press ⌘V after a dictation.",
  "onb.ax.granted": "Permission granted.",
  "onb.ax.step1": "Press the button below.",
  "onb.ax.step2":
    "In System Settings, turn on VoxLocal in the Accessibility list.",
  "onb.ax.step3": "Come back here: this page updates by itself.",
  "onb.ax.ask": "Ask for permission",
  "onb.ax.skip":
    "You can skip this step: the dictated text will stay in the clipboard and you paste it yourself.",
  "onb.model.title": "The transcription engine",
  "onb.model.lead":
    "Transcription happens on this computer, even without internet. The model is downloaded once.",
  "onb.model.downloading": "Downloading… {pct}%. You can go on meanwhile.",
  "onb.try.title": "Try a dictation",
  "onb.try.ptt": "Click in the box, hold {key}, speak and release.",
  "onb.try.toggle":
    "Click in the box, press {key}, speak and press {key} again.",
  "onb.try.placeholder": "The dictated text will appear here.",
  "onb.try.works": "It works. The same happens in any app.",
  "onb.try.atLogin": "Open VoxLocal at login",
  "onb.finish": "Start using VoxLocal",
  // stats
  "nav.stats": "Statistics",
  "stats.title": "Statistics",
  "stats.description":
    "What dictation has done for you, measured on your own dictations.",
  "stats.period": "Period",
  "stats.week": "Last 7 days",
  "stats.month": "Last 30 days",
  "stats.all": "All time",
  "stats.words": "Words dictated",
  "stats.saved": "Time saved",
  "stats.savedHint": "Compared with typing at {wpm} words per minute.",
  "stats.speed": "Speaking speed",
  "stats.wpm": "{n} wpm",
  "stats.faster": "{x}× faster than typing",
  "stats.dictations": "Dictations",
  "stats.perDay": "Words per day",
  "stats.byApp": "Words by app",
  "stats.empty": "Dictate something and your numbers will appear here.",
  "stats.dayWords": "{day}: {n} words",
  "stats.tableDay": "Day",
  "stats.tableWords": "Words",
  "stats.hours": "{h} h {m} min",
  "stats.minutes": "{m} min",
  // snippets
  "nav.snippets": "Snippets",
  "snippets.title": "Snippets",
  "snippets.description":
    "Say a short phrase and VoxLocal pastes the saved text instead: your address, a calendar link, a signature. The whole dictation must be the phrase.",
  "snippets.trigger": "When I say",
  "snippets.triggerPlaceholder": "my address",
  "snippets.text": "Paste",
  "snippets.textPlaceholder": "The text to paste",
  "snippets.add": "Save snippet",
  "snippets.empty": "No snippets yet.",
  "snippets.edit": "Edit",
  // vocabulary suggestions
  "vocab.suggestions": "Suggestions",
  "vocab.suggestionsHint":
    "Names and terms that come up often in your dictations. Add the ones written right.",
  "vocab.noSuggestions": "No suggestions for now: they appear as you dictate.",
  "vocab.addSuggestion": "Add {term}",
  "vocab.dismiss": "Ignore {term}",
  "vocab.times": "{n}×",
  "history.addToVocab": "Add the selection to the vocabulary",
  "history.addedToVocab": "“{term}” added to the vocabulary.",
  "history.selectFirst": "Select a word in the text first.",
};

type Key = keyof typeof en;

const it: Record<Key, string> = {
  "nav.general": "Generale",
  "nav.transcription": "Trascrizione",
  "nav.modes": "Modalità",
  "nav.providers": "Provider LLM",
  "nav.vocabulary": "Vocabolario",
  "nav.history": "Cronologia",
  "nav.faq": "FAQ",
  "nav.about": "Informazioni",
  "nav.sections": "Sezioni",
  "app.loadError": "Impossibile leggere le impostazioni: {error}",

  "common.save": "Salva",
  "common.saved": "Salvato.",
  "common.cancel": "Annulla",
  "common.delete": "Elimina",
  "common.remove": "Rimuovi",
  "common.close": "Chiudi",
  "common.continue": "Continua",
  "common.back": "Indietro",
  "common.stop": "Ferma",
  "common.test": "Prova",
  "common.download": "Scarica",
  "common.use": "Usa",
  "common.inUse": "In uso",
  "common.ready": "Pronto",
  "common.recommended": "consigliato",
  "common.systemMic": "Microfono di sistema",
  "common.pressKeys": "Premi i tasti…",
  "common.checking": "Controllo…",

  "general.title": "Generale",
  "general.dictation": "Dettatura",
  "general.uiLanguage": "Lingua dell'interfaccia",
  "general.shortcut": "Scorciatoia",
  "general.shortcutHint": "Funziona in qualsiasi app. Esc annulla.",
  "general.commandShortcut": "Scorciatoia del comando sulla selezione",
  "general.commandShortcutHint":
    "Selezioni un testo, la tieni premuta e dici cosa farne: “rendilo più formale”.",
  "general.activation": "Come si attiva",
  "general.ptt": "Tieni premuto per parlare",
  "general.toggle": "Premi per avviare e fermare",
  "general.language": "Lingua del dettato",
  "general.languageHint":
    "Fissarla rende la trascrizione più precisa e veloce.",
  "general.translateTo": "Traduci in",
  "general.translateToHint":
    "Usata dalla modalità Traduci e dalle modalità con {{target_language}}.",
  "general.microphone": "Microfono",
  "general.device": "Dispositivo",
  "general.testMic": "Prova il microfono",
  "general.system": "Sistema",
  "general.autostart": "Apri all'accesso",
  "general.restoreClipboard": "Ripristina gli appunti dopo l'incolla",
  "general.restoreClipboardHint":
    "Rimette negli appunti quello che c'era prima del dettato.",
  "general.retention": "Conserva la cronologia per",
  "general.days": "{n} giorni",
  "general.year": "1 anno",
  "general.forever": "Per sempre",

  "stt.title": "Trascrizione",
  "stt.description":
    "Il modello locale funziona offline e la voce non lascia il computer. I servizi cloud sono più leggeri per il computer ma inviano l'audio al provider.",
  "stt.engine": "Motore",
  "stt.local": "Locale sul computer",
  "stt.cloud": "{name} (cloud)",
  "stt.missingKey": "Manca la chiave di {name}",
  "stt.missingKeyHint": "Finché manca, VoxLocal usa il modello locale.",
  "stt.addKey": "Aggiungi la chiave",
  "stt.keepWarm": "Tieni il modello in memoria",
  "stt.keepWarmHint":
    "Più a lungo resta caricato, più è veloce il dettato successivo.",
  "stt.releaseNow": "Rilascia subito",
  "stt.minutes": "{n} minuti",
  "stt.always": "Sempre",
  "stt.models": "Modelli locali",
  "stt.downloading": "Scarico… {pct}%",
  "model.tiny": "Il più veloce, sbaglia spesso",
  "model.tiny-q5_1": "Il più leggero, sbaglia spesso",
  "model.base": "Veloce, impreciso con la voce reale",
  "model.base-q5_1": "Veloce e leggero, impreciso",
  "model.small": "Equilibrato",
  "model.small-q5_1": "Equilibrato e leggero",
  "model.medium-q5_0": "Preciso, più lento",
  "model.large-v3-turbo-q5_0": "Il più preciso a una velocità da dettatura",
  "model.large-v3-turbo": "Il più preciso, richiede più memoria",

  "modes.title": "Modalità",
  "modes.description":
    "Ogni modalità decide cosa succede al testo dopo la trascrizione. Si sceglie anche dal menu nella barra.",
  "modes.new": "Nuova modalità",
  "modes.newName": "Nuova modalità",
  "modes.fromLibrary": "Aggiungi dalla libreria",
  "modes.name": "Nome",
  "modes.processing": "Elaborazione",
  "modes.default": "Predefinito: {name}",
  "modes.none": "Nessuna: incolla il testo trascritto",
  "modes.model": "Modello",
  "modes.modelPlaceholder": "quello del provider",
  "modes.instructions":
    "Istruzioni per l'LLM. Variabili: {{language}}, {{vocabulary}}, {{target_language}}",
  "modes.message": "Messaggio inviato. {{text}} è il testo dettato.",
  "modes.useThis": "Usa questa modalità",
  "modes.isActive": "{name} è la modalità attiva.",
  "modes.reset": "Ripristina originale",
  "modes.restored": "Ripristinata.",
  "modes.deleted": "Modalità eliminata.",
  "modes.created": "Modalità creata.",
  "modes.active": "Attiva",
  "modes.translateTo": "Traduce in",
  "library.title": "Libreria di modalità",
  "library.description":
    "Modalità pronte: aggiungine una e modificala come vuoi.",
  "library.add": "Aggiungi",
  "library.added": "Aggiunta",
  "rules.title": "Modalità per app",
  "rules.description":
    "Quando detti in un'app di questo elenco, VoxLocal usa la sua modalità al posto di quella attiva. Basta una parte del nome: “Mail” vale anche per “Microsoft Outlook Mail”.",
  "rules.empty": "Nessuna regola: ogni dettato usa la modalità attiva.",
  "rules.app": "App",
  "rules.appPlaceholder": "Nome dell'app",
  "rules.uses": "usa",
  "rules.add": "Aggiungi una regola",

  "providers.title": "Provider LLM",
  "providers.description":
    "Il provider che riscrive il testo nelle modalità come Pulito ed Email ed esegue i comandi sulla selezione. Le chiavi restano nel portachiavi del sistema.",
  "providers.default": "Provider predefinito",
  "providers.defaultHint":
    "Usato dalle modalità che non ne indicano uno e dai comandi sulla selezione.",
  "providers.list": "Provider",
  "providers.configure": "Configura",
  "providers.cli": "Abbonamento tramite CLI",
  "providers.local": "In locale, nessuna chiave",
  "providers.keySaved": "Chiave salvata",
  "providers.keyMissing": "Chiave mancante",
  "providers.apiUrl": "Indirizzo API",
  "providers.cliModel": "quello predefinito del CLI",
  "providers.replaceKey": "Sostituisci la chiave API",
  "providers.apiKey": "Chiave API",
  "providers.keyStored": "salvata nel portachiavi",
  "providers.pasteKey": "incolla la chiave",
  "providers.verify": "Verifica",
  "providers.reachable": "Raggiungibile in {ms}.",
  "providers.removeKey": "Rimuovi chiave",
  "providers.keyRemoved": "Chiave rimossa.",

  "vocab.title": "Vocabolario",
  "vocab.description":
    "Nomi, sigle e termini tecnici scritti come vuoi tu. Aiutano la trascrizione e vengono passati all'LLM.",
  "vocab.placeholder": "Es. Kubernetes, Giorgio Rossi, ACME S.p.A.",
  "vocab.add": "Aggiungi",
  "vocab.empty":
    "Ancora nessun termine. Aggiungi le parole che la trascrizione sbaglia più spesso.",
  "vocab.removeTerm": "Rimuovi {term}",

  "history.title": "Cronologia",
  "history.clear": "Svuota la cronologia",
  "history.clearConfirm": "Elimina {n} dettati",
  "history.period": "Periodo",
  "history.today": "Oggi",
  "history.week": "Settimana",
  "history.all": "Tutto",
  "history.allModes": "Tutte le modalità",
  "history.allApps": "Tutte le app",
  "history.search": "Cerca nei dettati",
  "history.list": "Dettati",
  "history.now": "Adesso",
  "history.inProgress": "Dettato in corso",
  "history.noResults": "Nessun dettato con questi filtri.",
  "history.empty": "Qui compariranno i tuoi dettati.",
  "history.unknownApp": "App sconosciuta",
  "history.more": "Mostra altri",
  "history.pick": "Scegli un dettato per vederne i dettagli.",
  "history.deleteRow": "Elimina il dettato",
  "history.deleteRowConfirm": "Conferma: elimina il dettato",
  "history.clickAgain": "Clic di nuovo per eliminare",
  "history.command": "Comando",
  "history.said": "Detto",
  "history.selectionChanged": "Selezione originale",
  "history.hoverHint":
    "Passa sulle parti sottolineate per vedere cosa avevi detto.",
  "history.added": "Aggiunto dall'elaborazione: non l'avevi detto.",
  "history.copyText": "Copia il testo",
  "history.copySaid": "Copia il testo detto",
  "history.showSaid": "Mostra il testo detto",
  "history.showWritten": "Mostra il testo scritto",
  "history.copied": "{what} copiato negli appunti.",
  "history.textWord": "Testo",
  "history.saidWord": "Testo detto",
  "meta.app": "App",
  "meta.mode": "Modalità",
  "meta.language": "Lingua",
  "meta.microphone": "Microfono",
  "meta.transcription": "Trascrizione",
  "meta.processing": "Elaborazione",
  "meta.none": "Nessuna",
  "meta.local": "Locale",
  "time.audio": "Audio",
  "time.transcription": "Trascrizione",
  "time.processing": "Elaborazione",
  "time.paste": "Incolla",
  "time.total": "Totale",
  "status.ok": "Incollato",
  "status.llm_fallback": "Senza elaborazione",
  "status.cancelled": "Annullato",
  "status.paste_failed": "Negli appunti",
  "status.error": "Errore",
  "live.recording": "In ascolto",
  "live.transcribing": "Trascrivo",
  "live.postprocessing": "Riscrivo",
  "live.pasting": "Incollo",

  "about.title": "Informazioni",
  "about.version": "Versione",
  "about.personal": " (build personale)",
  "about.data": "Dati",
  "about.log": "Log",
  "about.updates": "Aggiornamenti",
  "about.updateAvailable": "È disponibile la versione {v}",
  "about.autoUpdates": "Cerca aggiornamenti automaticamente",
  "about.restartHint": "VoxLocal si riavvia dopo l'installazione.",
  "about.updatesHint":
    "All'avvio e ogni 6 ore. Gli aggiornamenti sono firmati.",
  "about.install": "Installa e riavvia",
  "about.installing": "Scarico e installo…",
  "about.checkNow": "Cerca ora",
  "about.found": "Trovata la versione {v}.",
  "about.upToDate": "Hai già l'ultima versione.",
  "about.privacy": "Privacy",
  "about.noTelemetry": "Nessuna telemetria",
  "about.noTelemetryHint":
    "VoxLocal non invia statistiche né dati d'uso. Audio e testo lasciano il computer solo se scegli un servizio cloud per la trascrizione o per l'elaborazione.",
  "about.reopenGuide": "Per rivedere la configurazione iniziale {link}.",
  "about.reopenGuideLink": "riapri la guida",

  "onb.step": "Passo {n} di {total}",
  "onb.mic.title": "Ti ascolto solo quando vuoi tu",
  "onb.mic.lead":
    "VoxLocal usa il microfono solo mentre tieni premuta la scorciatoia. La voce viene trascritta su questo computer e non viene salvata.",
  "onb.mic.denied": "L'accesso al microfono è stato negato.",
  "onb.mic.openSettings": "Apri Impostazioni di Sistema",
  "onb.mic.granted": "Ho dato il permesso",
  "onb.mic.speak": "Parla: le barre devono muoversi.",
  "onb.mic.allow": "Consenti il microfono",
  "onb.ax.title": "Scrivere al posto tuo",
  "onb.ax.lead":
    "Per incollare il testo nell'app che stai usando, macOS chiede il permesso di Accessibilità. VoxLocal lo usa solo per premere ⌘V dopo un dettato.",
  "onb.ax.granted": "Permesso concesso.",
  "onb.ax.step1": "Premi il pulsante qui sotto.",
  "onb.ax.step2":
    "In Impostazioni di Sistema attiva VoxLocal nell'elenco Accessibilità.",
  "onb.ax.step3": "Torna qui: questa pagina si aggiorna da sola.",
  "onb.ax.ask": "Chiedi il permesso",
  "onb.ax.skip":
    "Puoi saltare questo passo: il testo dettato resterà negli appunti e lo incollerai tu.",
  "onb.model.title": "Il motore di trascrizione",
  "onb.model.lead":
    "La trascrizione avviene su questo computer, anche senza internet. Serve scaricare il modello una volta sola.",
  "onb.model.downloading": "Scarico… {pct}%. Puoi continuare intanto.",
  "onb.try.title": "Prova a dettare",
  "onb.try.ptt":
    "Fai clic nel riquadro, tieni premuto {key}, parla e rilascia.",
  "onb.try.toggle":
    "Fai clic nel riquadro, premi {key}, parla e premi di nuovo {key}.",
  "onb.try.placeholder": "Il testo dettato comparirà qui.",
  "onb.try.works": "Funziona. Lo stesso succede in qualsiasi app.",
  "onb.try.atLogin": "Apri VoxLocal all'accesso",
  "onb.finish": "Inizia a usare VoxLocal",
  "nav.stats": "Statistiche",
  "stats.title": "Statistiche",
  "stats.description":
    "Cosa ha fatto la dettatura per te, misurato sui tuoi dettati.",
  "stats.period": "Periodo",
  "stats.week": "Ultimi 7 giorni",
  "stats.month": "Ultimi 30 giorni",
  "stats.all": "Da sempre",
  "stats.words": "Parole dettate",
  "stats.saved": "Tempo risparmiato",
  "stats.savedHint": "Rispetto a scrivere a {wpm} parole al minuto.",
  "stats.speed": "Velocità parlando",
  "stats.wpm": "{n} parole/min",
  "stats.faster": "{x} volte più veloce che scrivere",
  "stats.dictations": "Dettati",
  "stats.perDay": "Parole al giorno",
  "stats.byApp": "Parole per app",
  "stats.empty": "Detta qualcosa e qui compariranno i tuoi numeri.",
  "stats.dayWords": "{day}: {n} parole",
  "stats.tableDay": "Giorno",
  "stats.tableWords": "Parole",
  "stats.hours": "{h} h {m} min",
  "stats.minutes": "{m} min",
  "nav.snippets": "Snippet",
  "snippets.title": "Snippet",
  "snippets.description":
    "Dici una frase breve e VoxLocal incolla al suo posto il testo salvato: il tuo indirizzo, un link al calendario, una firma. Tutto il dettato deve essere la frase.",
  "snippets.trigger": "Quando dico",
  "snippets.triggerPlaceholder": "il mio indirizzo",
  "snippets.text": "Incolla",
  "snippets.textPlaceholder": "Il testo da incollare",
  "snippets.add": "Salva lo snippet",
  "snippets.empty": "Ancora nessuno snippet.",
  "snippets.edit": "Modifica",
  "vocab.suggestions": "Suggerimenti",
  "vocab.suggestionsHint":
    "Nomi e termini che tornano spesso nei tuoi dettati. Aggiungi quelli scritti giusti.",
  "vocab.noSuggestions": "Per ora nessun suggerimento: compaiono mentre detti.",
  "vocab.addSuggestion": "Aggiungi {term}",
  "vocab.dismiss": "Ignora {term}",
  "vocab.times": "{n}×",
  "history.addToVocab": "Aggiungi la selezione al vocabolario",
  "history.addedToVocab": "“{term}” aggiunto al vocabolario.",
  "history.selectFirst": "Seleziona prima una parola nel testo.",
};

export type Lang = "en" | "it";
export const UI_LANGUAGES: [Lang, string][] = [
  ["en", "English"],
  ["it", "Italiano"],
];

export function translate(
  lang: string,
  key: Key,
  vars: Record<string, string | number> = {},
): string {
  const dict: Record<Key, string> = lang === "it" ? it : en;
  return dict[key].replace(/\{(\w+)\}/g, (m, k) =>
    k in vars ? String(vars[k]) : m,
  );
}

/** Il traduttore per la lingua dell'interfaccia scelta nelle impostazioni. */
export function useT() {
  const { settings } = useSettings();
  return (key: Key, vars?: Record<string, string | number>) =>
    translate(settings.ui_language, key, vars);
}

/** Nome da mostrare per una modalità predefinita (come storage::display_name nel backend). */
const BUILTIN_IT: Record<string, [string, string]> = {
  raw: ["Grezzo", "Raw"],
  clean: ["Pulito", "Clean"],
  email: ["Email", "Email"],
  message: ["Messaggio", "Message"],
  bullets: ["Elenco puntato", "Bullet list"],
  translate_en: ["Traduci", "Translate"],
};
export function modeName(
  slug: string | null | undefined,
  name: string,
  lang: string,
): string {
  const b = slug ? BUILTIN_IT[slug] : undefined;
  return b && b[0] === name && lang !== "it" ? b[1] : name;
}

// La libreria di modalità pronte. Il compito è in inglese per tutti: l'LLM lo segue meglio e le
// regole comuni dicono di mantenere la lingua del dettato.
export interface LibraryMode {
  id: string;
  name: Record<Lang, string>;
  description: Record<Lang, string>;
  task: string;
}

export const MODE_LIBRARY: LibraryMode[] = [
  {
    id: "linkedin",
    name: { en: "LinkedIn post", it: "Post LinkedIn" },
    description: {
      en: "A strong first line, short paragraphs, up to three hashtags.",
      it: "Prima riga che aggancia, paragrafi brevi, al massimo tre hashtag.",
    },
    task: "Turn the dictation into a LinkedIn post: a strong first line, short paragraphs of one to three sentences, a clear takeaway at the end and at most three relevant hashtags on the last line. Keep the original language. Do not invent facts, numbers or names.",
  },
  {
    id: "x_post",
    name: { en: "Post for X", it: "Post per X" },
    description: {
      en: "One post of at most 280 characters, no hashtag spam.",
      it: "Un post di massimo 280 caratteri, senza raffiche di hashtag.",
    },
    task: "Turn the dictation into a single post for X (Twitter) of at most 280 characters: direct, conversational, at most one hashtag. Keep the original language. Do not invent facts.",
  },
  {
    id: "client_reply",
    name: { en: "Reply to a client", it: "Risposta a un cliente" },
    description: {
      en: "Polite and professional, with greeting and sign-off.",
      it: "Cortese e professionale, con saluto e chiusura.",
    },
    task: "Write a polite, professional reply to a client from the dictation: a greeting, the answer in clear short paragraphs, a next step if the dictation mentions one, and a sign-off. Keep the original language. Do not promise anything the dictation does not say.",
  },
  {
    id: "meeting_notes",
    name: { en: "Meeting notes", it: "Verbale di riunione" },
    description: {
      en: "Decisions, actions with owners, open points.",
      it: "Decisioni, azioni con responsabili, punti aperti.",
    },
    task: 'Turn the dictation into meeting notes with three sections: Decisions, Actions (with the owner and the date when the dictation names them) and Open points. Use short bullet points starting with "- ". Keep the original language. Leave out a section that has nothing in it.',
  },
  {
    id: "ai_prompt",
    name: {
      en: "Prompt for an AI assistant",
      it: "Prompt per un assistente AI",
    },
    description: {
      en: "Goal, context, constraints and output format, ready for ChatGPT or Claude.",
      it: "Obiettivo, contesto, vincoli e formato, pronto per ChatGPT o Claude.",
    },
    task: "Turn the dictation into a clear prompt for an AI assistant with four short parts: the goal, the context, the constraints and the expected output format. Write it as instructions addressed to the assistant. Keep the original language.",
  },
  {
    id: "summary",
    name: { en: "Summary", it: "Riassunto" },
    description: {
      en: "The point in two to four sentences.",
      it: "Il punto in due-quattro frasi.",
    },
    task: "Summarise the dictation in two to four plain sentences that keep every decision, number and name. Keep the original language.",
  },
  {
    id: "formal",
    name: { en: "Formal tone", it: "Tono formale" },
    description: {
      en: "Same content, polished for a manager or a client.",
      it: "Stesso contenuto, curato per un capo o un cliente.",
    },
    task: "Rewrite the dictation in a formal, polished tone suitable for a manager or a client. Keep the content and the original language; fix grammar and punctuation.",
  },
  {
    id: "casual",
    name: { en: "Casual tone", it: "Tono informale" },
    description: {
      en: "Friendly and direct, like a message to a colleague.",
      it: "Amichevole e diretto, come a un collega.",
    },
    task: "Rewrite the dictation in a friendly, casual tone, like a message to a colleague. Keep the content and the original language.",
  },
  {
    id: "self_corrections",
    name: { en: "Spoken corrections", it: "Correzioni a voce" },
    description: {
      en: "Keeps your words, applies “no, I mean…” corrections and “new line”.",
      it: "Tiene le tue parole, applica gli “anzi” e i “a capo”.",
    },
    task: 'Keep the dictation exactly as said, except: when the speaker corrects themselves ("at 3, no, at 4", "I mean", "anzi", "volevo dire") keep only the corrected version and drop the correction words; apply spoken formatting commands ("new line", "new paragraph", "bullet point", "a capo", "punto elenco") instead of writing them; fix punctuation and capitals. Do not rephrase anything else. Keep the original language.',
  },
  {
    id: "grammar",
    name: { en: "Grammar fix only", it: "Solo correzione grammaticale" },
    description: {
      en: "Spelling, grammar and punctuation, nothing else.",
      it: "Ortografia, grammatica e punteggiatura, nient'altro.",
    },
    task: "Fix only spelling, grammar and punctuation in the dictation. Do not change the wording, the order or the tone, except to drop the words the speaker corrected. Keep the original language.",
  },
  {
    id: "todo",
    name: { en: "To-do list", it: "Lista di cose da fare" },
    description: {
      en: "One task per line, starting with a verb.",
      it: "Un compito per riga, che inizia con un verbo.",
    },
    task: 'Turn the dictation into a to-do list: one task per line starting with "- [ ] " and a verb, with the date or the person when the dictation names them. Keep the original language.',
  },
];

// Le domande della scheda FAQ: cosa sa fare VoxLocal, nella lingua dell'interfaccia.
export const FAQ: Record<
  Lang,
  { title: string; intro: string; items: [string, string][] }
> = {
  en: {
    title: "What VoxLocal can do",
    intro: "Answers to the questions people ask in their first week.",
    items: [
      [
        "What does VoxLocal do?",
        "You hold a shortcut, speak and release: the text appears in the app you are using, in Mail, Slack, Word, the browser or your code editor. Transcription runs on your computer.",
      ],
      [
        "How do I dictate?",
        "Hold ⌥ Space (Alt+Space on Windows), speak and release. If you prefer, switch to “press to start and stop” in General. Esc cancels at any time.",
      ],
      [
        "What is the selection command?",
        "Select some text in any app, hold ⌥⇧ Space (Alt+Shift+Space) and say what to do with it: “make it more formal”, “translate into German”, “turn it into a list”. VoxLocal replaces the selection with the result. It needs an LLM provider.",
      ],
      [
        "What are modes?",
        "A mode decides what happens to the text after transcription. Raw pastes it as said; Clean fixes punctuation and removes hesitations; Email, Message, Bullet list and Translate rewrite it. You pick the active one from the menu bar.",
      ],
      [
        "Can I add new modes?",
        "Yes. In Modes, “Add from the library” has ready-made modes such as LinkedIn post, Meeting notes and Reply to a client. You can edit any mode or write your own instructions.",
      ],
      [
        "Can a mode depend on the app?",
        "Yes. In Modes, “Modes per app” lets you say, for example, that Slack uses Message and Mail uses Email. The rule wins over the active mode.",
      ],
      [
        "Which language does Translate use?",
        "The one you choose in General, “Translate into”. Modes you write yourself can use it with {{target_language}}.",
      ],
      [
        "Does my voice leave the computer?",
        "No, with the local model (the default). The audio is kept in memory only while it is transcribed. It leaves the computer only if you choose a cloud transcription service, and the text only if you use a cloud LLM.",
      ],
      [
        "Do I need an API key?",
        "Not to dictate. The modes that rewrite text and the selection command need an LLM: you can run one on your computer with Ollama or LM Studio, or use the key of a provider such as OpenAI, Anthropic or Gemini. Keys stay in the system keychain.",
      ],
      [
        "Which languages does it understand?",
        "More than 90. Fix the dictation language in General for the best accuracy and speed; “Detect automatically” works too, but it can confuse short sentences.",
      ],
      [
        "What does the history keep?",
        "Every dictation with the app it went to, what you said and what was written, the mode and the timings. You can search it, filter it, delete one dictation or clear everything. It stays on this computer.",
      ],
      [
        "What are snippets?",
        "Short phrases that paste saved text. Say “my address” and VoxLocal pastes your full address instead of the words. You create them in Snippets.",
      ],
      [
        "Can I correct myself while speaking?",
        "Yes. Say “at 3, no, at 4” and the Clean mode writes “at 4”. Say “new line”, “new paragraph” or “bullet point” and the text is formatted instead of written out.",
      ],
      [
        "Does it work if I speak quietly?",
        "Yes. VoxLocal raises quiet or whispered speech before transcribing it, so you can dictate in an open-plan office.",
      ],
      [
        "How does the vocabulary learn?",
        "Vocabulary suggests names and terms that come up often in your dictations; add them with one click. In History you can also select a word and add it.",
      ],
      [
        "How much time am I saving?",
        "Statistics shows the words you dictated, your speaking speed and the time saved compared with typing, measured on your own dictations.",
      ],
      [
        "What if the LLM does not answer?",
        "VoxLocal pastes the transcription anyway after 15 seconds and tells you. A dictation is never lost: it is always in the history.",
      ],
    ],
  },
  it: {
    title: "Cosa sa fare VoxLocal",
    intro: "Le risposte alle domande che si fanno nella prima settimana.",
    items: [
      [
        "Cosa fa VoxLocal?",
        "Tieni premuta una scorciatoia, parli e rilasci: il testo compare nell'app che stai usando, in Mail, Slack, Word, nel browser o nell'editor di codice. La trascrizione avviene sul tuo computer.",
      ],
      [
        "Come detto?",
        "Tieni premuto ⌥ Spazio (Alt+Spazio su Windows), parla e rilascia. Se preferisci, in Generale passi a “premi per avviare e fermare”. Esc annulla in qualsiasi momento.",
      ],
      [
        "Cos'è il comando sulla selezione?",
        "Selezioni un testo in qualsiasi app, tieni premuto ⌥⇧ Spazio (Alt+Shift+Spazio) e dici cosa farne: “rendilo più formale”, “traducilo in tedesco”, “fanne un elenco”. VoxLocal sostituisce la selezione con il risultato. Serve un provider LLM.",
      ],
      [
        "Cosa sono le modalità?",
        "Una modalità decide cosa succede al testo dopo la trascrizione. Grezzo lo incolla com'è; Pulito sistema punteggiatura ed esitazioni; Email, Messaggio, Elenco puntato e Traduci lo riscrivono. La modalità attiva si sceglie dal menu nella barra.",
      ],
      [
        "Posso aggiungere modalità?",
        "Sì. In Modalità, “Aggiungi dalla libreria” offre modalità pronte come Post LinkedIn, Verbale di riunione e Risposta a un cliente. Ogni modalità si può modificare, oppure puoi scrivere le tue istruzioni.",
      ],
      [
        "La modalità può dipendere dall'app?",
        "Sì. In Modalità, “Modalità per app” ti permette di dire, per esempio, che Slack usa Messaggio e Mail usa Email. La regola vince sulla modalità attiva.",
      ],
      [
        "In che lingua traduce Traduci?",
        "In quella che scegli in Generale, “Traduci in”. Le modalità che scrivi tu possono usarla con {{target_language}}.",
      ],
      [
        "La mia voce lascia il computer?",
        "No, con il modello locale (quello predefinito). L'audio resta in memoria solo il tempo di trascriverlo. Esce dal computer solo se scegli una trascrizione in cloud, e il testo solo se usi un LLM in cloud.",
      ],
      [
        "Mi serve una chiave API?",
        "Per dettare no. Le modalità che riscrivono il testo e il comando sulla selezione hanno bisogno di un LLM: puoi farlo girare sul computer con Ollama o LM Studio, oppure usare la chiave di un provider come OpenAI, Anthropic o Gemini. Le chiavi restano nel portachiavi del sistema.",
      ],
      [
        "Quali lingue capisce?",
        "Più di 90. Fissa la lingua del dettato in Generale per avere la massima precisione e velocità; “Rileva automaticamente” funziona, ma può confondere le frasi brevi.",
      ],
      [
        "Cosa conserva la cronologia?",
        "Ogni dettato con l'app in cui è finito, quello che hai detto e quello che è stato scritto, la modalità e i tempi. Puoi cercare, filtrare, eliminare un dettato o svuotare tutto. Resta su questo computer.",
      ],
      [
        "Cosa sono gli snippet?",
        "Frasi brevi che incollano un testo salvato. Dici “il mio indirizzo” e VoxLocal incolla l'indirizzo completo al posto delle parole. Si creano in Snippet.",
      ],
      [
        "Posso correggermi mentre parlo?",
        "Sì. Dici “alle 3, anzi alle 4” e la modalità Pulito scrive “alle 4”. Dici “a capo”, “nuovo paragrafo” o “punto elenco” e il testo viene formattato invece di essere scritto.",
      ],
      [
        "Funziona se parlo sottovoce?",
        "Sì. VoxLocal amplifica la voce bassa o sussurrata prima di trascriverla, così puoi dettare anche in un open space.",
      ],
      [
        "Come impara il vocabolario?",
        "Il Vocabolario ti suggerisce nomi e termini che tornano spesso nei dettati; li aggiungi con un clic. Dalla Cronologia puoi anche selezionare una parola e aggiungerla.",
      ],
      [
        "Quanto tempo sto risparmiando?",
        "Le Statistiche mostrano le parole dettate, la velocità con cui parli e il tempo risparmiato rispetto a scrivere, misurati sui tuoi dettati.",
      ],
      [
        "E se l'LLM non risponde?",
        "Dopo 15 secondi VoxLocal incolla comunque la trascrizione e te lo dice. Un dettato non si perde mai: è sempre nella cronologia.",
      ],
    ],
  },
};
