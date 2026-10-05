#!/usr/bin/env node
// Top-N per-element self time with the factor each function is filed under.
//   node top-labeled.mjs perfn.json B [20]
import { readFileSync } from "node:fs";
import { CATEGORY, categoryOf } from "./categories.mjs";
const [file, c, n] = process.argv.slice(2);
const p = JSON.parse(readFileSync(file, "utf8"))[c];
Object.entries(p).filter(([k]) => !k.startsWith("(")).sort((a, b) => b[1] - a[1]).slice(0, Number(n ?? 20)).forEach(([k, v], i) => {
  const cat = categoryOf(k); const id = (/\[(\d+)\]/.exec(k) || [])[1] ?? k; const ev = CATEGORY[cat]?.fns[id] ?? "";
  console.log(`${String(i + 1).padStart(2)} ${k.padEnd(19)} ${v.toFixed(1).padStart(6)} ns  ${cat.padEnd(9)} ${ev}`);
});
