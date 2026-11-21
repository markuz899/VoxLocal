import { useEffect, useState } from "react";
import { api, type Stats as StatsData } from "../api";
import { useSettings } from "../App";
import { useT } from "../i18n";
import { PageTitle, Select } from "../ui";

/** Velocità di battitura di riferimento: un adulto medio scrive circa 40 parole al minuto. */
const TYPING_WPM = 40;

type Period = "7" | "30" | "all";

function startOf(period: Period): { since: number; days: number } {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  if (period === "all") return { since: 0, days: 30 };
  const days = Number(period);
  return { since: today.getTime() - (days - 1) * 86_400_000, days };
}

export default function Stats() {
  const t = useT();
  const { settings } = useSettings();
  const lang = settings.ui_language;
  const locale = lang === "it" ? "it-IT" : "en-GB";
  const [period, setPeriod] = useState<Period>("30");
  const [data, setData] = useState<StatsData | null>(null);

  useEffect(() => {
    const { since, days } = startOf(period);
    api.getStats(since, days).then(setData, () => setData(null));
  }, [period]);

  const num = (n: number) => n.toLocaleString(locale);
  const duration = (ms: number) => {
    const mins = Math.max(0, Math.round(ms / 60_000));
    return mins >= 60
      ? t("stats.hours", { h: Math.floor(mins / 60), m: mins % 60 })
      : t("stats.minutes", { m: mins });
  };

  const speakingMin = (data?.audio_ms ?? 0) / 60_000;
  const wpm =
    data && speakingMin > 0 ? Math.round(data.words / speakingMin) : 0;
  // Tempo che servirebbe a scriverle meno il tempo passato a parlarle.
  const savedMs = data
    ? Math.max(0, (data.words / TYPING_WPM) * 60_000 - data.audio_ms)
    : 0;
  const faster =
    wpm > 0
      ? (wpm / TYPING_WPM).toLocaleString(locale, { maximumFractionDigits: 1 })
      : null;

  return (
    <>
      <div className="flex items-start justify-between gap-4">
        <PageTitle
          title={t("stats.title")}
          description={t("stats.description")}
        />
        <Select
          label={t("stats.period")}
          value={period}
          onChange={(v) => setPeriod(v as Period)}
          options={[
            ["7", t("stats.week")],
            ["30", t("stats.month")],
            ["all", t("stats.all")],
          ]}
        />
      </div>

      {!data || data.dictations === 0 ? (
        <p className="text-muted">{t("stats.empty")}</p>
      ) : (
        <>
          <dl className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Tile label={t("stats.words")} value={num(data.words)} />
            <Tile
              label={t("stats.saved")}
              value={duration(savedMs)}
              hint={t("stats.savedHint", { wpm: TYPING_WPM })}
            />
            <Tile
              label={t("stats.speed")}
              value={t("stats.wpm", { n: wpm })}
              hint={faster ? t("stats.faster", { x: faster }) : undefined}
            />
            <Tile label={t("stats.dictations")} value={num(data.dictations)} />
          </dl>
          <DayChart days={data.days} lang={lang} title={t("stats.perDay")} />
          {data.apps.length > 0 && (
            <AppBars
              apps={data.apps}
              title={t("stats.byApp")}
              locale={locale}
            />
          )}
        </>
      )}
    </>
  );
}

function Tile({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="rounded-[10px] border border-line bg-surface p-4">
      <dt className="text-[12px] text-muted">{label}</dt>
      <dd className="m-0 mt-1 text-[24px] font-semibold tracking-tight">
        {value}
      </dd>
      {hint && <p className="mt-1 text-[12px] text-muted">{hint}</p>}
    </div>
  );
}

/** Parole al giorno: barre sottili ancorate alla base, 2px tra l'una e l'altra, tooltip al passaggio. */
function DayChart({
  days,
  lang,
  title,
}: {
  days: number[];
  lang: string;
  title: string;
}) {
  const t = useT();
  const [hover, setHover] = useState<number | null>(null);
  const locale = lang === "it" ? "it-IT" : "en-GB";
  const max = Math.max(1, ...days);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const dayLabel = (i: number) =>
    new Date(
      today.getTime() - (days.length - 1 - i) * 86_400_000,
    ).toLocaleDateString(locale, {
      day: "numeric",
      month: "short",
    });

  return (
    <section className="mb-8">
      <h2 className="mb-3 text-[15px] font-semibold">{title}</h2>
      <div className="rounded-[10px] border border-line bg-surface p-4">
        <div
          className="relative flex h-40 items-end gap-[2px] border-b border-line"
          role="img"
          aria-label={`${title}: ${days.map((n, i) => `${dayLabel(i)} ${n}`).join(", ")}`}
          onMouseLeave={() => setHover(null)}
        >
          {days.map((n, i) => (
            // L'area sensibile è tutta la colonna, più grande della barra.
            <div
              key={i}
              className="flex h-full flex-1 items-end"
              onMouseEnter={() => setHover(i)}
            >
              <div
                className={`w-full rounded-t-[4px] ${hover === i ? "bg-cobalt" : "bg-cobalt/75"}`}
                style={{
                  height: n > 0 ? `${Math.max(2, (n / max) * 100)}%` : 0,
                }}
              />
            </div>
          ))}
          {hover !== null && (
            <div
              className="pointer-events-none absolute -top-2 z-10 -translate-x-1/2 -translate-y-full rounded-md border border-line bg-surface px-2 py-1 text-[12px] whitespace-nowrap shadow"
              style={{ left: `${((hover + 0.5) / days.length) * 100}%` }}
            >
              {t("stats.dayWords", {
                day: dayLabel(hover),
                n: days[hover].toLocaleString(locale),
              })}
            </div>
          )}
        </div>
        <div className="mt-1.5 flex justify-between text-[12px] text-muted">
          <span>{dayLabel(0)}</span>
          <span>{dayLabel(days.length - 1)}</span>
        </div>
        {/* La stessa informazione come tabella, per chi usa uno screen reader. */}
        <table className="sr-only">
          <thead>
            <tr>
              <th>{t("stats.tableDay")}</th>
              <th>{t("stats.tableWords")}</th>
            </tr>
          </thead>
          <tbody>
            {days.map((n, i) => (
              <tr key={i}>
                <td>{dayLabel(i)}</td>
                <td>{n}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

/** Parole per app: barre orizzontali, il valore in testo accanto (non serve colore per leggerlo). */
function AppBars({
  apps,
  title,
  locale,
}: {
  apps: [string, number][];
  title: string;
  locale: string;
}) {
  const max = Math.max(1, ...apps.map(([, n]) => n));
  return (
    <section>
      <h2 className="mb-3 text-[15px] font-semibold">{title}</h2>
      <ul className="space-y-2 rounded-[10px] border border-line bg-surface p-4">
        {apps.map(([app, n]) => (
          <li
            key={app}
            className="grid grid-cols-[120px_1fr_auto] items-center gap-3 text-[13px]"
          >
            <span className="truncate">{app}</span>
            <span
              className="h-2.5 rounded-r-[4px] bg-cobalt/75"
              style={{ width: `${(n / max) * 100}%` }}
            />
            <span className="text-muted tabular-nums">
              {n.toLocaleString(locale)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
