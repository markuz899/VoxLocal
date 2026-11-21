import { useEffect, useState } from "react";
import { api, errorText, type Snippet } from "../api";
import { useT } from "../i18n";
import { Button, Notice, PageTitle, TextInput } from "../ui";

export default function Snippets() {
  const t = useT();
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [trigger, setTrigger] = useState("");
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  const reload = () => api.listSnippets().then(setSnippets);
  useEffect(() => {
    reload();
  }, []);

  const save = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await api.saveSnippet(trigger, text);
      setTrigger("");
      setText("");
      reload();
    } catch (err) {
      setError(errorText(err));
    }
  };

  return (
    <>
      <PageTitle
        title={t("snippets.title")}
        description={t("snippets.description")}
      />
      <form
        onSubmit={save}
        className="mb-6 space-y-3 rounded-[10px] border border-line bg-surface p-4"
      >
        <label className="block">
          <span className="mb-1 block text-[12px] text-muted">
            {t("snippets.trigger")}
          </span>
          <TextInput
            className="w-full"
            placeholder={t("snippets.triggerPlaceholder")}
            value={trigger}
            onChange={(e) => setTrigger(e.target.value)}
          />
        </label>
        <label className="block">
          <span className="mb-1 block text-[12px] text-muted">
            {t("snippets.text")}
          </span>
          <textarea
            rows={3}
            placeholder={t("snippets.textPlaceholder")}
            value={text}
            onChange={(e) => setText(e.target.value)}
            className="w-full resize-y rounded-md border border-line bg-surface p-2 text-[13px] placeholder:text-muted"
          />
        </label>
        <Button
          type="submit"
          variant="primary"
          disabled={!trigger.trim() || !text.trim()}
        >
          {t("snippets.add")}
        </Button>
        <Notice tone="error" text={error} />
      </form>

      {snippets.length === 0 ? (
        <p className="text-muted">{t("snippets.empty")}</p>
      ) : (
        <ul className="divide-y divide-line rounded-[10px] border border-line bg-surface">
          {snippets.map((s) => (
            <li key={s.id} className="flex items-start gap-4 px-4 py-3">
              <div className="min-w-0 flex-1">
                <div className="font-medium">“{s.trigger}”</div>
                <div className="selectable mt-0.5 text-[13px] whitespace-pre-wrap text-muted">
                  {s.text}
                </div>
              </div>
              <Button
                variant="ghost"
                onClick={() => {
                  setTrigger(s.trigger);
                  setText(s.text);
                }}
              >
                {t("snippets.edit")}
              </Button>
              <Button
                variant="ghost"
                onClick={() => api.deleteSnippet(s.id).then(reload)}
              >
                {t("common.delete")}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
