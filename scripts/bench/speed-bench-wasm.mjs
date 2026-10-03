#!/usr/bin/env node
// Time the speed-bench cases on the WebAssembly build.
//
//   node scripts/bench/speed-bench-wasm.mjs [FILTER] [--json] [--module <dir>]
//
// The cases are `speed-bench-cases.json`, the same file
// `rust/examples/speed_bench.rs` times natively, so the two tables line up
// row for row. `--module` points at a wasm-pack output directory (default:
// the committed bundle in src/wasm/generated/), which is how two builds — say
// with and without wasm-opt — are compared on one machine.
//
// Each case is timed as `setup source` minus `setup` alone, both through the
// `bench_execute` export: a fresh unbounded interpreter whose result is never
// converted to JS. Best of `repeats` for each, as on the native side.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..");

const args = process.argv.slice(2);
const json = args.includes("--json");
const moduleFlag = args.indexOf("--module");
const moduleDir = moduleFlag >= 0
  ? resolve(args[moduleFlag + 1])
  : join(repoRoot, "src", "wasm", "generated");
const filter = args.find((a, i) => !a.startsWith("--") && args[i - 1] !== "--module");

const wasm = await import(join(moduleDir, "ajisai_core.js"));
await wasm.default({ module_or_path: readFileSync(join(moduleDir, "ajisai_core_bg.wasm")) });

const suite = JSON.parse(readFileSync(join(here, "speed-bench-cases.json"), "utf8"));
const repeats = suite.repeats ?? 5;

async function best(source) {
  let min = Infinity;
  for (let i = 0; i < repeats; i++) {
    const started = performance.now();
    await wasm.bench_execute(source);
    min = Math.min(min, performance.now() - started);
  }
  return min;
}

const rows = [];
for (const c of suite.cases) {
  if (filter && !c.name.includes(filter)) continue;
  const source = c.source.repeat(c.repeat ?? 1);
  const base = c.setup ? await best(c.setup) : 0;
  const total = await best(`${c.setup} ${source}`);
  const ms = Math.max(0, total - base);
  const nsPerElem = (ms * 1e6) / c.elements;
  rows.push({ name: c.name, ms: Number(ms.toFixed(3)), nsPerElem: Number(nsPerElem.toFixed(2)) });
  if (!json) {
    console.log(`${c.name.padEnd(22)} ${ms.toFixed(2).padStart(10)} ms ${nsPerElem.toFixed(1).padStart(10)} ns/elem`);
  }
}
if (json) console.log(JSON.stringify(rows, null, 2));
