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

export const LANGUAGES: [string, string][] = [
  ["auto", "Rileva automaticamente"],
  ["it", "Italiano"],
  ["en", "Inglese"],
  ["es", "Spagnolo"],
  ["fr", "Francese"],
  ["de", "Tedesco"],
  ["pt", "Portoghese"],
  ["nl", "Olandese"],
  ["pl", "Polacco"],
  ["ro", "Rumeno"],
  ["el", "Greco"],
  ["sv", "Svedese"],
  ["da", "Danese"],
  ["no", "Norvegese"],
  ["fi", "Finlandese"],
  ["cs", "Ceco"],
  ["hu", "Ungherese"],
  ["uk", "Ucraino"],
  ["ru", "Russo"],
  ["tr", "Turco"],
  ["ar", "Arabo"],
  ["hi", "Hindi"],
  ["zh", "Cinese"],
  ["ja", "Giapponese"],
  ["ko", "Coreano"],
];

export function formatBytes(n: number): string {
  if (n >= 1e9)
    return `${(n / 1e9).toLocaleString("it-IT", { maximumFractionDigits: 1 })} GB`;
  return `${Math.round(n / 1e6)} MB`;
}

const dateFmt = new Intl.DateTimeFormat("it-IT", {
  day: "numeric",
  month: "short",
  hour: "2-digit",
  minute: "2-digit",
});

export function formatDate(ms: number): string {
  return dateFmt.format(new Date(ms));
}

export function formatMs(ms: number | null): string {
  if (ms == null) return "–";
  return ms < 1000
    ? `${ms} ms`
    : `${(ms / 1000).toLocaleString("it-IT", { maximumFractionDigits: 1 })} s`;
}
