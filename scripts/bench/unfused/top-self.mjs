#!/usr/bin/env node
// Self-time table from a .cpuprofile:  node top-self.mjs <file.cpuprofile> [N]
import { readFileSync } from "node:fs";
const p = JSON.parse(readFileSync(process.argv[2], "utf8"));
const top = Number(process.argv[3] ?? 20);
const byId = new Map(p.nodes.map((n) => [n.id, n]));
const self = new Map();
const dt = p.timeDeltas; let total = 0;
for (let i = 0; i < p.samples.length; i++) {
  const n = byId.get(p.samples[i]); const d = dt[i] ?? 0; total += d;
  const cf = n.callFrame; const key = cf.functionName || `(anon ${cf.url}:${cf.lineNumber})`;
  self.set(key, (self.get(key) ?? 0) + d);
}
const rows = [...self].sort((a, b) => b[1] - a[1]).slice(0, top);
for (const [k, v] of rows) console.log(`${(v / 1000).toFixed(1).padStart(9)} ms ${(100 * v / total).toFixed(1).padStart(5)}%  ${k}`);
console.log(`total ${(total / 1000).toFixed(1)} ms`);
