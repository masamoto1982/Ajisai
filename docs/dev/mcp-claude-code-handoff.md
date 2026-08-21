# Ajisai MCP development handoff for Claude Code

Updated: 2026-08-11 (work-meter recalibration; previously trace provenance, response compaction, exactDisplay, onboarding)

- Current tracker: [`mcp-readiness.md`](./mcp-readiness.md)
- Host profiles: [`mcp-host-profiles.md`](./mcp-host-profiles.md)

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
`exactTerms` and its `exactDisplay` rendering.

## 2. Semantics that must survive every adapter

Keep these distinctions at the wire boundary:

| Ajisai outcome | MCP treatment |
|---|---|
| successful value | normal tool result |
| `NIL(reason)` | normal tool result carrying its absence reason |
| Ajisai language `ERROR` | normal structured tool result with diagnosis |
| invalid MCP request, missing backend, timeout or adapter failure | MCP `isError`, `status: "hostError"`, stable `error.code` |

Never translate every Ajisai `ERROR` into `isError`; that discards the language's
diagnostic model. Never serialize arbitrary-precision integers as JSON numbers.
For algebraic values, `semantics.exactTerms` is the value and the rational
approximation is only a convenience representation; `semantics.exactDisplay`
writes those same terms short and is present in exactly the cases they are.

One nuance to keep straight when documenting this: `exactTerms` is the exact
*stored* form, not a canonical form for equality. `8 SQRT` holds `{1/1, 8}` and
`2 SQRT 2 SQRT +` holds `{2/1, 2}`, and `=` decides they are the same number.
Never suggest comparing terms — or `exactDisplay` strings — to decide equality.

A host failure is machine-readable in both directions: `error.code` is stable
and the message is model-facing. Do not put host paths, environment-variable
names or spawn diagnostics into it; that belongs on stderr, and
`HostError.detail` is where a backend hands it over.

## 3. Current implementation map

### Rust semantic boundary

- `rust/src/agent/api.rs`: typed, source-only `compute`, `check` and
  `infer_contracts` boundary. It performs no filesystem or terminal I/O.
- `rust/src/cli/mod.rs`: native `ajisai agent` command used by the Node
  adapter. It accepts `-` for standard input, which is how the adapter passes
  source.
- `rust/src/agent/run_render.rs` and `rust/src/agent/report.rs`: shared report
  assembly and JSON serialization.
- `rust/src/interpreter/word_candidates.rs`: edit-distance "did you mean" for
  an unrecognized Word name.
- `rust/src/interpreter/debug_next_checks.rs`: the repair-checklist table.
  Every entry needs a stable `code` and both locales; the tests in
  `debug_next_checks_tests.rs` fail if either locale carries the other's
  language.
- `rust/src/types/value_protocol.rs`: shared exact-value protocol helpers.
  `exact_terms` and `exact_display` both derive from one `algebraic_normal_form`
  extraction, which is what makes "present in exactly the same cases" a
  structural property rather than a convention two serializers must remember.
  The result schema states the same pairing as `dependentRequired`.
- `rust/src/wasm_interpreter_bindings/wasm_value_conversion.rs`: WASM exposure
  of the same exact-term representation.
- `docs/dev/agent-cli-output-contract.md`: implemented native agent envelope,
  guarded by `scripts/check-agent-cli-contract.mjs`.

### MCP adapter

- `tools/mcp-server/index.js`: stdio server, four tools, Resources, execution
  gate, startup backend selection, asset validation and MCP provenance. Its
  entry-point guard compares **real** paths: the bin is reached through a
  `node_modules/.bin` symlink, and a `resolve()`-only comparison silently made
  every launch-by-name a no-op.
- `tools/mcp-server/doctor.js`: `--version`, `--doctor`, `--help`. Reached only
  when arguments are present, never once a transport is open. It is the one
  surface that deliberately prints host detail — paths, versions, spawn
  failures — because an operator is reading it.
- `tools/mcp-server/mcp-quickstart.md`: hand-written MCP preface that
  `sync-assets.js` composes with `SKILL.md` into `assets/quickstart.md`. Its
  fenced ```ajisai blocks carry `tool=`/`status=`/`stack=` attributes and are
  executed against the live backend by the self-test, so the guidance a model
  is told to trust cannot quietly stop being true.
- `tools/mcp-server/host-error.js`: the host-failure vocabulary. A new failure
  mode needs a code here and in `result.schema.json`, not a new prose string.
- `tools/mcp-server/result.schema.json`: the single `outputSchema` for all four
  tools and the `ajisai://schema/result` Resource; change them together.
