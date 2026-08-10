#!/usr/bin/env node
// Runs every golden case against both the native-CLI and WASM-worker
// backends directly (bypassing the server's own backend auto-selection) and
// asserts they agree on every stable semantic field
// (docs/dev/mcp-claude-code-handoff.md §5.4). `runtimeMetrics` and the `mcp`
// provenance block are excluded: they carry host/engine metadata and
// execution counters, not language semantics.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { LIMITS } from "../index.js";
import { NativeCliBackend } from "./native-cli.js";
import { WasmWorkerBackend } from "./wasm-worker.js";

const here = dirname(fileURLToPath(import.meta.url));

function resolveNativeBin() {
  if (process.env.AJISAI_BIN) return process.env.AJISAI_BIN;
  const repoRoot = process.env.AJISAI_REPO
    ? resolve(process.env.AJISAI_REPO)
    : resolve(here, "..", "..", "..");
  for (const profile of ["debug", "release"]) {
    const candidate = join(repoRoot, "rust", "target", profile, "ajisai");
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

const bin = resolveNativeBin();
if (!bin) {
  console.error(
    "backend parity test requires a built native `ajisai` binary; run " +
      "`cargo build --manifest-path rust/Cargo.toml --bin ajisai` first, or set AJISAI_BIN.",
  );
  process.exit(1);
}
if (!WasmWorkerBackend.isAvailable()) {
  console.error(
    "backend parity test requires the packaged WASM bundle; run `npm run build:mcp-wasm` first.",
  );
  process.exit(1);
}

const native = new NativeCliBackend({
  bin,
  wallTimeMs: LIMITS.wallTimeMs,
  responseBytes: LIMITS.responseBytes,
  executionSteps: LIMITS.executionSteps,
});
const wasm = new WasmWorkerBackend({
  wallTimeMs: LIMITS.wallTimeMs,
  executionSteps: LIMITS.executionSteps,
});

// Every field of the schema-1 envelope except the host-metrics/provenance
// fields a backend's own process/thread model incidentally affects.
const STABLE_FIELDS = [
  "schemaVersion",
  "status",
  "stack",
  "stackDisplay",
  "output",
  "message",
  "diagnosis",
  "aiDiagnostic",
  "errorFlowTrace",
  "contractDecls",
];

function stableView(envelope) {
  const view = {};
  for (const field of STABLE_FIELDS) view[field] = envelope[field];
  return view;
}

const golden = JSON.parse(readFileSync(join(here, "..", "golden", "cases.json"), "utf8"));

let failures = 0;
for (const goldenCase of golden.cases) {
  const [nativeEnvelope, wasmEnvelope] = await Promise.all([
    native.compute(goldenCase.source),
    wasm.compute(goldenCase.source),
  ]);
  const left = JSON.stringify(stableView(nativeEnvelope));
  const right = JSON.stringify(stableView(wasmEnvelope));
  const pass = left === right;
  console.log(`${pass ? "PASS" : "FAIL"}  ${goldenCase.name}`);
  if (!pass) {
    failures += 1;
    console.error(`  native: ${left}`);
    console.error(`  wasm:   ${right}`);
  }
}

if (failures) {
  console.error(`${failures} golden case(s) diverged between the native and WASM backends.`);
  process.exit(1);
}
console.log("native and WASM backends agree on every golden case's stable fields.");
