import { useCallback, useEffect, useMemo, useState } from "react";
import {
  api,
  errorText,
  events,
  type HistoryFacets,
  type HistoryFilter,
  type HistoryItem,
  type PipelineState,
} from "../api";
import { diffWords } from "../diff";
import { formatDate, formatMs, LANGUAGES } from "../format";
import { Button, Notice, Select, TextInput } from "../ui";

const PAGE = 50;

const STATUS: Record<HistoryItem["status"], string> = {
  ok: "Incollato",
  llm_fallback: "Senza elaborazione",
  cancelled: "Annullato",
  paste_failed: "Negli appunti",
  error: "Errore",
};

const LIVE: Record<Exclude<PipelineState, "idle">, string> = {
  recording: "In ascolto",
  transcribing: "Trascrivo",
  postprocessing: "Riscrivo",
  pasting: "Incollo",
};

type Period = "today" | "week" | "all";

/** Inizio di oggi e di sette giorni fa, nell'ora locale. */
function periodStarts() {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  return { today: today.getTime(), week: today.getTime() - 6 * 86_400_000 };
}

// Un colore tenue stabile per ogni app, come gli avatar della cronologia nella landing.
const TONES = ["#dbe4fb", "#d8efe3", "#f6e2c5", "#f3d9dc", "#e3dcf3"];
function tone(app: string) {
  let h = 0;
  for (const c of app) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return TONES[h % TONES.length];
}

function AppBadge({ app }: { app: string | null }) {
  const name = app ?? "?";
  return (
    <span
      aria-hidden
      className="grid h-8 w-8 shrink-0 place-items-center rounded-full text-[12px] font-medium text-[#16181a]"
      style={{ background: app ? tone(app) : "var(--line)" }}
    >
      {name.slice(0, 2).toUpperCase()}
    </span>
  );
}

