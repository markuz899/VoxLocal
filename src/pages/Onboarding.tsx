import { useEffect, useState } from "react";
import {
  api,
  errorText,
  events,
  type ModelView,
  type PermissionStatus,
} from "../api";
import { useSettings } from "../App";
import { formatBytes, formatHotkey, isMac } from "../format";
import { Button, HotkeyInput, LevelMeter, Notice, Select, Toggle } from "../ui";

type Step = "microphone" | "accessibility" | "model" | "try";

export default function Onboarding({ onDone }: { onDone: () => void }) {
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
        <span>
          Passo {index + 1} di {steps.length}
        </span>
        <div className="h-[3px] flex-1 rounded-full bg-line">
          <div
            className="h-full rounded-full bg-cobalt transition-[width] duration-300"
            style={{ width: `${((index + 1) / steps.length) * 100}%` }}
          />
        </div>
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
          Indietro
        </Button>
        {step === "try" ? (
          <FinishButton onDone={onDone} />
        ) : (
          <Button variant="primary" onClick={next}>
            Continua
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
      <Title>Ti ascolto solo quando vuoi tu</Title>
      <Lead>
        VoxLocal usa il microfono solo mentre tieni premuta la scorciatoia. La
        voce viene trascritta su questo computer e non viene salvata.
      </Lead>
      {perm?.microphone === "denied" ? (
        <div className="space-y-3">
          <Notice tone="error" text="L'accesso al microfono è stato negato." />
          <Button onClick={() => api.openPermissionSettings("microphone")}>
            Apri Impostazioni di Sistema
          </Button>
          <Button
            variant="ghost"
            onClick={async () => setPerm(await api.permissionsStatus())}
          >
            Ho dato il permesso
          </Button>
        </div>
      ) : granted ? (
        <div className="space-y-4">
          <Select
            label="Microfono"
            value={settings.audio_device ?? ""}
            onChange={(v) => update({ audio_device: v || null })}
            options={[
              ["", "Microfono di sistema"],
              ...devices.map((d): [string, string] => [d, d]),
            ]}
          />
          <LevelMeter bars={36} />
          <p className="text-[12px] text-muted">
            Parla: le barre devono muoversi.
          </p>
          <Notice tone="error" text={error} />
        </div>
      ) : (
        <Button variant="primary" onClick={ask}>
          Consenti il microfono
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
  const granted = perm?.accessibility === "granted";

  // Il permesso si concede in Impostazioni di Sistema: ricontrollo finché non arriva.
  useEffect(() => {
    if (granted) return;
    const t = setInterval(
      async () => setPerm(await api.permissionsStatus()),
      1500,
    );
    return () => clearInterval(t);
  }, [granted, setPerm]);

  return (
    <>
      <Title>Scrivere al posto tuo</Title>
      <Lead>
        Per incollare il testo nell'app che stai usando, macOS chiede il
        permesso di Accessibilità. VoxLocal lo usa solo per premere ⌘V dopo un
        dettato.
      </Lead>
      {granted ? (
        <Notice tone="ok" text="Permesso concesso." />
      ) : (
        <div className="space-y-3">
          <ol className="list-decimal space-y-1 pl-5 text-[13px]">
            <li>Premi il pulsante qui sotto.</li>
            <li>
              In Impostazioni di Sistema attiva VoxLocal nell'elenco
              Accessibilità.
            </li>
            <li>Torna qui: questa pagina si aggiorna da sola.</li>
          </ol>
          <div className="flex gap-2 pt-2">
            <Button
              variant="primary"
              onClick={() => api.requestAccessibility()}
            >
              Chiedi il permesso
            </Button>
            <Button onClick={() => api.openPermissionSettings("accessibility")}>
              Apri Impostazioni di Sistema
            </Button>
          </div>
          <p className="pt-2 text-[12px] text-muted">
            Puoi saltare questo passo: il testo dettato resterà negli appunti e
            lo incollerai tu.
          </p>
        </div>
      )}
    </>
  );
}

function ModelStep() {
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
      <Title>Il motore di trascrizione</Title>
      <Lead>
        La trascrizione avviene su questo computer, anche senza internet. Serve
        scaricare il modello una volta sola.
      </Lead>
      {selected && (
        <div className="space-y-4 rounded-[10px] border border-line bg-surface p-4">
          <div className="flex items-center justify-between gap-4">
            <Select
              label="Modello"
              value={settings.local_model}
              onChange={(v) => update({ local_model: v })}
              options={models.map((m): [string, string] => [
                m.id,
                `${m.label}, ${formatBytes(m.size)}${m.recommended ? " (consigliato)" : ""}`,
              ])}
            />
            {selected.installed ? (
              <Notice tone="ok" text="Pronto" />
            ) : progress ? null : (
              <Button variant="primary" onClick={download}>
                Scarica
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
                Scarico… {Math.floor(progress.pct)}%. Puoi continuare intanto.
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
      ? `Fai clic nel riquadro, tieni premuto ${key}, parla e rilascia.`
      : `Fai clic nel riquadro, premi ${key}, parla e premi di nuovo ${key}.`;

  return (
    <>
      <Title>Prova a dettare</Title>
      <Lead>{instruction}</Lead>
      <textarea
        autoFocus
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="Il testo dettato comparirà qui."
        className="h-28 w-full resize-none rounded-[10px] border border-line bg-surface p-3 text-[14px] placeholder:text-muted"
      />
      {text.trim() && (
        <Notice
          tone="ok"
          text="Funziona. Lo stesso succede in qualsiasi app."
        />
      )}
      <div className="mt-6 space-y-3">
        <div className="flex items-center justify-between">
          <span>Scorciatoia</span>
          <div className="flex items-center gap-2">
            <HotkeyInput
              value={settings.hotkey}
              onChange={async (hotkey) => setError(await update({ hotkey }))}
            />
            <Select
              label="Come si attiva"
              value={settings.hotkey_mode}
              onChange={(v) => update({ hotkey_mode: v as "ptt" | "toggle" })}
              options={[
                ["ptt", "Tieni premuto"],
                ["toggle", "Premi per avviare e fermare"],
              ]}
            />
          </div>
        </div>
        <Notice tone="error" text={error} />
        <div className="flex items-center justify-between">
          <span>Apri VoxLocal all'accesso{isMac ? " al Mac" : ""}</span>
          <Toggle
            label="Apri all'accesso"
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
        Inizia a usare VoxLocal
      </Button>
    </div>
  );
}
