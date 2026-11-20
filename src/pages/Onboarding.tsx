import { useEffect, useState } from "react";
import {
  api,
  errorText,
  events,
  type ModelView,
  type PermissionStatus,
} from "../api";
import { useSettings } from "../App";
import { formatBytes, formatHotkey } from "../format";
import { UI_LANGUAGES, useT } from "../i18n";
import { Button, HotkeyInput, LevelMeter, Notice, Select, Toggle } from "../ui";

type Step = "microphone" | "accessibility" | "model" | "try";

export default function Onboarding({ onDone }: { onDone: () => void }) {
  const t = useT();
  const { settings, update } = useSettings();
  const [perm, setPerm] = useState<PermissionStatus | null>(null);
  const steps: Step[] = perm?.needs_accessibility
    ? ["microphone", "accessibility", "model", "try"]
    : ["microphone", "model", "try"];
  const [index, setIndex] = useState(0);
  const step = steps[Math.min(index, steps.length - 1)];

  useEffect(() => {
    api.permissionsStatus().then(setPerm);
  }, []);

  const next = () => setIndex((i) => Math.min(i + 1, steps.length - 1));
  const back = () => setIndex((i) => Math.max(i - 1, 0));

  return (
    <div className="mx-auto flex h-full max-w-[560px] flex-col px-8 pt-12 pb-8">
      <div className="mb-8 flex items-center gap-3 text-[12px] text-muted">
        <span>{t("onb.step", { n: index + 1, total: steps.length })}</span>
        <div className="h-[3px] flex-1 rounded-full bg-line">
          <div
            className="h-full rounded-full bg-cobalt transition-[width] duration-300"
            style={{ width: `${((index + 1) / steps.length) * 100}%` }}
          />
        </div>
        {/* La lingua si sceglie subito, prima di leggere il resto. */}
        <Select
          label={t("general.uiLanguage")}
          value={settings.ui_language}
          onChange={(ui_language) => update({ ui_language })}
          options={UI_LANGUAGES}
        />
      </div>
      <div className="flex-1">
        {step === "microphone" && (
          <MicrophoneStep perm={perm} setPerm={setPerm} />
        )}
        {step === "accessibility" && (
          <AccessibilityStep perm={perm} setPerm={setPerm} />
        )}
        {step === "model" && <ModelStep />}
        {step === "try" && <TryStep />}
      </div>
      <footer className="flex items-center justify-between pt-6">
        <Button variant="ghost" onClick={back} disabled={index === 0}>
          {t("common.back")}
        </Button>
        {step === "try" ? (
          <FinishButton onDone={onDone} />
        ) : (
          <Button variant="primary" onClick={next}>
            {t("common.continue")}
          </Button>
        )}
      </footer>
    </div>
  );
}

function Title({ children }: { children: React.ReactNode }) {
  return (
    <h1 className="mb-3 text-[26px] leading-tight font-semibold tracking-tight">
      {children}
    </h1>
  );
}

function Lead({ children }: { children: React.ReactNode }) {
  return <p className="mb-6 max-w-[52ch] text-[14px] text-muted">{children}</p>;
}

