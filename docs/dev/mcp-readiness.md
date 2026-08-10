# MCP product readiness

Status date: 2026-08-10 (updated after the packaged WASM backend landed). This
is an implementation tracker, not a language specification. Percentages
measure completion of the concrete exit criteria below; they are not
forecasts.

The next-agent implementation handoff is
[`mcp-claude-code-handoff.md`](./mcp-claude-code-handoff.md).

## Product position

Ajisai's MCP product is a bounded, deterministic and diagnostic computation
kernel for AI agents. Its exactness claim is limited to the numeric domain the
language supports. The browser playground remains a supported, independent
host and shares the value protocol with the native CLI.

## Progress

| phase | weight | complete | weighted contribution |
|---|---:|---:|---:|
| P0 — lossless semantic boundary | 35% | 100% | 35.0% |
| P1 — local stdio beta | 35% | 95% | 33.25% |
| P2 — agent evaluation | 20% | 55% | 11.0% |
| P3 — remote service | 10% | 0% | 0.0% |
| **Overall** | **100%** | — | **79.25%** |

### P0 — lossless semantic boundary (100%)

Completed:

- CLI and WASM expose algebraic normal-form `exactTerms` through one Rust
  extraction function.
- Arbitrary-precision integers cross JSON boundaries as decimal strings.
- A committed result schema describes exact terms, result provenance and
  applied limits.
- The canonical Word registry digest is derived from `spec/words.json`.
- Rust unit tests and the real CLI/MCP integration test guard the boundary.
- The agent CLI contract lists only implemented commands, with a blocking
  source-to-document drift check.
- Committed golden cases cover success, reason-carrying NIL, language ERROR,
  algebraic output and execution-budget exhaustion through the real MCP path.
- Native compute now enters through a typed, source-only Rust `agent_api`; CLI
  rendering consumes the same I/O-free report object.
- Static checking and user-Word contract inference are also available through
  that API; the native JSON commands consume those typed results.
- The packaged MCP adapter calls the common native `agent` boundary for all
  three operations; Node no longer normalizes command-specific result shapes.

All P0 exit criteria are complete.

### P1 — local stdio beta (95%)

Completed:

- Four focused, source-only tools: compute, check, contract inference and Word
  contract lookup.
- Structured output schemas and read-only/idempotent MCP annotations.
- Static guide, vocabulary, result-schema and per-Word contract resources.
- Source, wall-time, output, step and concurrency limits.
- A dedicated Rust agent profile caps materialization, numeric-literal digits,
  numeric work, BigInt bits and algebraic term growth.
- Real-backend MCP self-test is a blocking CI quality gate.
- CI packs the allowlisted npm tarball, installs it into an empty prefix and
  verifies the installed copy against the real backend.
- The npm package carries generated contracts, vocabulary, guide and engine
  provenance. A byte-for-byte drift check keeps those assets synchronized, and
  the package smoke test runs without a repository artifact root.
- A host-neutral `rust/src/agent` module (compute/check/infer-contracts,
  report assembly, contract checking) now compiles for native and `wasm32`
  alike, with no filesystem or terminal I/O. The native CLI (`rust/src/cli`)
  is a thin adapter over it; both host paths render the identical schema-1
  envelope.
- A one-shot WASM entry point
  (`rust/src/wasm_interpreter_bindings/wasm_agent.rs`) exposes that module to
  Node as `agent_compute`/`agent_check`/`agent_infer_contracts`, returning the
  same JSON envelope text the native `ajisai agent` CLI prints. The browser
  playground's stateful `AjisaiInterpreter` session API is unchanged and
  unaffected.
- The Node adapter now defines a small backend interface
  (`tools/mcp-server/backend/`) with two implementations: `NativeCliBackend`
  (a native subprocess per call, as before) and `WasmWorkerBackend` (the WASM
  entry point run inside a `worker_threads` Worker, one per call, terminated
  on the existing hard wall-time limit — never run synchronously on the stdio
  server's main thread). `AJISAI_BIN`/a discoverable local build selects the
  native backend; otherwise the packaged WASM backend is used, with no
  `AJISAI_REPO` or `AJISAI_BIN` required.
- `tools/mcp-server/backend/parity-test.js` (`npm run test:mcp-backends` at
  the repo root) runs every golden case against both backends directly and
  asserts they agree on every stable semantic field (status, stack,
  stackDisplay, diagnosis, aiDiagnostic, errorFlowTrace, output, message,
  contractDecls), excluding host-specific `runtimeMetrics` counters. All
  golden cases currently match byte-for-byte between backends.
- The package smoke test (`pack-smoke.js`) now proves two scenarios against
  the packed, installed tarball: computation succeeds with neither
  `AJISAI_REPO` nor `AJISAI_BIN` set (the packaged WASM backend), and separately
  through an explicit `AJISAI_BIN` (the native/Docker path).
- `scripts/rebuild-mcp-wasm.sh` (`npm run build:mcp-wasm`) regenerates the
  committed Node-target WASM bundle
  (`tools/mcp-server/wasm/generated/`), mirroring `rebuild-wasm.sh` for the
  browser build.

Remaining exit criterion:

- Publish a non-private, versioned npm package. The backend is now
  self-contained rather than native-binary dependent, so the previous
  technical blocker is resolved; publishing itself is a deliberate release
  decision this PR does not make.

### P2 — agent evaluation (55%)

A first versioned prompt corpus now covers tool intent and backend semantics for
rationals, decimals, algebraics, vector broadcast, NIL, diagnostics, static
checking and contracts. It is intentionally only a seed: expansion to 100–200
prompts remains necessary. A trace scorer now measures tool selection,
end-to-end semantics, missing traces and irrelevant activation; the committed
perfect reference trace verifies the scorer only. Real model traces, baseline
comparisons and first-attempt generation rate remain to be collected. A
separate repair scorer now requires the expected structured diagnosis before a
corrected attempt can count, with seed cases for unknown Words, stack shape and
malformed source. Its perfect reference is a harness fixture; real-model repair
rates remain unmeasured.
Corpus and trace contracts now reject duplicate/unknown IDs, unknown tools,
malformed expectations and incomplete reference fixtures before scoring.
The selection corpus now has 22 prompts, adding large-integer precision,
rational reduction, pairwise vectors, domain NIL, exact comparison, modulus,
static-check failure, alias lookup and additional irrelevant intents.
A reproducible local benchmark now reports p50, p95 and maximum latency across
compute, checking, inference and registry lookup, with a blocking one-second
p95 budget after warmup. Remote-service latency remains unmeasured.
A deliberately narrow JavaScript `Number` baseline now compares five rational,
decimal and integer cases, including exactly representable controls. Broader
SymPy/Wolfram and raw-model comparisons remain outstanding.

### P3 — remote service (0%)

Streamable HTTP, authentication, quotas, rate limiting, audit telemetry,
container distribution and registry publication are intentionally deferred
until local evaluation demonstrates demand.

## Release rule

Do not market the server as a public exact-computation MCP until P0 reaches
100%. Do not operate a remote service before P2 has reproducible results. The
stdio beta may continue to mature without removing or weakening the playground.
