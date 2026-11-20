import { useCallback, useEffect, useState } from "react";
import { api, errorText, type HistoryItem } from "../api";
import { formatDate, formatMs } from "../format";
import { Button, Notice, PageTitle, TextInput } from "../ui";

const PAGE = 50;

const STATUS: Record<HistoryItem["status"], string | null> = {
  ok: null,
  llm_fallback: "Incollato senza elaborazione",
  cancelled: "Annullato",
  paste_failed: "Non incollato",
  error: "Errore",
};

export default function History() {
  const [items, setItems] = useState<HistoryItem[]>([]);
  const [query, setQuery] = useState("");
  const [more, setMore] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (q: string, offset: number) => {
    try {
      const page = await api.getHistory(q, PAGE, offset);
      setItems((prev) => (offset === 0 ? page : [...prev, ...page]));
      setMore(page.length === PAGE);
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  // Ricerca mentre si scrive, con un piccolo ritardo.
  useEffect(() => {
    const t = setTimeout(() => load(query, 0), 150);
    return () => clearTimeout(t);
  }, [query, load]);

  const copy = async (text: string) => {
    await api.copyText(text);
    setNotice("Copiato negli appunti.");
    setTimeout(() => setNotice(null), 2000);
  };

  return (
    <>
      <PageTitle
        title="Cronologia"
        description="I dettati restano solo su questo computer."
      />
      <div className="mb-4 flex gap-2">
        <TextInput
          type="search"
          className="flex-1"
          placeholder="Cerca nei dettati"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {confirmClear ? (
          <>
            <Button
              variant="danger"
              onClick={async () => {
                await api.clearHistory();
                setConfirmClear(false);
                load(query, 0);
              }}
            >
              Elimina tutto
            </Button>
            <Button variant="ghost" onClick={() => setConfirmClear(false)}>
              Annulla
            </Button>
          </>
        ) : (
          <Button
            variant="ghost"
            disabled={items.length === 0}
            onClick={() => setConfirmClear(true)}
          >
            Svuota
          </Button>
        )}
      </div>
      <Notice tone="ok" text={notice} />
      <Notice tone="error" text={error} />

      {items.length === 0 ? (
        <p className="text-muted">
          {query
            ? "Nessun dettato contiene queste parole."
            : "Qui compariranno i tuoi dettati."}
        </p>
      ) : (
        <ul className="divide-y divide-line rounded-[10px] border border-line bg-surface">
          {items.map((item) => (
            <Entry
              key={item.id}
              item={item}
              onCopy={copy}
              onDelete={async () => {
                await api.deleteDictation(item.id);
                setItems((prev) => prev.filter((i) => i.id !== item.id));
              }}
            />
          ))}
        </ul>
      )}
      {more && (
        <div className="mt-4 text-center">
          <Button onClick={() => load(query, items.length)}>
            Mostra altri
          </Button>
        </div>
      )}
    </>
  );
}

function Entry({
  item,
  onCopy,
  onDelete,
}: {
  item: HistoryItem;
  onCopy: (t: string) => void;
  onDelete: () => void;
}) {
  const [showRaw, setShowRaw] = useState(false);
  const processed = item.raw_text !== item.final_text;
  const status = STATUS[item.status];
  const text = showRaw ? item.raw_text : item.final_text;

  return (
    <li className="group px-4 py-3">
      <div className="mb-1 flex items-center gap-2 text-[12px] text-muted">
        <span>{formatDate(item.created_at)}</span>
        {item.mode && <span>{item.mode}</span>}
        <span>{formatMs(item.total_ms)}</span>
        {status && (
          <span className="text-warn" title={item.error ?? undefined}>
            {status}
          </span>
        )}
      </div>
      <p className="selectable whitespace-pre-wrap">{text}</p>
      <div className="mt-2 flex gap-1 opacity-60 group-hover:opacity-100 focus-within:opacity-100">
        <Button variant="ghost" className="px-1" onClick={() => onCopy(text)}>
          Copia
        </Button>
        {processed && (
          <Button
            variant="ghost"
            className="px-1"
            onClick={() => setShowRaw(!showRaw)}
          >
            {showRaw ? "Mostra elaborato" : "Mostra originale"}
          </Button>
        )}
        <Button variant="ghost" className="px-1" onClick={onDelete}>
          Elimina
        </Button>
      </div>
    </li>
  );
}