- `tools/mcp-server/golden/limits.json`: one entry per declared limit. The
  self-test fails if its key set and the served `LIMITS` differ, so a new
  ceiling cannot be declared without saying how it is exercised.
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

- `eval/cases.json`: 65 intent/semantic cases, each asked in English and
  Japanese (130 prompts).
- `eval/reference-traces.json`: scorer-conformance fixture, **not a real model
  result** — and now says so in its own `provenance` block rather than only in
  prose here.
- `capture-traces.js` and `capture-traces.test.js`: the capture harness and its
  scripted-client test. The test proves prompt assembly and tool-call
  extraction; it is not a model measurement, and says so when it passes.
- `score-traces.js`: tool selection, semantic success, missing trace and
  irrelevant activation metrics.
- `eval/repair-cases.json`, `eval/reference-repair-traces.json` and
  `score-repairs.js`: diagnosis-driven repair harness. The reference is again a
  harness fixture, not model evidence.
- `evaluation-contract.js` and `validate-evaluation.js`: reject malformed,
  duplicated or selectively incomplete evaluation data.
- `benchmark.js` and `eval/performance.json`: post-warmup local latency gate
  (committed p95 budget 1 second) and response-size gate
  (`medianResponseBytesBudget`). Response bytes are deterministic for a fixed
  corpus and engine, so that budget is exact — lower it when a response
  genuinely shrinks, never raise it to paper over an accidental regression.
  The one exception is a deliberate, committed envelope change: `2600` moved
  to `2760` when `observationDigest` (competitive-advantage-work-order-2026-08.md
  Phase 1) became a field every `compute`/`check` response carries — the new
  value is the exact re-measured median with the field present, not a
  rounded-up guess, so the budget stays exact for the new deterministic
  reality rather than exact for a baseline the wire format no longer has.
- `number-baseline.js` and `eval/number-baseline.json`: deliberately narrow
  comparison with JavaScript `Number`, including exact controls. Do not present
  it as a general language or CAS benchmark.

## 4. Current readiness and honest interpretation

The weighted tracker is 81.4%:

- P0 semantic boundary: 100%.
- P1 local stdio beta: 100%.
- P2 agent evaluation: 57%.
- P3 remote service: 0%, deliberately deferred.

P1 completion means the package is self-contained (the WASM backend needs
neither `AJISAI_REPO` nor `AJISAI_BIN`), its declared limits match what it
enforces, and its failures are structured. It does not mean the package has
been published; that is a release decision.

P2 reference traces still only prove the scorers. No real-model success rate
has been established, and the diagnosis work in this round — candidate Words,
stable check codes — is a plausible improvement to repair rate with **no
measurement behind it**. Do not report it as one.

## 5. Recommended next pull request

