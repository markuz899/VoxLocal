// Overlay della dettatura: livello del microfono e stato della pipeline.
import { events, type PipelineState } from "../api";

const BARS = 16;
const LABELS: Record<PipelineState, string> = {
  idle: "",
  recording: "In ascolto",
  transcribing: "Trascrivo",
  postprocessing: "Elaboro",
  pasting: "Incollo",
};

const pill = document.querySelector<HTMLElement>(".pill")!;
const label = document.querySelector<HTMLElement>(".label")!;
const barsEl = document.querySelector<HTMLElement>(".bars")!;

const bars = Array.from({ length: BARS }, (_, i) => {
  const bar = document.createElement("i");
  bar.style.setProperty("--i", String(i));
  barsEl.append(bar);
  return bar;
});

let state: PipelineState = "idle";

events.state((s) => {
  state = s;
  pill.dataset.state = s;
  label.textContent = LABELS[s];
  if (s !== "recording") {
    for (const bar of bars) {
      bar.classList.remove("on");
      bar.style.height = "";
    }
  }
});

// Barre simmetriche dal centro: più la voce è forte, più si allargano e crescono.
events.level((rms) => {
  if (state !== "recording") return;
  const level = Math.min(1, Math.sqrt(rms) * 2.2);
  const half = BARS / 2;
  bars.forEach((bar, i) => {
    const dist = Math.abs(i - (half - 0.5)) / half;
    const on = dist < level;
    bar.classList.toggle("on", on);
    bar.style.height = on ? `${4 + (1 - dist) * level * 18}px` : "";
  });
});
