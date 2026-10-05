#!/usr/bin/env node
// Run one case from cases.mjs in a loop, for --cpu-prof (step 2).
//   node --cpu-prof --cpu-prof-dir=<dir> --cpu-prof-interval 100 profile-case.mjs B [n] [seconds]
// The operand is built in the same bench_execute run (setup + src), so the
// profile includes the (fused, ~3 ns/element) setup; see top-self.mjs.
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { CASE_TABLE } from "./cases.mjs";
const here = dirname(fileURLToPath(import.meta.url));
const dir = process.env.AJISAI_WASM_DIR ?? join(here, "..", "..", "..", "src", "wasm", "generated");
const wasm = await import(join(dir, "ajisai_core.js"));
await wasm.default({ module_or_path: readFileSync(join(dir, "ajisai_core_bg.wasm")) });
const id = process.argv[2] ?? "B", n = Number(process.argv[3] ?? 100000), secs = Number(process.argv[4] ?? 4);
const setupOnly = id.endsWith("@setup");
const c = CASE_TABLE.find((x) => x.id === id.replace("@setup", ""));
const src = c.kind === "map" ? (setupOnly ? `${c.setup(n)} LENGTH` : `${c.setup(n)} ${c.src} LENGTH`) : c.src(n);
const end = performance.now() + secs * 1000; let runs = 0;
while (performance.now() < end) { await wasm.bench_execute(src); runs++; }
console.error(`${id}: ${runs} runs of ${n} elements`);
console.log(runs);
