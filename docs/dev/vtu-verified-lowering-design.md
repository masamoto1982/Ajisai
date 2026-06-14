# VTU Verified Lowering — Design Note (design only; not approved for implementation)

Status: **design proposal**. This document describes a *possible* future
layer. It does not authorize any implementation. Per the work order
(`docs/dev/ai-first-competitive-upgrade-instructions.md` §6.3), the lowering
body must not be implemented until this design is reviewed and approved.

Authority for Ajisai semantics is `SPECIFICATION.html`. This note proposes an
*observational-equivalent* optimization only and defines no new language
semantics.

> **The one-bit rule (binding on any future implementation).**
> No optimization described here may change the numeric result by even one
> bit. A lowered block must produce a value that is bit-exact identical to the
> value the existing exact Fraction / continued-fraction path produces, for
> every input, or it must not be used. "Faster but slightly different" is out
> of scope and explicitly rejected.

---

## 0. Purpose and positioning

The Virtual Tensor Unit today is purely observational: it *classifies* pure,
shape-aware kernels (`VtuHint`) and *counts* structural work
(`RuntimeMetrics`, aggregated by `energyProxyScore`), but it never changes how
a value is computed (`docs/dev/virtual-tensor-unit-design.md`). Every number
flows through exact rational / CF arithmetic backed by `BigInt`.

Verified lowering is the proposed final VTU step and Ajisai's research-grade
differentiator: the **verification × low-movement** quadrant. The idea is
*not* "drop exactness to go fast". It is:

> Prove, ahead of execution, that a block's every intermediate and final
> value stays inside a fixed machine width (i64) and that the machine-width
> computation is bit-exact with the exact path. Only then run the cheap
> machine-width kernel. Otherwise run the existing exact path unchanged.

This keeps Ajisai's "no hidden truncation" guarantee (SPEC §2.3) intact: the
fast path is a *proven-equivalent* substitution, never an approximation.

## 1. Target blocks

The candidate set is intentionally narrow for the first design:

- A `QuantizedBlock` (`rust/src/interpreter/quantized_block.rs`) whose
  `vtu_hint.suitability == VtuSuitability::StrongCandidate`. Today that means
  `KernelKind::MapUnaryPure` and `KernelKind::PredicateUnaryPure` — pure,
  elementwise, embarrassingly parallel kernels with `DataMovementClass::Low`.
- `purity == QuantizedPurity::Pure`. Side-effecting or unknown-purity blocks
  are never eligible (they cannot be re-ordered or proven in isolation).
- Operating over a dense tensor (`types::DenseTensor`), which already stores
  `numerators: Vec<i64>`, `denominators: Vec<i64>`, an `is_pure_integer`
  flag, and a `valid_mask` for NIL lanes. The dense representation is the
  natural substrate for a machine-width loop.

Explicitly **out of scope** for the first design (deferred to later rounds,
each requiring its own review):

- `FoldBinaryPure` / `ScanBinaryPure` (`WeakCandidate`): reductions carry an
  accumulator and raise order/associativity questions. Not a v1 target.
- Any block touching `ExactScalar` (continued-fraction irrationals): these
  are unbounded by construction and can never be proven i64-bounded.
- Any block with NIL/UNKNOWN-producing semantics that the dense kernel cannot
  reproduce bit-exactly (see §2.4).

## 2. Proof obligation

Lowering a block to an i64 kernel is permitted only when **all** of the
following are discharged statically (before running the kernel). If any
cannot be proven, the block is not lowered (§3).

### 2.1 Rationality / denominator-1 (where required)

For an integer-domain kernel, every operand lane and every intermediate must
have denominator 1 (be an integer). The dense tensor's `is_pure_integer`
flag is the fast precondition; it must be re-checked per operand, not
trusted across operations that can introduce a denominator (e.g. `DIV`).

For a rational-domain kernel (numerator/denominator both i64), the obligation
is on *both* limbs: see §2.2 and §2.3. A v1 implementation may restrict
itself to the integer domain and reject anything with a non-unit denominator,
which is simpler to prove and still covers the common case.