function MicrophoneStep({
  perm,
  setPerm,
}: {
  perm: PermissionStatus | null;
  setPerm: (p: PermissionStatus) => void;
}) {
  const t = useT();
  const { settings, update } = useSettings();
  const [devices, setDevices] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const granted =
    perm?.microphone === "granted" || perm?.microphone === "unknown";

  useEffect(() => {
    api.listInputDevices().then(setDevices);
  }, []);

  // Il misuratore resta acceso finché si è su questo passo, così si vede che il microfono sente.
  useEffect(() => {
    if (!granted) return;
    api.micTest(true).catch((e) => setError(errorText(e)));
    return () => {
      api.micTest(false);
    };
  }, [granted, settings.audio_device]);

  const ask = async () => {
    await api.requestMicrophone();
    setPerm(await api.permissionsStatus());
  };

  return (
    <>
      <Title>{t("onb.mic.title")}</Title>
      <Lead>{t("onb.mic.lead")}</Lead>
      {perm?.microphone === "denied" ? (
        <div className="space-y-3">
          <Notice tone="error" text={t("onb.mic.denied")} />
          <Button onClick={() => api.openPermissionSettings("microphone")}>
            {t("onb.mic.openSettings")}
          </Button>
          <Button
            variant="ghost"
            onClick={async () => setPerm(await api.permissionsStatus())}
          >
            {t("onb.mic.granted")}
          </Button>
        </div>
      ) : granted ? (
        <div className="space-y-4">
          <Select
            label={t("general.microphone")}
            value={settings.audio_device ?? ""}
            onChange={(v) => update({ audio_device: v || null })}
            options={[
              ["", t("common.systemMic")],
              ...devices.map((d): [string, string] => [d, d]),
            ]}
          />
          <LevelMeter bars={36} />
          <p className="text-[12px] text-muted">{t("onb.mic.speak")}</p>
          <Notice tone="error" text={error} />
        </div>
      ) : (
        <Button variant="primary" onClick={ask}>
          {t("onb.mic.allow")}
        </Button>
      )}
    </>
  );
}

function AccessibilityStep({
  perm,
  setPerm,
}: {
  perm: PermissionStatus | null;
  setPerm: (p: PermissionStatus) => void;
}) {
  const t = useT();
  const granted = perm?.accessibility === "granted";

  // Il permesso si concede in Impostazioni di Sistema: ricontrollo finché non arriva.
  useEffect(() => {
    if (granted) return;
    const timer = setInterval(
      async () => setPerm(await api.permissionsStatus()),
      1500,
    );
    return () => clearInterval(timer);
  }, [granted, setPerm]);

  return (
    <>
      <Title>{t("onb.ax.title")}</Title>
      <Lead>{t("onb.ax.lead")}</Lead>
      {granted ? (
        <Notice tone="ok" text={t("onb.ax.granted")} />
      ) : (
        <div className="space-y-3">
          <ol className="list-decimal space-y-1 pl-5 text-[13px]">
            <li>{t("onb.ax.step1")}</li>
            <li>{t("onb.ax.step2")}</li>
            <li>{t("onb.ax.step3")}</li>
          </ol>
          <div className="flex gap-2 pt-2">
            <Button
              variant="primary"
              onClick={() => api.requestAccessibility()}
            >
              {t("onb.ax.ask")}
            </Button>
            <Button onClick={() => api.openPermissionSettings("accessibility")}>
              {t("onb.mic.openSettings")}
            </Button>
          </div>
          <p className="pt-2 text-[12px] text-muted">{t("onb.ax.skip")}</p>
        </div>
      )}
    </>
  );
}

