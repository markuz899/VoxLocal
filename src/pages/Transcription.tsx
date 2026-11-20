import { useEffect, useState } from "react";
import {
  api,
  errorText,
  events,
  type ModelView,
  type ProviderView,
} from "../api";
import { useSettings } from "../App";
import { formatBytes } from "../format";
import { useT } from "../i18n";
import { Button, Group, Notice, PageTitle, Row, Select } from "../ui";

export default function Transcription() {
  const t = useT();
  const { settings, update } = useSettings();
  const lang = settings.ui_language;
  const [models, setModels] = useState<ModelView[]>([]);
  const [providers, setProviders] = useState<ProviderView[]>([]);
  const [progress, setProgress] = useState<Record<string, number>>({});
  const [error, setError] = useState<string | null>(null);

  const refresh = () => api.listModels().then(setModels);

  useEffect(() => {
    refresh();
    api.listProviders().then(setProviders);
    const offs = [
      events.modelProgress((p) =>
        setProgress({ [p.id]: (p.done / p.total) * 100 }),
      ),
      events.modelDone((d) => {
        setProgress({});
        setError(d.error);
        refresh();
      }),
    ];
    return () => offs.forEach((p) => p.then((off) => off()));
  }, []);

  const run = async (f: () => Promise<unknown>) => {
    setError(null);
    try {
      await f();
    } catch (e) {
      setError(errorText(e));
    }
    refresh();
  };

  const cloud = providers.filter((p) => p.stt_model);
  const engine = settings.stt_provider;
  const cloudSelected = cloud.find((p) => p.id === engine);
  const keepWarm: [string, string][] = [
    ["0", t("stt.releaseNow")],
    ["2", t("stt.minutes", { n: 2 })],
    ["10", t("stt.minutes", { n: 10 })],
    ["30", t("stt.minutes", { n: 30 })],
    ["-1", t("stt.always")],
  ];

  return (
    <>
      <PageTitle title={t("stt.title")} description={t("stt.description")} />
      <Notice tone="error" text={error} />

      <Group>
        <Row label={t("stt.engine")}>
          <Select
            label={t("stt.engine")}
            value={engine}
            onChange={async (v) => setError(await update({ stt_provider: v }))}
            options={[
              ["local", t("stt.local")],
              ...cloud.map((p): [string, string] => [
                p.id,
                t("stt.cloud", { name: p.label }),
              ]),
            ]}
          />
        </Row>
        {cloudSelected && !cloudSelected.has_key && (
          <Row
            label={
              <span className="text-warn">
                {t("stt.missingKey", { name: cloudSelected.label })}
              </span>
            }
            hint={t("stt.missingKeyHint")}
          >
            <Button onClick={() => (window.location.hash = "providers")}>
              {t("stt.addKey")}
            </Button>
          </Row>
        )}
        <Row label={t("stt.keepWarm")} hint={t("stt.keepWarmHint")}>
          <Select
            label={t("stt.keepWarm")}
            value={String(settings.keep_warm_minutes)}
            onChange={async (v) =>
              setError(await update({ keep_warm_minutes: Number(v) }))
            }
            options={keepWarm}
          />
        </Row>
      </Group>

      <Group title={t("stt.models")}>
        {models.map((m) => {
          const inUse = m.id === settings.local_model;
          const pct = progress[m.id];
          const note = t(`model.${m.id}` as Parameters<typeof t>[0]);
          return (
            <Row
              key={m.id}
              label={
                <span className={inUse ? "font-medium" : ""}>
                  {m.label}
                  {m.recommended && (
                    <span className="ml-2 text-[12px] text-cobalt">
                      {t("common.recommended")}
                    </span>
                  )}
                </span>
              }
              hint={
                pct !== undefined
                  ? t("stt.downloading", { pct: Math.floor(pct) })
                  : `${note}, ${formatBytes(m.size, lang)}`
              }
            >
              {inUse && m.installed ? (
                <span className="text-[12px] text-muted">
                  {t("common.inUse")}
                </span>
              ) : m.installed ? (
                <>
                  <Button
                    variant="ghost"
                    onClick={() => run(() => api.deleteModel(m.id))}
                  >
                    {t("common.delete")}
                  </Button>
                  <Button
                    onClick={() => run(() => update({ local_model: m.id }))}
                  >
                    {t("common.use")}
                  </Button>
                </>
              ) : (
                <Button
                  disabled={pct !== undefined}
                  onClick={() =>
                    run(async () => {
                      await api.downloadModel(m.id);
                      setProgress({ [m.id]: 0 });
                    })
                  }
                >
                  {t("common.download")}
                </Button>
              )}
            </Row>
          );
        })}
      </Group>
    </>
  );
}
