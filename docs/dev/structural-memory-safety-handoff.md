# Structural Memory Safety — Session Handoff (non-canonical)

> Canonical semantics live in `SPECIFICATION.html`. This is an engineering
> handoff for continuing the structural-memory-safety effort in a fresh session.
> It points at the two design docs that carry the detail and records what is
> done, what remains, and the working conventions that kept every increment
> sound and green.

## The three documents to read first

1. `docs/dev/structural-memory-safety-roadmap.md` — the five-phase plan and the
   status of each phase.
2. `docs/dev/structural-constraint-ledger.md` — the Phase 5 constraint ledger
   (the uploaded "maximize constraints enforceable by structure" instruction
   applied to Ajisai), item-by-item.
3. `docs/dev/space-contract-design.md` — the Phase 2 (space contracts) cost
   model and inference plan; read before touching Phase 2.2.

## Goal

Raise Ajisai's memory safety to Rust's level and, where the design allows, past
it — redefining "memory safety" from *pointer-UB avoidance* (already given, since
Ajisai values are immutable persistent structures) into Ajisai's own idiom:
linear resources, contract-declared space, exhaustion-as-bubble, and a compiler-
enforced `unsafe` floor — plus a systematic convention→structure sweep.

## What is DONE (all merged to `main`)

| Phase | What landed | PR |
| --- | --- | --- |
| 1.1 | `#:contract` gains the `linear`/`affine`/`droppable` linearity axis (parse + record) | #1332 |
| 1.2 | Linearity **enforced**: `KEEP` on a handle-discharging word (`KILL`/`AWAIT`) is an error; spec §4.7 (normative) + §7.14 | #1333 |
| 2.1 | `#:contract` gains the `space:const/linear/superlinear/unbounded` axis (parse + record) | #1336 |
| 3 | Materialization over the water level (`RANGE`/`FILL`) becomes a diagnosable `NIL` (`SpaceExhausted`), VENT-recoverable, not an abort; `RANGE`/`FILL` become `Projecting`/`CreatesNil`; Water-Levels spec row | #1334 |
| 4 | Crate-wide `#![deny(unsafe_code)]` with one audited allow-island in `parallel.rs` (+ wasm-bindgen glue allows); soundness pin for the `fill_parallel` clamp | #1335 |
| 5.1 | Constraint ledger + `hover_syntax` **tokenizes** (item 9) — found & fixed COND, TRANSPOSE | #1337 |
| 5.2 | `hover_syntax` **names only real words** (item 10) — found & fixed COMPARE-WITHIN, FLOW | #1338 |
| 5.3 | Every concrete `hover_syntax` **runs** (item 10b) — found & fixed >CF, DEL | #1339 |
| 5.4 | `stack_effect` prose **arity matches machine `mass`** (item 11) | #1340 |
| 5.5 | Authored LOOKUP examples **run** (item 12) — found & fixed 3 drifted authored examples | #1341 |
| 5.6 | Authored example **results match execution** (item 12b) | #1342 |
| 5.7 | Manifest capability allow-list **validated at parse** (item 13) | #1343 |
| 2.2 | Space inference **enforced**: provenance-aware slot simulation gives every word a sound growth-class bound with an exactness witness; `ajisai check` rejects a declaration the inference provably exceeds, notes an unprovable one; spec §7.14 "Space growth" paragraph | (this PR) |

Net: 4 language/runtime/impl phases + the full ledger sweep. The Phase 5 sweep
alone caught **7 real defects** (5 broken doc examples across two separate
corpora + 2 non-self-contained examples).

Most Phase 5 checks live in `rust/src/builtins/builtin_word_details_tests.rs`
(now ~300 lines — watch the 500-line budget; split if it grows much more).

## What REMAINS (recommended order)

### Phase 2.3 — precise value-parametric `f(shape)` (deepening)
Phase 2.2 landed the coarse growth-class enforcement (see the DONE table). The
inference (`rust/src/interpreter/word_space.rs`) tracks slot provenance
(literal / input / unknown) and carries an exactness witness, so
`[ 0 10 ] RANGE` proves `const`, a bare `RANGE` proves `unbounded`, and anything
it cannot model (higher-order, recursion, unresolved, lazy `^`/COND paths)
degrades soundly to a note. The remaining refinement is to turn `unbounded` into
a value-parametric bound where the constraining numeric value is statically
known — the precise `f(shape)` of `space-contract-design.md` §"Increment plan"
2.3. Two calibration notes for whoever picks this up: `RANGE`/`FILL` are
`Dynamic`-mass, so the sim carries a small `space_arity_override` to inspect
their operand at all; and the coarse builtin `tight` flags in `builtin_space`
are the audited surface that licenses an `error`, so widen them only with a probe.

### Ledger finishing touches (small, optional)
- **`FORC`/`UNFOLD`/`PRECOMPUTE` concretization** — these three `hover_syntax`
  examples are still schematic (excluded by the item-10b `is_schematic` guard:
  `FORC` starts with the `!` modifier; `UNFOLD`/`PRECOMPUTE` contain `...`).
  Making them concrete runnable examples is honest polish but each needs care
  (protected-entry force; a convergent generator; `PRECOMPUTE` is
  definition-time-only so it may be irreducibly schematic).
