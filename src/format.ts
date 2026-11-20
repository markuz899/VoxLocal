// Helper di presentazione. Nessuna libreria: Intl e poche righe bastano.

export const isMac = navigator.userAgent.includes("Mac");

const MAC_SYMBOLS: Record<string, string> = {
  Alt: "⌥",
  Option: "⌥",
  Shift: "⇧",
  Control: "⌃",
  Ctrl: "⌃",
  Super: "⌘",
  Command: "⌘",
  Cmd: "⌘",
  CommandOrControl: "⌘",
  CmdOrCtrl: "⌘",
};

/** "Alt+Space" → "⌥ Space" su macOS, "Alt + Space" altrove. */
export function formatHotkey(hotkey: string): string {
  const parts = hotkey
    .split("+")
    .map((p) => p.replace(/^Key/, "").replace(/^Digit/, ""));
  if (!isMac) {
    return parts.map((p) => (p === "Super" ? "Win" : p)).join(" + ");
  }
  return parts.map((p) => MAC_SYMBOLS[p] ?? p).join(" ");
}

/**
 * Da un evento di tastiera alla sintassi di tauri-plugin-global-shortcut.
 * `null` finché manca un tasto "vero" o un modificatore (una lettera da sola non è globale).
 */
export function hotkeyFromEvent(e: KeyboardEvent): string | null {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  const code = e.code;
  if (/^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/.test(code)) return null;
  const isFn = /^F([1-9]|1[0-9]|2[0-4])$/.test(code);
  if (mods.length === 0 && !isFn) return null;
  return [...mods, code].join("+");
}

// Codici delle lingue che whisper riconosce bene; i nomi li dà Intl nella lingua dell'interfaccia.
const LANGUAGE_CODES = [
  "it",
  "en",
  "es",
  "fr",
  "de",
  "pt",
  "nl",
  "pl",
  "ro",
  "el",
  "sv",
  "da",
  "no",
  "fi",
  "cs",
  "hu",
  "uk",
  "ru",
  "tr",
  "ar",
  "hi",
  "zh",
  "ja",
  "ko",
];

export function languageName(code: string, uiLang: string): string {
  try {
    const name =
      new Intl.DisplayNames([uiLang], { type: "language" }).of(code) ?? code;
    return name.charAt(0).toLocaleUpperCase(uiLang) + name.slice(1);
  } catch {
    return code;
  }
}

/** Opzioni per una select di lingue; con `auto` aggiunge "Rileva automaticamente" in testa. */
export function languageOptions(
  uiLang: string,
  auto = false,
): [string, string][] {
  const list = LANGUAGE_CODES.map((c): [string, string] => [
    c,
    languageName(c, uiLang),
  ]);
  return auto
    ? [
        [
          "auto",
          uiLang === "it" ? "Rileva automaticamente" : "Detect automatically",
        ],
        ...list,
      ]
    : list;
}

const locale = (uiLang: string) => (uiLang === "it" ? "it-IT" : "en-GB");

export function formatBytes(n: number, uiLang = "en"): string {
  if (n >= 1e9)
    return `${(n / 1e9).toLocaleString(locale(uiLang), { maximumFractionDigits: 1 })} GB`;
  return `${Math.round(n / 1e6)} MB`;
}

export function formatDate(ms: number, uiLang = "en"): string {
  return new Intl.DateTimeFormat(locale(uiLang), {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(ms));
}

export function formatMs(ms: number | null, uiLang = "en"): string {
  if (ms == null) return "–";
  return ms < 1000
    ? `${ms} ms`
    : `${(ms / 1000).toLocaleString(locale(uiLang), { maximumFractionDigits: 1 })} s`;
}
