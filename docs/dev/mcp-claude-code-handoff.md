# Ajisai MCP development handoff for Claude Code

Updated: 2026-08-10  
Baseline commit at handoff: `38709cf`  
Current tracker: [`mcp-readiness.md`](./mcp-readiness.md)

This document is the starting point for the next pull request. Read it together
with `AGENTS.md` files found in the working tree, the readiness tracker, and
`tools/mcp-server/README.md`. Re-check the baseline commit after checkout: the
repository may squash the preceding MCP work into a new commit.

## 1. Product decision and non-goals

Ajisai is being positioned as a **bounded, deterministic and diagnostic
computation kernel for AI agents**, distributed first through local MCP stdio.
It is not trying to win adoption by asking users to replace a major general
purpose language.

The supported claim is **exactness in Ajisai's supported numeric domain**. Do
not change this to an unqualified “no rounding error” claim: explicit rounding,
unsupported transcendental operations and resource exhaustion still have their
documented semantics.

The browser playground is a supported, independent host. **Do not remove,
replace or weaken it while building the MCP product.** Native CLI, WASM and MCP
must continue to agree on shared value-protocol semantics, especially
`exactTerms`.

## 2. Semantics that must survive every adapter

Keep these distinctions at the wire boundary:

| Ajisai outcome | MCP treatment |
|---|---|
| successful value | normal tool result |
| `NIL(reason)` | normal tool result carrying its absence reason |
| Ajisai language `ERROR` | normal structured tool result with diagnosis |
| invalid MCP request, missing backend, timeout or adapter failure | MCP `isError` |

Never translate every Ajisai `ERROR` into `isError`; that discards the language's
diagnostic model. Never serialize arbitrary-precision integers as JSON numbers.
For algebraic values, `semantics.exactTerms` is canonical and the rational
approximation is only a convenience representation.

## 3. Current implementation map

### Rust semantic boundary

- `rust/src/cli/agent_api.rs`: typed, source-only `compute`, `check` and
  `infer_contracts` boundary. It performs no filesystem or terminal I/O.
- `rust/src/cli/mod.rs`: native `ajisai agent` command used by the Node adapter.
- `rust/src/cli/run_render.rs` and `rust/src/cli/report.rs`: shared report
  assembly and JSON serialization.
- `rust/src/types/value_protocol.rs`: shared exact-value protocol helpers,
  including algebraic `exact_terms`.
- `rust/src/wasm_interpreter_bindings/wasm_value_conversion.rs`: WASM exposure
  of the same exact-term representation.
- `docs/dev/agent-cli-output-contract.md`: implemented native agent envelope,
  guarded by `scripts/check-agent-cli-contract.mjs`.

### MCP adapter

- `tools/mcp-server/index.js`: stdio server, four tools, Resources, execution
  gate, native subprocess backend and MCP provenance.
- `tools/mcp-server/result.schema.json`: tool `outputSchema` and the
  `ajisai://schema/result` Resource; change them together.
- `tools/mcp-server/assets/`: packaged vocabulary, complete Word contracts,
  quickstart and engine/registry metadata.
- `tools/mcp-server/sync-assets.js`: generates or byte-checks those assets.
  Run it after changing `spec/words.json`, `docs/word-manifest.json`, `SKILL.md`
  or the root package version.
- `tools/mcp-server/selftest.js` and `golden/cases.json`: protocol and real
  backend regression coverage.
- `tools/mcp-server/pack-smoke.js`: packs, independently installs and exercises
  the npm tarball without using a repository path for static resources.

### Evaluation

- `eval/cases.json`: 22 intent/semantic cases.
- `eval/reference-traces.json`: scorer-conformance fixture, **not a real model
  result**.
- `score-traces.js`: tool selection, semantic success, missing trace and
  irrelevant activation metrics.
- `eval/repair-cases.json`, `eval/reference-repair-traces.json` and
  `score-repairs.js`: diagnosis-driven repair harness. The reference is again a
  harness fixture, not model evidence.
- `evaluation-contract.js` and `validate-evaluation.js`: reject malformed,
  duplicated or selectively incomplete evaluation data.
- `benchmark.js` and `eval/performance.json`: post-warmup local latency gate;
  current committed p95 budget is 1 second.
- `number-baseline.js` and `eval/number-baseline.json`: deliberately narrow
  comparison with JavaScript `Number`, including exact controls. Do not present
  it as a general language or CAS benchmark.

## 4. Current readiness and honest interpretation

At handoff the weighted tracker is 75.75%:

- P0 semantic boundary: 100%.
- P1 local stdio beta: 85%.
- P2 agent evaluation: 55%.
- P3 remote service: 0%, deliberately deferred.

P0 completion means the semantic/wire boundary is ready for continued local
integration. It does not mean the npm package is zero-configuration. The
package includes its static Resources, but execution still needs
`AJISAI_BIN`. P2 reference traces only prove the scorers; no real-model success
rate has been established yet.

## 5. Recommended next pull request

Prioritize the remaining P1 backend work before adding more surface area:

1. Define a small backend interface in the Node adapter for `compute`, `check`
   and `infer_contracts`; keep the existing native CLI implementation as one
   backend.
2. Add a packaged Node `worker_threads` WASM backend and make it the default
   only after it emits the same schema-1 envelope as the native agent API.
3. Terminate the Worker on the existing hard wall-time limit. Do not run WASM
   synchronously on the stdio server's main thread.
4. Run every golden case against both backends and compare stable semantic
   fields, excluding incidental timing/counter differences.
5. Extend the package smoke test so computation succeeds with neither
   `AJISAI_REPO` nor `AJISAI_BIN` set. Keep `AJISAI_BIN` as an explicit optional
   override for native/Docker use.

Important implementation constraint: `rust/src/lib.rs` currently exposes the
CLI module only for non-WASM targets, while the existing browser
`AjisaiInterpreter.execute` result is not the native agent envelope. Do not
paper over that mismatch with a large Node-side normalizer. Prefer extracting a
host-neutral typed agent/report module in Rust and exporting a one-shot WASM
entry point that serializes the same envelope. Preserve the playground's
session-oriented interpreter API alongside it.

If that refactor is too large for one reviewable PR, first land only the backend
interface plus native-backend conformance tests. Do not mark P1 complete until
the installed tarball actually runs without a native binary.

## 6. Work after the WASM backend

P2 should then proceed with evidence rather than more perfect fixtures:

1. Expand the prompt corpus toward 100–200 versioned cases with balanced
   positive and irrelevant intents.
2. Capture real model traces, including model/version, prompt-template digest,
   temperature/tool-choice settings and timestamp.
3. Report first-attempt generation success and diagnosis-driven repair success.
4. Add fair SymPy/Wolfram/raw-model baselines with identical tasks and explicit
   supported-domain exclusions.
5. Keep raw traces separate from committed reference fixtures and never label a
   reference-fixture score as model performance.

Do not begin P3 HTTP operation merely because local tests pass. Remote service
work requires demonstrated demand and reproducible P2 results, then auth,
quota, rate limiting, audit telemetry and container distribution.

## 7. Required checks before each commit

From the repository root:

```sh
npm ci
npm ci --prefix tools/mcp-server
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml --all-targets
npm run check
npm run lint
npm test
npm run check:agent-cli-contract
npm run check:mcp-assets
npm run check:mcp-evaluation
npm run test:mcp
npm run test:mcp-pack
npm run eval:mcp
npm run eval:mcp-performance
npm run eval:mcp-number-baseline
npm run eval:mcp-traces
npm run eval:mcp-repairs
git diff --check
```

`npm run test:mcp-pack` may use already lockfile-installed dependencies when a
development sandbox cannot access the npm registry; CI must exercise the clean
install path. Performance numbers vary by host, so record the measured values
but do not silently raise the committed budget to fix a regression.

## 8. Definition of done for the next PR

- The change is reviewable and does not add new MCP tools without demonstrated
  need.
- Native and any new backend return equivalent stable semantic fields for all
  golden cases.
- `NIL`, language `ERROR`, host failure and timeout remain distinct.
- Algebraic `exactTerms` survive every tested host boundary.
- The installed package is tested, not only the repository copy.
- Generated assets and documentation pass their drift checks.
- The playground builds and its existing tests remain green.
- `docs/dev/mcp-readiness.md` is updated only for exit criteria actually met.
- Changes are committed on the current branch and the PR description reports
  limitations and environment warnings honestly.

## 9. Known traps

- Do not accept file paths in MCP execution tools.
- Do not use blocking `spawnSync` in the stdio server.
- Do not treat MCP itself as the competitive moat; the value is the combination
  of supported-domain exactness, vector/dataflow semantics and diagnostics.
- Do not claim `reference-traces.json` represents an LLM.
- Do not hand-edit packaged assets; regenerate with `sync:assets` and check the
  resulting diff.
- Do not publish while `tools/mcp-server/package.json` remains `private` or
  while execution depends on an unstated local binary.
- Do not delete or subordinate the browser playground to the MCP package.
