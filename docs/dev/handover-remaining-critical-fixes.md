# Handover — remaining critical-fixes work (CS4, CS5) and lint/warning debt

This is a handover for a **fresh session**. It continues the "Ajisai 重大問題改修"
program. Change sets CS1, CS2, CS3 (observation + ownership), and CS6 are merged
into `main`. What remains is **CS4** and **CS5** from the instruction, plus some
pre-existing lint/warning debt that is unrelated to that program.

Authority order is unchanged: `SPECIFICATION.html` is canonical (`§2.5`);
`CLAUDE.md` is archived/non-canonical. The maintained differential oracle is
`tools/ajisai-repro/` and the CI `reference-differential` job is a **blocking**
gate (any CLI-vs-reference divergence over the Core conformance corpus fails).

## Working conventions established by CS1–CS3, CS6 (follow these)

- **Branch**: develop on `claude/ajisai-critical-fixes-r45e82`. Each merged PR
  auto-deletes the remote branch, so restart from main every time:
  `git fetch origin main && git checkout -B claude/ajisai-critical-fixes-r45e82 origin/main`,
  then a fresh `git push -u` creates a new PR (do **not** force-push onto merged
  history).
- **Provenance**: `docs/provenance/*` is a tripwire in the Quality Gate. Run
  `npm run provenance:attest` **after** `git add`-ing every new/changed file
  (the attestation set comes from `git ls-files`; attesting before `git add`
  produced a stale hash and a red Quality Gate twice in this program). Then
  `git add docs/provenance` and commit.
- **Commit shape**: failing test → implementation → contract/generated/docs → CI.
  Keep test additions separate from the fix so history shows the reproduction.
- **Full local gate before pushing**:
  `cd rust && cargo test --lib && cargo test --tests && cargo fmt --check &&
   cargo check --features wasm --target wasm32-unknown-unknown` (needs
  `rustup target add wasm32-unknown-unknown`), then from repo root
  `npm run check && npm run lint && npm test && npm run check:file-size &&
   npm run check:traceability && npm run check:semantic-firewall &&
   npm run check:formalization-coverage && npm run word:manifest:check &&
   npm run check:skill && npm run provenance:check`, and finally
  `python3 tools/ajisai-repro/compare.py --conformance` (needs the release CLI:
  `cargo build --bin ajisai --release`).
- CI clippy/fmt are **advisory** (`continue-on-error` unless
  `AJISAI_STRICT_QUALITY=true`); `cargo check`/`cargo test`/the differential/the
  quality checks are what actually block. Do not rely on clippy to gate, but do
  keep your own changed files clippy- and fmt-clean.

---

## CS4 — separate `UNKNOWN` from NIL at the type level

### Why

Ajisai's core semantics treat three things as distinct: operational absence
(NIL / Bubble), the logical undetermined value (`UNKNOWN`, U), and misuse/errors
(`§4.5`, `§7.5`). But `UNKNOWN` is currently **represented as a NIL node**:

- `rust/src/types/value_operations.rs:72` — `Value::unknown()` builds
  `ValueData::Nil` carrying `NilReason::LogicallyUnknown`.
- `:90` — `unknown_with_agreed_prefix` (the Tier-2 starved comparison result)
  does the same, plus an `agreedPrefix` diagnostic.
- `:110` — `is_unknown()` is `matches!(self.nil_reason(), Some(LogicallyUnknown))`.
- `:375`/`:406`-ish — `is_nil_storage` / `is_operational_nil` exist precisely to
  paper over the shared storage.

So every NIL call site must remember to check `is_unknown()` first or it risks
treating U as NIL (wrong NIL passthrough, wrong diagnostics). `VENT` is the sharp
case: it keeps a non-NIL top and skips its fallback, so U-as-storage-NIL would
mis-route. The separation is not guaranteed by the type, only by discipline.

### Scope anchors (measured on current main)

- `NilReason::LogicallyUnknown` appears in **7** places (`rg LogicallyUnknown rust/src`).
- `ValueData::Nil` is matched in **~61** non-test sites (`rg 'ValueData::Nil' rust/src | grep -v test`).
  These are the exhaustive-match audit surface once a new variant is added.

