#!/usr/bin/env node
// Per-element call counts on the instrumented copy (step 3).
//
//   node instrument-wasm.mjs src/wasm/generated /tmp/counted
//   node count-calls.mjs /tmp/counted [--only B,C] [--n 10000] [--top 25] [--json out.json]
//
// For each case: counters after bench_execute(setup + src) minus counters
// after bench_execute(setup), divided by n. Allocator entry points are the
// functions behind the __wbindgen_malloc/realloc/free exports (see
// wasm-inspect.mjs): ALLOC/DEALLOC/REALLOC below.
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { CASE_TABLE } from "./cases.mjs";
import { inspect } from "./wasm-inspect.mjs";

const args = process.argv.slice(2);
const opt = (k) => { const i = args.indexOf(k); return i >= 0 ? args[i + 1] : undefined; };
const dir = resolve(args[0]);
const only = opt("--only")?.split(",");
const N = Number(opt("--n") ?? 10000);
const TOP = Number(opt("--top") ?? 25);

const here = dirname(fileURLToPath(import.meta.url));
const orig = inspect(readFileSync(join(here, "..", "..", "..", "src", "wasm", "generated", "ajisai_core_bg.wasm")));
const callee = (exportName) => [...orig.byIndex.get(orig.exports.find((e) => e.nm === exportName).idx).calls.keys()][0];
const ALLOC = callee("__wbindgen_malloc"), DEALLOC = callee("__wbindgen_free"), REALLOC = callee("__wbindgen_realloc");

const wasm = await import(join(dir, "ajisai_core.js"));
const ex = await wasm.default({ module_or_path: readFileSync(join(dir, "ajisai_core_bg.wasm")) });
const counters = Object.entries(ex).filter(([k]) => k.startsWith("__cnt_")).map(([k, g]) => [Number(k.slice(6)), g]);
const reset = () => { for (const [, g] of counters) g.value = 0; };
const snap = () => new Map(counters.map(([i, g]) => [i, g.value >>> 0]));

async function countOf(source) { reset(); await wasm.bench_execute(source); return snap(); }

const results = {};
for (const c of CASE_TABLE) {
  if (only && !only.includes(c.id)) continue;
  const [setup, src] = c.kind === "map" ? [c.setup(N), `${c.src} LENGTH`] : ["0", c.src(N)];
  const fullSrc = c.kind === "map" ? `${setup} ${src}` : src;
  const base = c.kind === "map" ? await countOf(`${setup} LENGTH`) : await countOf("0");
  const full = await countOf(fullSrc);
  const per = new Map(); for (const [i, v] of full) { const d = (v - (base.get(i) ?? 0)) / N; if (d > 0.001) per.set(i, d); }
  const sum = [...per.values()].reduce((a, b) => a + b, 0);
  results[c.id] = { alloc: per.get(ALLOC) ?? 0, dealloc: per.get(DEALLOC) ?? 0, realloc: per.get(REALLOC) ?? 0, calls: sum, per: Object.fromEntries([...per].sort((a, b) => b[1] - a[1])) };
  console.log(`${c.id.padEnd(4)} per element: alloc ${results[c.id].alloc.toFixed(2)}  dealloc ${results[c.id].dealloc.toFixed(2)}  realloc ${results[c.id].realloc.toFixed(2)}  function calls ${sum.toFixed(1)}`);
  if (TOP > 0) console.log("     " + [...per].sort((a, b) => b[1] - a[1]).slice(0, TOP).map(([i, v]) => `${i}:${v.toFixed(2)}`).join(" "));
}
const out = opt("--json"); if (out) writeFileSync(out, JSON.stringify({ N, ALLOC, DEALLOC, REALLOC, results }, null, 2));
