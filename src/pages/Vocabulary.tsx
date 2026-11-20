import { useEffect, useState } from "react";
import { api, errorText } from "../api";
import { Button, Notice, PageTitle, TextInput } from "../ui";

export default function Vocabulary() {
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
      for (const t of input
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean)) {
        await api.addTerm(t);
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
        title="Vocabolario"
        description="Nomi, sigle e termini tecnici scritti come vuoi tu. Aiutano la trascrizione e vengono passati all'LLM."
      />
      <form onSubmit={add} className="mb-5 flex gap-2">
        <TextInput
          className="flex-1"
          placeholder="Es. Kubernetes, Giorgio Rossi, ACME S.p.A."
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <Button type="submit" variant="primary" disabled={!input.trim()}>
          Aggiungi
        </Button>
      </form>
      <Notice tone="error" text={error} />
      {terms.length === 0 ? (
        <p className="text-muted">
          Ancora nessun termine. Aggiungi le parole che la trascrizione sbaglia
          più spesso.
        </p>
      ) : (
        <ul className="flex flex-wrap gap-2">
          {terms.map((t) => (
            <li
              key={t}
              className="flex items-center gap-1 rounded-full border border-line bg-surface py-1 pr-1 pl-3"
            >
              <span className="selectable">{t}</span>
              <button
                type="button"
                aria-label={`Rimuovi ${t}`}
                onClick={() => api.removeTerm(t).then(reload)}
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
