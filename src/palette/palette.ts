// Barra dei comandi sulla selezione (src-tauri/src/palette.rs): scrivi cosa fare del testo
// selezionato, scegli un comando recente o parla (🎤). Invio applica, Esc chiude.
import { invoke } from "@tauri-apps/api/core";
import { formatHotkey } from "../format";

type Data = { words: number; hotkey: string; recent: string[]; lang: string };

const $ = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;
const field = $<HTMLInputElement>("field");
const list = $<HTMLUListElement>("list");

const PRESETS = {
  it: [
    "Rendilo più formale",
    "Correggi gli errori",
    "Rispondi in modo cordiale",
    "Accorcialo",
    "Traduci in inglese",
    "Riassumi in tre punti",
  ],
  en: [
    "Make it more formal",
    "Fix the mistakes",
    "Reply in a friendly way",
    "Make it shorter",
    "Translate into Italian",
    "Summarize in three bullet points",
  ],
};

let items: string[] = [];
let shown: string[] = [];
// -1: si applica quello che è scritto nel campo
let sel = -1;
let done = false;

const cancel = () => {
  if (done) return;
  done = true;
  invoke("palette_cancel");
};
const run = (instruction: string) => {
  if (done || !instruction.trim()) return;
  done = true;
  invoke("palette_run", { instruction });
};

// 🎤 o di nuovo la scorciatoia (lato Rust): il comando si detta invece di scriverlo
const speak = () => {
  if (done) return;
  done = true;
  invoke("palette_speak");
};
$("speak").addEventListener("mousedown", (e) => {
  e.preventDefault();
  speak();
});

function render() {
  const q = field.value.trim().toLowerCase();
  shown = q ? items.filter((c) => c.toLowerCase().includes(q)) : items;
  list.replaceChildren(
    ...shown.map((c, i) => {
      const li = document.createElement("li");
      li.role = "option";
      li.textContent = c;
      li.ariaSelected = String(i === sel);
      li.addEventListener("mousedown", (e) => {
        e.preventDefault();
        run(c);
      });
      return li;
    }),
  );
  $("group").hidden = shown.length === 0;
  // La finestra segue la scheda: niente spazio vuoto sotto la lista. Si somma l'altezza naturale
  // di ogni parte (bordi e righe nascoste compresi): scrollHeight della scheda dipende dalla
  // finestra e perdeva i bordi a ogni nuovo render, quindi la barra si accorciava.
  const card = document.querySelector<HTMLElement>(".card");
  if (!card) return;
  const parts = [...card.children] as HTMLElement[];
  const height =
    parts.reduce(
      (h, el) => h + el.offsetHeight + el.scrollHeight - el.clientHeight,
      0,
    ) +
    card.offsetHeight -
    card.clientHeight;
  invoke("palette_resize", { height });
}

field.addEventListener("input", () => {
  sel = field.value.trim() ? -1 : 0;
  render();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    cancel();
  } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    e.preventDefault();
    const min = field.value.trim() ? -1 : 0;
    const next = sel + (e.key === "ArrowDown" ? 1 : -1);
    sel = Math.max(min, Math.min(shown.length - 1, next));
    render();
  } else if (e.key === "Enter") {
    e.preventDefault();
    run(sel >= 0 && shown[sel] ? shown[sel] : field.value);
  }
});
// Un clic fuori dalla barra la chiude, come Esc (solo dopo che ha avuto il focus).
let focused = false;
window.addEventListener("focus", () => (focused = true));
window.addEventListener("blur", () => focused && cancel());

invoke<Data>("palette_data").then((d) => {
  const it = d.lang === "it";
  document.documentElement.lang = it ? "it" : "en";
  const presets = PRESETS[it ? "it" : "en"];
  const seen = new Set(d.recent.map((c) => c.toLowerCase()));
  items = [
    ...d.recent,
    ...presets.filter((c) => !seen.has(c.toLowerCase())),
  ].slice(0, 6);
  field.placeholder = it
    ? "Cosa faccio del testo selezionato?"
    : "What should I do with the selected text?";
  $("words").textContent = it ? `${d.words} parole` : `${d.words} words`;
  $("group").textContent = d.recent.length
    ? it
      ? "Comandi recenti e suggeriti"
      : "Recent and suggested commands"
    : it
      ? "Comandi suggeriti"
      : "Suggested commands";
  const key = formatHotkey(d.hotkey);
  $("speak-label").textContent = it ? "Parla" : "Speak";
  $("speak").title = it ? `Parla (${key})` : `Speak (${key})`;
  $("hint").textContent = it
    ? `Invio per applicare, ${key} per parlare`
    : `Enter to apply, ${key} to speak`;
  $("esc").textContent = it ? "annulla" : "cancel";
  sel = 0;
  render();
  field.focus();
});
