# i128 fast path for the Möbius (unary Gosper) CF state

Status: prototype (non-canonical design note). Follow-up to
`arith-sqrt-i128-fastpath.md`, extending the i128 continued-fraction fast path
to the unary Gosper transform. No surface syntax or value-semantics change.

## Motivation

`arith-sqrt-i128-fastpath.md` gave the quadratic-surd state an i128 fast path,
covering pure square roots. The next-most-common irrational arithmetic is a
**rational combined with one root** — `r + √n`, `√n − r`, `r · √n` — which the
arithmetic layer compiles into a **unary Möbius** Gosper transform
`(a·x + b)/(c·x + d)` over the operand stream `x = √n`. Its streaming step
(`step_mobius`) holds `a,b,c,d` as `BigInt` and, per ingested operand term,
computes `a·p+b`, `c·p+d`, a GCD normalization, and the floor/emit test — all
allocating.

Unlike the surd, the Möbius coefficients **grow** as terms are ingested, so a
fixed i128 state cannot last forever; the fast path must promote mid-stream.

## Mechanism

Adds `CfState::MobiusSmall { a,b,c,d: i128, x, x_done }`, built by
`from_exact_real` when the transform's `a,b,c,d` fit i128 (and the fast path is
enabled). `step_mobius_small` runs the same emit/ingest/normalize logic in
checked i128 (`mobius_small_emit`, `mobius_small_ingest`,
`normalize_mobius_i128`, reusing `floor_div_i128`).

**Promotion carries no term.** When a step would overflow i128, when an ingested
operand term does not fit i128, when the operand ends (rational tail), or when
the safety budget is hit, the state promotes to the BigInt `Mobius` state via
`promote_mobius_small`. The subtle case is overflow *during ingest*: the operand
term `p` has already been consumed from `x`, so `promote_mobius_small` takes a
`pending: Option<BigInt>` and applies that ingest in BigInt before handing off —
so no term is lost and the emitted stream is identical to running in BigInt all
along. Tail/budget/overflow-on-emit promotions carry no pending term and let the
existing BigInt `step_mobius` produce the rational tail or apply its budget rule
unchanged.

The inner operand `x` keeps its own fast path (a `SqrtSmall` for `√n`), so a
Möbius over a surd is fully i128 until promotion.

## Correctness

`mobius_small_matches_bigint_path` is a differential test: for `r ⊕ √n` across
`n ∈ [2,30]` (non-squares) and `r = rn/rd` with `rn ∈ [−5,5]`, `rd ∈ [1,4]`,
over the four Möbius-producing operations, the i128 path and a forced BigInt
`Mobius` expansion (over the same inner `√n` stream) must agree **term for term**
over a 200-term budget — 100+ transforms. Because the inner surd layer is
already differentially verified, this isolates the Möbius layer. As before,
term-equality means no decided comparison or rendered value can change; the NICF
conformance suites run unchanged.

## Measured effect

`cargo run --release --example sqrt_cf_bench` (now also expands ~96 `r ⊕ √n`
Möbius transforms alongside the surds, fast path off vs on):

```
  fast path OFF (BigInt): ~36.0 s   (4000-rep variant)
  fast path ON  (i128):   ~ 9.5 s
  speedup: ~3.79x
```

The combined speedup rises to **~3.79×**: Möbius coefficients grow, so each
BigInt term is more expensive than a surd term, and the i128 path saves more.
`set_sqrt_small_fast_path(false)` forces the BigInt path for both states (the A/B
hook; correctness identical either way).

## Scope and next step

Covers unary Möbius (`√ ⊕ rational`). Binary `√ ⊕ √` (e.g. `√2 + √3`) flows
through `Gosper::Bihomographic` / `step_bihom` with eight coefficients, still
`BigInt`. The same i128-with-promotion treatment there is the final A1 slice,
reusing this PR's `floor_div_i128`, `gcd_u128`, the `SmallEmit`/promotion pattern,
and the differential-test harness.
