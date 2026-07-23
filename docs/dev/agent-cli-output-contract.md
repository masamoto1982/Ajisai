# Agent CLI Output Contract (`ajisai --json`)

Status: contract document (non-canonical for language semantics).
Authority for Ajisai semantics: `SPECIFICATION.html` only.
This file is the authority for the *shape* of the `ajisai` CLI's `--json`
output, which AI agents and verification scripts consume.

## 1. Commands and exit codes

```
ajisai run <file.ajisai> [--json] [--explain] [--lang <ja|en>] [--step-limit <N>]
ajisai check <file.ajisai> [--json] [--explain] [--contract] [--lang <ja|en>]  # tokenize + parse + resolve (+ optional contract check); never executes
ajisai contract <file.ajisai> [--json]  # report each user word's inferred contract (§11.2); never executes
ajisai coverage <file.ajisai> [--json]  # contract coverage ratio (§14); never executes
ajisai modifier <phrase...> [--json] [--lang <ja|en>]  # infer the modifier for an intent phrase; never executes
ajisai fmt <file.ajisai> [--write] [--check]  # rewrite source into canonical form; never executes (§17)
ajisai repl [--json]  # interactive session; stack and definitions persist (§16)
ajisai test <file-or-dir> [--json]  # run test files, check `#@` directives (§18)
ajisai build <dir>  # run a project (ajisai.toml); confine to allowed capabilities; verify ajisai.lock (§19)
ajisai lock <dir> [--check]  # write/verify ajisai.lock: realized identities + required capabilities (§19)
ajisai new <dir>  # scaffold a new project: ajisai.toml + a runnable src/main.ajisai (§19)
ajisai version [--json]
```

`--explain` adds a deterministic plain-language projection of the diagnosis
(`explanation`, §10). `--contract` adds a light, execution-free flow-mass and
NIL-flow check to `check` (`planCheck`, §11). `--receipt` adds an execution
receipt to a successful `run` (`receipt`, §15). `--lang` selects the language
for all plain-language output (default `ja`). All are additive and never change
the structured fields; with none of `--explain`, `--contract`, or `--receipt`,
output is unchanged. `--contract` raises exit 1 when it finds a malformed (over-consuming)
plan; advisories and notes do not change the exit code.

`--step-limit <N>` overrides the execution step budget for `run` (the water
level of SPECIFICATION.html §5.3; default 100,000). `N` must be a positive
integer; `0` or a non-number is a CLI usage error (exit 2). The budget is a
runtime safety control, not a language semantic: it changes only *whether*
`ExecutionLimitExceeded` is raised, never the shape of any output field.
Exceeding the budget is an ordinary language error (exit 1, `status:
"error"`).

| Exit code | Meaning |
|---|---|
| 0 | Success. `status` is `"ok"`. |
| 1 | Language error. `status` is `"error"` and `diagnosis` is non-null. |
| 2 | CLI usage error (bad arguments, unreadable file). No JSON is emitted; the message goes to stderr. |

Pipe-safety guarantee: with `--json`, stdout carries **exactly one JSON
document and nothing else**. Program output (`PRINT` etc.) is collected into
the `output` array, never written to stdout. Usage errors and the human
(text-mode) error rendering go to stderr.

`check` performs tokenization, a structural bracket scan, and a *static,
best-effort* word resolution (builtins, canonical aliases, words the file
defines with `DEF`, words imported from modules the file `IMPORT`s).
Constructs that depend on runtime state — user dictionaries referenced as
`DICT@WORD`, dynamically built definitions — are accepted without
verification. `check` failing means the program cannot run; `check` passing
does not prove it will.

## 2. Top-level envelope (`run` and `check`)

```json
{
  "schemaVersion": 1,
  "status": "ok | error",
  "stack": [ ... ],
  "stackDisplay": [ "..." ],
  "output": [ "..." ],
  "message": null,
  "diagnosis": { ... } | null,
  "errorFlowTrace": [ ... ],
  "aiDiagnostic": { ... } | null,
  "runtimeMetrics": { "vtu": { ... } },
  "explanation": { ... } | null,
  "planCheck": { ... } | null,
  "receipt": { ... } | null
}
```

| Field | Type | Meaning |
|---|---|---|
| `schemaVersion` | number | Version of this envelope. Currently `1`. |
| `status` | string | `"ok"` (exit 0) or `"error"` (exit 1). |
| `stack` | array | Final data stack, bottom to top, as value protocol nodes (§3). Empty for `check`. |
| `stackDisplay` | array of string | The same stack as Stack-projection display strings (the text the GUI's Stack area renders), bottom to top. Strings keep their `'...'` quotes here. Convenience view of `stack`; the structured nodes stay authoritative. |
| `output` | array of string | Ordered `PRINT` payloads produced during the run, as rendered at the output boundary (a `Text`-role value is emitted without its display quotes, so `'TEST'` appears as `TEST`). Empty for `check`. |
| `message` | string \| null | The raw error display string, when `status` is `"error"`. |
| `diagnosis` | object \| null | Structured diagnosis of the failure (§4). Null when `status` is `"ok"`. |
| `errorFlowTrace` | array | Ordered observation log of word errors **and NIL productions** (§6). May be non-empty even when `status` is `"ok"`: a division by zero, for example, bubbles to NIL (SPEC Bubble Rule) and the run succeeds, but the projection is traced here with a full diagnosis. |
| `aiDiagnostic` | object \| null | Machine-oriented classification of the failure (§5). Null when `status` is `"ok"`. |
| `runtimeMetrics` | object | VTU observation counters (§7). All zeros for `check`. |
| `explanation` | object \| null | Plain-language projection of the diagnosis (§10). Present only with `--explain`; `null` otherwise. |
| `planCheck` | object \| null | Light contract / flow-mass check (§11). Present only with `check --contract`; `null` otherwise. |
| `contractDecls` | object \| null | Opt-in `#:contract` word declarations checked against the inferred contract (§11.1). Present only with `check --contract`; `null` otherwise. |
| `receipt` | object \| null | Execution receipt (§15). Present only with `run --receipt` on a successful run; `null` otherwise. |

