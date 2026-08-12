# MCP product readiness

Status date: 2026-08-11 (updated after the work-meter recalibration; before that, trace provenance, response compaction, algebraic short display and
host-failure work). This is an implementation tracker, not a language
specification. Percentages measure completion of the concrete exit criteria
below; they are not forecasts.

The next-agent implementation handoff is
[`mcp-claude-code-handoff.md`](./mcp-claude-code-handoff.md). Host-by-host
resource ceilings are compared in
[`mcp-host-profiles.md`](./mcp-host-profiles.md).

## Product position

Ajisai's MCP product is a bounded, deterministic and diagnostic computation
kernel for AI agents. Its exactness claim is limited to the numeric domain the
language supports. The browser playground remains a supported, independent
host and shares the value protocol with the native CLI.

## Progress

| phase | weight | complete | weighted contribution |
|---|---:|---:|---:|
| P0 — lossless semantic boundary | 35% | 100% | 35.0% |
| P1 — local stdio beta | 35% | 100% | 35.0% |
| P2 — agent evaluation | 20% | 57% | 11.4% |
| P3 — remote service | 10% | 0% | 0.0% |
| **Overall** | **100%** | — | **81.4%** |

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

### P1 — local stdio beta (100%)

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
- The package smoke test (`pack-smoke.js`) proves four scenarios against the
  packed, installed tarball: computation with neither `AJISAI_REPO` nor
  `AJISAI_BIN` set (the packaged WASM backend); the `ajisai-mcp-server` bin
  serving MCP when **launched by name** through `node_modules/.bin`;
  `--doctor` passing on the installed copy; and an explicit `AJISAI_BIN`
  selecting the native backend, asserted through `mcp.backend.kind`.

  The last scenario previously proved nothing. The backend is resolved once
  per process, so setting `AJISAI_BIN` and constructing a second in-process
  server reused the WASM backend the first scenario had already fixed — the
  "native/Docker path" assertion passed on a machine with no native binary at
  all. Both native scenarios are now spawned processes, and `npm run test:pack`
  consequently requires a built binary (the root `npm run test:mcp-pack`
  builds it first).
- `scripts/rebuild-mcp-wasm.sh` (`npm run build:mcp-wasm`) regenerates the
  committed Node-target WASM bundle
  (`tools/mcp-server/wasm/generated/`), mirroring `rebuild-wasm.sh` for the
  browser build.

- The declared limit table and its enforcement now match. `responseBytes` is
  applied by both backends on the same units (UTF-8 bytes of the schema-1
  envelope text) instead of by the native one alone through `execFile`'s
  `maxBuffer`. `golden/limits.json` holds one entry per declared limit, the
  self-test fails if that key set and the served `LIMITS` differ, and the
  backend parity test compares the outcome class at every boundary — which is
  what the golden cases could never have caught, since none of them produced
  an oversized response.
- Named resource ceilings report themselves. `AjisaiError::ResourceLimitExceeded`
  carries the ceiling's own name, its configured value and the observed size;
  `diagnosis.resourceLimit` surfaces them under the same identifiers the host
  publishes in `mcp.limits`. The source-byte and numeric-literal guards used to
  surface as `Custom` / cause class `unknown`, which told a reader nothing.
- Host failures are structured. A stable `error.code`, a `retryable` flag and
  the declared `limit` a failure is about, in an envelope the shared result
  schema validates. Model-facing messages no longer carry binary paths,
  environment-variable names or spawn diagnostics; that detail goes to stderr.
  The self-test's own string-matching on `"CLI not found"` is gone with it.
- Saturating `concurrentExecutions` queues briefly before answering, so a
  burst is back-pressure rather than a caller-side retry loop.
- The backend is selected once at startup and named in `mcp.backend.kind`.
  Per-request selection meant a `cargo build` finishing mid-session silently
  moved later calls to a different execution path with nothing saying so.
- Packaged assets are validated at startup, so a corrupt registry stops the
  server rather than appearing as a generic failure on some later request.
- All four tools share one output schema, so `word_contract`'s answer shape is
  declared rather than discoverable only by calling it, and an unmatched name
  answers with `suggestions`.
- The native backend passes source on stdin instead of writing a temporary
  file with three synchronous calls per request, and no longer appends a
  newline that made its effective source-byte ceiling one byte tighter than
  the WASM backend's.
- `tools/mcp-server/package.json` is no longer `private`, carries repository,
  licence and publish metadata, and the README gives copy-pasteable client
  configuration JSON.
