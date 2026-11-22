import { useEffect, useState } from "react";
import {
  api,
  errorText,
  events,
  type Meeting,
  type MeetingAction,
  type MeetingState,
} from "../api";
import { isMac } from "../format";
import { useSettings } from "../App";
import { useT } from "../i18n";
import { Button, Notice, PageTitle, Select, TextInput, Toggle } from "../ui";
import { clock } from "./Files";

export default function Meetings() {
  const t = useT();
  const [state, setState] = useState<MeetingState>({
    recording: false,
    started_at: null,
    system_error: null,
    mic_silent: false,
  });
  const [list, setList] = useState<Meeting[]>([]);
  const [open, setOpen] = useState<number | null>(null);
  const [progress, setProgress] = useState<Record<number, number>>({});
  const [permission, setPermission] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = () =>
    api.listMeetings().then((l) => {
      setList(l);
      // aperta da una fonte di "Chiedi": quella riunione
      const wanted = Number(sessionStorage.getItem("openMeeting"));
      sessionStorage.removeItem("openMeeting");
      setOpen((o) =>
        wanted && l.some((m) => m.id === wanted)
          ? wanted
          : (o ?? l[0]?.id ?? null),
      );
    });

  useEffect(() => {
    api.meetingState().then(setState);
    api.screenPermission(false).then(setPermission);
    reload();
    const offs = [
      events.meetingState(setState),
      events.meetingProgress(({ id, pct }) =>
        setProgress((p) => ({ ...p, [id]: pct })),
      ),
      events.meetingDone(() => reload()),
      events.historyChanged(() => reload()),
    ];
    return () => offs.forEach((o) => o.then((f) => f()));
  }, []);

  // durante la registrazione: il microfono muto si scopre solo dopo qualche secondo
  useEffect(() => {
    if (!state.recording) return;
    const id = setInterval(() => api.meetingState().then(setState), 2000);
    return () => clearInterval(id);
  }, [state.recording]);

  const run = async (f: () => Promise<unknown>) => {
    setError(null);
    try {
      await f();
    } catch (e) {
      setError(errorText(e));
    }
  };

  return (
    <>
      <PageTitle
        title={t("meetings.title")}
        description={t("meetings.description")}
      />
      <Recorder
        state={state}
        permission={permission}
        onStart={() => run(async () => setState(await api.meetingStart()))}
        onStop={() =>
          run(async () => {
            const id = await api.meetingStop();
            setOpen(id);
            await reload();
          })
        }
        onPermission={() =>
          run(async () => setPermission(await api.screenPermission(true)))
        }
      />
      <Notice tone="error" text={error} />
      <Options />

      {list.length === 0 ? (
        <p className="text-muted">{t("meetings.empty")}</p>
      ) : (
        <div className="grid grid-cols-[240px_minmax(0,1fr)] gap-4">
          <ul className="space-y-1">
            {list.map((m) => (
              <li key={m.id}>
                <button
                  type="button"
                  onClick={() => setOpen(m.id)}
                  className={`w-full rounded-md px-3 py-2 text-left ${open === m.id ? "bg-cobalt-soft" : "hover:bg-paper"}`}
                >
                  <div className="truncate font-medium">
                    {m.title || when(m.started_at)}
                  </div>
                  <div className="text-[12px] text-muted">
                    {clock(m.duration_ms)}
                    {m.status === "transcribing" &&
                      ` · ${t("meetings.transcribing", { pct: progress[m.id] ?? 0 })}`}
                    {m.status === "error" && ` · ${t("meetings.failed")}`}
                  </div>
                </button>
              </li>
            ))}
          </ul>
          {open !== null && (
            <Detail
              key={open}
              id={open}
              pct={progress[open]}
              onDeleted={() => {
                setOpen(null);
                reload();
              }}
              onChanged={reload}
            />
          )}
        </div>
      )}
    </>
  );
}

const when = (ms: number) =>
  new Date(ms).toLocaleString(undefined, {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });

function Recorder({
  state,
  permission,
  onStart,
  onStop,
  onPermission,
}: {
  state: MeetingState;
  permission: boolean;
  onStart: () => void;
  onStop: () => void;
  onPermission: () => void;
}) {
  const t = useT();
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!state.recording) return;
    const id = setInterval(() => setNow(Date.now()), 500);
    return () => clearInterval(id);
  }, [state.recording]);

  return (
    <div className="mb-6 rounded-[10px] border border-line bg-surface p-4">
      {state.recording ? (
        <div className="flex items-center gap-4">
          <span
            className="h-3 w-3 animate-pulse rounded-full bg-red-500"
            aria-hidden="true"
          />
          <div className="flex-1">
            <div className="font-medium">{t("meetings.recording")}</div>
            <div className="text-[13px] text-muted tabular-nums">
              {clock(now - (state.started_at ?? now))}
            </div>
          </div>
          <Button variant="primary" onClick={onStop}>
            {t("meetings.stop")}
          </Button>
        </div>
      ) : (
        <div className="flex flex-wrap items-center gap-4">
          <p className="m-0 min-w-[200px] flex-1 text-[13px] text-muted">
            {t("meetings.how")}
          </p>
          <MicPicker />
          <Button variant="primary" onClick={onStart}>
            {t("meetings.start")}
          </Button>
        </div>
      )}
      {state.recording && state.mic_silent && (
        <div className="mt-3">
          <Notice tone="error" text={t("meetings.micSilent")} />
        </div>
      )}
      {state.recording && state.system_error && (
        <div className="mt-3">
          <Notice
            tone="info"
            text={t("meetings.micOnly", { why: state.system_error })}
          />
        </div>
      )}
      {!state.recording && !permission && (
        <div className="mt-3 flex items-center gap-3">
          <p className="m-0 flex-1 text-[13px] text-warn">
            {t("meetings.permission")}
          </p>
          <Button onClick={onPermission}>{t("meetings.grant")}</Button>
        </div>
      )}
    </div>
  );
}

