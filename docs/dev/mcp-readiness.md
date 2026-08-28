# MCP product readiness

Status date: 2026-08-13 (updated after the P1-2 entry-surface round, the
resource-progress diagnosis fix, the first model baseline, the P1-1 corpus round
and the collection-word billing round; before that, the work-meter
recalibration, trace provenance, response compaction, algebraic short display
and host-failure work). This is an implementation tracker, not a language
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
| P2 — agent evaluation | 20% | 75% | 15.0% |
| P3 — remote service | 10% | 0% | 0.0% |
| **Overall** | **100%** | — | **85.0%** |

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
  characters for √2, ending in `...]`) and the node's own `value` (a rational
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

**Rational addition is priced as the multiplication it performs.** It was
priced as linear — `max(limbs)`, the cost of an *integer* add. Ajisai's `+` is
rational: `a/b + c/d` is `(ad + cb)/(bd)` and then a gcd, three multiplications
and a Euclid. Measured with the literal parsed once
(`cargo run --release --example work_meter_calibration`), a 4096-digit addition
takes **745 µs** against the same-width multiplication's **366 µs** — and used
to be charged **213 units** against that multiplication's **45,369**. A 4,500×
pricing error, in the dangerous direction, on the one path with no other bound:
a chain of wide additions could spend the whole `wallTimeMs` budget while the
meter read a fraction of a percent. Corrected, that path charges ~66,000
units/ms instead of ~286, and every schema is now one price at one width.

The remaining spread is between paths bounded by something else: a scalar
operation on machine words (~940 units/ms) is bounded by `executionSteps`, and
a dense `i64` lane (~6,400 units/ms) by `maxMaterializedElements` — 10,000,000
units of lane work is about 1.6 s, inside `wallTimeMs`. What mattered was that
no *unbounded* path sat far below the rest, and none does now.

**Writing a value down is budgeted by work now, not by term count.** The
dominant cost of an algebraic value was never computing it. `stackDisplay` is a
continued fraction (SPEC §4.2.3), and expanding one needs enclosure refinement
that grows with the term count; the budget was 32 partial quotients however
much each cost. Measured at 2 / 4 / 8 / 16 / 32 terms, the run took 0.1 ms
throughout while the render took 5 ms / 57 ms / 951 ms / 9.1 s / **147 s** —
about 12× per doubling, priced by nothing, and caught only by `wallTimeMs`,
which answers with a timeout that names nothing.

`CF_OBSERVATION_WORK_BUDGET` makes it a work budget, charged per
floor-and-reciprocate step at `terms³ × limbs` before the step runs. The same
table now reads 4.7 ms / 18.9 ms / 4.9 ms / 5.8 ms / 0.0 ms. `2 SQRT` still
expands to all 32 quotients — the common case pays nothing — while a value too
dear to expand renders the `...]` truncation it was always entitled to, or the
`[ ...]` undetermined marker when not even `a0` is affordable. Nothing is lost:
for an algebraic value the CF is a rendering and `exactTerms` is the value. The
same budget bounds `best_rational_approximation`, which feeds the
`approximate: true` rational on the wire through the identical expansion.

**Seven ceilings had `boundary` coverage; nine do now.** With the display cost
bounded, both of the limits that were shadowed by it became reachable at their
declared values:

- `numericWork` — a twelve-factor multiquadratic cascade reaches 4,096 terms
  for ~8.4M units and succeeds; the thirteenth doubling costs 16,799,744 and is
  refused by name. It had been `injectedLimit` for three rounds and three
  different reasons: arithmetic outside the scalar path was not charged at all,
  then the refusal's 27 MB envelope became `responseTooLarge`, then the twelfth
  doubling could not be reached because rendering 32 terms took 147 s.
- `bigintBits` — nineteen multiplications by a 4096-digit literal reach 77,824
  digits and succeed; the twentieth reaches 272,133 bits and is refused by
  name, in ~16 ms and 4.2 KB of source.

**All declared ceilings are live now, and a test says so.**
`algebraicTerms` was the last one shadowed: the doubling that would first
exceed 4,096 terms charges 16,799,744 units against a 10,000,000 work budget,
so `numericWork` answered every time and the term ceiling had never fired in
its life.