- **The bin entry actually starts the server.** The entry-point guard compared
  `resolve(process.argv[1])` with `import.meta.url`, and `resolve()` does not
  follow symlinks — so every launch through `node_modules/.bin`
  (`npx -y ajisai-mcp-server`, a bare `"command": "ajisai-mcp-server"`) fell
  through the guard and exited 0 having served nothing: a client saw a server
  that started, offered no tools and reported no error. Only naming `index.js`
  directly ever worked. The self-test and the pack smoke test both imported
  `createServer` rather than launching the executable, so nothing covered the
  path both npm-based README recipes used. Real paths are compared now, and
  `pack-smoke.js` spawns the installed bin.
- The server answers for itself from a terminal: `--version` (adapter version,
  engine version, registry digest), `--doctor` (Node floor, packaged assets,
  backend selection and two real computations through the selected backend,
  exit 0/1) and `--help`. Terminal commands are reached only when arguments
  are present and never after a transport opens, so server-mode stdout stays
  protocol-only.
- Results and the `ajisai://limits` resource carry `mcp.serverVersion` beside
  `mcp.engineVersion`. The adapter and the engine are separately released, and
  a stored envelope naming only the engine could not say which adapter wrote
  it — so a missing field was indistinguishable from a field that adapter
  version never sent.
- **A result costs less without saying less.** Two-space indentation on the
  text content block cost about a third of it, and an optional field carrying
  no value was sent as `null`, so a plain success advertised `message`,
  `diagnosis`, `aiDiagnostic` and `contractDecls`, all empty. Both are gone.
  Across the seven benchmark cases the text block's median fell 32%
  (1,618 → 1,094 bytes) and the whole response's 22% (3,049 → 2,376), measured
  before and after on the same corpus.

  What did **not** change is the mirroring. MCP asks a tool with an output
  schema to also return the serialized JSON in a text block, and that is a
  text-only client's only route to the result — so replacing it with a prose
  summary, which is what the original proposal asked for, is the one compaction
  that loses information. The self-test now pins that a text-only client can
  still tell a value, a reason-carrying NIL, a language error and a host
  failure apart from the text alone, and that the text parses back to exactly
  the structured result.

  Consequently the proposal's ">= 50% content reduction" target is **not met**,
  and deliberately so: 32% is what is available without removing information.
  The remaining levers were measured and declined — pruning empty arrays
  (≈1 point) and dropping all-zero `runtimeMetrics` (≈7 points) both make a
  field's presence conditional, so `output.length` would start throwing on
  exactly the results that have nothing to report. Nested `null` pruning
  (≈4 points on the largest diagnosis) was declined for a different reason: it
  turns one stated rule into a transformation a reader must apply mentally to
  every nested object.

  `eval:performance` now reports median and maximum response size alongside
  latency and fails against a committed `medianResponseBytesBudget`, so the
  padding cannot return unnoticed. Response bytes are deterministic for a fixed
  corpus and engine, so that gate is exact rather than machine-dependent.
  Diagnosis-observation and diagnosis-driven repair rates were re-scored after
  the change and are unmoved.
- **An algebraic value can be read without decoding anything.**
  `semantics.exactDisplay` writes the multiquadratic normal form as one short
  string — `sqrt(2)`, `2/1*sqrt(2)`, `1/1 + sqrt(2)`, `sqrt(2) - sqrt(3)` —
  beside the `exactTerms` it renders. Both derive from a single extraction in
  `value_protocol.rs`, both host serializers emit them together, and
  `result.schema.json` states the pairing as `dependentRequired`, so the wire
  cannot carry one without the other.

  What it replaces is not a missing field but a misleading first impression:
  the two renderings a consumer meets before the terms are `stackDisplay` (the
  SPEC §4.2.3 continued fraction, *truncated at a display budget* — ~194
  characters for √2, ending in `...)`) and the node's own `value` (a rational
  approximation flagged `approximate`). One looks complete and is not; the
  other looks exact and is not. Neither is changed: `stackDisplay` remains the
  shared projection the CLI, REPL and playground render, and altering it stays
  a spec-level decision.

  Recorded rather than smoothed over: `exactDisplay` renders the *stored* form,
  and the stored form is not canonical for equality — `8 SQRT` holds `{1/1, 8}`
  while `2 SQRT 2 SQRT +` holds `{2/1, 2}`, and `=` decides they are equal.
  Reducing the display would make it disagree with the terms beside it, so the
  README, the quickstart and the schema all say instead that comparison decides
  equality and string comparison does not.

  Golden coverage: a scaled algebraic value, a multi-term algebraic value, and
  a rational and a vector of rationals pinned to carry *neither* field — the
  golden runner gained `expectAbsent` for that, since a missing pointer and a
  pointer holding `null` were previously indistinguishable. The backend parity
  test compares the whole `stack`, so native/WASM agreement on the new field is
  covered case by case. The SKILL.md generator now shows the short form for an
  algebraic example, cutting the §6 `2 SQRT` line from ~194 characters to 62
  while still printing only what the real interpreter returned.
