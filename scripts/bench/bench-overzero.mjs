#!/usr/bin/env node
// Time block walks (MAP, FILTER) with and without one number over zero.
//
//   node scripts/bench/bench-overzero.mjs [--rounds <n>] [--module <dir>]
//
// One element over zero (`1/0`, `0/0`), or one zero divisor met inside the
// block, should cost one lane's work, not the whole walk's route: each case
// below comes in a pair, "none" and "one", and the two should time alike.
// The element-wise DIV pair, which has no block, is the reference.
//
// Each case is timed as `setup body` minus `setup` alone through the
// `bench_execute` export, best of 7, as speed-bench-wasm.mjs does. With
// `--rounds` above 1 every case is timed that many times and the median
// reported. `--module` points at a wasm-pack output directory (default: the
// committed bundle in src/wasm/generated/).

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..");

const args = process.argv.slice(2);
const flag = (name) => {
  const at = args.indexOf(name);
  return at >= 0 ? args[at + 1] : undefined;
};
const moduleDir = flag("--module")
  ? resolve(flag("--module"))
  : join(repoRoot, "src", "wasm", "generated");
const rounds = Number(flag("--rounds") ?? 1);

const wasm = await import(join(moduleDir, "ajisai_core.js"));
await wasm.default({ module_or_path: readFileSync(join(moduleDir, "ajisai_core_bg.wasm")) });

const N = 100000;
async function best(src, k = 7) {
  let m = Infinity;
  for (let i = 0; i < k; i++) {
    const t = performance.now();
    await wasm.bench_execute(src);
    m = Math.min(m, performance.now() - t);
  }
  return m;
}

const cases = [
  ["整数 MAP・なし", `1 ${N} RANGE`, `[ 2 MUL 1 ADD ] MAP`],
  ["整数 MAP・0/0 が1つ", `1 0 ${N - 1} RANGE DIV 0 MUL 1 ${N} RANGE ADD`, `[ 2 MUL 1 ADD ] MAP`],
  ["分数 MAP・なし", `1 1 ${N} RANGE DIV`, `[ 2 MUL 1 ADD ] MAP`],
  ["分数 MAP・1/0 が1つ", `1 0 ${N - 1} RANGE DIV`, `[ 2 MUL 1 ADD ] MAP`],
  ["ブロック内 DIV・なし", `1 ${N} RANGE`, `[ 'X' BIND 1 X DIV ] MAP`],
  ["ブロック内 DIV・除数0が1つ", `0 ${N - 1} RANGE`, `[ 'X' BIND 1 X DIV ] MAP`],
  ["比較 FILTER・なし", `1 1 ${N} RANGE DIV`, `[ 1/2 GT ] FILTER`],
  ["比較 FILTER・1/0 が1つ", `1 0 ${N - 1} RANGE DIV`, `[ 1/2 GT ] FILTER`],
  ["有限判定 FILTER・なし", `1 1 ${N} RANGE DIV`, `[ 0 MUL 0 EQ ] FILTER`],
  ["有限判定 FILTER・1/0 が1つ", `1 0 ${N - 1} RANGE DIV`, `[ 0 MUL 0 EQ ] FILTER`],
  ["要素ごと DIV・なし", `1`, `1 ${N} RANGE DIV`],
  ["要素ごと DIV・除数0が1つ", `1`, `0 ${N - 1} RANGE DIV`],
];

const median = (xs) => {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
};

for (const [name, setup, body] of cases) {
  const samples = [];
  for (let r = 0; r < rounds; r++) {
    samples.push(Math.max(0, (await best(`${setup} ${body}`)) - (await best(setup))));
  }
  const ms = median(samples);
  console.log(`${name}\t${ms.toFixed(2)} ms\t${((ms * 1e6) / N).toFixed(1)} ns/elem`);
}
