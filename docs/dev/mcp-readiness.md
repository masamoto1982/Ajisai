# MCP product readiness

Status date: 2026-08-09. This is an implementation tracker, not a language
specification. Percentages measure completion of the concrete exit criteria
below; they are not forecasts.

## Product position

Ajisai's MCP product is a bounded, deterministic and diagnostic computation
kernel for AI agents. Its exactness claim is limited to the numeric domain the
language supports. The browser playground remains a supported, independent
host and shares the value protocol with the native CLI.

## Progress

| phase | weight | complete | weighted contribution |
|---|---:|---:|---:|
| P0 — lossless semantic boundary | 35% | 100% | 35.0% |
| P1 — local stdio beta | 35% | 60% | 21.0% |
| P2 — agent evaluation | 20% | 5% | 1.0% |
| P3 — remote service | 10% | 0% | 0.0% |
| **Overall** | **100%** | — | **57.0%** |

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

### P1 — local stdio beta (60%)

Completed:

- Four focused, source-only tools: compute, check, contract inference and Word
  contract lookup.
- Structured output schemas and read-only/idempotent MCP annotations.
- Static guide, vocabulary, result-schema and per-Word contract resources.
- Source, wall-time, output, step and concurrency limits.
- Real-backend MCP self-test is a blocking CI quality gate.

Remaining exit criteria:

- Replace per-call native CLI processes with a packaged WASM worker backend;
  retain the native CLI as an optional backend.
- Publish a non-private, versioned npm package with clean-install and
  `npm pack` smoke tests on supported Node versions.
- Add explicit algebraic-term, BigInt-bit and materialization limits to the
  runtime agent profile rather than relying only on existing runtime defaults.

### P2 — agent evaluation (5%)

Only protocol-level fixtures exist. A versioned 100–200 prompt evaluation set,
baseline comparisons, tool-selection measurements, first-attempt generation
rate and diagnosis-driven repair rate remain to be implemented.

### P3 — remote service (0%)

Streamable HTTP, authentication, quotas, rate limiting, audit telemetry,
container distribution and registry publication are intentionally deferred
until local evaluation demonstrates demand.

## Release rule

Do not market the server as a public exact-computation MCP until P0 reaches
100%. Do not operate a remote service before P2 has reproducible results. The
stdio beta may continue to mature without removing or weakening the playground.