The three are not independent dials. Building a large *exact* value requires
work — not as an implementation artifact but as what exactness means, since
every digit and every term was actually computed — so each size ceiling carries
a minimum work cost, and any profile is subject to
`work_to_reach(size ceiling) < max_numeric_work`. Violating it declares a
ceiling nothing can hit: the program is still refused, but under the wrong
name, and the name is what an agent repairs from — "you exceeded the work
budget" and "stop multiplying surds like that" are different instructions.

It is satisfied by lowering the size ceiling rather than raising the work
budget, because the work ceiling bounds a *run* and a size ceiling bounds one
value *inside* it: a control that catches a specific shape faster and with a
better name belongs inside the general one, not above it. Raising
`max_numeric_work` to ~20M would also have worked, at the price of doubling the
compute an agent may spend per call and eroding the property that a named
ceiling fires before the anonymous `wallTimeMs` timeout.

`max_algebraic_terms` is now **512**, derived rather than rounded: `exactTerms`
for 512 terms is 31,745 bytes, 3.0% of `responseBytes`, where 4,096 terms was
278,110 bytes and 26.5% — a quarter of the whole response for one number that
renders as `[ ...]` and that no computation wants. Nine two-radical factors
reach 512 terms and succeed; the tenth doubling is refused by name in ~13 ms at
21% of the work budget, so a re-measure of `ALGEBRAIC_PAIR_UNITS` cannot
silently change which ceiling answers. `max_bigint_bits` is deliberately *not*
lowered alongside it: 78,913 digits is a number a program can want, and its 14%
margin to `numericWork` is a measurement to record, not a defect to fix by
mutilating a useful limit. Treating the two as a matched pair would have been
tidiness rather than correctness.

`rust/src/agent/profile_liveness_tests.rs` is the part that matters. It fails
the build if any declared size ceiling sits past what the work budget can pay
for, and if the work ceiling stops being reachable once the size ceilings bind.
Three rounds of external review did not find this, because nothing checked it.
Now something does.

**A ceiling now reports the number it judges by.**
`runtimeMetrics.executionSteps` was read from a `RuntimeMetrics` field that no
code ever wrote, sitting beside the `Interpreter::execution_step_count` every
limit check increments. Two counters for one fact, and the reported one was the
one that was always zero — a 21,000-step fold and an empty program described
their work identically, and the ceiling an agent was told to plan against could
not be observed at all. The phantom field is gone and the report reads the
counter the check reads.

Alongside it, `resourceUsage` separates *what a run spent* from *how the runtime
went about it*. Every key names a `mcp.limits` key and carries the same number
the ceiling compared against, so an agent can subtract:
`{ "executionSteps": 22, "numericWork": 20, "collectionWork": 340 }`. Only the accumulating ceilings
appear — `bigintBits` and `algebraicTerms` are checked per result and never
accumulated, so there is no peak to report, and none is invented, which is the
same discipline that made the phantom field a defect rather than a feature.
`runtimeMetrics.executionSteps` stays as a compatibility alias carrying the
same reading, because removing a field is what a schema version is for; that it
lived in the optimizer object beside cache-hit counters is how nobody noticed
it was constant.

**What replaced the `wallTimeMs` over-case was itself a finding, and it has now
been acted on.** That case was the four-factor product, and it stopped timing
out the moment rendering stopped costing seconds. The candidate that replaced
it was `UNIQUE`, which was quadratic and priced by nothing: 48 s at the
100,000-element materialization ceiling, as *one* execution step out of a
hundred thousand. The work meter priced arithmetic, and the collection Words did
real work that no ceiling counted.

**`collectionWork` is the eleventh declared ceiling, and it closes that.** What
it charges is deliberately not the element count, because measurement ruled
that out: the same `UNIQUE` over the same 16,000 elements costs 0.52 ms with one
distinct value and 682 ms with sixteen thousand — a factor of 1,300 no length
can see — an element that is itself a 64-element vector costs 41x more to probe
when the elements share a prefix than when they differ at the first position,
and an algebraic element costs 3.0 µs to compare against 5.8 ns for a machine
word. So the price is *operations × the measured cost of one element*: leaves
per element, limbs per leaf, and 512 units for an algebraic one. The copy and
comparison families know their operation count before they start and are
pre-charged at the entry; the equality scans do not — the count is the distinct
count, which is what the scan is finding out — and are charged per element
against the distinct values already in hand, which bounds that element's probes.
`[ 0 99999 ] RANGE UNIQUE` is now refused by name in 164 ms. The derivation,
including what it over-charges and by how much, is in
[`collection-word-billing-2026-08-13.md`](./collection-word-billing-2026-08-13.md);
`examples/collection_word_calibration` reproduces every table in it.