`version --json` emits only `{ "schemaVersion", "status", "version" }`.
`modifier --json` emits `{ "schemaVersion", "status", "modifier": { ... } }` (§12).
`coverage --json` emits `{ "schemaVersion", "status", "coverage": { ... } }` (§14).

### Compatibility policy

- **Additive changes** (new fields anywhere in the envelope) do **not** bump
  `schemaVersion`. Consumers must ignore unknown fields.
- **Breaking changes** (removing or renaming a field, changing a field's
  type, changing exit-code semantics) bump `schemaVersion`.
- Protocol *string values* (`why`, `when`, `kind`, `recoverability`,
  `displayHint`, absence reasons, ...) come from the language's existing
  protocol vocabulary (`as_protocol_str` in the Rust sources, the same
  strings the WASM/GUI boundary uses). New variants may appear over time;
  consumers must treat unrecognized values as opaque, not as errors.

## 3. Stack value nodes

The same wire shape the GUI receives from the WASM boundary, produced by the
shared `types::value_protocol` mapping:

```json
{
  "type": "number | string | boolean | datetime | vector | nil | truthValue | process_handle | supervisor_handle",
  "value": ...,
  "displayHint": "unassigned | rawNumber | interval | text | truthValue | timestamp | nil | continuedFraction",
  "semantics": { ... }
}
```

- `number` / `datetime` values are exact rationals:
  `{ "numerator": "...", "denominator": "..." }` (decimal strings, arbitrary
  precision — never floats).
- `vector` values are arrays of nodes (tensors are hydrated to nested
  vectors; interior nodes of rank ≥ 2 carry no `semantics`).
- The logical Unknown (U) of the three-valued logic serializes as
  `{ "type": "truthValue", "value": "unknown" }` — never as `nil`.
- `semantics` (when present): `semanticKind`, `shape`, `capabilities`,
  `origin`, optional `truthValue` (`"true" | "false" | "unknown"`), optional
  `absence` (§6a), and optional `approximate: true` for exact-irrational
  values rendered as a best rational approximation.

## 4. `diagnosis`

Serialization of the interpreter's `DebugDiagnosis` — identical naming to the
WASM `diagnosis_to_js` boundary:

```json
{
  "when": "tokenize | parseStructure | resolveWord | executeWord | nilPropagation | assertion | hostIo | optimizationValidation | unknown",
  "why": "typoOrUnknownName | stackShape | valueShape | domain | index | vectorLength | nilFlow | environment | effect | userLogic | contractViolation | optimizerMismatch | internalInvariant | unknown",
  "summary": "one-line human summary",
  "where": { "kind": "userWord | coreWord | builtinWord | moduleWord | hostEnvironment | optimizer | unknown",
             "word": "...?", "module": "...?", "dictionary": "...?" },
  "evidence": [ "category=...", "stackLenBefore=...", ... ],
  "nextChecks": [ { "label": "...", "detail": "..." } ],
  "agreedPrefix": null
}
```

- `nextChecks` is the agent's repair checklist: ordered, machine-stable
  labels with human guidance. It is always present (possibly short, never
  fabricated).
- `agreedPrefix` is non-null only for continued-fraction comparisons that
  returned Unknown within budget (SPEC §4.5.0 / §7.4.1): the number of
  leading partial quotients that matched.
- Missing host capability (e.g. `MUSIC@PLAY` under the terminal CLI, which
  has no audio device) is reported as `when: "hostIo"`, `why: "environment"`,
  `where.kind: "hostEnvironment"`, with the capability named in `evidence`
  (`missingCapability=audio`). This marks an environment limitation, not a
  program bug.

## 5. `aiDiagnostic`

Serialization of the interpreter's `AiDiagnosticPayload`: stable protocol
fields so agents never have to parse display strings.

```json
{
  "kind": "unknownWord | stackUnderflow | divisionByZero | ... | null",
  "recoverability": "fixInput | fixProgram | fixHost | fixCapabilityOrForce | addBudgetOrFixRecursion | handleUnknownOrNil | inspectContext",
  "semanticArea": "exact-real-arithmetic | exact-real-comparison | k3-truth | hosted-effect | unknown-or-absence | stack-value-shape | unknown",
  "word": "... | null",
  "semanticRole": "Primitive | Derived | HostedEffect | Extension | Unknown",
  "algebraicFamily": "exact-arithmetic | observation | k3-truth | hosted-effect | ...",
  "absenceReason": "divisionByZero | emptySequence | ... | null",
  "truthValue": "true | false | unknown | null",
  "effect": "... | null",
  "nextChecks": [ { "label": "...", "detail": "..." } ]
}
```

