# Ajisai algebraic simplification — language-wide rollout plan

> Status: Design plan / non-canonical. `SPECIFICATION.md` remains the canon; the
> mathematical formalization and this plan are design lenses for compression and
> review, not a second authority. This plan extends the Phase 3–5 outline in
> `ajisai-algebraic-simplification-review.md` from a representative sample to the
> whole surface vocabulary.

## 0. Why this plan

The review doc and the `algebra_primitives` registry proved the method on a
representative slice (19 coverage entries, 22 primitives). The slice covers
about one tenth of Ajisai's surface vocabulary. This plan scales the same
method — *few semantic primitives, traceable derived words, isolated hosted
effects, unchanged observations* — across the entire language, as a ratcheted
sequence of small, observation-preserving PRs rather than one big-bang refactor.

## 1. Grounded inventory (the scope of "the whole language")

Measured from the current tree:

| Surface layer | Size | Source of truth |
| --- | --- | --- |
| Core words | ~87 across 15 categories | `rust/src/builtins/builtin_word_definitions.rs` |
| Module words | ~90 across 8 modules (ALGO, CRYPTO, IO, JSON, MATH, MUSIC, SERIAL, TIME) | `rust/src/interpreter/modules/` |
| Symbol sugar / aliases | `+ - * / % < <= > >= = <> ! & ? . .. , ,, ' ~ ^ …` | `rust/src/core_word_aliases.rs`, `rust/src/surface_forms.rs` |
| Conformance cases | 53 | `tests/conformance/index.html` |
| Law-test files | 14 | `rust/tests/*_laws.rs` |
| Algebraic classification today | 19 entries / 22 primitives | `docs/formalization-coverage.json` |

Core-word categories (count): cast 13, vector 12, control 11, higher-order 8,
arithmetic 8, comparison 7, modifier 6, tensor 5, module 4, logic 3, dictionary
3, constant 3, plus io/conversion/staging singletons.

### Existing machine-readable semantics to reuse

`rust/src/coreword_registry.rs` already pins per-word axes that are strong
*evidence* for algebraic classification (and a reconciliation target), even
though it carries no algebraic role today:

- `WordPurity` ∈ {Pure, Observable, Effectful}
- `Partiality` ∈ {Total, Partial, Projecting}
- `NilPolicy` ∈ {Passthrough, CreatesNil, RejectsNil, ConsumesNil, PreservesReason}
- `MassContract` ∈ {Fixed{consumes,produces}, Dynamic}
- `SafetyLevel` ∈ {A, B, C, D, Quarantined}

The plan uses these as corroborating evidence and adds a reconciliation check,
**not** as the home of algebraic role (see D1).

## 2. Target model

The end state is an explicit *algebra of Ajisai*:

1. A closed, justified set of **semantic primitives** (the `algebra_primitives`
   registry, grown from 22 to whatever the whole vocabulary genuinely needs —
   the goal is *few*, so every new primitive must earn its place).
2. Every surface word assigned exactly one `semantic_role` ∈
   `{Primitive, Derived, Sugar, HostedEffect, Exploratory, NotPortableYet}`.
3. `derived_from` forming a DAG rooted at primitives; every edge resolves
   (already enforced by the validator).
4. Sugar tracked through a desugar map that is observation-transparent.
5. Hosted effects isolated from Core by `capability.check` + `Eff` append.
6. Completeness and reconciliation enforced in CI so the system cannot regress
   or silently grow new unclassified words.

Success is the review's final criterion, made total: *Ajisai reads as a small
set of primitives plus algebraically derived words, for the entire surface, not
a sample.*

## 3. Architectural decisions (recommended)

**D1 — Algebraic role lives in the coverage JSON (design lens), not exported
from the Rust registry.** Rationale: the formalization is deliberately separate
from implementation, and §8 of the brief forbids leaking Rust enum names as
semantic axes. The Rust registry's purity/nil/mass axes are used as *evidence*
and a *reconciliation target*, but `semantic_role`/`derived_from`/
`algebraic_family` stay in `formalization-coverage.json`.

