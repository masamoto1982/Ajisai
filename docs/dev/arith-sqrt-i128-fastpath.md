# i128 fast path for the quadratic-surd CF state

Status: prototype (non-canonical design note). First step into the exact
arithmetic kernel, after the internal-GOTO control-flow series. No surface
syntax or value-semantics change; the canonical definition remains
`SPECIFICATION.html`.

## Motivation

Rational arithmetic is already fast (~61 ns for a subtract + two compares, via
the `Fraction` `Small(i64,i64)`/`Big(BigInt)` split). The genuinely expensive
arithmetic is the **irrational continued-fraction core** in
`continued_fraction.rs`: every term of a lazy CF is produced by `BigInt`
recurrences, and comparison streams up to `DEFAULT_COMPARISON_BUDGET = 256`
terms. The most common irrational is a plain square root, whose CF is generated
by the quadratic-surd state

```
a_i = ⌊(P_i + ⌊√D⌋) / Q_i⌋
P_{i+1} = a_i·Q_i − P_i
Q_{i+1} = (D − P_{i+1}²) / Q_i
```

with `D`, `P`, `Q` held as `BigInt` — so every term allocated several `BigInt`s
even though, by a classical bound, `P_i` and `Q_i` stay below `2·√D` and remain
tiny for ordinary radicands.

## Mechanism

This adds `CfState::SqrtSmall { big_d, sqrt_floor, p_i, q_i: i128 }`, the same
recurrence in `i128`. `CfIter::from_exact_real` builds it whenever `D`, `⌊√D⌋`,
and the starting `Q` fit `i128` (the overwhelmingly common case). Each step runs
`sqrt_small_step` with checked i128 arithmetic and a `floor_div_i128` that
matches `BigInt::div_floor` exactly. The only way to overflow `i128` is a
radicand near the ceiling; that step returns `None` and the state **promotes
back** to the BigInt `Sqrt` state and recomputes there, so the emitted term
sequence is always identical to the BigInt path — this is purely a fast path,
never a semantic change.

It mirrors, one layer down, the `Small`/`Big` split that already makes
`Fraction` fast.

## Correctness

`sqrt_small_matches_bigint_path` is a differential test: for every surd
`√(num/den)` with `num ≤ 80`, `den ≤ 16` (200+ genuine surds), the `SqrtSmall`
path and a forced BigInt `Sqrt` expansion must agree **term for term** over a
300-term budget. Because comparison and display are derived from these terms,
term-equality establishes that no decided order or rendered value can change.
The existing NICF comparison-conformance and differential suites run unchanged.

## Measured effect

`cargo run --release --example sqrt_cf_bench` (expand ~36 surds to 64 CF terms
and compare adjacent pairs, fast path off vs on):

```
  fast path OFF (BigInt): ~2031 ms
  fast path ON  (i128):   ~ 892 ms
  speedup: ~2.28x
```

A **~2.28×** speedup on square-root CF expansion and surd comparison — the work
behind every guard or observation over a square root.

`set_sqrt_small_fast_path(false)` forces the BigInt path (the A/B hook;
correctness is identical either way).

## Scope and next steps

This covers pure square roots (`AlgebraicSqrt`) and their comparison — the most
common irrational. Compound irrationals (`√2 + √3`, `√2 · 3`) flow through the
`Gosper::Bihomographic` / `Mobius` streaming states, which still use `BigInt`
coefficients. Applying the same i128-with-promotion treatment to those
(`step_mobius`, `step_bihom`) is the natural follow-up, reusing this PR's
`floor_div_i128` helper and differential-test pattern.
