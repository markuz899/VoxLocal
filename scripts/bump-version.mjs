// Porta la versione dell'app a quella data in package.json, Cargo.toml e Cargo.lock.
// node scripts/bump-version.mjs 0.2.1
import { readFileSync, writeFileSync } from "node:fs";

const next = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(next ?? ""))
  throw new Error("serve una versione x.y.z");

const edit = (file, from, to) => {
  const text = readFileSync(file, "utf8");
  const out = text.replace(from, to);
  if (out === text) throw new Error(`versione non trovata in ${file}`);
  writeFileSync(file, out);
};

edit("package.json", /"version": "[^"]+"/, `"version": "${next}"`);
edit("src-tauri/Cargo.toml", /^version = "[^"]+"/m, `version = "${next}"`);
edit(
  "src-tauri/Cargo.lock",
  /(name = "voxlocal"\nversion = )"[^"]+"/,
  `$1"${next}"`,
);
