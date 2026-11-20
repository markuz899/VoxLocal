import { useEffect, useState } from "react";
import { api, errorText, type ProviderView } from "../api";
import { useSettings } from "../App";
import { formatMs } from "../format";
import { useT } from "../i18n";
import {
  Button,
  Group,
  Notice,
  PageTitle,
  Row,
  Select,
  TextInput,
} from "../ui";

export default function Providers() {
  const t = useT();
  const { settings, update } = useSettings();
  const [providers, setProviders] = useState<ProviderView[]>([]);
  const [open, setOpen] = useState<string | null>(null);

  const reload = () => api.listProviders().then(setProviders);
  useEffect(() => {
    reload();
  }, []);

  const keyState = (p: ProviderView): { text: string; tone: string } => {
    if (p.kind === "cli")
      return { text: t("providers.cli"), tone: "text-muted" };
    if (!p.needs_key) return { text: t("providers.local"), tone: "text-muted" };
    return p.has_key
      ? { text: t("providers.keySaved"), tone: "text-ok" }
      : { text: t("providers.keyMissing"), tone: "text-warn" };
  };

  const usable = providers.filter((p) => p.kind !== "none");

  return (
    <>
      <PageTitle
        title={t("providers.title")}
        description={t("providers.description")}
      />
      <Group>
        <Row label={t("providers.default")} hint={t("providers.defaultHint")}>
          <Select
            label={t("providers.default")}
            value={settings.default_llm}
            onChange={(default_llm) => update({ default_llm })}
            options={usable.map((p): [string, string] => [p.id, p.label])}
          />
        </Row>
      </Group>
      <Group title={t("providers.list")}>
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
                {t("providers.configure")}
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
  const t = useT();
  const { settings } = useSettings();
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
      return t("common.saved");
    });

  const isCli = provider.kind === "cli";

  return (
    <div className="space-y-3 bg-paper/50 px-4 py-4">
      <div className="flex items-center justify-between">
        <span className="font-medium">{provider.label}</span>
        <Button variant="ghost" onClick={onClose}>
          {t("common.close")}
        </Button>
      </div>
      {!isCli && (
        <label className="block">
          <span className="mb-1 block text-[12px] text-muted">
            {t("providers.apiUrl")}
          </span>
          <TextInput
            className="w-full"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
          />
        </label>
      )}
      <label className="block">
        <span className="mb-1 block text-[12px] text-muted">
          {t("modes.model")}
        </span>
        <TextInput
          className="w-full"
          value={model}
          placeholder={isCli ? t("providers.cliModel") : ""}
          onChange={(e) => setModel(e.target.value)}
        />
      </label>
      {provider.needs_key && (
        <label className="block">
          <span className="mb-1 block text-[12px] text-muted">
            {provider.has_key
              ? t("providers.replaceKey")
              : t("providers.apiKey")}
          </span>
          <TextInput
            className="w-full"
            type="password"
            autoComplete="off"
            value={key}
            placeholder={
              provider.has_key
                ? t("providers.keyStored")
                : t("providers.pasteKey")
            }
            onChange={(e) => setKey(e.target.value)}
          />
        </label>
      )}
      <div className="flex flex-wrap items-center gap-2 pt-1">
        <Button variant="primary" onClick={save}>
          {t("common.save")}
        </Button>
        <Button
          onClick={() =>
            act(async () =>
              t("providers.reachable", {
                ms: formatMs(
                  await api.checkProvider(provider.id),
                  settings.ui_language,
                ),
              }),
            )
          }
        >
          {t("providers.verify")}
        </Button>
        {provider.has_key && (
          <Button
            variant="danger"
            onClick={() =>
              act(async () => {
                await api.deleteApiKey(provider.id);
                return t("providers.keyRemoved");
              })
            }
          >
            {t("providers.removeKey")}
          </Button>
        )}
      </div>
      {notice && <Notice tone={notice.tone} text={notice.text} />}
    </div>
  );
}
