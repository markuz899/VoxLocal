import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { useT } from "../i18n";
import { Button, Notice, PageTitle, TextInput } from "../ui";

export default function Vocabulary() {
  const t = useT();
  const [terms, setTerms] = useState<string[]>([]);
  const [input, setInput] = useState("");
  const [error, setError] = useState<string | null>(null);

  const reload = () => api.getVocabulary().then(setTerms);
  useEffect(() => {
    reload();
  }, []);

  const add = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      // Più termini in una volta, separati da virgola.
      for (const term of input
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean)) {
        await api.addTerm(term);
      }
      setInput("");
      reload();
    } catch (err) {
      setError(errorText(err));
    }
  };

  return (
    <>
      <PageTitle
        title={t("vocab.title")}
        description={t("vocab.description")}
      />
      <form onSubmit={add} className="mb-5 flex gap-2">
        <TextInput
          className="flex-1"
          placeholder={t("vocab.placeholder")}
          aria-label={t("vocab.title")}
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <Button type="submit" variant="primary" disabled={!input.trim()}>
          {t("vocab.add")}
        </Button>
      </form>
      <Notice tone="error" text={error} />
      {terms.length === 0 ? (
        <p className="text-muted">{t("vocab.empty")}</p>
      ) : (
        <ul className="flex flex-wrap gap-2">
          {terms.map((term) => (
            <li
              key={term}
              className="flex items-center gap-1 rounded-full border border-line bg-surface py-1 pr-1 pl-3"
            >
              <span className="selectable">{term}</span>
              <button
                type="button"
                aria-label={t("vocab.removeTerm", { term })}
                onClick={() => api.removeTerm(term).then(reload)}
                className="grid h-5 w-5 place-items-center rounded-full text-muted hover:bg-paper hover:text-ink"
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