**D2 — Coverage becomes exhaustive over the surface vocabulary, via a ratchet.**
A generated word manifest lists every core + module + sugar word. The validator
cross-checks coverage against the manifest and reports the *unclassified* set.
An explicit `unclassified_allowlist` lets CI start non-fatal and tighten
category by category, so classification is incremental, never big-bang.

**D3 — One canonical primitive registry; closed families; sugar via desugar
map.** Keep `algebra_primitives` the single inventory. `algebraic_family` stays
a closed set (extend only deliberately). Each `Sugar` entry records its
desugaring target and is covered by a desugar law (`desugar_laws.rs`).

**D4 — Each consolidation is test-gated.** No derived word is moved onto a
shared primitive implementation (Phase D) until its law tests and conformance
cases are grouped and green first (Phase C). Tests precede code.

## 4. Phased rollout

### Phase A — Inventory & scaffolding (one PR, no semantic change)

- Add a generator `scripts/generate-word-manifest.mjs` that emits the full
  surface vocabulary (core specs + module words + aliases) to a manifest, from
  the existing registries (no hand-maintained list).
- Extend `check-formalization-coverage.mjs`:
  - cross-check every coverage `id`/`surface` against the manifest;
  - compute and print the unclassified set and a coverage percentage;
  - read an `unclassified_allowlist`; fail only on words that are neither
    classified nor allowlisted (ratchet starts with everything allowlisted).
- Wire into the existing Quality Gate job (already runs the coverage check).

Outcome: a live "classified surface words %" metric, starting near 10%.

### Phase B — Classify by algebraic family, category by category

One small PR per category. For each word: declare any new primitive it rests
on, set `semantic_role` / `derived_from` / `algebraic_family`, link its law
tests and conformance cases, and remove it from the allowlist. Recommended
order (cheapest/most-settled first, building on the proven slice):

1. **logic** — done (meet/join/involution).
2. **arithmetic** (ADD SUB MUL DIV MOD CEIL FLOOR ROUND) — Derived from
   `exact-real.bihomographic` + `bubble.passthrough`; CEIL/FLOOR/ROUND are
   projecting observations.
3. **comparison** (EQ NEQ LT LTE GT GTE COMPARE-WITHIN) — Derived from
   `exact-real.budgeted-order` + `k3.domain`; COMPARE-WITHIN is the budget
   primitive surfaced.
4. **constant** (TRUE FALSE NIL) — Primitive injections (TRUE/FALSE k3.domain;
   NIL bubble).
5. **vector / tensor** (LENGTH GET TAKE … SHAPE RANK RESHAPE TRANSPOSE FILL) —
   Derived from `structure-lift.*`; reshape/transpose from a reshape-group
   primitive (may need `algebra.structure.reshape-group`).
6. **cast / string** (STR NUM BOOL CHR CHARS JOIN TOKENIZE TRIM* SUBSTITUTE
   STARTS-WITH? ENDS-WITH? >CF) — Derived over `exact-scalar.codepoint-sequence`
   and scalar domains; predicates project into `k3.domain`.
7. **dictionary** (DEF DEL LOOKUP) + **module** (IMPORT* UNIMPORT*) — Primitive
   dictionary lookup / state-transformer composition; FORC as a consumption
   policy.
8. **modifier** (KEEP STAK EAT TOP FLOW VENT) — Derived modifier combinators;
   FLOW (`~`) is Sugar (no-op marker); VENT (`^`) is the bubble handler.
9. **higher-order** (MAP FILTER FOLD SCAN UNFOLD ALL ANY COUNT) — Derived
   recursion schemes over `state-transformer.composition` + `structure-lift`.
10. **control** (COND EVAL EXEC FORC IDLE PRECOMPUTE; SPAWN AWAIT KILL MONITOR
    STATUS SUPERVISE) — split: COND/EVAL/EXEC Derived state-transformer; the
    child-runtime words Exploratory (already so).
11. **modules** MATH (Derived exact-arithmetic: SQRT POW ABS GCD LCM MIN MAX
    NEG SIGN INTERVAL IS-EXACT …) vs TIME/IO/CRYPTO/SERIAL/MUSIC/JSON
    (HostedEffect or Exploratory) — classify the module boundary explicitly so
    Core never absorbs wall-clock/entropy/transport.