**With that, no source program reaches `wallTimeMs`, and its coverage moved to
`hostGate`.** The slowest program the named ceilings admit runs about 1.4 s
(≈0.7 s of `numericWork` plus ≈0.65 s of `collectionWork` at each meter's
slowest measured path), so 5,000 ms is now a backstop on their sum rather than
the thing that catches expensive programs — which is what a wall clock should
be. It is exercised through the adapter that enforces it, against a backend
built with a 1 ms deadline. Finding a source-program boundary pair for it again
would mean a ceiling had gone quiet.

**One measurement found a defect rather than a price.** `LENGTH` reads a count
and is documented O(1), but `extract_vector_elements` deep-copied the whole
element vector to call `.len()` on the copy: 18 ms and 100,000 clones at the
materialization ceiling, and the third most expensive linear Word in the family.
It reads the header now and charges nothing.

`golden/limits.json` carries the measurements,
`examples/work_meter_calibration` and `examples/collection_word_calibration`
reproduce them, and `interpreter/work_meter_calibration_tests.rs`,
`interpreter/collection_meter_tests.rs` plus `types/exact/cf_budget_tests.rs`
pin the shapes without a wall clock in CI.

**Still unpriced, and recorded rather than fixed**: the text family. `STR` on a
BigInt is a decimal conversion, quadratic in the digit count — the output-side
mirror of `numericLiteralDigits`. At the widths `bigintBits` admits it measures
in the tens of milliseconds rather than seconds, so it is a follow-up and not a
hole of the size this round closed.

### P2 — agent evaluation (75%)

**A trace now says what produced it, and a real one has been captured.**
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

**The corpus is now bilingual and 130 prompts, and every metric the baseline
will report exists and is exercised.** Three changes, all of them things a
credential does not gate:

- **日英 1:1.** Every case carries an `en` and a `ja` prompt naming the same
  task, so both askings share one expected tool and one expected result. The
  scorers report per language and publish the difference as `languageGap`.
  Ajisai is a Japanese-authored language with an English tool surface, so "does
  a Japanese prompt reach the same tool with the same source" is a product
  question; a single-language corpus could not ask it. The contract rejects a
  pair whose two sides are the same string — a copied prompt would score twice
  and report a comparison it never made.
- **65 cases, 130 prompts** (was 22 and 22), inside the 100–200 target this
  tracker has carried since the corpus was seeded. The new cases are the
  collection family (16), higher-order (5), text (5), the four resource
  ceilings, four diagnostic shapes, and two more irrelevant-intent cases. Every
  reference argument is verified against the real engine by `npm run eval:mcp`.
- **First-attempt generation rate is its own metric.** It was folded into
  `semanticSuccessRate`, which required the right tool *and* the right source,
  so a tool-selection failure and a code-generation failure were reported as one
  number and neither was named. They have different repairs, so they are now
  counted separately — generation over the positive cases only, since a case
  whose correct answer is no call has nothing to generate.

`eval/reference-traces.json` is now generated from the corpus
(`npm run eval:reference-traces`, drift-checked in `eval:validate`): a perfect
fixture *is* the corpus answering itself, and hand-maintaining 130 of them meant
a new case failed `--require-perfect` for a reason that had nothing to do with
the scorer it asserts.

The repair corpus gained the ceiling the collection round added. The claim that
`collectionWork` is worth being a ceiling of its own is that it sends a repair
at the collection rather than at the arithmetic; `repair-collection-ceiling`
makes that measurable, and its source contains no arithmetic at all, so a
repaired attempt that succeeds can only have shrunk the collection.

**The first model baseline has been collected.** `claude-opus-5`, 130 selection
prompts and 8 repair prompts, captured 2026-08-13 against server 0.3.0 / engine
0.2.0-beta.1. The trace documents are committed under `eval/traces/`; every
number below is reproducible by re-scoring them, and none of them is asserted —
`--require-perfect` still refuses a model trace.

