# Handoff: D1 — scalar–scalar arithmetic fast path (bypass the tensor wrapper)

Status: **not implemented**. This is a handoff spec for whoever picks up D1.
It is a *value-model* optimization adjacent to the arithmetic kernel, not a
change to the exact-arithmetic algorithms. The canonical language definition is
`SPECIFICATION.html`; D1 must not change any observable result.

## Context and motivation

The internal-GOTO series (#1168–#1171) compiled Ajisai's guarded recursive loop
end to end (backward jump, COND dispatch, literal vectors, compiled clause
bodies). After that, the countdown loop still spends **~9.8 µs/iter**, yet the
raw arithmetic in it is tiny: a rational subtract + two compares is **~61 ns**
(measured; `Fraction` already has an `i64/i128` fast path). So **>99% of the
per-iteration cost is the value model around the arithmetic, not the arithmetic**.

Where it goes: a single-element `[ N ]` is promoted to a **1-lane dense tensor**
(`Value::from_vector_promoted` → `try_collect_dense` → `ValueData::Tensor { data:[N], shape:[1] }`).
So the loop's `[ 1 ] - DOWN` and `[ 0 ] >` are **tensor** operations: they route
through interval/SIMD/sparse/exact-real pre-checks and finally the broadcast
machinery — cloning operand `Value`s, dispatching on shape, allocating a result
tensor — to do what is really one `Fraction::sub` and one `Fraction::cmp`.

**D1 goal:** when both top-of-stack operands are simple rational scalars (a bare
`Scalar`, or a 1-lane `Tensor` / single-element `Vector` wrapping a rational),
compute the result directly with `Fraction` and push it, bypassing the tensor
dispatch — *while producing a byte-for-byte identical result Value* (same shape,
same wrapping, same semantic hint).

Expected payoff: this is the actual loop-iteration win the kernel PRs left on
the table. Target the `tail_call_bench` countdown number (~9.8 µs/iter).

## Where the code is

All paths below are in `rust/src/interpreter/`.

### Arithmetic (`+ - * /`)
- `arithmetic.rs::op_add/op_sub/op_mul/op_div` → `apply_exact_arithmetic_schema`
  (`arithmetic.rs:166`).
- That function, for `StackTop` mode, runs a sequence of pre-checks **each of
  which clones the two top values** via `stacktop_pair` (`arithmetic.rs:155`):
  `push_interval_schema_result`, `push_simd_schema_result`,
  `sparse_mul_candidate` (mul only), `push_exact_real_schema_result`.
- Falls through to `apply_binary_arithmetic` (`arithmetic.rs:333`) →
  `apply_binary_broadcast_with_metrics` → `push_result`.
- Useful existing helpers: `extract_scalar_from_value` (`arithmetic.rs:241`),
  `is_scalar_value` (`arithmetic.rs:268`), `extract_exact_real_from_value`
  (`arithmetic.rs:260`), `nil_passthrough_binary` (NIL rule).

### Comparison (`< <= > >= = !=`)
- `comparison.rs` (`op_lt`, `op_gt`, …). The guard `[ 0 ] >` is a tensor
  comparison; the same wrapper overhead applies. The rational compare itself is
  already `Fraction::cmp` with an i128 fast path (`fraction.rs:447`). D1 should
  add the analogous scalar fast path here too — it is half of every guard.

### Value model
- `types/value_operations.rs`: `from_vector_promoted` / `from_tensor` /
  `from_fraction` (how results are wrapped). `ValueData` variants:
  `Scalar(Fraction)`, `Tensor{data,shape}`, `Vector(children)`,
  `ExactScalar(ExactReal)`, `Nil`.
- The semantic hint lives in the `SemanticRegistry` stack hints, set by
  `push_result` / the op layer — see how `apply_binary_arithmetic` + the
  interpreter set hints (`RawNumber` for numeric results).

## The invariants that MUST hold (most important section)

A scalar fast path is only correct if it is indistinguishable from the current
path. Pin each of these before optimizing:

1. **Result shape/wrapping.** Determine the current output Value for each input
   shape and match it exactly:
   - `[ 1 ] [ 2 ] +` (two 1-lane tensors) → currently a **1-lane tensor** `[ 3 ]`
     (renders `[ 3/1 ]`), *not* a bare scalar. Confirm with the CLI and match.
   - bare scalars on the stack (e.g. `2 3 /` after `NUM`) → bare scalar.
   - So the fast path must reproduce the operand's wrapping (scalar→scalar,
     1-lane tensor→1-lane tensor, 1-vec→whatever the current code yields).