- `ajisai://guide/quickstart` is now an MCP preface plus the generated
  `SKILL.md`, not `SKILL.md` alone. The generated guide opens on a CLI run
  loop a connected client cannot issue and never states which of the four
  tools to call. The preface covers tool selection, the `ok`/`error`/
  `hostError` branch, reason-carrying NIL, and the algebraic-value trap
  (`exactTerms` is the value; `stackDisplay` is a *truncated* continued
  fraction and `value` is a marked rational approximation). Every example in
  it is executed against the live backend by the self-test.

All P1 exit criteria are complete. `npm publish` itself remains a release
decision, not an implementation gap — and the README now says so in place of
an `npm install -g ajisai-mcp-server` recipe that resolved to E404, leading
instead with the checkout path, which needs no build because the WASM bundle
is committed.

**The work meter prices operand width now, not operation count.** It used to
charge a term-pair count, so multiplying two 4096-digit numbers and two
one-digit numbers cost the same unit — a meter that cannot see the size of the
numbers cannot bound the thing it exists to bound. Work is charged in
limb-multiply units, and an algebraic term pair carries a measured constant
because it is not one bignum multiply but a coefficient product, a radicand
product, a square-free decomposition against a growing basis and an ordered-map
insert. On the reference container the rational and algebraic paths now run at
~57,000-86,000 units/ms; before, they differed by about 1400x, so a ceiling
calibrated for one was meaningless for the other.

**Plain rational arithmetic is charged and bounded for the first time.** The
scalar fast path returns before the exact-real path that did the charging, so
Tier 0 work reached neither the meter nor any size ceiling: 400 chained
multiplications of a 4096-digit literal — 0.4% of the `executionSteps` budget —
spent 40 seconds building a multi-megabyte integer in silence. The same chain
is refused at ~1.5 seconds by `bigintBits`, which now reports itself by name.
Ordinary arithmetic is unaffected and pinned so: the heaviest ordinary case
measured charges 8,192 units.

**A charge no longer depends on the shape of its operands.** It used to. Two
sites in the engine charged — `push_scalar_fastpath_result` and
`push_exact_real_schema_result` — and both were reached only when both operands
were scalar-shaped. Every other route out of `apply_exact_arithmetic_schema`
(SIMD, sparse, rational broadcast, and the exact-real recursive broadcast that
carries irrational lanes) charged nothing and checked no size ceiling, and
`add_values` — what `SUM` folds with — had no interpreter to charge against at
all. So `2 3 *` was priced at 1 unit and `[ 2 ] 3 *` at nothing, and
`algebraicTerms` was a ceiling a vector literal switched off.

An earlier version of this section blamed `MAP`/`FOLD` blocks, and that was
wrong. `[ 1 21000 ] RANGE 1 { * } FOLD` was refused all along with
`numericWork of 10000573 exceeds the limit of 10000000`, and the same fold
through a user word or through `EXEC` was charged to the same unit —
higher-order application was already path-invariant. What escaped was
`[ 1 21000 ] RANGE [ 1 ] { * } FOLD`, whose only difference is a vector-shaped
accumulator: a 271,233-bit integer past a declared `bigintBits` of 262,144,
charged zero. Straight-line source escaped identically, with no block in sight.

Charging and size-checking now live in `interpreter/arithmetic_meter.rs`, at
the dispatch entry rather than inside the routes it dispatches to. Work is
priced as lanes × operand width, so a scalar is the one-lane case of the same
formula and the existing calibration is unchanged: the heaviest ordinary case
still charges exactly 8,192 units, `2 3 *` and `[ 2 ] 3 *` both charge 1, and
`[ 1 21000 ] RANGE [ 1 ] { * } FOLD` is now refused at the same 10,000,573
units its scalar twin costs. A 1,000-lane vector multiply charges 1,000 — four
orders of magnitude below the ceiling, which is what keeps ordinary array work
untouched.

