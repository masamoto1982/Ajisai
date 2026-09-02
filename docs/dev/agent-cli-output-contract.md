# Agent CLI output contract (`ajisai --json`)

Status: implemented host contract. This document is not an authority for
language semantics; `SPECIFICATION.html` remains that authority. It documents
only commands and fields emitted by the current native CLI.

## Commands

```text
ajisai run <file.ajisai> [--json] [--step-limit <N>]
ajisai check <file.ajisai> [--json] [--contract]
ajisai contract <file.ajisai> [--json]
ajisai agent <compute|check|infer-contracts> <file.ajisai|->
ajisai test <file-or-dir> [--json]
ajisai repl [--json]
ajisai version [--json]
```

No other command or option is part of the current CLI contract. In particular,
this document does not reserve planned commands.

`agent` accepts `-` in place of a path and reads the program from standard
input, byte for byte, with nothing appended. An embedding host that already
holds the source needs no temporary file, no writable temporary directory, and
leaves no program on disk for the duration of the call.

| exit | meaning |
|---:|---|
| 0 | success |
| 1 | Ajisai language, check, contract, or test failure |
| 2 | CLI usage or host file-reading failure; JSON is not guaranteed |

With `--json`, commands that produce a JSON report write one document to
stdout. Program `PRINT` effects are captured in the document rather than mixed
into stdout. `--step-limit` is a positive integer and applies only to `run`;
the default is the host's derived step budget
(`interpreter::DEFAULT_MAX_EXECUTION_STEPS`, currently 23,190,000 — see
`docs/dev/mcp-host-profiles.md`, re-derived per-container and not a value to
hard-code elsewhere). `--contract` applies only to `check`.

## `run` and `check`

Both commands emit schema version 1:

```json
{
  "schemaVersion": 1,
  "status": "ok",
  "stack": [],
  "stackDisplay": [],
  "output": [],
  "message": null,
  "diagnosis": null,
  "errorFlowTrace": [],
  "aiDiagnostic": null,
  "runtimeMetrics": {},
  "resourceUsage": {},
  "contractDecls": null,
  "stackElided": null,
  "observationDigest": null
}
```

`status` is `ok` or `error`. Ajisai language errors use exit 1 and retain this
JSON envelope. `check` tokenizes, checks delimiter structure, performs
best-effort static Word resolution, and never executes the program. With
`--contract`, `contractDecls` contains the conservative result of comparing
`#:contract` declarations with inferred contracts.