## 6. `errorFlowTrace`

Ordered events, same shape as the WASM `collect_error_flow_trace`:

```json
{
  "kind": "wordError | nilProduced",
  "word": "...?",
  "absence": { "reason": "...?", "origin": "...", "recoverability": "...", "diagnosis": { ... }? },
  "stackLenBefore": 2,
  "stackLenAfter": 1,
  "message": "...",
  "diagnosis": { ... }?
}
```

(§6a) `absence` blocks — here and inside stack-value `semantics` — carry the
machine-readable reason a value is NIL: `reason` (e.g. `divisionByZero`,
`emptySequence`, `noData`), `origin`, and `recoverability`.

## 7. `runtimeMetrics`

```json
{ "vtu": { "tensorFlattenCount": 0, "tensorFlattenedElements": 0,
           "tensorRebuildCount": 0, "tensorRebuiltElements": 0,
           "broadcastCount": 0, "unaryFlatCount": 0, "allocatedElements": 0,
           "sameShapeElementwiseCount": 0, "projectedBroadcastCount": 0,
           "simdKernelUseCount": 0, "sparseCandidateCount": 0,
           "sparseCandidateElements": 0, "sparseCandidateNonzeroElements": 0,
           "sparseSkippableZeroElements": 0, "candidateBlockCount": 0,
           "rejectedBlockCount": 0, "fusionCandidateCount": 0,
           "bulkKernelUseCount": 0,
           "energyProxyScore": 0, "proxyVersion": 1, "suggestions": [ ] },
  "scalarFastpathCount": 0,
  "comparison": { "compareWithinCount": 0, "compareWithinLazyCount": 0,
                  "compareWithinUnknownCount": 0,
                  "compareWithinBudgetTermsConsumed": 0 } }
```

The 18 leading `vtu` fields are the Virtual Tensor Unit observation counters
(`docs/dev/virtual-tensor-unit-design.md`). They describe observed
structural work (data movement, allocation, kernel selection) and are
deterministic for a given program and input.

`scalarFastpathCount` and the `comparison` group are the Cost Model
observability surface (SPECIFICATION.html §4.8). `scalarFastpathCount` counts
small-rational scalar–scalar operations that took the fast lane. The
`comparison` group answers "when is the comparison budget consumed": the bare
relations decide the admitted domain exactly and spend nothing, so only
`COMPARE-WITHIN` moves these counters — `compareWithinLazyCount` is the
streamed subset that can spend budget, and `compareWithinBudgetTermsConsumed`
sums the NICF terms consumed on the Unknown results. Like the VTU counters,
these are observational proxies and never part of value identity (§4.2.2).

- `energyProxyScore` — a single deterministic integer aggregating those
  counters with fixed weights (`docs/quality/energy-proxy-score.md`).
- `proxyVersion` — the scoring-formula version. Scores are comparable only
  within one `proxyVersion`; it increments whenever a weight or the formula
  changes.
- `suggestions` — array of strings: mechanical, counter-derived observations
  about structural patterns that usually admit a cheaper equivalent program
  (e.g. fusable stages, repeated flat/nested round-trips). May be empty.

These are **proxies**: they describe structural work and are *not* a joule
measurement. Counter names and the score never assert an energy outcome
(per the standing policy in `docs/dev/virtual-tensor-unit-design.md`). The
score exists so that "same output, more structural work" is a CI-visible
regression (`energy_proxy_regression_tests.rs`).

## 8. Examples (actual output)

### 8.1 Successful run

`[ 1 2 ] [ 3 4 ] + PRINT` →  exit 0:

```json
{
  "schemaVersion": 1,
  "status": "ok",
  "stack": [],
  "output": [ "[ 4/1 6/1 ]" ],
  "message": null,
  "diagnosis": null,
  "errorFlowTrace": [],
  "aiDiagnostic": null,
  "runtimeMetrics": { "vtu": { "tensorFlattenCount": 2, "tensorFlattenedElements": 4,
    "tensorRebuildCount": 0, "tensorRebuiltElements": 0, "broadcastCount": 1,
    "unaryFlatCount": 0, "allocatedElements": 2, "sameShapeElementwiseCount": 1,
    "projectedBroadcastCount": 0, "simdKernelUseCount": 0, "sparseCandidateCount": 0,
    "sparseCandidateElements": 0, "sparseCandidateNonzeroElements": 0,
    "sparseSkippableZeroElements": 0, "candidateBlockCount": 0,
    "rejectedBlockCount": 0, "fusionCandidateCount": 0, "bulkKernelUseCount": 0 } }
}
```

### 8.2 Unknown word (exit 1)

`[ 2 3 ] FROBNICATE` → exit 1, abbreviated:

