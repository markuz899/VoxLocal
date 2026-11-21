// Barra dei comandi sulla selezione (src-tauri/src/palette.rs): scrivi cosa fare del testo
// selezionato o scegli un comando recente. Invio applica, Esc chiude.
import { invoke } from "@tauri-apps/api/core";

type Data = { words: number; recent: string[]; lang: string };

const $ = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;
const field = $<HTMLInputElement>("field");
const list = $<HTMLUListElement>("list");

const PRESETS = {
  it: [
    "Rendilo più formale",
    "Correggi gli errori",
    "Accorcialo",
    "Traduci in inglese",
    "Riassumi in tre punti",
  ],
  en: [
    "Make it more formal",
    "Fix the mistakes",
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
  // la finestra segue la scheda: niente spazio vuoto sotto la lista
  const height = document.querySelector(".card")?.scrollHeight ?? 0;
  if (height) invoke("palette_resize", { height });
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
  $("hint").textContent = it ? "Invio per applicare" : "Enter to apply";
  $("esc").textContent = it ? "annulla" : "cancel";
  sel = 0;
  render();
  field.focus();
});