Native `run` obtains this document from the typed, source-only Rust
`agent::api::compute` boundary. Terminal formatting is a consumer of that
report and is not part of computation semantics.
JSON `check` likewise consumes `agent::api::check`; human-readable check
output remains a terminal-only projection.
`agent::api` (`rust/src/agent/`) has no filesystem or terminal I/O and
compiles for `wasm32` as well as native, so the WASM one-shot entry point
(`rust/src/wasm_interpreter_bindings/wasm_agent.rs`, consumed by the MCP
adapter's `worker_threads` backend) renders the identical envelope; the
native CLI (`rust/src/cli`) is a thin file/terminal adapter over the same
module.

### `contractDecls`: gap identifiers

`LANG.CONTRACT.CHECK` fixes exactly three results for `check --contract`:
verified, cannot verify, violated. A `code` on a finding is the stable *reason*
behind a cannot-verify result — the breakdown of that one result, not a fourth
result of its own. `violated` (top-level bool) and each finding's `severity`
are computed exactly as before; gap identifiers add information, they do not
change what counts as a violation.

```json
{
  "violated": false,
  "findings": [
    {
      "severity": "note",
      "message": "`#:contract NORMALIZE`: declared `pure` but inferred `effectful` (unverified).",
      "code": "gap.recursiveDependency"
    }
  ],
  "gapSummary": {
    "declarationsChecked": 3,
    "verified": 1,
    "cannotVerify": 1,
    "violated": 1,
    "byGap": { "gap.recursiveDependency": 1 }
  }
}
```

- `code` is present (a string) only on a `"severity": "note"` finding, and is
  `null` for every `"severity": "error"` finding: a proven violation is not
  something inference merely failed to decide, so it carries no gap.
- The five gap ids, and no others: `gap.unresolvedWord` (a symbol the body
  calls does not resolve to any word), `gap.recursiveDependency` (inference
  re-entered a word that is already being inferred — direct or mutual
  recursion), `gap.dependencyUnknown` (a dependency's own inference could not
  complete), `gap.conservativeSeed` (inference fell back to the maximally
  cautious contract without going through one of the other reasons),
  `gap.unmodelledControlFlow` (the body reaches a control directive whose
  paths differ in stack height — `^`, `|` — or an unbalanced `[`/`{`
  delimiter, so no fixed arity describes it). `gap.opaqueReflection` was
  retired along with `REFLECT` (CodeBlock/Vector unification, docs/dev/
  type-unification-work-order-2026-08.md): every Vector is executable now,
  so there is no separate crossing whose contents inference cannot trust.
- `gapSummary.declarationsChecked` counts successfully-parsed `#:contract`
  declarations; `verified + cannotVerify + violated` always equals it. A
  malformed directive (one that never became a checkable declaration) still
  contributes an `error` finding and still sets `violated: true`, but is not
  one of the three counted here.
- `gapSummary.byGap` keys are sorted ascending and the object is always
  present (empty when nothing is unverifiable), so a caller can read it
  without a presence check.

### `contractDecls`: `outcome` and `declarations`

`verified` / `cannot verify` / `violated` are `LANG.FAILURE.TRICHOTOMY`
(value / reasoned absence / error) applied at check time instead of run time
(`spec/language-semantics.md`, `LANG.CONTRACT.CHECK`) — not by analogy, but
because a gap identifier already has the same character as a NIL reason, and
`ajisai contract` already returns the "value" case (the inferred contract
itself) as its own tool. `contractDecls.outcome` and `.declarations` state
that correspondence directly, in the runtime's own vocabulary:

```json
{
  "violated": false,
  "findings": [ ... ],
  "gapSummary": { ... },
  "outcome": "nil",
  "declarations": [
    { "word": "INC",       "outcome": "value" },
    { "word": "NORMALIZE", "outcome": "nil", "reason": "gap.recursiveDependency" },
    { "word": "BAD",       "outcome": "error", "category": "contractViolation" }
  ]
}
```

- `declarations` has one entry per successfully-parsed `#:contract`
  declaration, in source order, each carrying exactly the fields its outcome
  has: `reason` (a gap id) only for `"outcome": "nil"`; `category` (always
  the literal string `"contractViolation"` — not an `ErrorCategory`, which is
  the *runtime* error registry, and a contract violation is not a runtime
  error) only for `"outcome": "error"`.
- The file-level `outcome` is a fold of `declarations[].outcome`, derived from
  `LANG.FAILURE` rather than chosen: `error` propagates and halts, so one
  `error` anywhere decides the whole file; `nil` flows downstream only once
  nothing halted first, so it decides the file only when no `error` is
  present; a file with no declarations (or none outstanding) is `value`.
- **This does not make `check` evaluate the program.** The correspondence
  classifies outcomes, not mechanisms — division by zero, a failed parse and
  an out-of-range index already share one *outcome* (NIL) while sharing no
  *mechanism*, and an inference that could not decide joins that list on the
  same terms.
- `findings` and `violated` are **not removed or changed** — `LANG.OBSERVATION.PROTOCOL`
  permits only additive changes within a schema version, and `SCHEMA_VERSION`
  does not move for this change. They remain exactly what they were: `findings`
  / `violated` is a legacy projection of the identical result `outcome` /
  `declarations` now also states directly, and is planned for removal in a
  future breaking schema version. The exit code is unaffected either way: it
  is `1` when `violated` is `true` and `0` otherwise, exactly as before —
  `outcome: "nil"` (cannot verify) never fails the check.
- Merging gap identifiers into the NIL reason registry so a gap could be read
  back through `NIL-REASON` is deliberately **not** done here. See
  `docs/dev/trichotomy-unification.md` for why, and the condition under which
  it is worth revisiting.

### `contractDecls`: the `cost` declaration axis

A `#:contract` directive may also declare `cost`, one or more
`steps=CLASS`/`numeric=CLASS`/`collection=CLASS` terms (`CLASS` one of
`const`/`linear`/`superlinear`/`unbounded`), checked against the same three
counters `resourceUsage` reports (`executionSteps`/`numericWork`/
`collectionWork` respectively — see below). Design rationale, the class
lattice, and why this axis deliberately does not refine per call site are in
`docs/dev/cost-contract-design.md`.

```text
#:contract SUM-ALL cost steps=unbounded numeric=linear
```

- Each axis is checked only when declared; an omitted axis is not checked, the
  same rule the other declaration parts already follow.
- Unlike arity/purity/nil-free, a cost mismatch's `severity` is driven by
  *that axis's own* exactness witness, not the word's overall
  `ContractConfidence`: `error` only when the tighter class is proven
  attained, `note` otherwise. A `note` here can carry `code: null` even though
  it is a `note` — a cost bound can be a sound-but-unproven upper bound by the
  classification's own design (e.g. `MAP`'s step count), not because
  inference gave up on the word, so it is not always one of the six gap ids.
- The inferred class depends on **what actually feeds the word**, not on the
  word's name alone. A built-in's class is a function of its operands, so a
  compile-time-literal operand collapses it: `{ RANGE }` is `unbounded` on the
  collection axis (its charge is set by the operand's *value*, not its size),
  while `{ [ 0 10 ] RANGE }` is `const`. Likewise `{ ADD }` is `linear` on the
  numeric axis but `{ 1 2 ADD }` is `const`. Two words calling the same
  built-in can therefore verify different declarations — declaring the class
  the *call site* actually has is what makes the axis worth declaring.

### What the run cost, and what the runtime did

Two objects, because they answer different questions.

`resourceUsage` is **what this run spent of the budgets that could have refused
it**. Every key names a key of the host's declared limit profile and carries the
same number the ceiling compared against — read from the counter the check
reads, not a parallel copy — so an agent can subtract one from the other and
know what it has left.

```json
{ "executionSteps": 22, "numericWork": 20 }
```

Only the accumulating ceilings appear. `bigintBits` and `algebraicTerms` are
checked per result and never accumulated, so there is no peak to report, and
none is invented: a field carrying a number nothing measured is exactly the
defect this object exists to fix.

`runtimeMetrics` is **how the runtime went about it** — which cache answered,
which fast path fired, how often a plan was rebuilt. Optimizer observations,
useful for understanding a slowdown and useless for planning against a limit.

`runtimeMetrics.executionSteps` appears in both and carries the same reading. It
stays there because removing a field is what a schema version is for; it belongs
in `resourceUsage`. That it sat in the optimizer object is how it went unnoticed
that nothing ever wrote it: the value was `0` for every program ever run, beside
the `Interpreter::execution_step_count` that every limit check increments. Two
counters for one fact, and the reported one was the one that was always zero.

### `observationDigest`

A single `#`-prefixed 64-lowercase-hex BLAKE3 digest of the whole observation,
or `null`. Two runs agree on this field exactly when they agree on everything
an agent can observe: `status`, the stack (bottom to top, by value — not by
representation), `PRINT` output in order, the user dictionary (each word's
normalized name and its content identity, sorted by name), and the error
category. It lets a caller compare two implementations, two runs, or a run
against a recorded expectation, without transferring or diffing the values
themselves.

It does **not** include `stackDisplay`, `message`, `diagnosis`,
`aiDiagnostic`, `errorFlowTrace`, `runtimeMetrics`, `resourceUsage`, or
`contractDecls` — none of those are the observation; several of them
(`stackDisplay` in particular, SPEC §4.2.3's continued fraction truncated at a
display budget) are not even faithful to the value they render. A value's
`hint` (display role) is excluded the same way `PartialEq for Value` excludes
it; a NIL's reason is included the same way `PartialEq for Value` includes it.

**Guarantee, stated in one direction only:** equal observations always digest
equally. The converse does not hold without qualification for algebraic
(Tier 1 exact-irrational) scalars: the digest keys such a value by
`floor(value * 2^64)` — the same precision `impl Hash for Algebraic`
(`types/exact/algebraic.rs`) already uses for `HashMap`-based correctness
throughout the interpreter (`UNIQUE` / `GROUP` / `TALLY`), deliberately reused
rather than widened, because the cost of this key is `O(terms)` big-integer
square roots and a legitimately reachable algebraic value can carry hundreds
of terms — so two distinct algebraic numbers closer together than `2^-64`
fold to the same digest. This is a deliberate, bounded residual, not an
oversight — do not read `observationDigest` as proving two stacks differ when
it merely fails to prove they agree at that resolution. Every other domain
(rational, boolean, string, code block, NIL, vector, tensor) digests
injectively.

`observationDigest` is `null` exactly when the observation contains a Tier 2
`ExactReal::Computable` scalar (lazily refined, no canonical finite
representation) anywhere in the stack. No current Word constructs one.

The byte grammar is tagged (`AJISAI-OBS-1`, `rust/src/agent/observation_digest.rs`).
Changing the grammar is not a backward-compatible change even though it adds
no JSON field and does not move `SCHEMA_VERSION`: a value that used to digest
one way will digest another, so any caller comparing against a previously
recorded digest breaks. Bump the schema tag when that happens.

### An error report that cannot afford its stack

An error report carries two different things. The **diagnosis** is the answer:
why the program stopped and what to do about it. The **stack** is residual
state — whatever the program happened to be holding at the time. When the two
together exceed what a host will accept, sending the residue and losing the
answer is the wrong trade.

So on `status: "error"` only, slots whose values do not fit a byte budget are
replaced in place: `value` becomes `null`, `type`, `displayHint` and
`semantics` still say what the value was, and an `elided` record says what was
dropped.

```json
{
  "type": "vector",
  "value": null,
  "displayHint": "unassigned",
  "semantics": {},
  "elided": { "reason": "errorStackBudget", "approxBytes": 27178011, "elements": 100000 }
}
```

The envelope repeats it at the top level as `stackElided`, so one field answers
"was anything dropped":

```json
{
  "reason": "errorStackBudget",
  "budgetBytes": 65536,
  "slots": [ { "index": 0, "approxBytes": 27178011, "elements": 100000 } ]
}
```

Four rules make this safe to rely on.

- **Errors only.** A successful result *is* its stack; truncating it would
  change the answer. An oversized success stays oversized, and a host that
  cannot deliver it says so (`responseTooLarge`) rather than quietly shrinking
  it.
- **Values are dropped, never reasons.** `diagnosis`, `aiDiagnostic`,
  `errorFlowTrace`, `message` and `runtimeMetrics` are never elided.
- **Slots keep their index.** A dropped slot is replaced, never removed, so
  `stack` and `stackDisplay` stay the same length as the real stack and a
  diagnosis that points at stack depth still points at the same thing.
  `stackDisplay` carries a matching `<elided …>` marker, so a text-only reader
  learns the same facts.
- **An ordinary error is untouched.** The budget is 64 KiB — one sixteenth of
  the MCP adapter's 1 MiB `responseBytes` ceiling — and an ordinary diagnosis
  is roughly 15 KiB in total, so `stackElided` is absent and nothing changes
  byte for byte. It appears as `null` here and is omitted entirely from the MCP
  envelope, which drops null top-level fields.

Distinguishing an elided slot from a genuine `NIL`: a `NIL` has
`type: "nil"` and carries `semantics.absence`; an elided slot keeps the real
domain in `type` and carries `elided`.

### Stack value nodes

Stack nodes are produced by the same Rust value-protocol mapping as the WASM
playground boundary:

```json
{
  "type": "number",
  "value": { "numerator": "3", "denominator": "2" },
  "displayHint": "rawNumber",
  "semantics": {}
}
```

Arbitrary-precision integers are decimal strings, never JSON floating-point
numbers. Vectors contain arrays of value nodes. NIL carries normalized absence
metadata in `semantics.absence`. Logical Unknown is observed through the truth
axis rather than serialized as operational NIL.

An algebraic irrational retains the approximate rational compatibility view,
marks it with `approximate: true`, and carries its authoritative multiquadratic
normal form:

```json
{
  "type": "number",
  "value": { "numerator": "768398401", "denominator": "543339720" },
  "displayHint": "rawNumber",
  "semantics": {
    "approximate": true,
    "exactDisplay": "sqrt(2)",
    "exactTerms": [
      { "numerator": "1", "denominator": "1", "radicand": "2" }
    ]
  }
}
```

`exactTerms` encodes `Σ (numerator / denominator) √radicand`. When it is
present, the `value` rational is a display compatibility view and is not the
canonical value.

`exactDisplay` is the same normal form written as one short string —
`sqrt(2)`, `2/1*sqrt(2)`, `1/1 + sqrt(2)`, `sqrt(2) - sqrt(3)` — and is present
in exactly the cases `exactTerms` is. It exists because the two other
renderings of an algebraic value on the same report are each misleading as what
they resemble: `stackDisplay` is the SPEC §4.2.3 continued fraction *truncated
at a display budget* (√2 runs to ~194 characters and ends in `...]`), and
`value` is a rational approximation. It is a display: read it, compute with
`exactTerms`. Because it renders the stored normal form faithfully, two values
`=` decides are equal can still be written differently (`sqrt(8)` and
`2/1*sqrt(2)`); comparison decides equality, string comparison does not.

### Diagnosis and error flow

`diagnosis` is a structured failure explanation with `when`, `why`, `summary`,
`where`, `evidence`, `nextChecks`, `agreedPrefix`, `candidates`, and
`resourceLimit`. `aiDiagnostic` is its machine-oriented classification and
carries `candidates` and `resourceLimit` too. Consumers must treat new
protocol-string variants as opaque values rather than rejecting the report.

Each `nextChecks` entry is `{ code, title: { en, ja }, detail: { en, ja } }`.
`code` is the stable identifier — match on it. `title` and `detail` are display
text, free to be reworded or to gain a locale; a consumer that matched on the
old flat `label`/`detail` pair was matching on a mixed-language string that was
neither stable nor localizable.

`candidates` lists known Words closest to a name that did not resolve, best
match first, and is empty for every other cause class. It considers the
compiled-in vocabulary, the failing interpreter's own dictionary, and — for
`check` — the Words the same source defines.

`resourceLimit` is `{ resource, limit, observed }` and is present when a
declared ceiling fired. `resource` is the ceiling's own name
(`sourceBytes`, `numericLiteralDigits`, `numericWork`, `bigintBits`,
`algebraicTerms`, `executionSteps`) — the same identifier a host publishes in
its limit profile, so "too big" says what was too big and against what. A size
ceiling reports `aiDiagnostic.kind: "resourceLimitExceeded"` and
`recoverability: "reduceWorkOrRaiseLimit"`; the step budget keeps
`executionLimitExceeded` and `addBudgetOrFixRecursion`, because letting the
program run longer fixes one and not the other.

`errorFlowTrace` records Word errors and reason-carrying NIL production. A
successful run may therefore have a non-empty trace. Neither NIL nor an Ajisai
language `status: error` is a host transport failure.

## `contract`

`contract --json` emits a JSON array, not the `run` envelope. Each entry reports
a user Word's inferred `name`, `arity`, `purity`, `determinism`, NIL behavior,
order sensitivity, space class, a `cost` object keyed by its three axes
(`steps`/`numeric`/`collection`, each `"const"`/`"linear"`/`"superlinear"`/
`"unbounded"`), effects, confidence, and a paste-ready `suggested` declaration.
Inference registers definitions without executing their bodies.

`suggested` carries only terms the declaration checker can parse — arity,
purity, NIL behavior and `cost`. The space class is reported but never
suggested: there is no `space:` production in the declaration grammar, so a
suggested line carrying one is rejected as malformed and fails
`check --contract` outright.

Within `cost`, `suggested` codifies an axis only when inference *proves* that
axis's bound is attained (its `exact` witness); an axis whose bound is merely
a plausible upper bound is reported but left out, since an unproven bound
could only ever check as a note. When no axis is exact, the `cost` keyword
itself is omitted rather than emitted with zero terms, since the declaration
grammar rejects a bare `cost`.

The MCP adapter normalizes this legacy bare array into its common result
envelope under `contracts`; the native CLI shape remains unchanged in schema
version 1.
The array itself is produced by `agent::api::infer_contracts` so native and
other embedded hosts (including the WASM one-shot entry point) share inference
rather than reimplementing it.

## `test`

`test` evaluates files against `#@` directives for expected status, stack,
output, and error text. With `--json`, it emits a test-run report and exits 1 if
any case fails. Test directives are host tooling comments and do not change
Ajisai program semantics.

## `repl`

The REPL preserves stack and definitions across lines. In JSON mode, every
submitted program line produces one `run`-shaped JSON document. REPL
meta-commands (`:help`, `:stack`, `:reset`, and `:quit`) are host commands, not
Ajisai Words.

## `version`

`version --json` emits:

```json
{ "schemaVersion": 1, "status": "ok", "version": "0.2.0-beta.1" }
```

## `agent`

`agent` is the stable JSON-only host boundary used by the MCP adapter. Its
`compute`, `check`, and `infer-contracts` operations call the typed Rust agent
API and always return a schema-versioned object. In particular,
`infer-contracts` returns the array under `contracts`, avoiding the legacy bare
array emitted by the compatibility `contract --json` command.

## Compatibility

Additive fields do not change `schemaVersion`; consumers must ignore fields
they do not recognize. Removing or renaming fields, changing their types, or
changing exit-code meaning requires a schema-version increment.

The MCP-specific envelope and its exact-value definitions are additionally
published as `tools/mcp-server/result.schema.json`. That schema adds MCP
provenance and applied limits to the native report without changing Ajisai
semantics.
