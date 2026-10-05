#!/usr/bin/env node
// Factor breakdown of the routes the fused walk declines (step 1 of the
// unfused-route profile).
//
//   node scripts/bench/unfused/breakdown.mjs [--json out.json] [--only A,B,...] [--module <dir>]
//
// Every case is timed through AjisaiInterpreter.execute — the Playground's
// entry point — on a fresh interpreter whose operand was built by an untimed
// `setup` run. The timed source ends in a reducer (LENGTH) so the stack the
// host converts afterwards is one scalar; the same reducer timed on its own
// (`R`) is subtracted, which takes the display conversion and the per-run
// fixed cost out of the figure. bench_execute (no host conversion at all) is
// reported alongside as a cross-check. Best of 5 after one warm-up run.
// Each case is also run once untimed against a reference program that takes
// the fused route, and the two results must be identical.

import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (k) => { const i = args.indexOf(k); return i >= 0 ? args[i + 1] : undefined; };
const moduleDir = resolve(opt("--module") ?? join(here, "..", "..", "..", "src", "wasm", "generated"));
const only = opt("--only")?.split(",");
const SIZES = (opt("--sizes") ?? "1000,10000,100000").split(",").map(Number);
const REPEATS = 5;

const wasm = await import(join(moduleDir, "ajisai_core.js"));
await wasm.default({ module_or_path: readFileSync(join(moduleDir, "ajisai_core_bg.wasm")) });

import { CASE_TABLE as CASES, SUM } from "./cases.mjs";

async function runInterp(setup, src) {
  const it = new wasm.AjisaiInterpreter();
  try {
    if (setup) { const r = await it.execute(setup); if (r.status !== "OK") throw new Error("setup: " + r.message); }
    const t = performance.now();
    const r = await it.execute(src);
    const ms = performance.now() - t;
    if (r.status !== "OK") throw new Error(`${src.slice(0, 60)}: ${r.message}`);
    return { ms, r };
  } finally { it.free(); }
}

async function best(fn) {
  await fn(); // warm-up
  let min = Infinity;
  for (let i = 0; i < REPEATS; i++) min = Math.min(min, await fn());
  return min;
}

async function benchExec(source) {
  const t = performance.now(); await wasm.bench_execute(source); return performance.now() - t;
}

const value = (r) => JSON.stringify(r.stack.map((v) => v.value));

// Warm-up pass: every case once at the middle size, so V8's tier-up of the
// WebAssembly functions has happened before any case is timed.
for (const c of CASES) {
  if (only && !only.includes(c.id)) continue;
  const n = SIZES[Math.floor(SIZES.length / 2)];
  if (c.kind === "map") await runInterp(c.setup(n), `${c.src} LENGTH`); else await runInterp("", c.src(n));
}

const rows = [];
for (const c of CASES) {
  if (only && !only.includes(c.id)) continue;
  for (const n of SIZES) {
    let execMs, benchMs, same;
    if (c.kind === "map") {
      const setup = c.setup(n);
      const total = await best(async () => (await runInterp(setup, `${c.src} LENGTH`)).ms);
      const reducer = await best(async () => (await runInterp(setup, "LENGTH")).ms);
      execMs = total - reducer;
      const bt = await best(() => benchExec(`${setup} ${c.src} LENGTH`));
      const bs = await best(() => benchExec(`${setup} LENGTH`));
      benchMs = bt - bs;
      const got = (await runInterp(setup, `${c.src} ${SUM}`)).r;
      const want = (await runInterp(setup, `${c.ref} ${SUM}`)).r;
      // compare the whole Vector too, not only its sum
      const gotV = (await runInterp(setup, c.src)).r;
      const wantV = (await runInterp(setup, c.ref)).r;
      same = value(got) === value(want) && value(gotV) === value(wantV);
    } else {
      const src = c.src(n);
      const total = await best(async () => (await runInterp("", src)).ms);
      const empty = await best(async () => (await runInterp("", "0")).ms);
      execMs = total - empty;
      benchMs = (await best(() => benchExec(src))) - (await best(() => benchExec("0")));
      const got = (await runInterp("", src)).r;
      same = value(got) === JSON.stringify([{ numerator: String(c.refValue(n)), denominator: "1" }]);
    }
    const row = { id: c.id, n, execMs: +execMs.toFixed(3), execNs: +(execMs * 1e6 / n).toFixed(1), benchMs: +benchMs.toFixed(3), benchNs: +(benchMs * 1e6 / n).toFixed(1), same };
    rows.push(row);
    console.error(`${c.id.padEnd(4)} n=${String(n).padStart(6)}  execute ${row.execNs.toFixed(1).padStart(8)} ns/elem   bench_execute ${row.benchNs.toFixed(1).padStart(8)} ns/elem   same=${same}`);
  }
}
const out = opt("--json");
if (out) writeFileSync(out, JSON.stringify({ cases: CASES.map(({ id, what, kind }) => ({ id, what, kind })), rows }, null, 2));