P1 backend work, P1 onboarding, the algebraic short display and response
compaction are done (see the readiness tracker's P1 section). Three threads
remain, in this order:

0. **Charge arithmetic inside blocks.** Found while recalibrating the work
   meter and *not* fixed there, because it is bigger than what that round
   touched: an operation inside a `MAP` or `FOLD` block never reaches the
   meter. `[ 1 20000 ] RANGE [ 1 ] { * } FOLD` computes a 77,000-digit
   factorial in 580 ms charged **zero** units, with no size ceiling firing.
   Chained arithmetic written out in source is charged and bounded; the same
   arithmetic written as a loop is not, which is the shape any real program
   would use.

   This is also what blocks the remaining `injectedLimit` entries from
   becoming real boundary cases: without a charged loop construct, no source
   inside the 64 KiB `sourceBytes` budget can accumulate enough work to reach
   `numericWork`, and none can reach `bigintBits` either. Fix this first, then
   the boundary sources follow. `golden/limits.json` records the measurements.

1. **Recalibrate the numeric work meter** so the declared size ceilings bind
   before the wall clock. Today `numericWork`, `bigintBits` and
   `algebraicTerms` are declared truthfully and are unreachable at their
   declared values: multiplying two-radical sums grows about twelvefold in
   wall time per factor while charging a handful of work units, so `wallTimeMs`
   always fires first. `golden/limits.json` marks all three `injectedLimit`
   with that reason, and the moment a real boundary source exists they should
   move to `boundary` coverage. Related: plain rational arithmetic does not
   pass through the algebraic size guard at all, so a large integer product is
   bounded only by `executionSteps`.
2. **Collect real model traces** (see §6). The harness is not the bottleneck,
   and as of this round neither is the tooling: `npm run eval:capture` drives a
   real model over the four tools and writes a `model`-provenance trace under
   `eval/traces/`. **What is missing is credentials** — it resolves them the way
   the Anthropic SDK does and, finding none, exits non-zero having written
   nothing. Run it once to establish the baseline, then re-run it under the same
   model, prompt digest and tool-choice setting to compare against.

   The separation is now enforced rather than documented: a trace document is
   rejected unless it declares `provenance.source`, a `model` trace must record
   what makes it re-runnable, and `--require-perfect` is refused on anything
   that is not a `referenceFixture`. Do not weaken any of those to make a run
   pass — the flag asserts that the *scorer* works, and there is no
   corresponding assertion to make about a model.

3. **Provenance size, if it ever becomes the constraint.** Considered and
   rejected during the response-compaction work, recorded so it is not
   re-derived from scratch: on the *smallest* results the `mcp` block is the
   single largest field (416 bytes of a 1,165-byte exact-rational result), and
   about 290 of those are `limits`, identical on every response and already
   published at `ajisai://limits`. Dropping it would shrink small responses by
   a quarter — and would cost exactly what provenance is for, since a stored
   result would no longer say which ceilings produced it. Do not trade that for
   bytes without a demonstrated cost; `registryDigest` and the version pair are
   even less compressible and even more load-bearing.

Do not add MCP tools without demonstrated need. A `trace` tool mirroring the
playground's step mode, and the unused `prompts` capability, are both plausible
and both premature: show the demand in the evaluation corpus first.

## 6. Work after P1

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
npm run test:mcp-backends
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
- Both backends return equivalent stable semantic fields for all golden cases
  *and* the same outcome class at every declared limit boundary.
- Every declared limit still has exactly one `golden/limits.json` entry.
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
- Do not use blocking `spawnSync` — or any other synchronous I/O — in the
  stdio server. The native backend passes source on stdin precisely so it needs
  no temporary file, and must not gain one back.
- Do not declare a limit without adding its entry to `golden/limits.json`; the
  self-test will fail, and that is the point.
- Do not put host detail (paths, environment-variable names, spawn messages)
  into a model-facing error message. Use `HostError.detail`.
- Do not re-select the backend per request. It is fixed at startup and reported
  in `mcp.backend.kind`.
- Do not treat MCP itself as the competitive moat; the value is the combination
  of supported-domain exactness, vector/dataflow semantics and diagnostics.
- Do not claim `reference-traces.json` represents an LLM.
- Do not hand-edit packaged assets; regenerate with `sync:assets` and check the
  resulting diff. `assets/quickstart.md` is composed from `mcp-quickstart.md`
  and `SKILL.md`; edit the preface source, never the composed file.
- Do not test a launch path by importing `createServer`. That is not what a
  client does, and it is why a bin entry that served nothing survived a self
  test, a pack smoke test and a release-readiness review.
- Do not assert on a backend selection made after the first call in the same
  process. The backend is resolved once and cached, so a second in-process
  server ignores a changed `AJISAI_BIN` — spawn a process instead.
- Do not change shared `rust/src/types/value_protocol.rs` without rebuilding
  **both** committed WASM bundles (`npm run build:mcp-wasm` and
  `npm run build:wasm`). They are checked in, so a value-protocol change that
  rebuilds only the MCP one leaves the playground silently a version behind on
  a field the CLI already sends.
- Do not reduce `exactDisplay` to a canonical form. It renders the stored
  normal form, and reducing it would make the string disagree with the
  `exactTerms` printed beside it — trading a surprise a reader can see for one
  they cannot.
- Do not replace the text content block with a prose summary. MCP asks a tool
  with an output schema to also return the serialized JSON there, and it is the
  only route a text-only client has to the result. Compaction means removing
  padding, not information.
- Do not prune empty arrays or all-zero `runtimeMetrics` to save bytes. Both
  were measured (≈1 and ≈7 points of median respectively) and both make
  presence conditional, so `output.length` starts throwing on exactly the
  results that have nothing to report. `null`-valued fields are different:
  absent and `null` mean the same thing to every reader.
- Do not let a `nextChecks` locale carry another locale's language, and do not
  match on its display text anywhere; `code` is the stable identifier.
- Do not delete or subordinate the browser playground to the MCP package.
