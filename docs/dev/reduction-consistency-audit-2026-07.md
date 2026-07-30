# Reduction consistency audit — 2026-07

**Status: non-canonical.** This is a design note. It defines nothing (see
`LANG.AUTHORITY.SOURCES`). It records defects found after the ~60→10 concept
reduction so that a person or an agent can pick them up later; it is not a
change to Ajisai semantics.

## What this is

The concept reduction (see `concept-reduction-2026-07.md`) cut Ajisai from about
sixty design concepts to ten, across five merged pull requests (#1376, #1378,
#1379, #1380). Every gate was green at each merge. This audit was run afterward,
against `main` at `05966af1`, precisely because *green gates were not evidence of
consistency* — the reduction changed behavior in places no gate measured.

Twenty-seven distinct defects survive. Every finding below was reproduced
against the built CLI (`rust/target/release/ajisai`), the committed spec sources,
or a gate run — not inferred from documentation. Findings are grouped by how much
they matter, not by the order they were found.

The single most important line in this document: **the ten Words promoted out of
modules (SQRT, ABS, NEG, SIGN, MIN, MAX, SORT, UNIQUE, CONTAINS, INDEX-OF) have
zero conformance cases and were checked by a schema gate that never inspected
their arity, consumption, NIL policy, or projection.** That is the mechanical
reason most of the contract-level defects below exist, and it is why the
executorKey drift fixed in #1379 could be introduced in the first place.

---

## Root cause: the gates that did not measure what changed

A defect is only as fixable as the gate that would keep it fixed. Four gate gaps
account for most of what follows; closing them first is what makes the rest
durable rather than a list that re-rots.

1. **Conformance coverage is not enforced.** `scripts/check-conformance-coverage.mjs`
   exists and reports the truth —
   `covered: 59 (85.5%) / missing (10): ABS CONTAINS INDEX-OF MAX MIN NEG SIGN SORT SQRT UNIQUE` —
   but it is wired into neither `package.json`, CI, nor any git hook
   (`grep -rc check-conformance-coverage package.json .github/workflows/` → `0 0`).
   The missing ten are exactly the promoted Words. `LANG.CONFORMANCE.FAMILIES`
   requires "at least one conformance path" per contract.

2. **`check-word-schema-migration.mjs` inspects the wrong fields.** It validates
   `name`, `family`, manifest membership, clause existence, `executorKey`, the
   three documentation strings, `effects`, and aliases. It has **no arm** for
   `stack.inputs/outputs`, `consumption`, `nilPolicy`, `projection`, `errorWhen`,
   `purity`, `determinism`, `capability`, `hostedEffect`, or `interpretationRole`
   — the exact fields that drift below. Its `directive` set still lists the
   deleted word `FLOW`.

3. **The semantic firewall is defeated on two axes.** `check-semantic-firewall.sh`
   greps `[Hh]edged` — which does not match all-caps `HEDGED` — and its wrapper
   pattern `"[^"]*(…)"` only scans double-quoted strings, so backtick template
   literals are invisible. It also excludes `-g '!**/elastic/**'`, a directory
   that no longer exists. Both live GUI strings in D19 slip through.

4. **wasm-feature code is never linted.** CI runs `cargo clippy --all-targets`
   (default features) and `cargo check --features wasm`, but never
   `cargo clippy --lib --features wasm`. That build fails with **11 errors**
   today (D16), all in gutted module-removal bodies.

---

## Design decisions required (only the owner can make these)

Four defects cannot be "fixed" without first deciding which side is right — the
implementation or the specification. Each is a genuine language-design call, not
a mechanical repair. **The rest of the findings are unambiguous** (one side is
plainly wrong); these four are not.

| # | Question | If spec wins | If implementation wins |
|---|----------|--------------|------------------------|
| **DD1** | Is `[ 3 ] 3 EQ` TRUE or FALSE? | `LANG.VALUES.DISJOINT` says a one-element Vector is not a Scalar → **FALSE**; fix `comparison.rs` | keep TRUE; carve a documented exception into DISJOINT |
| **DD2** | Do ABS/NEG/SIGN/MIN/MAX/SQRT lift over vectors? | `exactArithmetic`+`LANG.COLLECTIONS.LIFT` say **yes**; make the executors lift | change these six words' `family`/`clauses` so they never claimed to |
| **DD3** | Do qualified names (`CORE@ADD`, `USER@…@…`) resolve? | `LANG.DICTIONARY.RESOLUTION` says two tiers, bare names → **remove the `@` resolver** | keep qualified resolution; rewrite the clause to describe it |
| **DD4** | Do LENGTH/GET consume their vector operand? | they retain it (pinned by conformance) → change the declaration to `retain` | change the executors to consume — but that breaks the pinned examples |

DD2 and DD4 interact with the vocabulary; DD1 and DD3 with the runtime. Until
they are settled, the contracts they touch stay red.

---

## Tier 1 — changes observable behavior

Each reproduced against the CLI at `05966af1`.

### D1. UNKNOWN survives as a live protocol value
A NIL is emitted over HostProtocolV1 carrying `truthValue: "unknown"` and the
`truthValued` capability.

```
$ printf 'NIL 1 LT\n' | ajisai run - --json   # stack[0]:
{"type":"nil","semantics":{"shape":"absence","truthValue":"unknown",
 "capabilities":[…,"truthValued"], …}}
```

- Producer: `rust/src/types/value_operations.rs:109` `ValueData::Nil => Some("unknown")`
- Emitted: `wasm_interpreter_bindings/wasm_value_conversion.rs:227`, `cli/report.rs:214`
- **Permitted by a normative source**: `spec/host-protocol-v1.schema.json:64`
  `"truthValue": { "enum": ["true","false","unknown"] }`
- Contradicts `LANG.VALUES.TRUTH` ("exactly two values") and `LANG.VALUES.NIL`
  ("absence rather than undecidability"). Reproduces for `NIL TRUE AND`,
  `NIL NOT`, `NIL 1 EQ`.

### D2. `[ 3 ] 3 EQ` is TRUE — a Vector equals a Scalar
`comparison.rs:370-375`. Contradicts `LANG.VALUES.DISJOINT`. Self-inconsistent:
`[ [ 3 ] ] 3 EQ` is FALSE. **See DD1.**

### D3. A stack CodeBlock is reported as `type: "nil"`
`value_protocol.rs:224` maps `CodeBlock(_)` → `("nil", Null)`. Confirmed:
`{ 1 ADD }` run with `--json` yields `type=nil, shape=codeBlock`. Contradicts
`LANG.VALUES.DISJOINT` (CodeBlock is one of six domains) and
`LANG.OBSERVATION.PROJECTIONS` ("Stack is the ordered typed values"). The GUI
switches on `item.type` (`src/gui/output-display-renderer.ts:304-332`), so its
`case 'block':` is unreachable and a stack block renders as `NIL`. (The mapping
predates the cut, but DISJOINT is a new clause it now contradicts.)

### D4. Six promoted words declare elementwise lifting they do not implement
ABS, NEG, SIGN, MIN, MAX, SQRT carry `family: exactArithmetic`
(law: `lifting: elementwise`) and clause `LANG.COLLECTIONS.LIFT`.

```
[ 1 2 3 ] [ 4 5 6 ] ADD  => [ 5/1 7/1 9/1 ]     # non-promoted sibling lifts
[ -1 -2 ] ABS            => error: expected scalar value, got non-scalar
[ 4 9 ] SQRT             => error: expected number, got other format
[ 1 5 ] [ 3 2 ] MIN      => error: expected scalar value, got non-scalar
```

Same shape as the executorKey bug. Their `errorWhen` lists lack `shapeMismatch`
(which all eight lifting siblings carry), so the drift is in `family`+`clauses`.
**See DD2.**

### D5. `stackTargetMode` — the deleted TOP/STAK axis — is declared by 9 of the 10 promoted words
```
$ node -e '…tally errorWhen…'
stackTargetMode   9   ABS NEG SIGN MIN MAX SQRT UNIQUE CONTAINS INDEX-OF
negativeInterval  1   SQRT
nonNumericOrInterval 1 SQRT
```
Exactly nine of the promoted ten (not SORT); no other Core word carries it.
Contradicts `LANG.MODIFIERS.CONSUMPTION` ("exactly one modifier axis"). SQRT
additionally declares two conditions naming the deleted interval domain. The
axis really is gone from the runtime (`1 2 .. ADD` → `Unknown word: ..`). Leaks
into `docs/word-reference.md:276,289,…`.

### D6. `;` and `;;` desugar to deleted modifiers, so they cannot execute
`tokenizer.rs:45-58` expands `;`→`. ,` and `;;`→`.. ,,`; those targets no longer
exist.
```
1 2 ; ADD    => error: Unknown word: .
1 2 ;; ADD   => error: Unknown word: ..
```
Contradicts `LANG.SOURCE.DESUGAR` ("Modifier punctuation … lower to canonical
concepts before evaluation"). Compounded in the GUI: the symbol palette
(`code-input-editor.ts:78`) offers `;` and `.`, and `gui-layout-state.ts:18-37`
still parses `..`/`;;` as a STAK target, with `gui-layout-state.test.ts:44-70`
asserting the deleted axis. `spec/gui-semantics.md` lists "modifier background
indication" as a requirement.

### D7. Resolution is not two-tier — qualified names resolve and execute
`resolve_word.rs:188-264` is a 0-to-3-layer `@` path resolver.
```
5 5 CORE@ADD                     => 10/1
5 5 DICT@CORE@ADD                => 10/1
{ 10 ADD } 'ADD10' DEF 5 EXAMPLE@ADD10  => 15/1
```
Contradicts `LANG.DICTIONARY.RESOLUTION` ("a name resolves in Core or in User").
`EXAMPLE` is a hard-coded module-era name. The `module_vocabulary`/`import_table`
tiers behind it are dead (never inserted into), but the `CORE@`/user paths are
live. **See DD3.**

### D8. SORT/UNIQUE/CONTAINS: three-way disagreement, and LOOKUP tells users the opposite
| Word | `words.json` | Rust default | runtime |
|------|-------------|--------------|---------|
| SORT | `passthrough` | Passthrough | **ERROR** |
| UNIQUE | `rejectNil` | Passthrough | ERROR |
| CONTAINS | `rejectNil` | Passthrough | ERROR |

```
NIL SORT   => error: SORT: expected vector, got non-vector
NIL ABS    => NIL          # correctly-declared sibling passes through
```
SORT's `passthrough` contradicts `LANG.FAILURE.PASSTHROUGH`. Worse,
`builtin_word_details.rs:135-136` renders Passthrough as the user-visible LOOKUP
string **"NIL operands pass through as NIL."** The corpus already records this and
deliberately leaves it untested: `tests/conformance/index.html:1423-1425`
("Suite stays silent; listed as a divergence").

### D9. Zero conformance coverage for all ten promoted words
`covered: 59 (85.5%) / missing (10): ABS CONTAINS INDEX-OF MAX MIN NEG SIGN SORT SQRT UNIQUE`.
0 occurrences of each in any `ajisai-source` block; ADD has 14. Contradicts
`LANG.CONFORMANCE.FAMILIES`. Root cause of D4, D5, D8, D10, D13. **Gate gap #1.**

### D10. LENGTH and GET declare `consumption: eat` but retain their vector
```
[ 1 2 3 ] LENGTH      => [ 1/1 2/1 3/1 ] 3/1
[ 1 2 3 ] ,, LENGTH   => [ 1/1 2/1 3/1 ] 3/1     # KEEP identical — axis unobservable
[ 1 2 3 ] 1 GET       => [ 1/1 2/1 3/1 ] 2/1     # 2 declared inputs, 1 consumed
```
Retention is intended (pinned at `tests/conformance/index.html:242,360`), so the
declaration is wrong; the schema already offers `retain`. **See DD4.**

### D11. `LANG.EFFECTS.OUTPUT` contradicts `LANG.DICTIONARY.MUTATION` and `words.json`
`language-semantics.md:301`: "Every other Word is pure … changes nothing else."
But `words.json` declares DEF `effectful`/`dictionaryWrite`, DEL
`effectful`/`dictionaryDelete`, LOOKUP `observational`/`dictionaryRead`, plus 7
`conditional`. Under `LANG.CONTRACT.REGISTRY`, `words.json` is authoritative for
purity/effects, so the kernel prose is wrong.

### D12. `LANG.COLLECTIONS.LIFT` over-claims and its own ERROR rule is violated
The clause says comparison Words lift and "Any other pairing is ERROR".
```
[ 1 2 ] [ 3 1 ] LT     => error (clause implies [ TRUE FALSE ])
[ 1 2 ] [ 3 1 ] EQ     => FALSE            # structural, not element-wise
[ 1 2 3 ] [ 3 ] ADD    => [ 4/1 5/1 6/1 ]  # 3-vs-1 must be ERROR per the rule
```
No comparison Word links `LANG.COLLECTIONS.LIFT` in `words.json`, so the clause
also contradicts the registry. EQ/NEQ declare `errorWhen:[shapeMismatch,
unsupportedComparison]` and raise neither.

### D16. `cargo clippy --lib --features wasm -- -D warnings` fails with 11 errors
Confirmed. The bodies are gutted module-removal remnants: `restore_import_state`
parses four fields and discards all; `restore_imported_modules` is an empty
`if let`; `reset_with(full)` is `if full { execute_reset() } else { execute_reset() }`
so `reset`/`reset_session` are indistinguishable. **Gate gap #4.**

### D25. Six more `rejectNil` declarations that pass NIL through or accept it
```
NIL { 1 ADD } MAP     => NIL          # words.json: rejectNil
NIL EXEC              => NIL          # words.json: rejectNil
[ 1 2 ] NIL CONTAINS  => FALSE        # words.json: rejectNil
```

### D26. Every unknown-word diagnosis still tells users to check module imports
`1 2 . ADD` → error advises "Check imports/definitions: module import 漏れ…".

---

## Tier 2 — misleads a reader of a canonical/normative document

### D17. The Reference still documents the deleted module system
`public/docs/index.html:1615-1656` teaches named dictionaries, `DICT@WORD`
resolution, a four-step ladder whose step 2 is "Words from modules you have
imported", an `Ambiguous word 'GREET': found in EXAMPLE@GREET, AUDIOLIB@GREET`
error (unreachable — only one user dictionary exists), and an "Importing word
groups" section. Contradicts `LANG.DICTIONARY.RESOLUTION` and — pointedly —
`LANG.AUTHORITY.PRESENT`, the clause that was *just added to forbid exactly this*.
`scripts/check-reading-surfaces.mjs:26-33` passes only because these names are
whitelisted as "resolution examples". **The clause does not obey its own rule.**

### D11 (prose side), D13, D14, D15, D18, D19, D24, D27
- **D13.** MOD/FLOOR/CEIL/ROUND declare `projection reason: undecidable`, absent
  from `LANG.FAILURE.PROJECT`'s reason list and contradicting `LANG.VALUES.EXACT`
  ("total and decidable"). `2 SQRT FLOOR` → `1/1`, never NIL. Only producer is
  `push_undecidable_nil` (`tensor_cmds.rs:24`), reachable only via the
  `ExactReal::Computable` variant, which nothing constructs.
- **D14.** `words.schema.json:45,60` requires `interpretationRole` (a deleted
  concept); all 69 entries populate it; nothing outside the two schema lines
  reads it. `capability` and `hostedEffect` likewise required and unread;
  `LANG.MACHINE.WORD` omits all three.
- **D15.** NIL?/NIL-REASON declare `projection reason: notAvailable`, not among
  the 14 `NilReason` protocol strings, with a condition
  (`…OrFieldAbsent`) naming the deleted Record domain. `1 NIL?` → Boolean (not a
  projection); `1 NIL-REASON` → reasonless NIL.
- **D18.** The conformance corpus pins the spelling `UNKNOWN`
  (`index.html:57`, "may only be loosened … never tightened") and retains ~90
  lines of empty section headers for PRECOMPUTE, MODULE WORDS, MATH@PI,
  MATH@ENCLOSE, Tier 2, water budgets — zero cases beneath them. This is the
  document `LANG.CONFORMANCE.CORPUS` calls "the decision procedure".
- **D19.** `execution-controller.ts:74` shows `[HEDGED-WINNER]` banners;
  `output-display-renderer.ts:538` emits `COMPARE-WITHIN: N calls … (UNKNOWN)`
  — a deleted Word and a deleted value, user-visible. **Gate gap #3** lets both
  through.
- **D24.** The `NIL` word produces a reasonless NIL (`NIL NIL-REASON` → NIL),
  contradicting `LANG.VALUES.NIL` ("carries a reason … the entire observable
  content"). SORT/UNIQUE declare `projection reason: null` on an unreachable
  `emptyVector` condition (`[ ] SORT` is a source error), so `sort.rs:100` is dead.
- **D27.** `generate-specification.mjs:7` inlines
  `docs/dev/specification-implementation-rules.html` into the canonical
  `SPECIFICATION.html` as §12 (a "Mandatory" list mandating file layout that
  `LANG.AUTHORITY.FREEDOM` calls unobservable), while `LANG.AUTHORITY.SOURCES`
  says `docs/dev/` "defines nothing".

---

## Tier 3 — dead code and stale assertions (no behavior change)

`SPECIFICATION.html` §12.1 is normative and forbids these: "When source code is
changed, nearby comments must be reviewed and updated so they remain accurate."

- **D20.** ~800 lines of dead numeric machinery for computable reals / π /
  intervals / comparison water: `types/exact/pi.rs` (173 lines, `pi()` has zero
  callers), `computable.rs`, `observation.rs` (`Water`, `RatInterval`, `Refine`),
  and in `value.rs` the `Computable` variant, `DEFAULT_COMPARISON_WATER`,
  `TIER2_INTERNAL_WATER`, `ExactCmp::Starved`, `cmp_within`, `observe_enclosure`,
  `tier2_pinned_integer`. Escapes dead-code detection because the items are `pub`.
- **D21.** ~20 comments asserting the opposite of adjacent code; a stray doc
  comment for a deleted `unknown()` constructor now heading `is_truth_value` with
  a broken intra-doc link `[ValueData::Unknown]`; `is_unknown()` and
  `ValueData::Unknown` are referenced in six files but do not exist;
  `surface_forms.rs:12` documents runtime aliases `.`→`TOP`, `~`→`FLOW` that are
  not in `CORE_WORD_ALIASES`.
- **D22.** Unreachable enum variants that still export protocol strings for
  deleted domains: `SemanticKind::{Record,Process,Supervisor,Unknown}`,
  `ValueShape::{Record,Handle,Unknown}`, `ValueOrigin::{ModuleWord,Optimizer}`,
  `ErrorCategory::UnknownModule`, `NilReason::{InvalidLens,MissingField,
  PortDisconnected,NoData}`, `Interpretation::{Interval,Timestamp}` (the latter in
  the **normative** `displayHint` enum, with a live `format_as_interval`).
- **D23.** Vacuous tests: `coreword_registry.rs:1015` loops over a now-empty
  collection (body never runs); `protocol_string_tests.rs:99` asserts only a
  string constant under a comment about "its own `ValueData::Unknown` variant";
  `arithmetic_operation_tests.rs:578` states "We can't yet drive the comparison
  path into the Undecidable" while pinning it.
- **Dead GUI:** `src/gui/module-selector-sheets.ts` (370 lines fed by
  `collect_available_modules()` → `[]`); `wasm-interpreter-types.ts:2-7`
  (`ExecutionMode` still offers `elastic-*`/`hedged-*`);
  `execution-worker-manager.ts:331` (a hedged two-worker race, unreachable
  because `get_execution_mode()` hard-returns `"greedy"`).
- **Correctly retained no-ops — do NOT delete these:** `collect_available_modules`,
  `collect_module_catalog_words_info`, `collect_module_words_info`,
  `collect_hedged_trace`, `set_execution_mode`, `get_execution_mode` — each
  pinned in `host-protocol-v1.schema.json` `x-ajisai-methods`, called by the GUI,
  honestly commented. HostProtocolV1 pins the method list; removing them is a V2
  break.

---

## Unconfirmed suspicions (not verified either way)

1. `ExactCmp::Absent → Undecided(0)` (`comparison.rs:119`) — needs a Scalar
   holding the nil `Fraction` sentinel paired with an `ExactScalar`. Could not
   construct one from the 69 Words, nor prove it impossible.
2. Whether the browser GUI actually renders a stack CodeBlock as `NIL` (D3) —
   traced the code path but did not run the GUI.
3. `Algebraic::floor_int` (`algebraic_floor.rs:27`) loops `bits *= 2` unbounded;
   terminates for a genuinely irrational value, but the invariant that
   `Algebraic` is never rational was not proven for every multiquadratic value
   the 69 Words build. Adversarial probing found no hang.
4. Case-count discrepancy: `grep -c 'class="ajisai-case"'` → 170 vs the coverage
   script's "169 case sources". Not chased.
5. Whether `LANG.AUTHORITY.PRESENT` binds `docs/word-reference.md` (making D5's
   leak a clause violation) or it is an orphan artifact.

---

## Verdict per audited area

1. **Semantic drift between authorities — NOT CONSISTENT.** Nine drifts; the
   promoted words are the epicentre. Mechanical cause: gate gap #2.
2. **Surviving traces of deleted concepts — NOT CONSISTENT.** Three change
   behavior (D1, D6, D7) plus user-visible GUI strings, ~800 dead lines, a dozen
   dead enum variants, ~20 lying comments. The six deliberate HostProtocolV1
   no-ops are correctly separated.
3. **Semantic gaps — NOT CONSISTENT, one clean sub-area.** UNKNOWN is not gone
   (D1). Tier/budget survives in four contracts (D13). Record survives as
   protocol strings (D22). Resolution is not two-tier (D7). **Comparison totality
   on the budget axis is genuinely clean** — extensive adversarial probing
   decided every case in finite time, and the water path is unreachable. Totality
   is instead broken by *domain*: `2 SQRT SQRT` errors on a numeric operand under
   no registered condition.
4. **Internal spec contradictions — NOT CONSISTENT.** D11, D12, D27, and
   `LANG.AUTHORITY.PRESENT` violating itself via D17.
5. **Under-tested reductions — NOT CONSISTENT, and this explains the rest.**
   Zero coverage for the promoted ten, against a clause that requires it, with the
   reporting gate wired into nothing.

For calibration: `cargo test --all-targets` (0 failures),
`cargo clippy --all-targets`, `npm run check/lint/test` (226 passing), and all
eleven wired gates pass. **Every defect above is invisible to the suite as
configured.** That is the finding behind the findings.
