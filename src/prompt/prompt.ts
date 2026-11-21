// Proposta di avviare o fermare le note di una riunione (src-tauri/src/meeting_watch.rs).
import { invoke } from "@tauri-apps/api/core";

const q = new URLSearchParams(location.hash.slice(1));
const kind = q.get("kind") === "end" ? "end" : "start";
const app = q.get("app") ?? "";
const title = q.get("title") ?? "";
const it = q.get("lang") === "it";
document.documentElement.lang = it ? "it" : "en";

const text = {
  start: {
    q: app
      ? it
        ? `Chiamata in ${app}`
        : `Call in ${app}`
      : it
        ? "Chiamata in corso"
        : "Call in progress",
    sub:
      title ||
      (it ? "Registro le note della riunione?" : "Record meeting notes?"),
    yes: it ? "Registra le note" : "Record notes",
    no: it ? "Non ora" : "Not now",
  },
  end: {
    q: it ? "La chiamata sembra finita" : "The call seems to have ended",
    sub: it
      ? "Da 2 minuti non arriva audio dalle app."
      : "No sound from the apps for 2 minutes.",
    yes: it ? "Ferma e trascrivi" : "Stop and transcribe",
    no: it ? "Continua" : "Keep recording",
  },
}[kind];

const set = (id: string, s: string) => {
  const el = document.getElementById(id);
  if (el) el.textContent = s;
};
set("q", text.q);
set(
  "sub",
  kind === "start" && title
    ? `${title}: ${it ? "registro le note?" : "record notes?"}`
    : text.sub,
);
set("yes", text.yes);
set("no", text.no);
document.body.dataset.kind = kind;

const close = () => invoke("prompt_close");
document.getElementById("no")?.addEventListener("click", close);
document.getElementById("yes")?.addEventListener("click", async () => {
  try {
    await invoke(kind === "start" ? "meeting_start" : "meeting_stop");
    if (kind === "end") await invoke("open_section", { section: "meetings" });
  } finally {
    close();
  }
});
// sparisce da sola se nessuno risponde
setTimeout(close, 25_000);
