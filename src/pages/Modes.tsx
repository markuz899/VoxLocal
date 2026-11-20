import { useEffect, useState } from "react";
import { api, errorText, type Mode, type ProviderView } from "../api";
import { useSettings } from "../App";
import { Button, Notice, PageTitle, Select, TextInput } from "../ui";

const BUILTIN = ["raw", "clean", "email", "message", "bullets", "translate_en"];

export default function Modes() {
  const { settings, update } = useSettings();
  const [modes, setModes] = useState<Mode[]>([]);
  const [providers, setProviders] = useState<ProviderView[]>([]);
  const [slug, setSlug] = useState(settings.active_mode);
  const [draft, setDraft] = useState<Mode | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [notice, setNotice] = useState<{
    text: string;
    tone: "ok" | "error";
  } | null>(null);

  const reload = async (select?: string) => {
    const list = await api.listModes();
    setModes(list);
    const pick = list.find((m) => m.slug === (select ?? slug)) ?? list[0];
    setSlug(pick.slug);
    setDraft(pick);
  };

  useEffect(() => {
    reload();
    api.listProviders().then(setProviders);
    // Solo al primo caricamento.
  }, []);

  const effectiveProvider = draft?.provider_id ?? settings.default_llm;

  // Suggerimenti per il campo modello, letti dal provider (se raggiungibile).
  useEffect(() => {
    setModels([]);
    if (!effectiveProvider || effectiveProvider === "none") return;
    api
      .listProviderModels(effectiveProvider)
      .then(setModels, () => setModels([]));
  }, [effectiveProvider]);

  if (!draft) return null;

  const builtin = BUILTIN.includes(draft.slug);
  const isRaw = draft.provider_id === "none";
  const defaultLabel =
    providers.find((p) => p.id === settings.default_llm)?.label ??
    settings.default_llm;
  const dirty =
    JSON.stringify(draft) !==
    JSON.stringify(modes.find((m) => m.slug === draft.slug));

  const act = async (f: () => Promise<unknown>, ok: string) => {
    try {
      await f();
      setNotice({ text: ok, tone: "ok" });
    } catch (e) {
      setNotice({ text: errorText(e), tone: "error" });
    }
  };

  const save = () =>
    act(async () => {
      await api.updateMode(draft);
      await reload(draft.slug);
    }, "Modalità salvata.");

  const select = (m: Mode) => {
    setSlug(m.slug);
    setDraft(m);
    setNotice(null);
  };

  return (
    <>
      <PageTitle
        title="Modalità"
        description="Ogni modalità decide cosa succede al testo dopo la trascrizione. Si sceglie anche dal menu nella barra."
      />
      <div className="flex gap-6">
        <div className="w-44 shrink-0">
          <ul className="divide-y divide-line rounded-[10px] border border-line bg-surface">
            {modes.map((m) => (
              <li key={m.slug}>
                <button
                  type="button"
                  onClick={() => select(m)}
                  className={`flex w-full items-center justify-between px-3 py-2 text-left ${
                    m.slug === slug ? "bg-cobalt-soft text-cobalt" : ""
                  }`}
                >
                  <span className="truncate">{m.name}</span>
                  {m.slug === settings.active_mode && (
                    <span
                      className="ml-2 h-1.5 w-1.5 shrink-0 rounded-full bg-cobalt"
                      title="Attiva"
                    />
                  )}
                </button>
              </li>
            ))}
          </ul>
          <Button
            className="mt-3 w-full"
            onClick={() =>
              act(async () => {
                const s = await api.createMode("Nuova modalità");
                await reload(s);
              }, "Modalità creata.")
            }
          >
            Nuova modalità
          </Button>
        </div>

        <div className="min-w-0 flex-1 space-y-4">
          <label className="block">
            <span className="mb-1 block text-[12px] text-muted">Nome</span>
            <TextInput
              className="w-full"
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </label>

          <div className="flex gap-3">
            <label className="flex-1">
              <span className="mb-1 block text-[12px] text-muted">
                Elaborazione
              </span>
              <Select
                label="Provider"
                className="w-full max-w-none"
                value={draft.provider_id ?? ""}
                onChange={(v) => setDraft({ ...draft, provider_id: v || null })}
                options={[
                  ["", `Predefinito: ${defaultLabel}`],
                  ["none", "Nessuna: incolla il testo trascritto"],
                  ...providers
                    .filter((p) => p.kind !== "none")
                    .map((p): [string, string] => [p.id, p.label]),
                ]}
              />
            </label>
            {!isRaw && (
              <label className="w-48">
                <span className="mb-1 block text-[12px] text-muted">
                  Modello
                </span>
                <TextInput
                  className="w-full"
                  list="mode-models"
                  placeholder="quello del provider"
                  value={draft.model ?? ""}
                  onChange={(e) =>
                    setDraft({ ...draft, model: e.target.value || null })
                  }
                />
                <datalist id="mode-models">
                  {models.map((m) => (
                    <option key={m} value={m} />
                  ))}
                </datalist>
              </label>
            )}
          </div>

          {!isRaw && (
            <>
              <label className="block">
                <span className="mb-1 block text-[12px] text-muted">
                  Istruzioni per l'LLM. Variabili: {"{{language}}"},{" "}
                  {"{{vocabulary}}"}
                </span>
                <textarea
                  spellCheck={false}
                  value={draft.system_prompt}
                  onChange={(e) =>
                    setDraft({ ...draft, system_prompt: e.target.value })
                  }
                  className="h-44 w-full resize-y rounded-[10px] border border-line bg-surface p-3 text-[13px] leading-relaxed"
                />
              </label>
              <label className="block">
                <span className="mb-1 block text-[12px] text-muted">
                  Messaggio inviato. {"{{text}}"} è il testo dettato.
                </span>
                <textarea
                  spellCheck={false}
                  rows={3}
                  value={draft.user_template}
                  onChange={(e) =>
                    setDraft({ ...draft, user_template: e.target.value })
                  }
                  className="w-full resize-y rounded-[10px] border border-line bg-surface p-3 font-mono text-[12px]"
                />
              </label>
            </>
          )}

          <div className="flex flex-wrap items-center gap-2 pt-2">
            <Button variant="primary" disabled={!dirty} onClick={save}>
              Salva
            </Button>
            {draft.slug !== settings.active_mode && (
              <Button
                onClick={() =>
                  act(async () => {
                    const err = await update({ active_mode: draft.slug });
                    if (err) throw err;
                  }, `${draft.name} è la modalità attiva.`)
                }
              >
                Usa questa modalità
              </Button>
            )}
            <span className="flex-1" />
            {builtin ? (
              <Button
                variant="ghost"
                onClick={() =>
                  act(async () => {
                    await api.resetMode(draft.slug);
                    await reload(draft.slug);
                  }, "Ripristinata.")
                }
              >
                Ripristina originale
              </Button>
            ) : (
              <Button
                variant="danger"
                onClick={() =>
                  act(async () => {
                    await api.deleteMode(draft.slug);
                    await reload("raw");
                  }, "Modalità eliminata.")
                }
              >
                Elimina
              </Button>
            )}
          </div>
          {notice && <Notice tone={notice.tone} text={notice.text} />}
        </div>
      </div>
    </>
  );
}