| metric | overall | en | ja |
|---|---:|---:|---:|
| tool selection accuracy | 0.469 | 0.462 | 0.477 |
| reached expected tool *first* | 0.338 | 0.385 | 0.292 |
| first-attempt generation rate | 0.331 | 0.322 | 0.339 |
| semantic success rate | 0.392 | 0.385 | 0.400 |
| irrelevant tool rate | **0.000** | 0.000 | 0.000 |

| repair metric | overall | en | ja |
|---|---:|---:|---:|
| diagnosis observed | 1.000 | 1.000 | 1.000 |
| diagnosis-driven repair | 0.750 | 0.750 | 0.750 |

**There is no language penalty.** The gap is −0.015 on selection and −0.017 on
generation: Japanese is *marginally ahead*, which at 65 pairs is indistinguishable
from zero. The question the pairing was built to ask has an answer, and it is the
reassuring one — an English tool surface does not cost a Japanese caller
anything measurable.

**The dominant failure is under-use, not misuse.** Of 118 positive prompts, 21
produced no tool call at all and another 46 called only `word_contract` — the
model looked the Word up and then answered from its own head. Together that is
57% of prompts that ended without running the program. Against
`irrelevantToolRate: 0.000` — perfect restraint on the six irrelevant-intent
cases — the shape is unambiguous: the model is not over-reaching for Ajisai, it
is declining to reach at all, most often for questions it believes it can answer
itself (`10 MOD 4`, sorting three integers, the length of a three-element
vector). For an engine whose whole proposition is that a model's own arithmetic
should not be trusted, **that is the product finding of this round**, and it is
the evidence P1-2's quickstart work was waiting for.

**Read case by case, the 67 have two causes, and neither is reluctance.**

- The 21 that called nothing land where the `compute` description did not
  reach. It said "exact rational, decimal, square-root and vector
  calculations" — the numeric third of the product — and never mentioned
  collections, higher-order blocks or text. Twelve of the 21 asked for
  collection work. A caller reading only that description cannot tell the
  engine sorts, deduplicates or groups, so declining was the correct reading of
  what it was told.
- The 46 that called only `word_contract` called it *twice*, at names Ajisai
  has never had: `vec-add`, `v+`, `vec-map`, `group-by`, `nil-or`,
  `nil-default`, `dict`. Those are not a model refusing to use the engine —
  they are a model trying to and not knowing what anything is called. Nothing
  readable before the first call named a single Word, because the vocabulary
  lives in `ajisai://vocabulary` (45 KB) and the Word table inside the 26 KB
  quickstart, and a caller has to *decide* to fetch either.

Both are entry-surface defects, and the entry surface is smaller than it looks:
with `tool_choice: auto`, the four tool descriptions are the only text read
before the first call. So that is where the fix went. `compute` now names every
family and the Words in them, says names are exact and case-sensitive, and
points at `ajisai://vocabulary`; `word_contract` says the whole list is one
resource read away instead of leaving probing as the only visible option. The
quickstart preface opens with the same table (§0) and is 7,176 bytes, inside the
8 KB the reevaluation set for it.

`tool-description.test.js` keeps both properties: every Word family the registry
declares must be announced by at least one of its Words, and every name the
descriptions mention must exist. The second is the failure the baseline
measured, pointed at ourselves — an entry surface that sends a caller after a
Word that is not there.

**The effect is measured, and it is the largest single movement this tracker has
recorded.** Same 65 cases, same 130 prompts, same model, same thin system
prompt; only the tool descriptions and the preface changed
(`claude-opus-5-after-entry-surface.json`).

| metric | before | after |
|---|---:|---:|
| tool selection accuracy | 0.469 | **0.862** |
| reached expected tool first | 0.338 | **0.762** |
| first-attempt generation rate | 0.331 | **0.585** |
| semantic success rate | 0.392 | **0.623** |
| irrelevant tool rate | 0.000 | **0.000** |

Read as counts, the two diagnosed causes are what moved: prompts that reached
the expected tool went 49 → 100 of 118, prompts that called nothing went 21 → 4,
and turns spent only guessing names in the registry went 46 → 13. **The gain
cost nothing in restraint** — `irrelevantToolRate` is still 0.000, so the model
did not start over-reaching, it stopped under-reaching. The language gap remains
negligible (0.031 selection, 0.017 generation, now marginally favouring English).