### 2.2 Value-range analysis (i64 bound)

Each input lane's numerator (and denominator, in the rational domain) must be
shown to lie within a derived interval, and the kernel's operation must be
shown to keep every intermediate and the result within `[i64::MIN, i64::MAX]`.

- Inputs: the dense tensor stores `i64` limbs already, so the *input* bound is
  structural. The obligation is on the *outputs/intermediates*.
- Per-op range propagation: for `+`, `-`, `*` the output magnitude is bounded
  by a function of the input bounds (e.g. for `*`, `|a|·|b|`). The proof
  computes a conservative bound from the input intervals and checks it does
  not exceed the i64 range. For a scalar-broadcast `MapUnaryPure` (the common
  case, e.g. `[ k ] *`), the per-lane bound is `|k| · max|lane|`.

### 2.3 Overflow analysis

Range analysis (§2.2) **is** the overflow proof when it shows the bound fits
i64. Where a tight static bound cannot be derived, the design permits a
*checked* machine-width kernel as a fallback proof strategy: run with
`checked_add` / `checked_mul`; on the first `None` (would-overflow), abandon
the lowered result and fall back to the exact path for the whole block. This
is **not** speculative lowering of the result (§3): the exact path is the sole
source of the returned value whenever any lane would overflow, so no
overflowed value is ever observable.

For the rational domain, overflow must be checked on both limbs *and* on the
gcd-reduction step (reduction multiplies cross terms); the v1 integer-domain
restriction sidesteps reduction entirely.

### 2.4 Bit-exactness with the exact path (NIL / UNKNOWN preservation)

The lowered kernel must reproduce the exact path's result *including its
absence semantics*:

- A lane that the exact path projects to NIL (e.g. division by zero, or an
  input lane already NIL per `valid_mask`) must be NIL in the lowered result,
  with the same `NilReason`. The dense `valid_mask` is the carrier; the proof
  must show the kernel preserves it lane-for-lane.
- The logical UNKNOWN (three-valued comparison, SPEC §7.5) arises only from
  CF comparison budgets, which are out of scope (§1, no `ExactScalar`); a v1
  integer kernel never produces UNKNOWN and must reject any block that could.
- The result tensor's `is_pure_integer` / denominators / shape must match
  what the exact path would have produced, so downstream display and
  semantics are identical.

The discharge of 2.4 is what makes this "verified" rather than "quantized":
the substitution is sound only if the observable value — number, shape, and
absence — is identical.

## 3. Failure handling: unconditional fallback, never speculation

If any obligation in §2 cannot be discharged, the block is executed by the
existing exact path, unchanged. This is a *static* decision (or, for the
checked-kernel strategy in §2.3, a decision made before any lowered value is
committed).

**Speculative lowering is explicitly not adopted.** The design does not run a
fast kernel and then "check the answer" against the exact path after the fact:
that would either (a) require running the exact path anyway, defeating the
purpose, or (b) risk a window where an unverified value is observable. The
only sanctioned model is *prove-then-run* (or *prove-bound, run-checked,
fall-back-on-breach* per §2.3, where the breach is detected before the result
is used).

Consequence: verified lowering can only ever *match or reduce* structural
work for a given output; it can never change the output. A block that fails
verification costs at most the (cheap, static) analysis before falling back.

## 4. Observation (counters)

Following the existing VTU policy (counter names describe observed work, never
assert an outcome — `docs/dev/virtual-tensor-unit-design.md`,
`docs/quality/energy-proxy-score.md`), the design proposes adding, when
implemented:

- `vtu_lowered_block_count` — blocks executed via a verified i64 kernel.
- `vtu_lowering_rejected_count` — blocks that were StrongCandidates but failed
  verification.
- A machine-readable rejection-reason tally (denominator, range, overflow,
  absence-mismatch, out-of-scope-kind), surfaced under `trace-quant` (§5) so
  a developer can see *why* a block was not lowered.