function ModelStep() {
  const t = useT();
  const { settings, update } = useSettings();
  const [models, setModels] = useState<ModelView[]>([]);
  const [progress, setProgress] = useState<{ id: string; pct: number } | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);

  const refresh = () => api.listModels().then(setModels);

  useEffect(() => {
    refresh();
    const offs = [
      events.modelProgress((p) =>
        setProgress({ id: p.id, pct: (p.done / p.total) * 100 }),
      ),
      events.modelDone((d) => {
        setProgress(null);
        setError(d.error);
        refresh();
      }),
    ];
    return () => offs.forEach((p) => p.then((off) => off()));
  }, []);

  // Al primo passaggio propongo il modello consigliato per questo computer.
  useEffect(() => {
    const rec = models.find((m) => m.recommended);
    const current = models.find((m) => m.id === settings.local_model);
    if (rec && current && !current.installed && rec.id !== current.id) {
      update({ local_model: rec.id });
    }
    // Solo quando arriva l'elenco: dopo, la scelta è dell'utente.
  }, [models]);

  const selected = models.find((m) => m.id === settings.local_model);
  const download = async () => {
    setError(null);
    try {
      await api.downloadModel(settings.local_model);
      setProgress({ id: settings.local_model, pct: 0 });
    } catch (e) {
      setError(errorText(e));
    }
  };

  return (
    <>
      <Title>{t("onb.model.title")}</Title>
      <Lead>{t("onb.model.lead")}</Lead>
      {selected && (
        <div className="space-y-4 rounded-[10px] border border-line bg-surface p-4">
          <div className="flex items-center justify-between gap-4">
            <Select
              label={t("modes.model")}
              value={settings.local_model}
              onChange={(v) => update({ local_model: v })}
              options={models.map((m): [string, string] => [
                m.id,
                `${m.label}, ${formatBytes(m.size, settings.ui_language)}${m.recommended ? ` (${t("common.recommended")})` : ""}`,
              ])}
            />
            {selected.installed ? (
              <Notice tone="ok" text={t("common.ready")} />
            ) : progress ? null : (
              <Button variant="primary" onClick={download}>
                {t("common.download")}
              </Button>
            )}
          </div>
          {progress && (
            <div>
              <div className="h-[6px] rounded-full bg-line">
                <div
                  className="h-full rounded-full bg-cobalt"
                  style={{ width: `${progress.pct}%` }}
                />
              </div>
              <p className="mt-2 text-[12px] text-muted">
                {t("onb.model.downloading", { pct: Math.floor(progress.pct) })}
              </p>
            </div>
          )}
          <Notice tone="error" text={error} />
        </div>
      )}
    </>
  );
}

function TryStep() {
  const t = useT();
  const { settings, update } = useSettings();
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    api.getAutostart().then(setAutostart);
  }, []);

  const key = formatHotkey(settings.hotkey);
  const instruction =
    settings.hotkey_mode === "ptt"
      ? t("onb.try.ptt", { key })
      : t("onb.try.toggle", { key });

  return (
    <>
      <Title>{t("onb.try.title")}</Title>
      <Lead>{instruction}</Lead>
      <textarea
        autoFocus
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={t("onb.try.placeholder")}
        className="h-28 w-full resize-none rounded-[10px] border border-line bg-surface p-3 text-[14px] placeholder:text-muted"
      />
      {text.trim() && <Notice tone="ok" text={t("onb.try.works")} />}
      <div className="mt-6 space-y-3">
        <div className="flex items-center justify-between">
          <span>{t("general.shortcut")}</span>
          <div className="flex items-center gap-2">
            <HotkeyInput
              value={settings.hotkey}
              onChange={async (hotkey) => setError(await update({ hotkey }))}
            />
            <Select
              label={t("general.activation")}
              value={settings.hotkey_mode}
              onChange={(v) => update({ hotkey_mode: v as "ptt" | "toggle" })}
              options={[
                ["ptt", t("general.ptt")],
                ["toggle", t("general.toggle")],
              ]}
            />
          </div>
        </div>
        <Notice tone="error" text={error} />
        <div className="flex items-center justify-between">
          <span>{t("onb.try.atLogin")}</span>
          <Toggle
            label={t("onb.try.atLogin")}
            checked={autostart}
            onChange={async (v) => {
              await api.setAutostart(v);
              setAutostart(await api.getAutostart());
            }}
          />
        </div>
      </div>
    </>
  );
}

function FinishButton({ onDone }: { onDone: () => void }) {
  const t = useT();
  const [error, setError] = useState<string | null>(null);
  return (
    <div className="flex items-center gap-3">
      <Notice tone="error" text={error} />
      <Button
        variant="primary"
        onClick={async () => {
          try {
            await api.finishOnboarding();
            onDone();
          } catch (e) {
            setError(errorText(e));
          }
        }}
      >
        {t("onb.finish")}
      </Button>
    </div>
  );
}