**The remaining failure moved from selection to writing, so the same fix was
applied one level down and measured too.** 31 of 130 prompts reached `compute`
and handed it source that did not produce the expected result. Every rule below
was executed against the engine before being written down, which corrected two
guesses made from reading the sources alone: `[1 2 3]` without inner spaces runs
fine, and so does `[ 1, 2, 3 ]` with commas. Neither is a cause.

| the model wrote | Ajisai wants | n |
|---|---|---:|
| `"hi" CHARS` | `'hi' CHARS` — a string is single-quoted | 8 |
| `[ 2 MOD 0 EQ ] FILTER` | `{ 2 MOD 0 = } FILTER` — a block is braces | 4 |
| `5 RANGE` | `[ 0 4 ] RANGE` — `RANGE` takes a bounds vector | 4 |
| `[ 1 2 ] [ 3 4 ] ZIP` | `[ [ 1 2 ] [ 3 4 ] ] ZIP` — one vector of vectors | 1 |

Those went into the `source` parameter description, which is read with the tool.
All four landed on their targets — the same prompts now produce
`[ 'a' 'b' 'a' ] UNIQUE`, `[ 0 4 ] RANGE`, `[ [ 1 2 ] [ 3 4 ] ] ZIP` and
`{ 2 MOD 0 EQ } FILTER`.

**Three captures, one corpus, one model:**

| metric | baseline | + entry surface | + syntax rules |
|---|---:|---:|---:|
| tool selection accuracy | 0.469 | 0.862 | 0.862 |
| reached expected tool first | 0.338 | 0.762 | 0.746 |
| first-attempt generation rate | 0.331 | 0.585 | **0.763** |
| semantic success rate | 0.392 | 0.623 | **0.777** |
| irrelevant tool rate | 0.000 | 0.000 | **0.083** |

Semantic success roughly doubled across the two rounds, and each round moved the
number it was aimed at: the first moved selection, the second moved generation.

**The second round cost something, and the number says so.** `irrelevantToolRate`
left 0.000 for the first time — one of the six irrelevant-intent cases, in
Japanese only. Asked for a haiku about hydrangeas, the model composed one and
then called `compute` to count the mora of each line
(`[ 'あめあがり' … ] { CHARS LENGTH } MAP`). Whether that is over-reach or a
counting engine used for a counting sub-task is a fair question, and the corpus
answers it as a miss. **The case is not being changed.** A negative case that
catches a change is doing its job, and rewriting it after it fires would make
every later restraint number meaningless.