2. **Semantic hint** of the result must equal the current path's hint
   (`RawNumber` etc.). A wrong hint changes display (e.g. TruthValue rendering).
3. **NIL passthrough.** If either operand is NIL, the existing
   `nil_passthrough_binary` rule governs (reasoned NIL out). The fast path must
   only engage for non-NIL rational operands.
4. **Irrational / `ExactScalar`.** Never take the rational fast path when either
   operand is `ExactScalar` (or otherwise non-rational) — those need the
   ExactReal/CF path (`push_exact_real_schema_result`).
5. **Interval** operands → must keep the interval path
   (`push_interval_schema_result`).
6. **Division by zero** → reasoned NIL bubble (`division_by_zero_bubble`),
   exactly as today; do not panic or push a wrong value.
7. **Target / consumption modes.** Engage only for `TargetMode::StackTop`. Handle
   `Consume`/`Keep` (`KEEP` retains operands and branches) correctly, or restrict
   the fast path to `Consume` and fall back for `Keep`. Reset execution modes the
   same way the normal path does.
8. **Comparison `UNKNOWN`.** For rationals the comparison is always decided
   (i128 cross-multiply), so no `UNKNOWN` arises — but make sure the fast path is
   only taken for rationals so the budgeted-`UNKNOWN` path is untouched for
   irrationals.

## Recommended approach

- Add a guarded fast path at the **top** of `apply_exact_arithmetic_schema`
  (and the comparison equivalent): peek (don't clone) the two top values; if both
  are simple rational scalars in the same wrapping and the modes are TOP+Consume
  (or a clearly-handled Keep), compute `Fraction::op`, build the result with the
  matching wrapping + hint, consume the operands, push, and return. Otherwise
  fall through to the existing code unchanged.
- Avoid the operand `Value` clones (`stacktop_pair`) on the fast path — read the
  fractions out by reference and only consume at the end.
- Keep it behind a toggle for A/B, exactly like the prior PRs:
  `Interpreter::set_scalar_fastpath_enabled(bool)` + a `scalar_fastpath_count`
  metric. (See `set_cond_dispatch_enabled` / `cond_dispatch_fast_count` in
  `interpreter_core.rs` for the established pattern.)

## Verification (non-negotiable)

- **Differential test:** fast path ON vs OFF must produce identical stacks
  (value + rendered form + hint) across operand shapes: bare scalar, 1-lane
  tensor, single-element vector, mixed, NIL operand, division-by-zero, KEEP mode,
  and a few comparisons. Mirror `rust/src/interpreter/cond_dispatch_tests.rs` /
  `vector_literal_tests.rs`.
- **Shadow validation** already compares compiled vs interpreted on every
  non-recursive call — leverage it.
- Run the full suites: `cargo test --lib`, `cargo test --tests`, the conformance
  / differential / arithmetic MC/DC tests.
- **Measure** with `cargo run --release --example tail_call_bench` (the
  countdown per-iteration number) plus a focused scalar-arithmetic microbench
  (A/B via the toggle).
- CI gates to satisfy: `cargo fmt --check`, `cargo clippy --all-targets -D
  warnings` (no new warnings), `npm run check:semantic-firewall`, and
  `npm run provenance:attest` (refresh the attestation; commit the result).

## Process notes (match the existing cadence)

- One step, one draft PR, measured. Branch from latest `main`.
- Add a short design note under `docs/dev/` (see
  `internal-goto-literal-vectors.md` for the house style and depth).
- This is the highest-leverage *loop* win remaining, but it is value-model work;
  it composes with — and is independent of — the arithmetic-kernel `i128` CF fast
  paths (`arith-sqrt-i128-fastpath.md` and the Möbius/Bihom follow-ups).