These are additive `RuntimeMetrics` fields. If they feed `energyProxyScore`,
that requires a `proxyVersion` bump and a weight-table update
(`docs/quality/energy-proxy-score.md`); the first cut may report them
*alongside* the score without changing the formula, to avoid re-baselining the
regression catalog prematurely.

## 5. Relationship to existing features

- **`force-no-quant` / `Interpreter::set_force_no_quant`**: already gates
  whether blocks are quantized at all. Verified lowering sits *below*
  quantization (it only considers already-quantized StrongCandidates), so
  `force-no-quant` transitively disables it. The design also proposes a
  dedicated off switch (e.g. a `force-no-lowering` feature or runtime flag)
  so lowering can be disabled independently while keeping quantization on —
  essential for the differential testing in §6.
- **`trace-quant`**: the natural home for lowering decisions and rejection
  reasons (§4), mirroring how quantization decisions are already traced.
- **`differential_tests.rs`**: already cross-checks that quantized and
  `force-no-quant` runs produce identical stacks. The same harness, extended
  with a lowering-on/lowering-off axis, is the primary correctness gate for
  §2.4 (every catalog program must produce byte-identical stacks with lowering
  on and off).
- **`energy_proxy_regression_tests.rs`**: the score guard; verified lowering
  should *lower* scores for covered blocks, which means tightening baselines
  in the enabling PR (per the re-baseline discipline).

## 6. Staged rollout and how each stage is measured

Each stage is a separate, independently reviewed PR. No stage proceeds without
the differential equality gate (§5) passing.

1. **Stage 0 — analysis only (no execution change).** Implement the proof
   checks (§2) as a pure function over a `QuantizedBlock` + operand metadata
   that returns `Lowerable { … } | Rejected(reason)`, plus the rejection
   counters (§4). Execution still always takes the exact path. Measure: the
   counters show how many real-program blocks *would* be lowerable; criterion
   (`interpreter-performance-benchmarks`) confirms no regression from running
   the analysis.
2. **Stage 1 — integer-domain `MapUnaryPure` scalar-broadcast, behind an
   off-by-default flag.** The narrowest useful kernel. Gate: differential
   equality (lowering on == off) across the full test suite, plus a dedicated
   bit-exactness property test (random integer tensors, random scalar).
   Measure: `energyProxyScore` drop on the covered catalog cases and criterion
   wall-time delta; record both in the PR.
3. **Stage 2 — integer-domain elementwise binary (`+`, `-`, `*`) same-shape
   and broadcast.** Extends the kernel surface; same gates. Measure as Stage 1.
4. **Stage 3 — rational domain (i64/i64 limbs) with checked reduction.** Only
   if Stages 1–2 show a worthwhile score/time win to justify the added proof
   complexity (§2.1–§2.3 on both limbs). Same gates.
5. **Stage 4 — enable by default.** Flip the flag only after Stages 1–3 have
   accumulated differential-equality evidence and a criterion baseline showing
   a real, measured improvement. `force-no-quant` and the dedicated lowering
   switch remain available.

Measurement instruments at every stage:

- **Correctness:** differential equality (lowering on/off) — the gate.
- **Structural effect:** `energyProxyScore` on the regression catalog
  (`docs/quality/energy-proxy-score.md`), re-baselined in the enabling PR.
- **Wall-time effect:** criterion `interpreter-performance-benchmarks`,
  compared against `bench-baselines/`.
- **Honesty:** any energy claim remains a *proxy* claim until physical joule
  measurement exists (standing policy). Verified lowering reduces *observed
  structural work*; that is what we report.

## 7. Binding constraints (restated)

- Do not implement the lowering body until this design is reviewed and
  approved (§6.3 of the work order).
- The one-bit rule: no optimization that changes the numeric result by even
  one bit is adopted (restated at the top; it governs §2.4 and §3).
- Prove-then-run only; no speculative lowering (§3).
- Every stage keeps the exact path as the unconditional fallback and the sole
  source of truth for any value that cannot be proven equivalent.