function Detail({
  id,
  pct,
  onDeleted,
  onChanged,
}: {
  id: number;
  pct?: number;
  onDeleted: () => void;
  onChanged: () => void;
}) {
  const t = useT();
  const { settings } = useSettings();
  const [m, setM] = useState<Meeting | null>(null);
  const [title, setTitle] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [notice, setNotice] = useState<{
    text: string;
    tone: "ok" | "error";
  } | null>(null);
  const load = () =>
    api.getMeeting(id).then((x) => {
      setM(x);
      setTitle(x?.title ?? "");
    });
  useEffect(() => {
    load();
    const off = events.meetingDone((done) => done === id && load());
    return () => {
      off.then((f) => f());
    };
  }, [id]);
  if (!m) return null;

  const me = settings.ui_language === "it" ? "Io" : "Me";
  const others = settings.ui_language === "it" ? "Altri" : "Others";
  const lines = m.lines
    .map(
      (l) =>
        `[${clock(l.start_ms)}] ${l.speaker === "me" ? me : others}: ${l.text}`,
    )
    .join("\n");
  const todo = actionsMarkdown(m.actions, t("meetings.actions"));
  const text = todo ? `${todo}\n\n${lines}` : lines;
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
    <section className="min-w-0">
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <TextInput
          className="min-w-0 flex-1"
          aria-label={t("meetings.name")}
          placeholder={when(m.started_at)}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onBlur={() =>
            title !== m.title && api.renameMeeting(id, title).then(onChanged)
          }
        />
        <span className="text-[12px] text-muted">{clock(m.duration_ms)}</span>
        {m.attendees.length > 0 && (
          <p className="m-0 w-full text-[12px] text-muted">
            {t("meetings.attendees", { names: m.attendees.join(", ") })}
          </p>
        )}
      </div>

      {m.status === "transcribing" && (
        <Notice
          tone="info"
          text={t("meetings.transcribing", { pct: pct ?? 0 })}
        />
      )}
      {m.status === "error" && (
        <Notice tone="error" text={m.error ?? t("meetings.failed")} />
      )}

      {m.status === "done" && (
        <>
          <div className="mb-3 flex flex-wrap gap-2">
            <Button
              variant="primary"
              disabled={busy || m.lines.length === 0}
              onClick={() =>
                act(async () => {
                  setBusy(true);
                  try {
                    const r = await api.summarizeMeeting(id);
                    setM({ ...m, ...r });
                  } finally {
                    setBusy(false);
                  }
                  return null;
                })
              }
            >
              {busy
                ? t("files.summarizing")
                : m.summary
                  ? t("meetings.resummarize")
                  : t("files.summarize")}
            </Button>
            <Button
              onClick={() =>
                act(
                  async () => (
                    await api.copyText(
                      m.summary ? `${m.summary}\n\n${text}` : text,
                    ),
                    t("files.copied")
                  ),
                )
              }
            >
              {t("files.copy")}
            </Button>
            <Button
              onClick={() =>
                act(async () => {
                  const path = await api.saveTextFile(
                    `${(m.title || when(m.started_at)).replace(/[/\\:]/g, "-")}.txt`,
                    m.summary ? `${m.summary}\n\n${text}` : text,
                  );
                  return path ? t("files.saved", { path }) : null;
                })
              }
            >
              {t("files.save")}
            </Button>
            <span className="flex-1" />
            {confirm ? (
              <>
                <Button
                  variant="danger"
                  onClick={() => api.deleteMeeting(id).then(onDeleted)}
                >
                  {t("meetings.deleteConfirm")}
                </Button>
                <Button variant="ghost" onClick={() => setConfirm(false)}>
                  {t("common.cancel")}
                </Button>
              </>
            ) : (
              <Button variant="ghost" onClick={() => setConfirm(true)}>
                {t("common.delete")}
              </Button>
            )}
          </div>
          {notice && <Notice tone={notice.tone} text={notice.text} />}
          {m.summary && (
            <div className="selectable mb-4 rounded-[10px] border border-line bg-surface p-4 text-[13px] whitespace-pre-wrap">
              {m.summary}
            </div>
          )}
          {m.actions.length > 0 && (
            <Actions
              actions={m.actions}
              onChange={(actions) => {
                setM({ ...m, actions });
                api.saveMeetingActions(id, actions);
              }}
              onReminders={() =>
                act(async () => {
                  const n = await api.addMeetingReminders(id);
                  return t("meetings.remindersAdded", { n });
                })
              }
              onCopy={() =>
                act(
                  async () => (
                    await api.copyText(actionsMarkdown(m.actions, "")),
                    t("files.copied")
                  ),
                )
              }
            />
          )}
          {m.lines.length === 0 ? (
            <p className="text-muted">{t("files.noSpeech")}</p>
          ) : (
            <div className="selectable max-h-[55vh] space-y-2 overflow-y-auto rounded-[10px] border border-line bg-surface p-4 text-[13px]">
              {m.lines.map((l, i) => (
                <p key={i} className="flex gap-3">
                  <span className="w-12 shrink-0 text-muted tabular-nums">
                    {clock(l.start_ms)}
                  </span>
                  <span
                    className={`w-12 shrink-0 font-medium ${l.speaker === "me" ? "text-cobalt" : ""}`}
                  >
                    {l.speaker === "me" ? me : others}
                  </span>
                  <span>{l.text}</span>
                </p>
              ))}
            </div>
          )}
        </>
      )}
    </section>
  );
}

/** Le cose da fare come elenco Markdown a caselle, con chi e quando. */
function actionsMarkdown(actions: MeetingAction[], title: string) {
  if (actions.length === 0) return "";
  const rows = actions.map((a) => {
    const who = a.who.trim() ? `**${a.who.trim()}**: ` : "";
    const due = a.due.trim() ? ` (${a.due.trim()})` : "";
    return `- [${a.done ? "x" : " "}] ${who}${a.what}${due}`;
  });
  return [title && `## ${title}`, ...rows].filter(Boolean).join("\n");
}