### Plan (compiler-assisted, staged; split across 2–3 PRs)

Use the exhaustive-match error as the worklist: add the variant first, then let
`cargo build` enumerate every `match` that must be classified.

**PR-1 — introduce the variant and split the predicates (no NIL merge yet).**
1. Add `ValueData::Unknown` (see `rust/src/types/mod.rs` `enum ValueData`).
   Give U its own diagnostic carrier — do **not** reuse NIL's `AbsenceMetadata`;
   add `UnknownMetadata` (or an `agreedPrefix: Option<usize>` field) so the
   Tier-2 `agreedPrefix` survives without a `reason = logicallyUnknown` NIL.
2. Reimplement `Value::unknown()` / `unknown_with_agreed_prefix()` on the new
   variant. Keep the predicate contract:
   `unknown().is_unknown() == true`, `unknown().is_nil() == false`,
   `unknown().is_operational_nil() == false`, `nil().is_unknown() == false`.
   Delete `is_nil_storage` (only the old shared representation needed it).
3. Fix the compiler-flagged matches minimally to keep behavior identical (U can
   temporarily route the same as before via explicit arms). Land green.

**PR-2 — audit every `ValueData::Nil` match.** Classify each: does U behave the
same as NIL here (then `ValueData::Nil | ValueData::Unknown`) or differently
(separate arm)? Do **not** blanket-merge. High-risk arms to review one by one:
arithmetic + generic NIL passthrough; comparison; K3 logic (`AND`/`OR`/`NOT`);
`VENT`; truthiness; vector/tensor shape + dense representation; display; CLI/WASM
protocol; arena; equality; semantic kind/capabilities; child runtime;
memoization/hash/word identity; the NIL diagnostic accessor words.

**PR-3 — retire `NilReason::LogicallyUnknown`** once nothing constructs or reads
it. If a persisted old format must be decoded, convert at the decode boundary
only; never let it back into the runtime.

### External contracts that MUST NOT change (add tests that pin them)

- Display: `UNKNOWN`. Protocol type/axis: `truthValue = unknown`. Capability:
  `truthValued`.
- K3: `NOT U = U`, `F AND U = F`, `T OR U = T` (full truth tables).
- NIL diagnostic accessors applied to U must **not** report `logicallyUnknown`
  as a NIL reason; operational NIL reason/origin/recoverability unchanged.
- Tier-2 comparison `agreedPrefix` diagnostic preserved.
- `VENT` keeps U as non-NIL and does not evaluate its fallback.
- Wire results (CLI `--json`, WASM) must not leak NIL absence metadata for U.

### Done when

- No `ValueData::Nil`-represented Unknown remains; `NilReason::LogicallyUnknown`
  is gone; the U/NIL split is a type invariant, not a predicate convention.
- `npm run check:semantic-firewall` and the K3 MC/DC tests pass; the wire
  protocol contracts above are pinned by tests; differential stays 0-divergent.

---

## CS5 — resource control for internal computation cost

### Why

The execution step budget (`max_execution_steps`, charged once per word in
`rust/src/interpreter/execute_builtin.rs`) does not price the expensive work
**inside** a word: algebraic term×term products, reciprocal conjugate recursion,
sign/bounds precision doubling, BigInt blow-up, and huge numeric-literal parses.
"Mathematically decidable" is not "terminates in safe time/memory". Ajisai must
stay exact but return a **diagnosable** runtime failure at a resource ceiling
rather than an approximation, wraparound, panic, OOM, or WASM trap.

### Current state (measured)

- No unified limits type: `rg 'RuntimeLimits|WorkKind|max_numeric_work' rust/src`
  → none.
- Materialization **is** already guarded: `rust/src/materialization_limit_tests.rs`
  and `dimension_limit_tests.rs` cover RANGE/FILL/RESHAPE element-count overflow.
  So CS5 should focus on **algebraic arithmetic, BigInt, and source/literal
  limits**, and merely fold the existing materialization guards into the unified
  structure — do not re-implement them.
