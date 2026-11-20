// node --experimental-strip-types src/diff.test.ts  (pnpm test)
import assert from "node:assert/strict";
import { diffWords } from "./diff.ts";

const join = (s: { text: string }[]) => s.map((x) => x.text).join("");

// Solo punteggiatura e maiuscole cambiate: niente da sottolineare.
const clean = diffWords(
  "ciao come stai tutto bene",
  "Ciao, come stai? Tutto bene.",
);
assert.equal(clean.filter((s) => s.changed).length, 0);
assert.equal(join(clean), "Ciao, come stai? Tutto bene.");

// Una parola sostituita: il pezzo nuovo dice cosa era stato detto.
const swap = diffWords("la partita è alle sette", "La partita è alle 19.");
const changed = swap.filter((s) => s.changed);
assert.equal(changed.length, 1);
assert.equal(changed[0].text.trim(), "19.");
assert.equal(changed[0].said, "sette");
assert.equal(join(swap), "La partita è alle 19.");

// Esitazioni tolte e saluto aggiunto: il testo ricomposto è identico al finale.
const email = diffWords(
  "ehm scrivi a Giorgio che domani si gioca",
  "Ciao Giorgio, domani si gioca.",
);
assert.equal(join(email), "Ciao Giorgio, domani si gioca.");
assert.ok(email.some((s) => s.changed && s.text.startsWith("Ciao")));

// Testi identici: un solo pezzo non cambiato.
assert.deepEqual(diffWords("uguale", "uguale"), [
  { text: "uguale", changed: false, said: "" },
]);

console.log("diff: ok");