export default function History() {
  const [facets, setFacets] = useState<HistoryFacets | null>(null);
  const [items, setItems] = useState<HistoryItem[]>([]);
  const [more, setMore] = useState(false);
  const [query, setQuery] = useState("");
  const [period, setPeriod] = useState<Period>("all");
  const [mode, setMode] = useState("");
  const [app, setApp] = useState("");
  const [selected, setSelected] = useState<number | null>(null);
  const [live, setLive] = useState<{
    state: PipelineState;
    app: string | null;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);

  const filter = useMemo<HistoryFilter>(() => {
    const starts = periodStarts();
    return {
      query,
      since: period === "all" ? null : starts[period],
      mode: mode || null,
      app: app || null,
    };
  }, [query, period, mode, app]);

  const load = useCallback(
    async (offset: number) => {
      try {
        const page = await api.getHistory(filter, PAGE, offset);
        setItems((prev) => (offset === 0 ? page : [...prev, ...page]));
        setMore(page.length === PAGE);
        if (offset === 0)
          setSelected((s) =>
            page.some((i) => i.id === s) ? s : (page[0]?.id ?? null),
          );
        const { today, week } = periodStarts();
        setFacets(await api.historyFacets(today, week));
      } catch (e) {
        setError(errorText(e));
      }
    },
    [filter],
  );

  const remove = async (id: number) => {
    try {
      await api.deleteDictation(id);
      if (selected === id) setSelected(null);
      await load(0);
    } catch (e) {
      setError(errorText(e));
    }
  };

  const clearAll = async () => {
    try {
      await api.clearHistory();
      setConfirmClear(false);
      setSelected(null);
      await load(0);
    } catch (e) {
      setError(errorText(e));
    }
  };

  // Ricerca mentre si scrive, con un piccolo ritardo.
  useEffect(() => {
    const t = setTimeout(() => load(0), 150);
    return () => clearTimeout(t);
  }, [load]);

  // Il dettato in corso compare in cima; quando finisce la lista si ricarica.
  useEffect(() => {
    const offs = [
      events.state((state, a) =>
        setLive(state === "idle" ? null : { state, app: a }),
      ),
      events.historyChanged(() => load(0)),
    ];
    return () => offs.forEach((p) => p.then((off) => off()));
  }, [load]);

  const current = items.find((i) => i.id === selected) ?? null;

  const periods: [Period, string, number | undefined][] = [
    ["today", "Oggi", facets?.today],
    ["week", "Settimana", facets?.week],
    ["all", "Tutto", facets?.total],
  ];

  return (
    <div className="flex h-[calc(100vh-5rem)] min-h-[480px] flex-col">
      <header className="mb-4 flex flex-wrap items-center gap-3">
        <h1 className="mr-auto text-[22px] font-semibold tracking-tight">
          Cronologia
        </h1>
        {confirmClear ? (
          <span className="flex items-center gap-2">
            <Button variant="danger" onClick={clearAll}>
              Elimina {facets?.total ?? ""} dettati
            </Button>
            <Button variant="ghost" onClick={() => setConfirmClear(false)}>
              Annulla
            </Button>
          </span>
        ) : (
          <Button
            variant="ghost"
            disabled={!facets?.total}
            onClick={() => setConfirmClear(true)}
          >
            Svuota la cronologia
          </Button>
        )}
        <div
          role="group"
          aria-label="Periodo"
          className="flex rounded-md border border-line bg-surface p-0.5"
        >
          {periods.map(([id, label, n]) => (
            <button
              key={id}
              type="button"
              aria-pressed={period === id}
              onClick={() => setPeriod(id)}
              className={`h-6 rounded px-2.5 text-[12px] ${
                period === id
                  ? "bg-cobalt-soft font-medium text-cobalt"
                  : "text-muted hover:text-ink"
              }`}
            >
              {label}
              {n !== undefined && <span className="ml-1 opacity-70">{n}</span>}
            </button>
          ))}
        </div>
        <Select
          label="Modalità"
          value={mode}
          onChange={setMode}
          options={[
            ["", "Tutte le modalità"],
            ...(facets?.modes ?? []).map(
              ([slug, name, n]): [string, string] => [slug, `${name} (${n})`],
            ),
          ]}
        />
        <Select
          label="App"
          value={app}
          onChange={setApp}
          options={[
            ["", "Tutte le app"],
            ...(facets?.apps ?? []).map(([a, n]): [string, string] => [
              a,
              `${a} (${n})`,
            ]),
          ]}
        />
      </header>
      <Notice tone="error" text={error} />

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(300px,5fr)_minmax(0,7fr)] overflow-hidden rounded-[10px] border border-line bg-surface">
        <div className="flex min-h-0 flex-col border-r border-line">
          <div className="border-b border-line p-2">
            <TextInput
              type="search"
              className="w-full"
              placeholder="Cerca nei dettati"
              aria-label="Cerca nei dettati"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <ul className="min-h-0 flex-1 overflow-y-auto" aria-label="Dettati">
            {live && (
              <li className="flex items-center gap-3 border-b border-line bg-cobalt-soft/50 px-3 py-2.5">
                <AppBadge app={live.app} />
                <div className="min-w-0 flex-1">
                  <div className="font-medium">
                    {live.app ?? "Dettato in corso"}
                  </div>
                  <div className="text-[12px] text-muted">Adesso</div>
                </div>
                <span className="flex items-center gap-1.5 rounded-full border border-cobalt/40 px-2 py-0.5 text-[12px] text-cobalt">
                  <span
                    className="h-2.5 w-2.5 animate-spin rounded-full border-2 border-cobalt border-r-transparent"
                    aria-hidden
                  />
                  {LIVE[live.state as Exclude<PipelineState, "idle">]}
                </span>
              </li>
            )}
            {items.length === 0 && !live && (
              <li className="p-4 text-muted">
                {query || mode || app || period !== "all"
                  ? "Nessun dettato con questi filtri."
                  : "Qui compariranno i tuoi dettati."}
              </li>
            )}
            {items.map((item) => (
              <li
                key={item.id}
                className={`group relative border-b border-line ${
                  item.id === selected ? "bg-cobalt-soft/60" : "hover:bg-paper"
                }`}
              >
                <button
                  type="button"
                  onClick={() => setSelected(item.id)}
                  aria-current={item.id === selected ? "true" : undefined}
                  className="flex w-full items-center gap-3 px-3 py-2.5 pr-11 text-left"
                >
                  <AppBadge app={item.target_app} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline gap-2">
                      <span className="min-w-0 truncate font-medium">
                        {item.target_app ?? "App sconosciuta"}
                      </span>
                      <span className="ml-auto shrink-0 text-[12px] text-muted">
                        {formatDate(item.created_at)}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="min-w-0 flex-1 truncate text-[13px] text-muted">
                        {item.final_text}
                      </span>
                      {/* Solo gli stati da notare: un dettato incollato è il caso normale. */}
                      {item.status !== "ok" && (
                        <span className="shrink-0 rounded-full border border-warn/50 px-1.5 text-[11px] leading-[18px] text-warn">
                          {STATUS[item.status]}
                        </span>
                      )}
                    </div>
                  </div>
                </button>
                <RowDelete onDelete={() => remove(item.id)} />
              </li>
            ))}
            {more && (
              <li className="p-3 text-center">
                <Button onClick={() => load(items.length)}>Mostra altri</Button>
              </li>
            )}
          </ul>
        </div>
        <div className="min-h-0 overflow-y-auto">
          {current ? (
            <Detail
              key={current.id}
              item={current}
              onDeleted={() => {
                setSelected(null);
                load(0);
              }}
            />
          ) : (
            <p className="p-6 text-muted">
              Scegli un dettato per vederne i dettagli.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

/** Cestino sulla riga: compare al passaggio o con la tastiera; il primo clic chiede conferma. */
function RowDelete({ onDelete }: { onDelete: () => void }) {
  const [armed, setArmed] = useState(false);
  return (
    <button
      type="button"
      aria-label={armed ? "Conferma: elimina il dettato" : "Elimina il dettato"}
      title={armed ? "Clic di nuovo per eliminare" : "Elimina"}
      onClick={() => (armed ? onDelete() : setArmed(true))}
      onBlur={() => setArmed(false)}
      className={`absolute top-1/2 right-2 grid h-7 -translate-y-1/2 place-items-center rounded-md px-1.5 text-[12px] transition-opacity focus-visible:opacity-100 ${
        armed
          ? "bg-rec text-white opacity-100"
          : "text-muted opacity-0 group-hover:opacity-100 hover:bg-line hover:text-ink"
      }`}
    >
      {armed ? (
        "Elimina"
      ) : (
        <svg
          viewBox="0 0 16 16"
          width="15"
          height="15"
          aria-hidden="true"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.4"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M2.5 4.5h11M6.5 4.5V3h3v1.5M4 4.5l.7 8.5h6.6l.7-8.5M6.8 7v4M9.2 7v4" />
        </svg>
      )}
    </button>
  );
}

function Detail({
  item,
  onDeleted,
}: {
  item: HistoryItem;
  onDeleted: () => void;
}) {
  const [showRaw, setShowRaw] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);
  const segments = useMemo(
    () => diffWords(item.raw_text, item.final_text),
    [item],
  );
  const processed = item.raw_text !== item.final_text;
  const changedCount = segments.filter((s) => s.changed).length;

  const copy = async (text: string, what: string) => {
    await api.copyText(text);
    setNotice(`${what} copiato negli appunti.`);
    setTimeout(() => setNotice(null), 2000);
  };

  const language =
    LANGUAGES.find(([code]) => code === item.language)?.[1] ?? item.language;
  const meta: [string, string | null][] = [
    ["App", item.target_app],
    ["Modalità", item.mode],
    ["Lingua", language],
    ["Microfono", item.audio_device ?? "Microfono di sistema"],
    [
      "Trascrizione",
      [
        item.stt_provider === "local" ? "Locale" : item.stt_provider,
        item.stt_model,
      ]
        .filter(Boolean)
        .join(", "),
    ],
    [
      "Elaborazione",
      item.llm_provider
        ? [item.llm_provider, item.llm_model].filter(Boolean).join(", ")
        : "Nessuna",
    ],
  ];
  const times: [string, number | null][] = [
    ["Audio", item.audio_ms],
    ["Trascrizione", item.stt_ms],
    ["Elaborazione", item.llm_ms],
    ["Incolla", item.paste_ms],
    ["Totale", item.total_ms],
  ];

  return (
    <article className="p-6">
      <div className="mb-1 text-[12px] text-muted">
        {[item.target_app, item.mode, formatDate(item.created_at)]
          .filter(Boolean)
          .join(", ")}
      </div>
      {item.status !== "ok" && (
        <p className="mb-3 text-[13px] text-warn">
          {STATUS[item.status]}
          {item.error ? `: ${item.error}` : ""}
        </p>
      )}

      <div className="selectable mt-3 text-[16px] leading-relaxed whitespace-pre-wrap">
        {showRaw
          ? item.raw_text
          : segments.map((s, i) =>
              s.changed ? (
                <mark
                  key={i}
                  tabIndex={0}
                  onMouseEnter={() => setNote(s.said)}
                  onFocus={() => setNote(s.said)}
                  className="rounded-sm bg-transparent text-inherit underline decoration-cobalt decoration-2 underline-offset-4 outline-none focus-visible:bg-cobalt-soft"
                >
                  {s.text}
                </mark>
              ) : (
                <span key={i}>{s.text}</span>
              ),
            )}
      </div>

      {processed && !showRaw && changedCount > 0 && (
        <aside
          aria-live="polite"
          className="mt-4 rounded-[10px] border border-line bg-paper p-3"
        >
          <div className="flex items-center gap-2 text-[12px] font-medium">
            <span className="h-2 w-2 rounded-full bg-cobalt" aria-hidden />{" "}
            Detto
          </div>
          <p className="mt-1 text-[14px]">
            {note === null
              ? "Passa sulle parti sottolineate per vedere cosa avevi detto."
              : note
                ? `“${note}”`
                : "Aggiunto dall'elaborazione: non l'avevi detto."}
          </p>
        </aside>
      )}

      <div className="mt-5 flex flex-wrap gap-2">
        <Button
          variant="primary"
          onClick={() => copy(item.final_text, "Testo")}
        >
          Copia il testo
        </Button>
        {processed && (
          <>
            <Button onClick={() => copy(item.raw_text, "Testo detto")}>
              Copia il testo detto
            </Button>
            <Button variant="ghost" onClick={() => setShowRaw(!showRaw)}>
              {showRaw ? "Mostra il testo scritto" : "Mostra il testo detto"}
            </Button>
          </>
        )}
        <span className="flex-1" />
        {confirm ? (
          <>
            <Button
              variant="danger"
              onClick={async () => {
                await api.deleteDictation(item.id);
                onDeleted();
              }}
            >
              Elimina
            </Button>
            <Button variant="ghost" onClick={() => setConfirm(false)}>
              Annulla
            </Button>
          </>
        ) : (
          <Button variant="ghost" onClick={() => setConfirm(true)}>
            Elimina
          </Button>
        )}
      </div>
      <Notice tone="ok" text={notice} />

      <dl className="mt-6 grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 border-t border-line pt-4 text-[13px]">
        {meta.map(([k, v]) => (
          <div key={k} className="contents">
            <dt className="text-muted">{k}</dt>
            <dd className="m-0">{v || "–"}</dd>
          </div>
        ))}
      </dl>
      <dl className="mt-4 grid grid-cols-[repeat(auto-fill,minmax(104px,1fr))] gap-x-4 gap-y-3 border-t border-line pt-4">
        {times.map(([k, v]) => (
          <div key={k}>
            <dt className="text-[12px] text-muted">{k}</dt>
            <dd className="m-0 text-[15px] font-medium">{formatMs(v)}</dd>
          </div>
        ))}
      </dl>
    </article>
  );
}