```json
{
  "schemaVersion": 1,
  "status": "error",
  "message": "Unknown word: FROBNICATE",
  "diagnosis": {
    "when": "resolveWord",
    "why": "typoOrUnknownName",
    "summary": "ResolveWord / FROBNICATE / TypoOrUnknownName (unknownWord) msg=\"Unknown word: FROBNICATE\"",
    "where": { "kind": "unknown", "word": "FROBNICATE" },
    "evidence": [ "category=unknownWord", "stackLenBefore=1", "stackLenAfter=1" ],
    "nextChecks": [
      { "label": "Check spelling", "detail": "word 名のスペルを確認する" },
      { "label": "Check alias canonicalization", "detail": "alias 展開後の canonical word 名を確認する" },
      { "label": "Check imports/definitions", "detail": "module import 漏れ、または user word 定義漏れを確認する" }
    ],
    "agreedPrefix": null
  },
  "aiDiagnostic": {
    "kind": "unknownWord", "recoverability": "fixProgram",
    "semanticArea": "unknown", "word": "FROBNICATE", "semanticRole": "Unknown",
    "algebraicFamily": "unknown", "absenceReason": null, "truthValue": null,
    "effect": null, "nextChecks": [ "... same as diagnosis.nextChecks ..." ]
  }
}
```

### 8.3 NIL bubble on a successful run

`[ 1 ] [ 0 ] DIV` → exit 0, `status: "ok"`, stack `[ NIL ]`, and:

```json
"errorFlowTrace": [ {
  "kind": "nilProduced",
  "word": "DIV",
  "absence": { "reason": "divisionByZero", "origin": "executionFailure", "recoverability": "recoverable" },
  "stackLenBefore": 2,
  "stackLenAfter": 1,
  "message": "NIL produced by DIV reason=divisionByZero",
  "diagnosis": { "why": "domain", "nextChecks": [ { "label": "Check divisor", "detail": "..." }, "..." ] }
} ]
```

## 9. Reading order for agents

1. Exit code. `0` → read `stack` / `output`; also scan `errorFlowTrace` for
   `nilProduced` events if a NIL was unexpected.
2. On `1`: read `diagnosis.why` + `diagnosis.where` to locate, then walk
   `nextChecks` in order. `aiDiagnostic.recoverability` says *what kind of
   change* fixes it (input vs program vs host).
3. `message` is for humans; never parse it when a structured field exists.

## 10. `explanation` (`--explain`)

A deterministic plain-language **projection** of the diagnosis — the L0
surface of the natural-language design note
(`docs/dev/natural-language-surface-design.md` §3). It is computed only when
`--explain` is passed; the field is `null` otherwise, so the default output is
byte-stable.

```json
{
  "lang": "ja | en",
  "headline": "what happened, one plain-language sentence",
  "nextStep": "what kind of change resolves it, one sentence",
  "details": [ "label: detail", "..." ]
}
```

- It is a **projection, not generation**: every sentence is keyed on an
  existing enum (`diagnosis.why`) or protocol string
  (`aiDiagnostic.recoverability`, the NIL `absence.reason`). It introduces no
  new diagnostic concept and cannot say anything the structured fields do not
  already encode — there is no model in the loop.
- `headline` distinguishes the three water-model outcomes the language keeps
  separate, as different *tones*: Stagnation (logical `UNKNOWN`, selected when
  `agreedPrefix` is non-null), Bubble (`NIL`, an absence with a reason), and a
  Channel error (malformed use). The mechanism terms themselves are never
  surfaced.
- `nextStep` is the `recoverability` value rendered as an action sentence.
- `details` is `diagnosis.nextChecks` flattened to `label: detail`, verbatim
  (authored in the core, currently Japanese, regardless of `lang`).
- `run` also projects a NIL that bubbled on an **otherwise successful run**
  (exit 0): the last `nilProduced` event becomes an `explanation` with the
  Bubble tone and `handleUnknownOrNil` next step.
- `lang` is a table swap over the enum-keyed sentences; adding a language does
  not touch the projection structure.

## 11. `planCheck` (`check --contract`)

A light, **execution-free** contract / flow-mass check — the "approach 2, light
version" of the natural-language design note
(`docs/dev/natural-language-surface-design.md` §4). It reuses the existing
static mass-conservation validator (SPEC §13.1) and the §7.14 `nil_policy`
contract; it does not search for or rewrite a plan. Present only with
`check --contract`; `null` otherwise.

```json
{
  "overConsumes": false,
  "minDepth": 0,
  "netMass": 1,
  "massKnown": true,
  "mayBubble": [ "DIV", "..." ],
  "hasFallback": false,
  "rejectsNil": [ "..." ],
  "findings": [ { "severity": "error | advisory | note", "message": "..." } ]
}
```

- `overConsumes` / `minDepth` / `netMass` / `massKnown` come from the §13.1
  validator over the statically known prefix. `overConsumes` (`minDepth < 0`)
  means the flow reads more operands than it provides — a malformed plan, and
  the only finding that raises exit 1. `massKnown` is `false` once a
  `Dynamic`-arity word (a user word, `STAK` fold, runtime-shaped vector op)
  froze the static analysis; the numbers then describe only the prefix.
- `mayBubble` lists words whose `nil_policy = CreatesNil` (they can project a
  domain miss to NIL, e.g. `DIV` `GET` `NUM`). A `Projecting` comparison
  (`LT`/`SORT`/…) projects to logical U, not NIL, and is deliberately not
  listed.
- `hasFallback` is `true` when a `^` (VENT) NIL fallback
  appears. An unguarded NIL source (`mayBubble` non-empty, `hasFallback` false)
  is an advisory, the `handleUnknownOrNil` prompt rendered ahead of execution.
