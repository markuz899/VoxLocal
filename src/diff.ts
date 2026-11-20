// "Cosa hai detto": confronto parola per parola fra il testo detto (raw) e quello scritto (final).
// Le parole uguali a meno di maiuscole e punteggiatura contano come uguali, così la modalità Pulito,
// che cambia solo virgole e maiuscole, non sottolinea l'intero testo.

export interface Segment {
  /** Il pezzo del testo finale, spazi compresi. */
  text: string;
  /** true se il pezzo è stato aggiunto o cambiato dall'elaborazione. */
  changed: boolean;
  /** Le parole dette al posto di questo pezzo ("" se è un'aggiunta pura). */
  said: string;
}

// ponytail: LCS O(n·m); oltre ~250.000 celle (dettati di migliaia di parole) niente evidenziazione
const MAX_CELLS = 250_000;

const norm = (w: string) =>
  w.toLocaleLowerCase("it").replace(/[^\p{L}\p{N}]/gu, "");

export function diffWords(raw: string, final: string): Segment[] {
  // Parole con lo spazio che le segue, così ricomponendo i pezzi il testo resta identico.
  const fin = final.match(/\S+\s*/g) ?? [];
  const said = raw.match(/\S+/g) ?? [];
  if (raw.trim() === final.trim() || fin.length * said.length > MAX_CELLS) {
    return [{ text: final, changed: false, said: "" }];
  }
  const a = said.map(norm);
  const b = fin.map(norm);
  const n = a.length;
  const m = b.length;
  // lcs[i][j] = lunghezza della sottosequenza comune più lunga di a[i..] e b[j..]
  const lcs = Array.from({ length: n + 1 }, () => new Uint16Array(m + 1));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      lcs[i][j] =
        a[i] === b[j] && a[i] !== ""
          ? lcs[i + 1][j + 1] + 1
          : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
    }
  }

  const out: Segment[] = [];
  const push = (text: string, changed: boolean, saidWords: string[]) => {
    const last = out[out.length - 1];
    if (last && last.changed === changed) {
      last.text += text;
      if (changed && saidWords.length)
        last.said = [last.said, ...saidWords].filter(Boolean).join(" ");
    } else {
      out.push({ text, changed, said: saidWords.join(" ") });
    }
  };

  let i = 0;
  let j = 0;
  let dropped: string[] = [];
  while (i < n || j < m) {
    if (i < n && j < m && a[i] === b[j] && a[i] !== "") {
      // Parole tolte subito prima di una uguale: le attacco al pezzo cambiato precedente.
      if (dropped.length && out.length && out[out.length - 1].changed) {
        const last = out[out.length - 1];
        last.said = [last.said, ...dropped].filter(Boolean).join(" ");
      }
      dropped = [];
      push(fin[j], false, []);
      i++;
      j++;
    } else if (j < m && (i >= n || lcs[i][j + 1] > lcs[i + 1][j])) {
      push(fin[j], true, dropped);
      dropped = [];
      j++;
    } else {
      dropped.push(said[i]);
      i++;
    }
  }
  // Parole tolte in fondo: vanno all'ultimo pezzo cambiato, se c'è.
  const lastChanged = [...out].reverse().find((s) => s.changed);
  if (dropped.length && lastChanged)
    lastChanged.said = [lastChanged.said, ...dropped].filter(Boolean).join(" ");
  return out;
}
