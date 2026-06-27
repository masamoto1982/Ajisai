# Agent CLI Output Contract (`ajisai --json`)

Status: contract document (non-canonical for language semantics).
Authority for Ajisai semantics: `SPECIFICATION.html` only.
This file is the authority for the *shape* of the `ajisai` CLI's `--json`
output, which AI agents and verification scripts consume.

## 1. Commands and exit codes

```
ajisai run <file.ajisai> [--json] [--explain] [--lang <ja|en>]
ajisai check <file.ajisai> [--json] [--explain] [--contract] [--lang <ja|en>]  # tokenize + parse + resolve (+ optional contract check); never executes
ajisai modifier <phrase...> [--json] [--lang <ja|en>]  # infer the modifier for an intent phrase; never executes
ajisai version [--json]
```

`--explain` adds a deterministic plain-language projection of the diagnosis
(`explanation`, §10). `--contract` adds a light, execution-free flow-mass and
NIL-flow check to `check` (`planCheck`, §11). `--lang` selects the language for
all plain-language output (default `ja`). All are additive and never change the
structured fields; with neither `--explain` nor `--contract`, output is
unchanged. `--contract` raises exit 1 when it finds a malformed (over-consuming)
plan; advisories and notes do not change the exit code.

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
  "planCheck": { ... } | null
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

`version --json` emits only `{ "schemaVersion", "status", "version" }`.
`modifier --json` emits `{ "schemaVersion", "status", "modifier": { ... } }` (§12).

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
           "energyProxyScore": 0, "proxyVersion": 1, "suggestions": [ ] } }
```

The 18 leading fields are the Virtual Tensor Unit observation counters
(`docs/dev/virtual-tensor-unit-design.md`). They describe observed
structural work (data movement, allocation, kernel selection) and are
deterministic for a given program and input.

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
- `hasFallback` is `true` when a `^` (VENT) or `=>` (OR-NIL) NIL fallback
  appears. An unguarded NIL source (`mayBubble` non-empty, `hasFallback` false)
  is an advisory, the `handleUnknownOrNil` prompt rendered ahead of execution.
- `findings` are the plain-language `planCheck` surface (L0), most severe
  first; `severity` is `error` (malformed plan), `advisory`, or `note`. Empty
  means the plan is clean over the known prefix.

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