- `findings` are the plain-language `planCheck` surface (L0), most severe
  first; `severity` is `error` (malformed plan), `advisory`, or `note`. Empty
  means the plan is clean over the known prefix.

## 11.1. `contractDecls` (`check --contract`)

Opt-in per-word contract declarations checked against the **inferred** contract
(`rust/src/interpreter/word_contract.rs`), the "connect an opt-in declaration to
a pre-execution check" step of `docs/dev/external-evaluation-response-strategy.md`
(P2). A declaration is a `#:contract` directive comment; like the `#@` test
directives it is **tooling only** and adds no language semantics (canonical
source: `SPECIFICATION.html`). The check registers the file's word definitions
and imports **without executing any word body** or top-level code, infers each
declared word's contract, and reports a declaration the inference contradicts.
Present only with `check --contract`; `null` otherwise.

Directive grammar (each part optional):

```text
#:contract NAME [ ( CONSUMES -- PRODUCES ) ] [pure|observable|effectful] [nil-free|may-nil]
```

```json
{
  "violated": false,
  "findings": [ { "severity": "error | note", "message": "..." } ]
}
```

- `violated` is `true` when any finding is an `error`; it contributes to the
  `check` exit code (1) exactly like a malformed `planCheck`.
- A declaration the inference **contradicts** is an `error`: a wrong arity, a
  purity the word exceeds (declared `pure`, inferred `effectful`), a
  `nil-free` word that can create NIL, or a name that is not defined.
- Because inference is deliberately **conservative** (SPEC §7.14), a declaration
  it cannot *disprove* — e.g. on a recursive word whose contract is inferred
  dynamically — is a `note`, never a false `error`. `may-nil` documents intent
  and never fails.
- Only arity, purity, and NIL-freedom are checked today; richer element types
  (`Scalar`/`Vector<n>`) are future work and are not yet part of the surface.

## 11.2. `contract` (the `contract` command)

`ajisai contract <file>` reports each user word's **inferred** contract — the
reporting companion to the `#:contract` checker (§11.1). It registers the file's
definitions and imports **without executing any word body or top-level code**,
then infers and renders each word's contract in source-definition order. It is
observational: a well-formed file always exits 0.

With `--json` the top-level document is an array (not the standard envelope),
one object per user word:

```json
[
  {
    "name": "INC",
    "arity": "( 1 -- 1 )",
    "purity": "pure",
    "determinism": "deterministic",
    "nil": "nil-propagating",
    "order": "order-independent",
    "effects": [],
    "confidence": "complete",
    "suggested": "#:contract INC ( 1 -- 1 ) pure nil-free"
  }
]
```

- `arity` is `"( c -- p )"` for a fixed flow or `"dynamic"`.
- `purity` ∈ `pure` / `observable` / `effectful`; `determinism` ∈
  `deterministic` / `non-deterministic`; `order` ∈ `order-independent` /
  `order-sensitive`.
- `nil` is the inferred NIL behavior (`nil-free`, `nil-propagating`,
  `may-create-nil`, `rejects-nil`, `consumes-nil`); `effects` lists inferred
  effect tags.
- `confidence` is `complete` or `conservative` (a recursive or otherwise
  unprovable word is `conservative`, and its `arity` is usually `dynamic`).
- `suggested` is a paste-ready `#:contract` line codifying the checkable subset
  (arity + purity + nil-free/may-nil), so `contract` → paste → `check --contract`
  round-trips.

## 12. `modifier` (the `modifier` command)

`ajisai modifier <phrase...>` infers the modifier — `TOP`/`STAK` × `EAT`/`KEEP`
plus the `^` (VENT) fallback — for an operation-intent phrase (approach 3,
design note §5). It executes nothing and always exits 0.

```json
{
  "schemaVersion": 1,
  "status": "ok",
  "modifier": {
    "target": "TOP | STAK",
    "consume": "EAT | KEEP",
    "fallback": false,
    "targetExplicit": false,
    "consumeExplicit": false,
    "ambiguous": false,
    "sugar": ".. ,, ^",
    "rationale": "plain-language explanation of the inference"
  }
}
```

- It is a **classification over a finite lattice**, not generation. Cue matching
  is case-insensitive substring containment over a controlled vocabulary
  (Japanese and English).
- An axis with no cue takes its default (`TOP` / `EAT`); `targetExplicit` /
  `consumeExplicit` say whether a cue was actually found.
- `ambiguous` is `true` when one axis received conflicting cues (e.g. both
  "keep" and "consume"). The design note routes this to approach 4 as a
  plain-language clarifying question rather than a guess.
- `sugar` is the Ajisai modifier sugar for the non-default choices (empty when
  both axes are at their default).
- `modifier.clarifications` and `planCheck.clarifications` carry the approach-4
  questions (§13).

## 13. `clarifications` (approach 4)

Plain-language clarifying questions for an *undecided* signal — the dialogue
layer of the natural-language design note
(`docs/dev/natural-language-surface-design.md` §6). Rather than guess, the
surface asks; each choice carries the Ajisai sugar it resolves to, so an answer
maps straight back to code. The array appears inside `modifier.clarifications`
(approach 3 ambiguity) and `planCheck.clarifications` (approach 2 unguarded NIL);
it is empty when nothing is undecided.