/** Le cose da fare della riunione: si spuntano, si tolgono, si mandano in Promemoria. */
function Actions({
  actions,
  onChange,
  onReminders,
  onCopy,
}: {
  actions: MeetingAction[];
  onChange: (actions: MeetingAction[]) => void;
  onReminders: () => void;
  onCopy: () => void;
}) {
  const t = useT();
  const open = actions.filter((a) => !a.done).length;
  return (
    <div className="mb-4 rounded-[10px] border border-line bg-surface p-4 text-[13px]">
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <h3 className="m-0 flex-1 text-[14px] font-semibold">
          {t("meetings.actions")}{" "}
          <span className="font-normal text-muted">
            {t("meetings.actionsOpen", { n: open })}
          </span>
        </h3>
        {isMac && (
          <Button disabled={open === 0} onClick={onReminders}>
            {t("meetings.toReminders")}
          </Button>
        )}
        <Button variant="ghost" onClick={onCopy}>
          {t("meetings.copyActions")}
        </Button>
      </div>
      <ul className="m-0 list-none space-y-1 p-0">
        {actions.map((a, i) => (
          <li key={i} className="group flex items-start gap-2">
            <input
              type="checkbox"
              className="mt-[3px]"
              checked={a.done}
              aria-label={a.what}
              onChange={() =>
                onChange(
                  actions.map((x, k) =>
                    k === i ? { ...x, done: !x.done } : x,
                  ),
                )
              }
            />
            <span
              className={`flex-1 ${a.done ? "text-muted line-through" : ""}`}
            >
              {a.who.trim() && (
                <span className="mr-1 font-medium">{a.who.trim()}:</span>
              )}
              {a.what}
              {a.due.trim() && (
                <span className="ml-1 text-muted">· {a.due.trim()}</span>
              )}
            </span>
            <button
              type="button"
              aria-label={t("common.delete")}
              className="px-1 text-muted opacity-0 group-hover:opacity-100 hover:text-ink focus-visible:opacity-100"
              onClick={() => onChange(actions.filter((_, k) => k !== i))}
            >
              ×
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Il microfono che registra la tua voce: lo stesso delle impostazioni, scelto qui prima di avviare. */
function MicPicker() {
  const t = useT();
  const { settings, update } = useSettings();
  const [devices, setDevices] = useState<string[]>([]);
  useEffect(() => {
    api.listInputDevices().then(setDevices);
  }, []);
  return (
    <Select
      label={t("general.microphone")}
      value={settings.audio_device ?? ""}
      onChange={(v) => update({ audio_device: v || null })}
      options={[
        ["", t("common.systemMic")],
        ...devices.map((d): [string, string] => [d, d]),
      ]}
    />
  );
}

/** Proposta automatica a inizio chiamata e nome dal calendario. */
function Options() {
  const t = useT();
  const { settings, update } = useSettings();
  const [calendar, setCalendar] = useState<string | null>(null);
  useEffect(() => {
    api.calendarAccess(false).then(setCalendar);
  }, []);
  const toggleCalendar = async (on: boolean) => {
    if (on) {
      const status = await api.calendarAccess(true);
      setCalendar(status);
      if (status !== "granted") return;
    }
    await update({ meeting_calendar: on });
  };
  return (
    <div className="mb-6 divide-y divide-line rounded-[10px] border border-line bg-surface">
      <label className="flex items-center gap-4 px-4 py-3">
        <span className="flex-1">
          <span className="block">{t("meetings.autoDetect")}</span>
          <span className="block text-[12px] text-muted">
            {t("meetings.autoDetectHint")}
          </span>
        </span>
        <Toggle
          label={t("meetings.autoDetect")}
          checked={settings.meeting_auto_detect}
          onChange={(v) => update({ meeting_auto_detect: v })}
        />
      </label>
      <label className="flex items-center gap-4 px-4 py-3">
        <span className="flex-1">
          <span className="block">{t("meetings.calendar")}</span>
          <span
            className={`block text-[12px] ${calendar === "denied" ? "text-warn" : "text-muted"}`}
          >
            {calendar === "denied"
              ? t("meetings.calendarDenied")
              : t("meetings.calendarHint")}
          </span>
        </span>
        <Toggle
          label={t("meetings.calendar")}
          checked={settings.meeting_calendar && calendar === "granted"}
          onChange={toggleCalendar}
        />
      </label>
    </div>
  );
}
