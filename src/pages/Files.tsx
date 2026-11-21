import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useState, useSyncExternalStore } from "react";
import { api, errorText, events, type FileDone } from "../api";
import { useT } from "../i18n";
import { Button, Notice, PageTitle } from "../ui";

type Job =
  | { kind: "idle" }
  | {
      kind: "running";
      name: string;
      stage: "decode" | "transcribe";
      pct: number;
    }
  | { kind: "done"; name: string; result: FileDone };

/*
 * La trascrizione gira nel backend anche se si cambia scheda: lo stato vive qui, fuori dal
 * componente, così tornando sulla pagina si ritrova l'avanzamento o il risultato.
 */
let job: Job = { kind: "idle" };
const subscribers = new Set<() => void>();
const setJob = (next: Job) => {
  job = next;
  subscribers.forEach((f) => f());
};
const subscribe = (f: () => void) => {
  subscribers.add(f);
  return () => subscribers.delete(f);
};
events.fileProgress(({ stage, pct }) => {
  if (job.kind === "running") setJob({ ...job, stage, pct });
});
events.fileDone((result) => {
  if (job.kind === "running") setJob({ kind: "done", name: job.name, result });
});

const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** 75 300 ms → "1:15", 3 725 000 ms → "1:02:05". */
export function clock(ms: number): string {
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

export default function Files() {
  const t = useT();
  const current = useSyncExternalStore(subscribe, () => job);
  const [over, setOver] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const start = async (path: string) => {
    setError(null);
    try {
      await api.transcribeFile(path);
      setJob({
        kind: "running",
        name: baseName(path),
        stage: "decode",
        pct: 0,
      });
    } catch (e) {
      setError(errorText(e));
    }
  };

  useEffect(() => {
    const off = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "over") setOver(true);
      else if (e.payload.type === "leave") setOver(false);
      else if (e.payload.type === "drop") {
        setOver(false);
        const path = e.payload.paths[0];
        if (path && job.kind !== "running") start(path);
      }
    });
    return () => {
      off.then((f) => f());
    };
  }, []);

  const choose = async () => {
    const path = await api.pickMediaFile();
    if (path) start(path);
  };

  return (
    <>
      <PageTitle
        title={t("files.title")}
        description={t("files.description")}
      />
      <Notice tone="error" text={error} />

      {current.kind === "running" ? (
        <Progress job={current} />
      ) : (
        <div
          className={`mb-6 grid place-items-center rounded-[10px] border-2 border-dashed px-6 py-10 text-center ${
            over ? "border-cobalt bg-cobalt-soft" : "border-line"
          }`}
        >
          <p className="font-medium">{t("files.drop")}</p>
          <p className="mt-1 mb-4 text-[12px] text-muted">
            {t("files.formats")}
          </p>
          <Button variant="primary" onClick={choose}>
            {t("files.choose")}
          </Button>
        </div>
      )}

      {current.kind === "done" && (
        <Result name={current.name} result={current.result} />
      )}
    </>
  );
}

function Progress({ job }: { job: Extract<Job, { kind: "running" }> }) {
  const t = useT();
  const label =
    job.stage === "decode"
      ? t("files.reading")
      : t("files.transcribing", { pct: job.pct });
  return (
    <div className="mb-6 rounded-[10px] border border-line bg-surface p-4">
      <div className="mb-2 flex items-center justify-between gap-4">
        <span className="truncate font-medium">{job.name}</span>
        <Button variant="ghost" onClick={() => api.cancelFile()}>
          {t("common.cancel")}
        </Button>
      </div>
      <div
        className="h-1.5 overflow-hidden rounded-full bg-paper"
        role="progressbar"
        aria-valuenow={job.stage === "decode" ? 0 : job.pct}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
      >
        <div
          className="h-full rounded-full bg-cobalt transition-[width]"
          style={{
            width: `${job.stage === "decode" ? 2 : Math.max(2, job.pct)}%`,
          }}
        />
      </div>
      <p className="mt-2 text-[12px] text-muted">{label}</p>
    </div>
  );
}

function Result({ name, result }: { name: string; result: FileDone }) {
  const t = useT();
  const [times, setTimes] = useState(true);
  const [summary, setSummary] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<{
    text: string;
    tone: "ok" | "error";
  } | null>(null);

  if (result.error) {
    return (
      <Notice
        tone={result.error === "cancelled" ? "info" : "error"}
        text={
          result.error === "cancelled" ? t("files.cancelled") : result.error
        }
      />
    );
  }
  if (result.segments.length === 0)
    return <p className="text-muted">{t("files.noSpeech")}</p>;

  const plain = result.segments.map((s) => s.text).join("\n");
  const timed = result.segments
    .map((s) => `[${clock(s.start_ms)}] ${s.text}`)
    .join("\n");
  const text = times ? timed : plain;
  const stem = name.replace(/\.[^.]+$/, "");

  const act = async (f: () => Promise<string | null>) => {
    setNotice(null);
    try {
      const msg = await f();
      if (msg) setNotice({ text: msg, tone: "ok" });
    } catch (e) {
      setNotice({ text: errorText(e), tone: "error" });
    }
  };

  return (
    <section aria-labelledby="file-result">
      <h2 id="file-result" className="mb-2 truncate text-[15px] font-semibold">
        {name}
        <span className="ml-2 text-[12px] font-normal text-muted">
          {clock(result.duration_ms)}
          {result.language && ` · ${result.language}`}
        </span>
      </h2>
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Button variant="ghost" onClick={() => setTimes(!times)}>
          {times ? t("files.hideTimes") : t("files.showTimes")}
        </Button>
        <Button
          onClick={() =>
            act(async () => (await api.copyText(text), t("files.copied")))
          }
        >
          {t("files.copy")}
        </Button>
        <Button
          onClick={() =>
            act(async () => {
              const path = await api.saveTextFile(`${stem}.txt`, text);
              return path ? t("files.saved", { path }) : null;
            })
          }
        >
          {t("files.save")}
        </Button>
        <Button
          variant="primary"
          disabled={busy}
          onClick={() =>
            act(async () => {
              setBusy(true);
              try {
                setSummary(await api.summarize(plain));
              } finally {
                setBusy(false);
              }
              return null;
            })
          }
        >
          {busy ? t("files.summarizing") : t("files.summarize")}
        </Button>
      </div>
      {notice && <Notice tone={notice.tone} text={notice.text} />}

      {summary && (
        <div className="mb-4 rounded-[10px] border border-line bg-surface p-4">
          <div className="mb-2 flex items-center justify-between">
            {/* Il titolo visibile lo scrive già l'LLM nel testo. */}
            <h3 className="sr-only">{t("files.summary")}</h3>
            <span />
            <Button
              variant="ghost"
              onClick={() =>
                act(
                  async () => (await api.copyText(summary), t("files.copied")),
                )
              }
            >
              {t("files.copy")}
            </Button>
          </div>
          <div className="selectable text-[13px] whitespace-pre-wrap">
            {summary}
          </div>
        </div>
      )}

      <div className="selectable max-h-[55vh] space-y-2 overflow-y-auto rounded-[10px] border border-line bg-surface p-4 text-[13px]">
        {result.segments.map((s, i) => (
          <p key={i} className="flex gap-3">
            {times && (
              <span className="w-12 shrink-0 text-muted tabular-nums">
                {clock(s.start_ms)}
              </span>
            )}
            <span>{s.text}</span>
          </p>
        ))}
      </div>
    </section>
  );
}