```json
{
  "kind": "targetAxis | consumeAxis | unguardedNil",
  "question": "a plain-language question",
  "choices": [ { "label": "...", "apply": "<Ajisai sugar> | null" } ]
}
```

- A question is raised only for an axis that is genuinely undecided
  (conflicting cues), never for a merely-defaulted one; the unguarded-NIL
  question is suppressed when a `^` (VENT) fallback is already present
  (minimization, design note §6).
- `apply` is the modifier sugar a choice resolves to (`.` `..` `,` `,,` `^`),
  or `null` for a "leave it as is" choice (no code change).
- **Deferred**: the comparison-UNKNOWN clarification (`agreedPrefix`, SPEC
  §7.4.1). The runtime U value carries `agreedPrefix`, but it is not yet
  surfaced to the CLI report, so there is no signal to drive that question here
  without a separate value-protocol change.

## 14. `coverage` (the `coverage` command)

`ajisai coverage <file.ajisai>` mechanically aggregates the **contract
coverage ratio** defined in
`docs/dev/capability-transition-measurement-design.md` §4: the fraction of
word occurrences that resolve to a definition carrying complete SPEC §7.14
contract metadata. It tokenizes and structure-checks like `check` (same exit
1 / exit 2 failure envelopes) but never executes. Coverage itself is
observational: uncovered and unknown words are reported in the ratio, never
as a failure, so a well-formed file always exits 0.

```json
{
  "schemaVersion": 1,
  "status": "ok",
  "coverage": {
    "transitionMetricsVersion": 1,
    "covered": 5,
    "total": 7,
    "ratioDisplay": "5/7",
    "excludedModifierCount": 0,
    "breakdown": { "core": 4, "module": 1, "userDefined": 1,
                   "userDictionary": 1, "unregistered": 0, "unknown": 0 },
    "uncovered": [ { "word": "DOUBLE", "kind": "userDefined", "count": 1 } ]
  }
}
```

- **Denominator** (`total`): `Symbol` token occurrences only. Number/string/
  vector literals, code-block brackets, `^` (NilCoalesce), pipeline and
  clause separators, and comments never enter. Modifier words (`TOP` `STAK`
  `EAT` `KEEP`) are excluded and counted in `excludedModifierCount`.
  Constant words (`TRUE` `FALSE` `NIL`) are registered §7.14 words and are
  counted. The ratio is reported as exact integers plus `ratioDisplay`
  (`"covered/total"`) — never a float. `"0/0"` for a file with no countable
  occurrences.
- **`breakdown` kinds**: `core` and `module` are covered; `userDefined`
  (words the file `DEF`s — no user-word contract mechanism exists yet),
  `userDictionary` (`DICT@WORD` runtime references), `unregistered` (in the
  core vocabulary but missing from the §7.14 registry — a registry gap), and
  `unknown` are uncovered. Resolution mirrors `check`'s static best-effort
  resolution, including short names imported via `'MODULE' IMPORT`.
- **`uncovered`**: canonical word names (aliases are canonicalized first)
  with per-word occurrence counts, in first-appearance order.
- `transitionMetricsVersion` versions the *counting rules* (design memo §6);
  ratios are comparable only within one version. It is independent of the
  envelope `schemaVersion`.

## 15. Execution receipt (`run --receipt`)

`run --json --receipt` attaches a `receipt` object to a successful run. It
records what the result was based on — the source, the content-identified words
that executed, the host capabilities required and granted, the observable host
effects in order, the water spent, and whether the compiled path agreed with
the reference path — plus a stable identity of the result. It is a provenance
record, **not** a proof of correctness or tamper-evidence. On an error run (or
without `--receipt`) the field is `null`.

Producing a receipt is observational: enabling it never changes the run's
result. Only stable, public facts appear — internal optimization details (SIMD
lane widths, shape-IC state, quantized-block internals, tier representations,
pointer identity, Rust `Debug` names, unstable cache keys) are never included.

```json
"receipt": {
  "schemaVersion": 1,
  "sourceIdentity": "#<hex>",
  "implementation": { "name": "ajisai-core", "version": "0.1.0" },
  "specification": { "declaredVersion": null },
  "executedWords": [
    { "resolvedName": "EXAMPLE@DBL", "contentIdentity": "#<hex>",
      "firstSeenOrder": 1, "callCount": 3 }
  ],
  "requiredCapabilities": [ "effect" ],
  "grantedCapabilities": [ "clock", "secureRandom", "jsonExport", "config", "effect" ],
  "observedEffects": [ { "order": 0, "kind": "print", "payload": "[ 10/1 ]" } ],
  "water": { "stepLimit": 100000, "stepsUsed": 3, "comparisonRefinements": 0 },
  "integrity": { "shadowValidationPerformed": false, "referenceAgreement": true,
                 "plainFallbacks": 0, "integrityMismatches": 0 },
  "absenceEvents": [
    { "kind": "nilProduced", "word": "DIV", "reason": "divisionByZero",
      "origin": "executionFailure", "recoverability": "recoverable" }
  ],
  "resultIdentity": "#<hex>"
}
```

