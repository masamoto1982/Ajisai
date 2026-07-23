# Structural Memory Safety — Roadmap (design note, non-canonical)

> Canonical semantics live in `SPECIFICATION.html`. This note is a planning
> document. It sequences a set of memory-safety improvements and records which
> language-surface changes each phase would require in the canonical spec. It
> makes no language-level guarantee on its own; each phase carries its own spec
> edit and conformance tests when it lands.

## Why this note exists

The goal is to raise Ajisai's memory safety to Rust's level and, where Ajisai's
design allows, past it. The framing is deliberate: at the *language* level
Ajisai already lacks the constructs Rust's borrow checker exists to police.
Values are immutable persistent structures (`ValueData::Vector(Arc<Vec<Value>>)`,
`Tensor`/`Record` behind `Arc`; `rust/src/types/mod.rs`), there are no
user-visible raw pointers, no mutable aliasing, and no manual `free`. So
"Rust-level" is close to already-met for the pointer-UB class, and the
interesting targets are the ones Rust itself does **not** fully close:

- deterministic discipline over the one value class that *is* a resource
  (handles),
- a *provable* bound on space, not just absence of undefined behavior,
- turning exhaustion into a diagnosable value instead of a process abort.

The unifying method is Ajisai's own identity — *check the computation before it
runs; keep partial failure visible* — applied to **space and resources** the way
it is already applied to numeric value integrity.

## The through-line: convention → structure

A later phase (Phase 5, below) generalises a single principle: **stop guarding
invariants by convention (comments, review, docs) and move them into structure
that a machine rejects before the program runs.** Ajisai already embodies this
with `#:contract` + `ajisai check` (opt-in machine-readable contracts checked
ahead of execution; `rust/src/cli/contract_decl.rs`, SPEC §7.14). Phases 1–4 are
the *first, highest-leverage instances* of that same move applied to memory and
resources; Phase 5 then rolls the method out across the remaining constraint
classes. The phases are ordered so each reuses the machinery the previous one
built.

## Phase 1 — Handles as linear/affine resources (highest leverage)

**Problem.** The only language-level values with resource semantics are
`ProcessHandle(u64)` and `SupervisorHandle(u64)` (`rust/src/types/arena.rs`),
produced/consumed by the quarantined runtime words `SPAWN`, `AWAIT`, `STATUS`,
`KILL`, `MONITOR`, `SUPERVISE` (`rust/src/coreword_registry.rs`). These are the
only place an Ajisai program can exhibit a Rust-ownership-class bug:
use-after-`KILL`, double-`KILL`, or a leaked (never-consumed) handle. Every
other value is immutable and cannot be misused this way.

**Leverage from existing design.**
- The `EAT`/`KEEP` consumption modifiers (SPEC §6.2) are already an affine
  substrate: `EAT` = consume (move), `KEEP` = branch (duplicate). Declaring a
  handle **non-duplicable and consume-exactly-once** is linear typing expressed
  in vocabulary the language already has.
- The contract checker already checks declarations against conservative
  inference *before* execution. Adding a linearity axis is an extension of an
  existing opt-in mechanism, not a new enforcement subsystem.

**Beyond a borrow checker.** Ajisai has no borrowable references, so no borrow
checker is needed — a pure move/linear discipline fits the concatenative stack
model exactly, and because contracts are machine- and AI-readable, an agent can
read the handle discipline *before* running the program.

**Increment plan.**
1. **Done (1.1).** Grammar: extend `#:contract` with an optional linearity term
   (`linear` / `affine` / `droppable`) parsed into `ContractDecl`
   (`rust/src/cli/contract_decl.rs`, `contract_linearity.rs`). Additive:
   unstated = unchecked, matching the existing `purity`/`nil` fields.
2. **Done (1.2).** First enforcing check: because a consumption modifier is its
   own token binding the following word, a `KEEP` applied to a handle-discharging
   word (`KILL`/`AWAIT`) is detectable directly on a word's body tokens and
   retains the handle past its one permitted consumption. `ajisai check` reports
   it as an `error` under `linear`/`affine`; `KEEP` on an observer
   (`STATUS`/`MONITOR`) and `EAT` on a discharger are correctly clean;
   `droppable` opts out. Sound and flow-insensitive — no false positives.
3. **Next (1.3).** Deeper flow-sensitive tracking: a handle produced and then
   dropped (consumed by a non-discharging word) or discharged across a call
   boundary; an undischarged obligation at end of a `linear` word body.
4. **Done (spec).** Handle linearity is stated normatively in §4.7 (handles are
   linear resources) and the opt-in contract discipline is documented in §7.14,
   cross-referencing §6.2 (EAT/KEEP). No new registry field is claimed — the
   resource role is a classification over the existing handle words.

## Phase 2 — Space as a contract (static footprint bounds) — beyond Rust

**Problem Rust leaves open.** Rust removes UB but does not bound memory; a Rust
program can OOM. Ajisai's banner is "check the computation before it runs" — so
extend that check from *value* correctness to *space*.

