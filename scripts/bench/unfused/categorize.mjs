#!/usr/bin/env node
// Sum per-element profile self time (from diff-profile.mjs, setup subtracted) by factor.
//   node categorize.mjs perfn.json
import { readFileSync } from "node:fs";
import { CATEGORY, categoryOf } from "./categories.mjs";
const evidence = (f, k) => { const id = (/\[(\d+)\]/.exec(f) || [])[1] ?? f; const e = CATEGORY[k]?.fns[id] ?? ""; return e.startsWith("callee") ? "inferred" : "confirmed"; };
const per = JSON.parse(readFileSync(process.argv[2], "utf8"));
const cats = [...Object.keys(CATEGORY), "unknown"];
const res = {};
for (const [c, fns] of Object.entries(per)) {
  const sum = Object.fromEntries(cats.map((k) => [k, 0])); const inf = Object.fromEntries(cats.map((k) => [k, 0])); let total = 0; const unk = [];
  for (const [f, ns] of Object.entries(fns)) { if (f.startsWith("(") || f === "compileForInternalLoader") continue; const k = categoryOf(f); sum[k] += ns; total += ns; if (k !== "unknown" && evidence(f, k) === "inferred") inf[k] += ns; if (k === "unknown" && ns > 0.8) unk.push(`${f.replace(/wasm-function\[(\d+)\]/, "$1")}:${ns.toFixed(1)}`); }
  res[c] = { total, sum, inferred: inf, unk };
  console.log(`${c}: total ${total.toFixed(0)} ns  ` + cats.map((k) => `${k} ${sum[k].toFixed(0)}[inf ${inf[k].toFixed(0)}] (${(100 * sum[k] / total).toFixed(0)}%)`).join("  ") + `\n   unknown>0.8: ${unk.join(" ")}`);
}
if (process.argv[3] === "--json") { const { writeFileSync } = await import("node:fs"); writeFileSync(process.argv[4], JSON.stringify(res, null, 1)); }