**A refusal now reports its resource rather than its residue.** It did not.
The fold above is refused by name, but the stack at that moment held a
21,000-element vector and an 81,649-digit partial product, so the envelope came
to 5,773,682 bytes — of which 5,571,973 were the stack — and the 1 MiB
`responseBytes` ceiling turned the whole thing into `responseTooLarge`. The
engine said `numericWork`; the agent was told its *answer* was too big, which
points it at shrinking output when the fix is to compute less.

An error report's answer is its diagnosis; the stack is residual state, and
`agent::error_stack` is where that distinction is spent. On `status: "error"`
only, slots whose values do not fit a 64 KiB budget are replaced in place —
`value` becomes `null`, `type`/`displayHint`/`semantics` still say what the
value was, and an `elided` record says what was dropped, repeated at the
envelope level as `stackElided`. The fold answers in 7,470 bytes with
`diagnosis.resourceLimit.resource: "numericWork"` intact. Values give way,
never reasons: `diagnosis`, `aiDiagnostic`, `errorFlowTrace`, `message` and
`runtimeMetrics` are never touched, and a successful result is never elided at
all — it *is* its stack, so an oversized one is still honestly refused.

`numericWork`, `bigintBits` and `algebraicTerms` stay `injectedLimit`, and for
the first time the reason is neither "it is not charged" nor "the diagnosis
does not survive". It is a **calibration** question. A source reaching
10,000,000 units exists inside the `sourceBytes` budget —
`[ 0 99999 ] RANGE` plus 101 additions charges 10,100,000 and is refused by
name in about 7 KB — but it spends 5.2 s on the reference container's debug
build, past the 5,000 ms `wallTimeMs`, so `wallTimeMs` would decide the case
instead. The prices disagree because the meter charges one unit per one-limb
lane while a boxed per-element operation costs far more wall time than a limb
multiply: that path runs at roughly 7,700 units/ms against the 57,000–86,000
the scalar chains were calibrated at. Widening operands instead of multiplying
lanes swaps which ceiling fires — reaching 10,000,000 units by repeated
multiplication needs an operand wider than 4,096 limbs by construction, so
`bigintBits` arrives first. Ordering the three so each is independently
reachable inside `wallTimeMs` is the open question. `golden/limits.json`
carries the measurements, and `docs/dev/mcp-reevaluation-2026-08-12.md` carries
the reproduction.

### P2 — agent evaluation (57%)

**A trace now says what produced it, and the harness that produces one exists.**
Every trace document declares `provenance.source` — `referenceFixture` or
`model` — and a `model` trace must additionally record the model id, prompt
digest, tool-choice setting, capture time and the server/engine/registry
versions it ran against. Documents without that block are rejected rather than
scored; the scorers print the provenance beside the metrics; and
`--require-perfect` is refused on anything that is not a fixture, because that
flag asserts the *scorer* runs and there is no matching assertion to make about
a model. Until this round the fixture and a real capture were the same shape, so
a fixture's `toolSelectionAccuracy: 1` could be read — or reported — as a model
result.

`npm run eval:capture` drives a real model over the four published tool
definitions, one call per corpus case at `tool_choice: auto` so the
irrelevant-intent cases can correctly produce no call, and writes to
`eval/traces/`, apart from the committed fixtures.

**No baseline has been collected, and the percentage above reflects that.** The
capture harness resolves credentials the way the Anthropic SDK does and, finding
none, exits non-zero having written nothing — a file that looks like a trace and
is not one would be worse than no file. Everything P2 still needs is downstream
of running it: first-attempt generation rate, diagnosis-observation and repair
rates for a real model, and the before/after comparison the `exactDisplay` and
response-compaction rounds are owed. The 2-point movement is for tooling and
enforcement, not for evidence.

#### Earlier P2 work

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
Diagnosis-driven repair now has more to work with: an unknown Word carries
`diagnosis.candidates` (the closest known names, from the compiled-in
vocabulary, the live dictionary, and for `check` the Words the same source
defines), `word_contract` answers an unmatched name with `suggestions`, and
every next-check carries a stable `code` with locale-separated display text so
the repair scorer counts an identifier rather than a mixed-language sentence.
Whether that measurably raises the repair rate is exactly what real model
traces are still needed to say; no claim is made here on the strength of the
harness fixtures.
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