- **`sourceIdentity` / `resultIdentity`**: `#`-prefixed content digests (the
  same hash family as §8.6 word identity). `resultIdentity` is computed from
  the canonical bytes of the final stack's value protocol (§3) — value kind,
  exact numerator/denominator, interpretation, absence reason/origin/
  recoverability, logical-Unknown diagnosis, and Vector/Tensor/Record
  structure — **never** from a display string. Equal inputs yield equal
  identities; a different result yields a different identity.
- **`executedWords`**: content-identified (user) words that ran, aggregated by
  word and ordered by first execution (`firstSeenOrder`, `callCount`). Core and
  module words carry no §8.6 content identity and are omitted; they belong to
  the implementation (`implementation`), not user provenance.
- **`requiredCapabilities` / `grantedCapabilities`**: capabilities Hosted words
  required during the run, and those the active host grants — protocol strings.
- **`observedEffects`**: every emitted host effect in order, with its stable
  `kind` tag and `payload`.
- **`water`**: the step budget (`stepLimit`), steps consumed (`stepsUsed`), and
  Tier-2 comparison refinements spent (`comparisonRefinements`).
- **`integrity`**: whether shadow validation ran, whether the compiled and
  reference paths agreed, and the fallback / mismatch counts. When
  `shadowValidationPerformed` is `false`, `referenceAgreement` means only that
  no disagreement was observed — read the two together.
- **`absenceEvents`**: NIL productions observed during the run, in order, each
  keeping its reason, origin, and recoverability — never collapsed to a generic
  failure.
- `receipt.schemaVersion` versions the receipt object shape independently of the
  envelope `schemaVersion`; additive fields keep it unchanged.

## 16. Interactive REPL (`repl`)

`ajisai repl` runs an interactive session over **one persistent interpreter**:
user dictionaries, imports, and the stack carry across lines within the session
(it is the production Core, not the Python reference). It reads one line at a
time from stdin. The banner, prompts, help, and `:reset` notices go to
**stderr**; stdout carries only per-line results, so a piped session stays
pipe-safe (the same guarantee as `run --json`).

Lines beginning with `:` are REPL **meta-commands**, handled by the host and
kept strictly separate from Ajisai surface syntax (they are not language words):

| Command | Effect |
|---|---|
| `:help` / `:h` / `:?` | Print the command list (to stderr). |
| `:reset` | Clear the stack, dictionaries, and imports. |
| `:quit` / `:q` / `:exit` | Leave the REPL (EOF / Ctrl-D also leaves). |

Any other line is evaluated as Ajisai. With `--json`, each evaluated line emits
**one JSON document** to stdout:

```json
{ "status": "ok | error",
  "stackDisplay": [ "..." ],
  "output": [ "..." ],
  "message": null }
```

- `stackDisplay` — the full stack after the line, bottom to top, as the same
  display strings `run --json` uses in `stackDisplay`.
- `output` — the `PRINT` payloads produced by **this line only**, in order
  (not cumulative across lines).
- `message` — the error display string when `status` is `"error"`; `null`
  otherwise. An error leaves the session usable and evaluation continues.

Without `--json`, each line prints its `output` payloads, then (on error)
`error: <message>`, then the stack — one value line, or `(empty stack)`.
Meta-commands produce no stdout. The REPL is a host driver over a pure
`(session, line) -> response` core; it never adds or changes any language word.

## 17. Source formatter (`fmt`)

`ajisai fmt <file>` rewrites Ajisai source into its **canonical written form**
without ever changing what the code means. It tidies only insignificant
whitespace — the spacing between tokens and the indentation at the start of
each line — and never adds or removes line breaks (a line break inside a `{ }`
block is a statement separator, SPEC §3.5), never touches the inside of a
string or comment, and never expands sugar (`;`, `>CF`, ...). Given input it
cannot rewrite safely (an unterminated string, or a newline inside a string) it
returns the input unchanged. It emits **plain text, not JSON** (so `--json`
does not apply), and never executes the program.

| Mode | Effect |
|---|---|
| (default) | Print the canonical form to stdout. Exit 0. |
| `--write` | Rewrite the file in place if it is not already canonical. Exit 0. |
| `--check` | Verify only. Exit 0 if already canonical, 1 otherwise (a message goes to stderr). Exit 2 on a read error. |

The canonical file is the formatter's content plus a single trailing newline
(an empty file stays empty). The formatter is **not** a syntax canon: it is
pinned, together with the GUI formatter (`src/gui/code-formatter.ts`), to a
shared corpus (`tests/formatter-corpus.json`) so the two implementations
produce identical output and neither drifts.

## 18. Test runner (`test`)

`ajisai test <file-or-dir>` runs Ajisai programs and checks each one's result
against expectations declared **in the source itself as `#@` directive
comments**. It adds **no language word** — there is no `ASSERT` in Core. A
directive line is an ordinary `#` comment (SPEC §3.4) that the interpreter
ignores; only the host runner reads the `@` marker. This keeps the test harness
strictly outside language semantics (§15.1): a test file runs identically under
`ajisai run`, which simply ignores the directives. Each file is executed through
the same production Core as `run`.

A directory argument is walked recursively for `*.ajisai` files in sorted
order; a file argument is run whatever its extension.

### Directives

One directive per line, anywhere in the file:

| Directive | Meaning |
|---|---|
| `#@ status ok` \| `#@ status error` | Expected outcome. Default is `ok` (the program must run without error). |
| `#@ stack <display>` | Expected final stack as space-joined display strings (the same rendering as `stackDisplay`). |
| `#@ output <line>` | Expected `PRINT` payload. Repeatable; the full ordered list of `output` lines must match exactly. |
| `#@ error <substring>` | The run must fail with a message containing `<substring>`. Implies `status error`. |

An unknown keyword, an empty `#@`, or an unknown `status` value is reported as a
failure for that file, so a mistyped directive never passes silently. A plain
`#` comment (no `@`) is never a directive.

### Report and exit codes

Default output is one `PASS <name>` / `FAIL <name>` line per file (failures list
their reasons), followed by a summary. With `--json`, stdout carries exactly one
document:

```json
{
  "schemaVersion": 1,
  "status": "ok | error",
  "total": 3,
  "passed": 2,
  "failed": 1,
  "results": [
    { "name": "tests/add.ajisai", "passed": true, "failures": [] }
  ]
}
```

| Exit code | Meaning |
|---|---|
| 0 | Every test passed. |
| 1 | At least one test failed. |
| 2 | Usage error: the path does not exist, a directory cannot be read, or it holds no `.ajisai` files. |

## 19. Projects: `build` and `lock`

A **project** is a directory with an `ajisai.toml` manifest. The manifest
declares *intent* — the project's name and version, its entry source, the host
capabilities it is allowed to use, and its local path dependencies. The
generated `ajisai.lock` records *realized fact* — the content identity of every
source, the content identity of each public word, the capabilities the run
actually required, the targeted specification version, and the manifest schema
version. This manifest/lockfile split (declared intent vs. realized identity) is
what makes a multi-file project run reproducibly, and it ties package identity
to *content* rather than to a name and version alone.

Both commands drive the same production Core as `run`; they add no language
semantics. Capability confinement reuses the runtime's existing capability gate:
a project host reports a capability as unavailable when the manifest does not
allow it, so a disallowed Hosted word fails through the ordinary
missing-capability path (§2.5, `why: environment`).

### Manifest (`ajisai.toml`)

A small, fixed TOML subset (hand-parsed; Core stays dependency-light):

```toml
[project]
name = "example"
version = "0.1.0"
entry = "src/main.ajisai"
specification = "1.0"        # optional

[capabilities]
allow = ["effect", "clock"]  # optional; capability protocol strings (§15)

[dependencies]
util = { path = "lib/util.ajisai" }   # local path to an Ajisai source file
```

Capability names are exactly the receipt's capability vocabulary
(`clock`, `secureRandom`, `serial`, `audio`, `jsonExport`, `config`, `effect`),
so the allow-list, the runtime gate, and the receipt's `requiredCapabilities`
all speak the same names. A dependency `path` names an Ajisai *source file*
relative to the manifest directory; dependency sources run before the entry, in
declared order, into one shared dictionary (a flat, direct-dependency
namespace). Transitive/sub-manifest dependencies and remote registries are out
of scope for this phase.

### `ajisai build <dir>`

Resolves the manifest, runs the composed project (dependencies, then entry)
confined to the allowed capabilities, and renders the result exactly as `run`
does (same envelope; `--json`, `--explain`, `--lang` apply). If an `ajisai.lock`
is present, a successful run is verified against it and refused on drift.

| Exit code | Meaning |
|---|---|
| 0 | The project ran successfully (and matched `ajisai.lock` if present). |
| 1 | A language error, a disallowed capability, or a lockfile mismatch. |
| 2 | A project setup error (missing/malformed manifest, unknown capability, unreadable source). |

### `ajisai lock <dir> [--check]`

Runs the project and writes `ajisai.lock` — canonical JSON, byte-stable across
regenerations from unchanged inputs — with the realized identities and required
capabilities:

```json
{
  "lockfileVersion": 1,
  "manifestSchemaVersion": 1,
  "project": { "name": "example", "version": "0.1.0" },
  "specification": "1.0",
  "capabilities": { "allowed": ["effect"], "required": ["effect"] },
  "sources": [
    { "role": "dependency", "name": "util", "path": "lib/util.ajisai", "sourceIdentity": "..." },
    { "role": "entry", "path": "src/main.ajisai", "sourceIdentity": "..." }
  ],
  "publicWords": [ { "name": "EXAMPLE@DOUBLE", "contentIdentity": "..." } ]
}
```

With `--check`, the lockfile is verified rather than written.

| Exit code | Meaning |
|---|---|
| 0 | Wrote `ajisai.lock` (default), or it was already current (`--check`). |
| 1 | The project failed to run, or (`--check`) the lockfile is stale or missing. |
| 2 | A project setup error, or the lockfile could not be written. |

### `ajisai new <dir>`

Scaffolds a new project at `<dir>`: an `ajisai.toml` (a valid instance of the
manifest format above, naming the project after the final path component and
allowing the `effect` capability) and a runnable `src/main.ajisai`. The
generated project builds and locks immediately — `ajisai build <dir>` succeeds
straight away. It refuses to overwrite: an existing `<dir>` is a usage error.
This writes template files only; it never executes.

| Exit code | Meaning |
|---|---|
| 0 | Scaffolded the project. |
| 2 | Usage error: an unsafe project name, an existing path, or an I/O failure. |
