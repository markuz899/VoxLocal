import { useEffect, useState } from "react";
import { api, errorText, type ProviderView } from "../api";
import { useSettings } from "../App";
import { formatMs } from "../format";
import {
  Button,
  Group,
  Notice,
  PageTitle,
  Row,
  Select,
  TextInput,
} from "../ui";

function keyState(p: ProviderView): { text: string; tone: string } {
  if (p.kind === "cli")
    return { text: "Abbonamento tramite CLI", tone: "text-muted" };
  if (!p.needs_key)
    return { text: "In locale, nessuna chiave", tone: "text-muted" };
  return p.has_key
    ? { text: "Chiave salvata", tone: "text-ok" }
    : { text: "Chiave mancante", tone: "text-warn" };
}

export default function Providers() {
  const { settings, update } = useSettings();
  const [providers, setProviders] = useState<ProviderView[]>([]);
  const [open, setOpen] = useState<string | null>(null);

  const reload = () => api.listProviders().then(setProviders);
  useEffect(() => {
    reload();
  }, []);

  const usable = providers.filter((p) => p.kind !== "none");

  return (
    <>
      <PageTitle
        title="Provider LLM"
        description="Il provider che riscrive il testo nelle modalità Pulito, Email e simili. Le chiavi restano nel portachiavi del sistema."
      />
      <Group>
        <Row
          label="Provider predefinito"
          hint="Usato dalle modalità che non ne indicano uno."
        >
          <Select
            label="Provider predefinito"
            value={settings.default_llm}
            onChange={(default_llm) => update({ default_llm })}
            options={usable.map((p): [string, string] => [p.id, p.label])}
          />
        </Row>
      </Group>
      <Group title="Provider">
        {usable.map((p) =>
          open === p.id ? (
            <ProviderEditor
              key={p.id}
              provider={p}
              onClose={() => setOpen(null)}
              onSaved={reload}
            />
          ) : (
            <Row
              key={p.id}
              label={p.label}
              hint={
                <span className={keyState(p).tone}>{keyState(p).text}</span>
              }
            >
              <Button variant="ghost" onClick={() => setOpen(p.id)}>
                Configura
              </Button>
            </Row>
          ),
        )}
      </Group>
    </>
  );
}

function ProviderEditor({
  provider,
  onClose,
  onSaved,
}: {
  provider: ProviderView;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [baseUrl, setBaseUrl] = useState(provider.base_url);
  const [model, setModel] = useState(provider.model);
  const [key, setKey] = useState("");
  const [notice, setNotice] = useState<{
    text: string;
    tone: "ok" | "error" | "info";
  } | null>(null);

  const act = async (f: () => Promise<string>) => {
    setNotice({ text: "…", tone: "info" });
    try {
      setNotice({ text: await f(), tone: "ok" });
      onSaved();
    } catch (e) {
      setNotice({ text: errorText(e), tone: "error" });
    }
  };

  const save = () =>
    act(async () => {
      if (baseUrl !== provider.base_url || model !== provider.model) {
        const { has_key: _ignored, ...config } = provider;
        await api.saveProvider({ ...config, base_url: baseUrl, model });
      }
      if (key.trim()) {
        await api.setApiKey(provider.id, key);
        setKey("");
      }
      return "Salvato.";
    });

  const isCli = provider.kind === "cli";

  return (
    <div className="space-y-3 bg-paper/50 px-4 py-4">
      <div className="flex items-center justify-between">
        <span className="font-medium">{provider.label}</span>
        <Button variant="ghost" onClick={onClose}>
          Chiudi
        </Button>
      </div>
      {!isCli && (
        <label className="block">
          <span className="mb-1 block text-[12px] text-muted">
            Indirizzo API
          </span>
          <TextInput
            className="w-full"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
          />
        </label>
      )}
      <label className="block">
        <span className="mb-1 block text-[12px] text-muted">Modello</span>
        <TextInput
          className="w-full"
          value={model}
          placeholder={isCli ? "quello predefinito del CLI" : ""}
          onChange={(e) => setModel(e.target.value)}
        />
      </label>
      {provider.needs_key && (
        <label className="block">
          <span className="mb-1 block text-[12px] text-muted">
            {provider.has_key ? "Sostituisci la chiave API" : "Chiave API"}
          </span>
          <TextInput
            className="w-full"
            type="password"
            autoComplete="off"
            value={key}
            placeholder={
              provider.has_key ? "salvata nel portachiavi" : "incolla la chiave"
            }
            onChange={(e) => setKey(e.target.value)}
          />
        </label>
      )}
      <div className="flex flex-wrap items-center gap-2 pt-1">
        <Button variant="primary" onClick={save}>
          Salva
        </Button>
        <Button
          onClick={() =>
            act(
              async () =>
                `Raggiungibile in ${formatMs(await api.checkProvider(provider.id))}.`,
            )
          }
        >
          Verifica
        </Button>
        {provider.has_key && (
          <Button
            variant="danger"
            onClick={() =>
              act(async () => {
                await api.deleteApiKey(provider.id);
                return "Chiave rimossa.";
              })
            }
          >
            Rimuovi chiave
          </Button>
        )}
      </div>
      {notice && <Notice tone={notice.tone} text={notice.text} />}
    </div>
  );
}
