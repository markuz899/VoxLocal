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
import { Button, Group, Notice, PageTitle, Row, Select } from "../ui";

// Cosa aspettarsi da ogni modello, misurato su un M1 (DECISIONS D27).
const NOTES: Record<string, string> = {
  tiny: "Il più veloce, sbaglia spesso",
  "tiny-q5_1": "Il più leggero, sbaglia spesso",
  base: "Veloce, impreciso con la voce reale",
  "base-q5_1": "Veloce e leggero, impreciso",
  small: "Equilibrato",
  "small-q5_1": "Equilibrato e leggero",
  "medium-q5_0": "Preciso, più lento",
  "large-v3-turbo-q5_0": "Il più preciso a una velocità da dettatura",
  "large-v3-turbo": "Il più preciso, richiede più memoria",
};

const KEEP_WARM: [string, string][] = [
  ["0", "Rilascia subito"],
  ["2", "2 minuti"],
  ["10", "10 minuti"],
  ["30", "30 minuti"],
  ["-1", "Sempre"],
];

export default function Transcription() {
  const { settings, update } = useSettings();
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

  return (
    <>
      <PageTitle
        title="Trascrizione"
        description="Il modello locale funziona offline e la voce non lascia il computer. I servizi cloud sono più leggeri per il computer ma inviano l'audio al provider."
      />
      <Notice tone="error" text={error} />

      <Group>
        <Row label="Motore">
          <Select
            label="Motore di trascrizione"
            value={engine}
            onChange={async (v) => setError(await update({ stt_provider: v }))}
            options={[
              ["local", "Locale sul computer"],
              ...cloud.map((p): [string, string] => [
                p.id,
                `${p.label} (cloud)`,
              ]),
            ]}
          />
        </Row>
        {cloudSelected && !cloudSelected.has_key && (
          <Row
            label={
              <span className="text-warn">
                Manca la chiave di {cloudSelected.label}
              </span>
            }
            hint="Finché manca, VoxLocal usa il modello locale."
          >
            <Button onClick={() => (window.location.hash = "providers")}>
              Aggiungi chiave
            </Button>
          </Row>
        )}
        <Row
          label="Tieni il modello in memoria"
          hint="Più a lungo è caldo, più è veloce il dettato successivo."
        >
          <Select
            label="Tieni il modello in memoria"
            value={String(settings.keep_warm_minutes)}
            onChange={async (v) =>
              setError(await update({ keep_warm_minutes: Number(v) }))
            }
            options={KEEP_WARM}
          />
        </Row>
      </Group>

      <Group title="Modelli locali">
        {models.map((m) => {
          const inUse = m.id === settings.local_model;
          const pct = progress[m.id];
          return (
            <Row
              key={m.id}
              label={
                <span className={inUse ? "font-medium" : ""}>
                  {m.label}
                  {m.recommended && (
                    <span className="ml-2 text-[12px] text-cobalt">
                      consigliato
                    </span>
                  )}
                </span>
              }
              hint={
                pct !== undefined
                  ? `Scarico… ${Math.floor(pct)}%`
                  : `${NOTES[m.id] ?? ""}, ${formatBytes(m.size)}`
              }
            >
              {inUse && m.installed ? (
                <span className="text-[12px] text-muted">In uso</span>
              ) : m.installed ? (
                <>
                  <Button
                    variant="ghost"
                    onClick={() => run(() => api.deleteModel(m.id))}
                  >
                    Elimina
                  </Button>
                  <Button
                    onClick={() => run(() => update({ local_model: m.id }))}
                  >
                    Usa
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
                  Scarica
                </Button>
              )}
            </Row>
          );
        })}
      </Group>
    </>
  );
}
