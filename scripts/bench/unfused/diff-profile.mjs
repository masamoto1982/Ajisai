#!/usr/bin/env node
// Per-element self time per function from two CPU profiles of the same case:
// one at a large n and one at a small n (step 2). The per-run constant —
// a fresh interpreter, its dictionary, the setup, the teardown — cancels:
//   ns/elem(f) = (self_large(f)/runs_large − self_small(f)/runs_small) / (n_large − n_small)
//
//   node diff-profile.mjs <large.cpuprofile> <runsL> <nL> <small.cpuprofile> <runsS> <nS> [top]
import { readFileSync } from "node:fs";
const [fl, rl, nl, fs_, rs, ns, topArg] = process.argv.slice(2);
function selfTimes(file) {
  const p = JSON.parse(readFileSync(file, "utf8"));
  const byId = new Map(p.nodes.map((n) => [n.id, n])); const m = new Map();
  for (let i = 0; i < p.samples.length; i++) { const cf = byId.get(p.samples[i]).callFrame; const k = cf.functionName || "(anonymous)"; m.set(k, (m.get(k) ?? 0) + (p.timeDeltas[i] ?? 0)); }
  return m; // microseconds
}
const L = selfTimes(fl), S = selfTimes(fs_);
const keys = new Set([...L.keys(), ...S.keys()]);
const rows = [];
for (const k of keys) { const v = ((L.get(k) ?? 0) / rl - (S.get(k) ?? 0) / rs) * 1000 / (nl - ns); rows.push([k, v]); }
rows.sort((a, b) => b[1] - a[1]);
const total = rows.reduce((a, [, v]) => a + v, 0);
const out = { totalNsPerElem: total, rows: rows.map(([k, v]) => ({ fn: k, ns: +v.toFixed(2) })) };
if (topArg === "--json") console.log(JSON.stringify(out));
else { for (const [k, v] of rows.slice(0, Number(topArg ?? 25))) console.log(`${v.toFixed(1).padStart(8)} ns/elem  ${k}`); console.log(`total ${total.toFixed(1)} ns/elem`); }