**What was changed instead is the resolution of the metric that caught it.** Six
negative cases means one miss moves the rate by 8 points overall and 17 in one
language — not enough to tell a regression from an unlucky prompt, which is the
wrong basis for trading away a measured gain. The negative set is now 20, and
the fourteen additions probe the boundary rather than the obvious: numeric asks
the closed domain genuinely excludes (sine, natural log, pi's digits), numeric
asks whose data the engine does not have (currency conversion, a weekday, a
distance), asks that contain a number but are not calculations (explain postfix
notation, estimate a reading time, name a variable), and programming asks in
another language. The existing six stay.

Growing the corpus breaks comparability, so the scorer now says which rates
survive it. `positiveSelectionAccuracy` (new) and `firstAttemptGenerationRate`
are computed over the positive cases, which did not change, so the series holds:

| metric | baseline | + entry surface | + syntax rules |
|---|---:|---:|---:|
| positive selection accuracy | 0.415 | 0.847 | 0.856 |
| first-attempt generation rate | 0.331 | 0.585 | 0.763 |

`toolSelectionAccuracy` and `semanticSuccessRate` mix the two classes and only
compare within one composition; `irrelevantToolRate` restarts, because its
denominator is what changed. Every trace document records `composition` so a
later reader can tell which corpus a number came from rather than inferring it.

### The expanded negatives, measured

`claude-opus-5-after-negatives.json` — 158 prompts, same server as the syntax
round, so the only difference from the previous capture is the 14 added cases.
The comparable series continues, and the two rounds' gains hold:

| metric | baseline | + entry surface | + syntax rules | + negatives |
|---|---:|---:|---:|---:|
| positive selection accuracy | 0.415 | 0.847 | 0.856 | 0.864 |
| first-attempt generation rate | 0.331 | 0.585 | 0.763 | 0.771 |

`irrelevantToolRate` is **0.175** — 7 activations in 40 negative prompts, the
first number this metric has had at usable resolution. It is not comparable to
the 0.083 of the previous round, and the movement is not a regression: the 14
new cases were chosen to be hard, and all 7 activations are theirs.

**The original six are clean, including the one that fired last round.** Twelve
prompts, no calls — the haiku/mora case did not repeat. One miss on six cases
was, as suspected, inside the noise of a single prompt. That is the argument for
the expansion restated as evidence.

**The transcendentals group is clean too, and that is a description working.**
Sine, natural log and pi's digits produced no call in either language. These are
numeric asks phrased exactly like the positive cases; what separates them is that
the closed domain excludes them, and the only place a caller learns that before
its first call is the "Out of domain: transcendentals" sentence in the `compute`
description. Three cases in two languages is not proof, but it is the sentence's
first test and it passed.

**All 7 activations are one behaviour, and it is not the failure the metric was
built to catch.** Read them together:

| case | what it called |
|---|---|
| `irrelevant-weekday` (en, ja) | Zeller's congruence, spelled out in Ajisai |
| `irrelevant-explain-rpn` (en, ja) | `3 4 ADD 5 MUL` vs `3 4 5 MUL ADD`, as worked examples |
| `irrelevant-currency` (ja) | 100 × a self-supplied 140–156 rate band |
| `irrelevant-estimate` (ja) | 300,000 words ÷ a self-supplied 400/500/600 wpm |
| `irrelevant-debug` (ja) | `[ 0 5 ] RANGE`, to show the off-by-one |

In none of them does the model claim the engine holds data it does not. It
supplies the missing constant itself and uses the engine for the arithmetic
underneath — a rate band it names, a reading speed it names, a calendar formula
it knows. `irrelevant-currency` is the clearest: the engine cannot know the yen
rate, the model does not pretend it can, and what it computes is 100 dollars
across a range it chose.

So `irrelevantToolRate` as defined — *any* call on a case whose expected tool is
null — is measuring two different things at one price:

- **claiming absent data**, which produces a confidently wrong answer and is the
  failure worth a metric;
- **using a calculator for a sub-task with self-supplied inputs**, which is what
  a careful assistant does and what 7 of 7 activations actually were.

**The cases are still not being changed, and neither is the metric — yet.**
Splitting the definition after seeing the traces is how a metric gets tuned into
agreeing with the model. What the traces license is the observation; separating
the two readings needs its own criterion, decided before the next capture rather
than after it, and ideally checked against the final answer text rather than the
call alone. The corpus records the calls, so that judgement remains available.

**Japanese activates more (0.25 vs 0.10), and the sample is too small to mean
anything.** Five of the seven are Japanese, but three of those are cases whose
English half also fired. The two one-sided ones (`irrelevant-estimate`,
`irrelevant-debug`) are a two-prompt difference. Against a language gap that is
−0.034 on positive selection — Japanese marginally *ahead* — this is noted and
not interpreted.

The `assets/quickstart.md` resource is deliberately **not** split. The
reevaluation left the 8 KB target open to be judged on size alone; the baseline
supplies the argument. It is fetched on demand rather than injected, so its 26 KB
costs nothing unless read, and a caller that fetches it wants the Word table.
The measured failure was too little information reaching the caller before its
first call, not too much after — splitting the reference into halves it has to
discover separately runs the wrong way.

**Two harness defects were found by the first capture and fixed before the
number above was taken.** Both had to be, because both made the metric measure
the harness:

- The capture recorded only the *first* `tool_use` block of a turn. 91 of 130
  turns made two calls, overwhelmingly a lookup followed by the compute that
  answers the request, so the discarded call was usually the real one. Scored
  that way the same traces read 0.323 selection accuracy against the 0.469 above
  — the difference is entirely the discard. A turn is one attempt; every call in
  it is now recorded, `toolSelectionAccuracy` asks whether the expected tool was
  reached, and `reachedExpectedToolFirstRate` reports the stricter reading beside
  it without making instinct a pass criterion.
- `score-repairs.js` replayed only `compute` and scored every other tool as
  nothing having happened. `1 2 AD` through `check` returns the identical
  `typoOrUnknownName` diagnosis naming the identical Word, so a model that
  statically checked before running was recorded as never having seen a
  diagnosis. It also made the two repair rates identical by construction. They
  now separate: 1.000 observed against 0.750 repaired.

**A repair-harness defect this round found, and one it did not.** Recording
only the first `tool_use` of a repair turn mis-scores a model that got *better*:
after this round it began statically checking before running, and `check` on
`1 ADD` reports nothing because a stack underflow is a runtime fact. The failing
`compute` behind it was the attempt, and the harness threw it away — the same
correction `score-traces.js` needed one round earlier, now applied to both ends
of the repair loop (a turn observed the diagnosis if any of its calls did, and
repaired if any of its calls produced the expected result).

It did not explain the number it was found chasing. The repair rate is 0.750,
down from 1.000, and after the fix it is still 0.750: on
`repair-stack-underflow` the model now spends its whole first turn on `check`
and `word_contract` and never produces a failure at all, so there is no
diagnosis to repair from. That is a corpus limitation rather than a regression —
it is the one case whose failure is invisible to static checking, so a model
that checks first will always miss it — and it is recorded rather than scored
around. The prediction was wrong and the fix was kept because it is correct
independently.

**A diagnosis defect the baseline found, fixed, and the fix measured.** On
`repair-collection-ceiling` the model repaired in the right *direction* — it
shrank the collection, not the arithmetic, which is the claim `collectionWork`
was split out to make true — but it shrank by one element (99,999 → 99,998) and
by twenty (→ 99,979), and both retries failed again.

The fault was in the diagnosis, not the model. A cumulative work meter aborts
the moment the budget is crossed, so `observed` (20,004,122) always sits a hair
over `limit` (20,000,000) however far over the request really was. Read
proportionally — which is the only way to read it — it says "shrink by 0.02%",
where the input had to shrink by 94%. Unlike `bigintBits` or `algebraicTerms`,
where `observed` is a real size and a real multiple of the limit, **for an
incrementally charged ceiling it carries no distance information at all.**

`diagnosis.resourceLimit.progress` now reports where the operation stopped:
`{ completed, total, unit }`. For a scan that is the answer rather than a hint
at it — the budget bought exactly `completed` elements of this data, so an input
of that size is the one that fits. It is emitted only where it exists: a copy or
a sort is charged in full before it runs, so its `observed` already says how far
over the request was and no progress figure is invented for it. A
`checkHowFarItGot` next-check states the instruction in both languages rather
than leaving it to be inferred.

**Re-captured against the same corpus and the same model, the repair rate went
0.750 → 1.000**, and in both languages the model retried with `[ 0 6032 ] RANGE`
— exactly the size the refusal advertised, where before it had shaved off one
element. Both trace documents are committed
(`claude-opus-5-repairs-baseline.json` before, `…-repairs-progress-fix.json`
after), which also makes this the first before/after comparison this tracker has
been able to run.

**What P2 still owes**: the before/after comparisons the `exactDisplay` and
response-compaction rounds are owed (the mechanism now exists and has been used
once, above), corpus growth beyond 65 cases, and remote-service latency. The percentage moves
to 75% for evidence collected, not for the product performing well — a 0.39
semantic success rate is a starting line.

#### Earlier P2 work

A first versioned prompt corpus now covers tool intent and backend semantics for
rationals, decimals, algebraics, vector broadcast, NIL, diagnostics, static
checking and contracts. It was intentionally only a seed; the expansion to
100–200 prompts it called for is done (130, above). A trace scorer now measures
tool selection, end-to-end semantics, missing traces and irrelevant activation;
the committed perfect reference trace verifies the scorer only. Real model
traces have since been captured (above); baseline *comparisons* across a change
remain to be collected. A
separate repair scorer now requires the expected structured diagnosis before a
corrected attempt can count, with cases for unknown Words, stack shape,
malformed source and the collection-work ceiling. Its perfect reference is a
harness fixture; real-model repair rates are reported above.
Corpus and trace contracts now reject duplicate/unknown IDs, unknown tools,
malformed expectations and incomplete reference fixtures before scoring.
The selection corpus reached 22 prompts in that round, adding large-integer
precision, rational reduction, pairwise vectors, domain NIL, exact comparison,
modulus, static-check failure, alias lookup and additional irrelevant intents.
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
