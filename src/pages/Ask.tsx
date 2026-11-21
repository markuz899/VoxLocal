import { Fragment, useEffect, useState } from "react";
import {
  api,
  errorText,
  type Answer,
  type AskTarget,
  type Source,
} from "../api";
import { useT } from "../i18n";
import { Button, Notice, PageTitle, TextInput } from "../ui";
import { clock } from "./Files";

/** Le domande della sessione restano se si cambia scheda. */
let history: { question: string; answer: Answer }[] = [];

export default function Ask() {
  const t = useT();
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [items, setItems] = useState(history);
  const [target, setTarget] = useState<AskTarget | null>(null);

  useEffect(() => {
    api.askTarget().then(setTarget);
  }, []);

  const ask = async (q: string) => {
    if (!q.trim() || busy) return;
    setError(null);
    setBusy(true);
    try {
      const answer = await api.ask(q);
      history = [{ question: q, answer }, ...history];
      setItems(history);
      setQuestion("");
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const examples = [t("ask.example1"), t("ask.example2"), t("ask.example3")];

  return (
    <>
      <PageTitle title={t("ask.title")} description={t("ask.description")} />
      <form
        className="mb-2 flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          ask(question);
        }}
      >
        <TextInput
          className="flex-1"
          aria-label={t("ask.title")}
          placeholder={t("ask.placeholder")}
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
        />
        <Button
          type="submit"
          variant="primary"
          disabled={busy || !question.trim()}
        >
          {busy ? t("ask.thinking") : t("ask.send")}
        </Button>
      </form>
      {target && (
        <p
          className={`mb-4 text-[12px] ${target.available && !target.local ? "text-warn" : "text-muted"}`}
        >
          {!target.available
            ? t("ask.noLlm")
            : target.local
              ? t("ask.local", { name: target.label })
              : t("ask.cloud", { name: target.label })}
        </p>
      )}
      <Notice tone="error" text={error} />

      {items.length === 0 && (
        <div className="flex flex-wrap gap-2">
          {examples.map((q) => (
            <button
              key={q}
              type="button"
              onClick={() => ask(q)}
              className="rounded-full border border-line px-3 py-1.5 text-[13px] text-muted hover:bg-paper hover:text-ink"
            >
              {q}
            </button>
          ))}
        </div>
      )}

      <div className="space-y-6">
        {items.map((it, i) => (
          <AnswerView
            key={items.length - i}
            question={it.question}
            answer={it.answer}
          />
        ))}
      </div>
    </>
  );
}

function AnswerView({
  question,
  answer,
}: {
  question: string;
  answer: Answer;
}) {
  const t = useT();
  const [open, setOpen] = useState<number | null>(null);
  // "[2][4]" nel testo diventa un rimando cliccabile alla fonte
  const parts = answer.text.split(/(\[\d+\])/g);
  return (
    <section className="rounded-[10px] border border-line bg-surface p-4">
      <h2 className="mb-2 text-[15px] font-semibold">{question}</h2>
      <div className="selectable text-[14px] leading-relaxed whitespace-pre-wrap">
        {parts.map((p, i) => {
          const m = p.match(/^\[(\d+)\]$/);
          if (!m) return <Fragment key={i}>{p}</Fragment>;
          const n = Number(m[1]);
          if (!answer.sources[n - 1]) return <Fragment key={i}>{p}</Fragment>;
          return (
            <button
              key={i}
              type="button"
              onClick={() => setOpen(open === n ? null : n)}
              aria-label={t("ask.source", { n })}
              className={`mx-0.5 rounded px-1 align-super text-[11px] font-medium ${open === n ? "bg-cobalt text-white" : "bg-cobalt-soft text-cobalt"}`}
            >
              {n}
            </button>
          );
        })}
      </div>
      {open !== null && answer.sources[open - 1] && (
        <SourceView n={open} s={answer.sources[open - 1]} />
      )}
      {answer.sources.length > 0 && (
        <p className="mt-3 text-[12px] text-muted">
          {t("ask.basedOn", { n: answer.sources.length })}
        </p>
      )}
    </section>
  );
}

function SourceView({ n, s }: { n: number; s: Source }) {
  const t = useT();
  const when = new Date(s.at).toLocaleString(undefined, {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
  const label =
    s.kind === "meeting"
      ? `${s.title || t("ask.meeting")} · ${when}${s.offset_ms !== null ? ` · ${clock(s.offset_ms)}` : ""}`
      : `${t("ask.dictation")}${s.title ? ` · ${s.title}` : ""} · ${when}`;
  const go = () => {
    if (s.kind === "meeting")
      sessionStorage.setItem("openMeeting", String(s.id));
    window.location.hash = s.kind === "meeting" ? "meetings" : "history";
  };
  return (
    <div className="mt-3 rounded-md border border-line bg-paper/40 p-3 text-[13px]">
      <div className="mb-1 flex items-center gap-2 text-[12px] text-muted">
        <span className="font-medium text-ink">[{n}]</span>
        <span className="flex-1 truncate">{label}</span>
        <Button variant="ghost" onClick={go}>
          {s.kind === "meeting" ? t("ask.openMeeting") : t("ask.openHistory")}
        </Button>
      </div>
      <div className="selectable whitespace-pre-wrap">{s.text}</div>
    </div>
  );
}