**Leverage.** Vectors/tensors carry shape (`DenseTensor.shape`), words have known
stack effects, and arithmetic is exact rational (digit growth is deterministic).
A symbolic worst-case footprint as a function of input shape is therefore
inferable, declarable, and checkable — the same shape the `nil-free`/`may-nil`
check already has, but over allocation. Surface: `ajisai check --space`; contract
term carries a footprint bound `f(shape)`.

## Phase 3 — Exhaustion as a bubble, not a crash — beyond Rust

**Problem Rust leaves open.** Rust aborts the process on OOM by default —
unrecoverable. Ajisai already makes partial failure a first-class value (the
`NIL` bubble) and already has materialization ceilings
(`rust/src/interpreter/runtime_limits.rs`,
`rust/src/materialization_limit_tests.rs`).

**Done.** The materialization water level (`max_materialized_elements`) now
routes the *budget miss* of the well-formed generative words `RANGE` and `FILL`
onto a diagnosable `NIL` (`NilReason::SpaceExhausted`, `AbsenceOrigin::SpaceBudget`)
instead of a channel error, recoverable at a pipeline's end by a single `^`
(`VENT`). The two words become `Projecting`/`CreatesNil` (matching the
DIV/GET/NUM/CHR family); malformed requests (an infinite `RANGE`, a
non-conforming `RESHAPE`) stay ordinary errors. Making exhaustion a value in the
flow is pure Ajisai idiom and is strictly past Rust's abort-on-OOM. Spec: the
new "Materialization (expansion) budget" row in the Water Levels table, and the
`RANGE`/`FILL` classification in §7.14.

**Next (3.2).** Extend the projection to any other generative/expansion path
that can exceed the water level (e.g. tensor broadcast, repeated `CONCAT`), and
carry the overflowing shape/count in the `AbsenceMetadata` diagnosis so a tool
can report *what* overflowed, not only that something did.

## Phase 4 — Drive implementation `unsafe` toward zero (the "Rust-level" floor)

**Problem.** This is about the interpreter, not the language. The only
substantive `unsafe` is the work-stealing parallel path (`SendPtr`/`SendMutPtr`,
`from_raw_parts_mut`; `rust/src/interpreter/parallel.rs`). Its soundness rests on
a disjoint-index-range invariant and join-before-read.

**Plan (either/both).**
- Replace with a safe scoped abstraction where the ~270µs `std::thread::scope`
  overhead (noted in `parallel.rs:11`) is acceptable, then raise
  `#![forbid(unsafe_code)]`.
- Where replacement is too costly, make the existing shadow-validation /
  `IntegrityMode` (`rust/src/interpreter/shadow_validation.rs`,
  `docs/dev/physical-resilience-design.md`) the *documented, always-on net* for
  the `unsafe` path: fast (parallel) and reference (sequential) results must
  agree on `data + hint + absence-core + host effects`, or the sequential result
  wins.

Phase 4 can proceed in parallel with 1–3.

## Phase 5 — Maximise structurally-enforced constraints (future, user-supplied)

Recorded in advance from the uploaded instruction "構造で守れる制約を最大化する
改修指示書." Its thesis — *move invariants from convention (comment/review/doc)
into structure a machine rejects before run* — is the same principle Ajisai
already implements as `#:contract` + `ajisai check`. Phase 5 is therefore not a
new paradigm but a **systematic rollout of the contract mechanism across the
remaining constraint classes** the instruction enumerates, adapted to Ajisai's
surfaces:

| Instruction's constraint class | Convention today | Structural target in Ajisai |
| --- | --- | --- |
| Value invariants (`NOT NULL`, enum, range, newtype) | word doc / review | contract requirements + role/shape checks in `ajisai check` |
| Access control / resource ownership | word doc | Phase-1 handle linearity; word safety-level gates |
| Integrity / transaction | word doc | contract guarantees; VTU shape/exactness invariants |
| Performance / space budget | none | Phase-2 space contracts; Phase-3 water levels |
| Deploy / config shape | ad-hoc | contract + manifest/lockfile checks (`rust/src/cli/manifest.rs`, `lockfile.rs`) |

The instruction's own residue rule maps cleanly: what Ajisai cannot close at a
single check boundary (behavioral compatibility of word replacement à la LSP,
time/history-dependent rules, distributed integrity) stays as executable
specification — conformance tests, contract tests, and the diagnosable
NIL/UNKNOWN/error model — rather than being forced into a false structural
guarantee. Sequencing: Phase 5 begins only after 1–4 land, because it depends on
the linearity axis (Phase 1) and the space/water-level machinery (Phases 2–3)
being available as the "structure" it moves constraints into.

## Sequencing summary

1. Phase 1 (handle linearity) — start here; smallest surface, closes the only
   real language-level hazard, reuses the contract checker + EAT/KEEP.
2. Phase 3 (exhaustion → bubble) — quick payoff from an existing-limits branch.
3. Phase 2 (space contracts) — the ambitious beyond-Rust target.
4. Phase 4 (implementation `unsafe` → 0) — in parallel throughout.
5. Phase 5 (structural-constraint rollout) — after 1–4, per the uploaded
   instruction.
