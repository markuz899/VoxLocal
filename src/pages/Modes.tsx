import { useEffect, useState } from "react";
import {
  api,
  errorText,
  type AppMode,
  type Mode,
  type ProviderView,
} from "../api";
import { useSettings } from "../App";
import { languageOptions } from "../format";
import { MODE_LIBRARY, modeName, useT, type Lang } from "../i18n";
import { Button, Notice, PageTitle, Select, TextInput, Toggle } from "../ui";

const BUILTIN = ["raw", "clean", "email", "message", "bullets", "translate_en"];

export default function Modes() {
  const t = useT();
  const { settings, update } = useSettings();
  const lang = settings.ui_language;
  const [modes, setModes] = useState<Mode[]>([]);
  const [providers, setProviders] = useState<ProviderView[]>([]);
  const [slug, setSlug] = useState(settings.active_mode);
  const [draft, setDraft] = useState<Mode | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [library, setLibrary] = useState(false);
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
    }, t("common.saved"));

  const select = (m: Mode) => {
    setSlug(m.slug);
    setDraft(m);
    setLibrary(false);
    setNotice(null);
  };

  const label = (m: Mode) => modeName(m.slug, m.name, lang);

  return (
    <>
      <PageTitle
        title={t("modes.title")}
        description={t("modes.description")}
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
                    m.slug === slug && !library
                      ? "bg-cobalt-soft text-cobalt"
                      : ""
                  }`}
                >
                  <span className="truncate">{label(m)}</span>
                  {m.slug === settings.active_mode && (
                    <span
                      className="ml-2 h-1.5 w-1.5 shrink-0 rounded-full bg-cobalt"
                      title={t("modes.active")}
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
                const s = await api.createMode(t("modes.newName"));
                await reload(s);
                setLibrary(false);
              }, t("modes.created"))
            }
          >
            {t("modes.new")}
          </Button>
          <Button
            className={`mt-2 w-full ${library ? "border-cobalt text-cobalt" : ""}`}
            onClick={() => setLibrary(!library)}
          >
            {t("modes.fromLibrary")}
          </Button>
        </div>

        {library ? (
          <Library
            existing={modes.map((m) => m.name)}
            onAdded={async (s) => {
              await reload(s);
            }}
          />
        ) : (
          <div className="min-w-0 flex-1 space-y-4">
            <label className="block">
              <span className="mb-1 block text-[12px] text-muted">
                {t("modes.name")}
              </span>
              <TextInput
                className="w-full"
                value={draft.name}
                onChange={(e) => setDraft({ ...draft, name: e.target.value })}
              />
            </label>

            <div className="flex gap-3">
              <label className="flex-1">
                <span className="mb-1 block text-[12px] text-muted">
                  {t("modes.processing")}
                </span>
                <Select
                  label={t("modes.processing")}
                  className="w-full max-w-none"
                  value={draft.provider_id ?? ""}
                  onChange={(v) =>
                    setDraft({ ...draft, provider_id: v || null })
                  }
                  options={[
                    ["", t("modes.default", { name: defaultLabel })],
                    ["none", t("modes.none")],
                    ...providers
                      .filter((p) => p.kind !== "none")
                      .map((p): [string, string] => [p.id, p.label]),
                  ]}
                />
              </label>
              {!isRaw && (
                <label className="w-48">
                  <span className="mb-1 block text-[12px] text-muted">
                    {t("modes.model")}
                  </span>
                  <TextInput
                    className="w-full"
                    list="mode-models"
                    placeholder={t("modes.modelPlaceholder")}
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

            {(draft.slug === "translate_en" ||
              draft.system_prompt.includes("{{target_language}}")) && (
              <label className="block">
                <span className="mb-1 block text-[12px] text-muted">
                  {t("modes.translateTo")}
                </span>
                <Select
                  label={t("modes.translateTo")}
                  value={settings.translate_to}
                  onChange={(translate_to) => update({ translate_to })}
                  options={languageOptions(lang)}
                />
              </label>
            )}

            {!isRaw && (
              <>
                <label className="block">
                  <span className="mb-1 block text-[12px] text-muted">
                    {t("modes.instructions")}
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
                    {t("modes.message")}
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
                {t("common.save")}
              </Button>
              {draft.slug !== settings.active_mode && (
                <Button
                  onClick={() =>
                    act(
                      async () => {
                        const err = await update({ active_mode: draft.slug });
                        if (err) throw err;
                      },
                      t("modes.isActive", { name: label(draft) }),
                    )
                  }
                >
                  {t("modes.useThis")}
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
                    }, t("modes.restored"))
                  }
                >
                  {t("modes.reset")}
                </Button>
              ) : (
                <Button
                  variant="danger"
                  onClick={() =>
                    act(async () => {
                      await api.deleteMode(draft.slug);
                      await reload("raw");
                    }, t("modes.deleted"))
                  }
                >
                  {t("common.delete")}
                </Button>
              )}
            </div>
            {notice && <Notice tone={notice.tone} text={notice.text} />}
          </div>
        )}
      </div>
      <AppRules modes={modes} />
    </>
  );
}

/** La libreria di modalità pronte: un clic e la modalità è tua, da modificare come vuoi. */
function Library({
  existing,
  onAdded,
}: {
  existing: string[];
  onAdded: (slug: string) => void;
}) {
  const t = useT();
  const { settings } = useSettings();
  const lang: Lang = settings.ui_language === "it" ? "it" : "en";
  const [error, setError] = useState<string | null>(null);

  return (
    <div className="min-w-0 flex-1">
      <h2 className="text-[17px] font-semibold">{t("library.title")}</h2>
      <p className="mb-4 text-muted">{t("library.description")}</p>
      <ul className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
        {MODE_LIBRARY.map((m) => {
          const added = existing.includes(m.name[lang]);
          return (
            <li
              key={m.id}
              className="flex flex-col rounded-[10px] border border-line bg-surface p-4"
            >
              <span className="font-medium">{m.name[lang]}</span>
              <span className="mt-1 flex-1 text-[13px] text-muted">
                {m.description[lang]}
              </span>
              <Button
                className="mt-3 self-start"
                disabled={added}
                onClick={async () => {
                  try {
                    onAdded(await api.createModeWithTask(m.name[lang], m.task));
                  } catch (e) {
                    setError(errorText(e));
                  }
                }}
              >
                {added ? t("library.added") : t("library.add")}
              </Button>
            </li>
          );
        })}
      </ul>
      <Notice tone="error" text={error} />
    </div>
  );
}

/** Regole "app → modalità": vincono sulla modalità attiva quando il dettato parte in quell'app. */
function AppRules({ modes }: { modes: Mode[] }) {
  const t = useT();
  const { settings, update } = useSettings();
  const [recent, setRecent] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const rules = settings.app_modes;

  useEffect(() => {
    api.historyFacets(0, 0).then(
      (f) => setRecent(f.apps.map(([a]) => a)),
      () => setRecent([]),
    );
  }, []);

  const save = async (next: AppMode[]) =>
    setError(await update({ app_modes: next }));
  const modeOptions = modes.map((m): [string, string] => [
    m.slug,
    modeName(m.slug, m.name, settings.ui_language),
  ]);

  return (
    <section className="mt-10" aria-labelledby="rules-h">
      <h2 id="rules-h" className="mb-1 text-[17px] font-semibold">
        {t("rules.title")}
      </h2>
      <p className="mb-4 max-w-[60ch] text-muted">{t("rules.description")}</p>
      <div className="mb-3 flex items-start gap-3 rounded-[10px] border border-line bg-surface px-4 py-3">
        <div className="flex-1">
          <div className="font-medium">{t("rules.auto")}</div>
          <p className="max-w-[60ch] text-[12px] text-muted">
            {t("rules.autoHint")}
          </p>
        </div>
        <Toggle
          label={t("rules.auto")}
          checked={settings.app_styles_auto}
          onChange={async (v) => setError(await update({ app_styles_auto: v }))}
        />
      </div>
      <div className="divide-y divide-line rounded-[10px] border border-line bg-surface">
        {rules.length === 0 && (
          <p className="px-4 py-3 text-muted">{t("rules.empty")}</p>
        )}
        {rules.map((r, i) => (
          <div key={i} className="flex items-center gap-3 px-4 py-2.5">
            <TextInput
              aria-label={t("rules.app")}
              list="recent-apps"
              className="w-56"
              value={r.app}
              placeholder={t("rules.appPlaceholder")}
              onChange={(e) =>
                save(
                  rules.map((x, k) =>
                    k === i ? { ...x, app: e.target.value } : x,
                  ),
                )
              }
            />
            <span className="text-muted">{t("rules.uses")}</span>
            <Select
              label={t("meta.mode")}
              value={r.mode}
              onChange={(mode) =>
                save(rules.map((x, k) => (k === i ? { ...x, mode } : x)))
              }
              options={modeOptions}
            />
            <span className="flex-1" />
            <Button
              variant="ghost"
              onClick={() => save(rules.filter((_, k) => k !== i))}
            >
              {t("common.remove")}
            </Button>
          </div>
        ))}
      </div>
      <datalist id="recent-apps">
        {recent.map((a) => (
          <option key={a} value={a} />
        ))}
      </datalist>
      <Button
        className="mt-3"
        onClick={() =>
          save([
            ...rules,
            {
              app: recent.find((a) => !rules.some((r) => r.app === a)) ?? "",
              mode: modes[1]?.slug ?? "raw",
            },
          ])
        }
      >
        {t("rules.add")}
      </Button>
      <Notice tone="error" text={error} />
    </section>
  );
}