- Uncharged hotspots: `rust/src/types/exact/algebraic.rs` `mul` (`:227`),
  `mul_fraction` (`:212`), `sign` (`:249`); `algebraic_field.rs` `reciprocal`
  (`:21`) and `MqTerms::mul` (`:75`).

### Plan

1. `RuntimeLimits` (a struct on the interpreter / execution context, **not** a
   global) with fields like `max_execution_steps`, `max_numeric_work`,
   `max_bigint_bits`, `max_algebraic_terms`, `max_materialized_elements`,
   `max_source_bytes`, `max_numeric_literal_digits`. Documented defaults;
   injectable small limits for tests; child runtimes inherit the parent's.
2. A work meter with `charge(kind, count)` / `reserve_elements(count)` that
   returns `Result`. Charge algebraic term-pair products, reciprocal/inverse
   recursion and inner multiplications, sign/bounds precision doubling, CF /
   enclosure refinement, tensor/vector allocation+flatten, and BigInt result bit
   length. Use `checked_mul`/`checked_add` for sizes so overflow itself is a
   limit failure.
3. Thread a budget/meter into the exact-arithmetic API so internal failure is
   representable — return `Result<AlgebraicResult, _>` (or pass `&mut meter`) —
   starting with the most dangerous `mul`, `reciprocal`, `sign`, and map to a
   diagnosable Ajisai error at the interpreter boundary. Prefer reusing
   `ExecutionLimitExceeded` unless you also update `SPECIFICATION.html` + the
   conformance suite for a new public error category (avoid new categories if you
   can).
4. Tokenizer/literal ceilings (source bytes, literal digits, denominator BigInt
   parse, nesting depth) checked **before** building the big value.
5. Attacker-input tests that fire each guard at a **low injected limit** (no need
   to actually allocate huge values), asserting a specific diagnosable error
   synchronously/deterministically — never "slow ⇒ pass". Also: a limit failure
   must not leave a corrupted partial stack observable.

### Constraints

- Never convert a resource ceiling to an approximation or wraparound.
- Conformance results must not depend on a specific limit value (limits are a
  safety control, not value semantics); all conformance must pass under default
  limits.
- Detect **before** the huge allocation/computation where possible (pre-estimate
  or charge inside the loop), not after.

---

## Lint / warning debt (pre-existing, unrelated to CS1–CS6)

Fixed already in the PR that ships this handover:
- The 4 default-build dead-code warnings (`collect_core_builtin_definitions`,
  `user_dictionary_names`/`user_dictionary_words`, `CatalogWord.description`) —
  all wasm-only consumers; annotated `#[cfg_attr(not(feature = "wasm"),
  allow(dead_code|unused_imports))]`. Default `cargo build` is now warning-clean.
- Semantic firewall was passing **vacuously in CI** (runner lacked `rg`, so every
  `if rg …; then FAIL` was a no-op). The script now hard-fails if `rg` is absent
  and CI installs ripgrep before the step. Verified it passes for real locally.

Deferred (document-only; low priority, CI does not block on these):
- **~45 clippy warnings** in files unrelated to this program (`rust/src/elastic/`,
  `.../audio/`, `.../cast/`, `types/fraction*`, `coreword_registry.rs`, …):
  `contains()` vs `iter().any()`, manual `RangeInclusive::contains`, `doc list
  item overindented`, `iter().cloned().collect()` → `to_vec()`, missing `Default`
  impls, `from_str`/`cmp` trait-confusion, redundant closures, etc. Most are
  `cargo clippy --fix`-able. CI clippy is advisory, so this is a cleanup sweep,
  not a blocker; do it as its own PR to keep the diff reviewable and never mix it
  into a semantics change.
- **wasm-build-only** `unused import: word_identity::content_digest` (re-exported
  for the host-only `cli` consumers, unused under `--target wasm32`). Needs a
  target/cfg-aware annotation; left out of the quick fix because the cli-vs-wasm
  cfg interplay wasn't traced. Low priority (wasm check does not fail on
  warnings).
- CI actions emit a **Node 20 deprecation** notice (GitHub forces Node 24). Bump
  the `actions/*` runtimes when convenient.