12. **symbol sugar** — each alias `Sugar` with a desugar target; covered by a
    desugar law.

### Phase C — Reconcile with the Rust registry axes (review Phase 3)

Add a reconciliation check (in the coverage validator, reading the exported
registry metadata) that flags contradictions between the design lens and the
implementation axes, e.g.:

- `semantic_role = HostedEffect` ⟺ `WordPurity = Effectful` and a capability;
- `derived_from` contains `bubble.passthrough` ⟺ `NilPolicy = Passthrough`;
- a `Derived` binary arithmetic/logic word ⟺ `MassContract = Fixed{2,1}`;
- a `Projecting` partiality ⟺ family `observation` or a projecting note.

Also: group law-test functions and conformance case names by primitive vs
derived-word example, so tests read as algebra families with word-level cases.

### Phase D — Consolidate small derived words onto shared primitives (review Phase 4)

Only after C is green for a family, and only where observations are identical:

- comparison: route EQ/NEQ/LT/LTE/GT/GTE through one budgeted-order core
  (mirror of the already-unified `logic_kleene`);
- arithmetic: make the bihomographic coefficient dispatch visibly single-source;
- vector/tensor: ensure elementwise ops share the structure-lift helpers;
- string predicates: share one codepoint-sequence core.

Each consolidation is a separate PR backed by pre-existing law tests; the
diff must not change any conformance expectation.

### Phase E — Lock in (review Phase 5)

- Flip the completeness gate to required: every non-exempt surface word has a
  `semantic_role`; allowlist empty (or a small, documented `Exploratory` set).
- Keep the closed `derived_from` vocabulary, closed families, and the
  registry-reconciliation check required in CI.

## 5. Artifacts created or changed

- `docs/dev/archive/ajisai-algebraic-simplification-rollout-plan.md` (this file).
- `scripts/generate-word-manifest.mjs` (new) and a manifest output.
- `scripts/check-formalization-coverage.mjs` (extended: manifest cross-check,
  unclassified ratchet, registry reconciliation).
- `docs/formalization-coverage.json` (entries grow toward full coverage;
  `algebra_primitives` grows as genuinely needed; `unclassified_allowlist`).
- `.github/workflows/test.yml` (new checks run inside the existing Quality Gate).
- Law-test / conformance grouping and naming (Phase C).

## 6. Guardrails (from the brief's §8 — non-negotiable)

- Observation-preserving: no user-visible output change; conformance
  expectations are never edited to suit implementation.
- `SPECIFICATION.md` stays canonical; formulas/metadata are subordinate lenses.
- No Rust enum names or Debug forms leak as semantic axes (reconciliation reads
  the curated registry metadata, not Debug strings).
- Keep the distinctions: Boolean ≠ Number; NIL ≠ FALSE ≠ numeric zero; U is a
  truth value, not absence; Error ≠ NIL.
- No `sqrt(...)` wrappers or `~n/d` approximations; exact-real observations stay
  canonical.
- Hosted effects never enter Core determinism.
- Ratchet, not big-bang: every PR is small, reversible, and green.

## 7. Sequencing, metrics, done criteria

- Cadence: Phase A (1 PR) → Phase B (≈10–12 category PRs) → Phase C (1–2 PRs
  per family alongside B) → Phase D (opt-in consolidation PRs) → Phase E (1 PR).
- Tracking metric: *classified surface words %* (and *primitive count*, which
  should stay small — a rising primitive count is a smell to review).
- Done when: 100% of surface words are classified or explicitly, minimally
  exempt; every `derived_from` resolves; families closed; registry
  reconciliation green; all existing tests and conformance unchanged.

## 8. Open decisions for the maintainer

1. **Exhaustiveness target** — classify *all* ~180 surface words, or Core-only
   first and modules later? (Plan assumes all, modules last.)
2. **Primitive budget** — is there a soft cap on `algebra_primitives` we want to
   defend (e.g. ≤ ~30)? This keeps "few primitives" honest.
3. **Reconciliation strictness** — should registry/role contradictions be fatal
   from the start, or warn-then-ratchet like the unclassified gate?
