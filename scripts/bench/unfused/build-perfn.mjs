#!/usr/bin/env node
// Collect diff-profile.mjs results for every case in <dir>/runs.txt into
// <dir>/perfn.json, subtracting the matching setup-only profile.
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
const here = dirname(fileURLToPath(import.meta.url)); const dir = process.argv[2];
const runs = Object.fromEntries(readFileSync(join(dir, "runs.txt"), "utf8").trim().split("\n").map((l) => { const [c, a, b] = l.split(" "); return [c, [a, b]]; }));
const get = (c) => { const [a, b] = runs[c]; const f = (d) => join(dir, `${c}-${d}`, readdirSync(join(dir, `${c}-${d}`))[0]);
  return JSON.parse(execFileSync("node", [join(here, "diff-profile.mjs"), f("L"), a, "100000", f("S"), b, "100", "--json"])); };
const setupOf = { B: "B@setup", BB: "B@setup", C: "B@setup", K: "B@setup", F: "B@setup", D: "D@setup", E: "E@setup" };
const out = {};
for (const c of Object.keys(runs).filter((c) => !c.includes("@"))) {
  const m = new Map(get(c).rows.map((r) => [r.fn, r.ns]));
  if (setupOf[c] && runs[setupOf[c]]) for (const r of get(setupOf[c]).rows) m.set(r.fn, (m.get(r.fn) ?? 0) - r.ns);
  out[c] = Object.fromEntries([...m].filter(([, v]) => Math.abs(v) >= 0.05));
}
writeFileSync(join(dir, "perfn.json"), JSON.stringify(out, null, 1));