- **Item 12b follow-through** — 7 authored `result` strings abstain (free prose
  like "the first element, 10"). Rewriting them into `Pushes <value>.` form would
  extend the value-check coverage; purely optional.

### Phase 1.3 / 3.2 (deepenings)
- **1.3** — flow-sensitive handle linearity: a handle dropped by a non-discharging
  word, discharge across a call boundary, an undischarged obligation at a
  `linear` word body's end. Same dataflow flavor as 2.2.
- **3.2** — extend exhaustion-as-bubble to other expansion paths (tensor
  broadcast, repeated `CONCAT`) and carry the overflowing shape in
  `AbsenceMetadata` so a tool can report *what* overflowed.

## Working conventions (the playbook that kept every PR green)

**Branch & PR flow.** Work on `claude/ajisai-memory-safety-eh3pd7`. Each increment
is one PR. After each merge, **re-create the branch from the freshly-merged
`main`**: `git fetch origin main && git checkout -B claude/ajisai-memory-safety-eh3pd7 origin/main`.
PRs are opened as drafts; the maintainer marks ready and merges.

**The soundness discipline (non-negotiable).** Every check *abstains* rather than
risk a false failure — assert only when you can *prove* a violation; skip
(note/continue) on anything uncertain. This mirrors the `#:contract` inference
philosophy ("an unprovable declaration is a note, never a false error"). It is
why Phase 2.2 is hard and was deferred.

**Probe-then-implement.** For every ledger check, first write a throwaway
`#[cfg(test)] mod __probe` that sweeps the whole corpus and prints
failures/abstains, calibrate against reality, *then* write the real check and
remove the probe. This is how every hidden bug was found without false positives.
When removing a probe appended to a file, verify the file is byte-identical to
origin afterward (`git diff --stat`) — probe removal twice left stray trailing
blank lines; `git checkout origin/main -- <file>` restores a pristine untouched file.

**The local gate (run ALL of these before pushing — the Quality Gate CI runs
them and they caught real issues mid-session):**
- `cd rust && cargo test --lib` and `cargo clippy --lib --all-targets` (expect 0 warnings)
- `node scripts/check-file-size-budget.mjs` — **every Rust file ≤ 500 lines** (§14.1); this bit twice — split into a sibling `*_tests.rs` module when a file crosses it.
- `npm run check:semantic-firewall`, `npm run check:traceability`
- `npm run generate:skill` then `npm run check:skill` — **SKILL.md is generated from `hover_syntax`**; any `hover_syntax` edit makes it stale. Authored LOOKUP examples and `stack_effect` are *not* in SKILL.md.
- `npm run word:manifest:check` if word metadata changed.
- **Provenance (this tripped CI twice):** run `git add -A` **first**, *then*
  `npm run provenance:attest`, then `git add -A` again, then commit.
  `generate-source-attestation.mjs` enumerates **git-tracked** files, so
  attesting before staging a new *tracked source* file misses it. The
  attestation set is the Rust/TS sources — **not** `docs/dev/*.md` and **not**
  generated SKILL.md — so a docs-only or SKILL-only change leaves the root
  unchanged and needs no re-attest (verify with `npm run provenance:check`).

**fmt quirk.** The container's `rustfmt` version disagrees with CI's on some
pre-existing files (e.g. `arithmetic_operation_tests.rs`, `parallel.rs`, older
test files) — those diffs are **not yours** and CI accepts them. Only fix
`cargo fmt --check` diffs that fall in files/lines *you* changed. Module-order
diffs in a `mod.rs` (rustfmt sorts `mod` declarations) *are* real and version-
stable — fix those.

**Commit/PR footers.** Commit messages end with the `Co-Authored-By` +
`Claude-Session` trailer; PR bodies end with the Claude Code footer and mirror
the repo's `.github/pull_request_template.md` sections.

## Key file map

| Concern | File |
| --- | --- |
| Contract declaration parse/check + axes | `rust/src/cli/contract_decl.rs`, `contract_linearity.rs`, `contract_space.rs` |
| Handle words / linearity substrate | `coreword_registry.rs` (SPAWN/KILL/…); `EAT`/`KEEP` modifiers |
| Space/exhaustion runtime | `rust/src/interpreter/runtime_limits.rs`; `vector_ops/structure.rs` (RANGE), `tensor_cmds.rs` (FILL); `error.rs` (`NilReason::SpaceExhausted`) |
| `unsafe` island | `rust/src/interpreter/parallel.rs`; crate deny in `lib.rs` |
| Doc-example ledger checks | `rust/src/builtins/builtin_word_details_tests.rs` |
| Builtin metadata source of truth | `rust/src/builtins/builtin_word_definitions.rs` (specs), `builtin_word_lookup_docs.rs` (authored examples) |
| Manifest/capabilities | `rust/src/cli/manifest.rs`, `project.rs`; `rust/src/interpreter/host.rs` (`HostCapability`) |
| Word-contract inference (for 1.3 / 2.2) | `rust/src/interpreter/word_contract.rs`, `word_contract_lattice.rs` |
