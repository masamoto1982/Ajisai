# Denominator growth in exact tensors — 2026-08

This is a development-status note, not a normative source. Normative profile
semantics remain under `spec/`.

## Why exactness is the requirement, not a preference

The Tensor Profile exists so Ajisai can host a language model. The purpose of
that model is to give Ajisai a completely free surface syntax: free-form input
is mapped to a canonical Ajisai program by a learned front end.

That makes the model part of the language, and it changes what "good numerics"
means. A syntax front end must be *specifiable*. If the same input can map to
different programs on different hardware, the language has no definition — only
a current implementation. Floating point cannot offer that: an operator's
result depends on association order, fused multiply-add availability, and the
backend's libm. The profile handles this honestly for approximate dtypes by
declaring tolerances, but a declared tolerance is exactly what a parser cannot
have. Two programs are not "within 1e-6" of each other.

Exact rational arithmetic gives reproducibility rather than accuracy, and
reproducibility is the property the free-syntax goal actually needs. Every
backend must produce the identical rational, because there is nothing left
unspecified for a backend to decide.

The price is representation size, and that price is what this note is about.

## Where the growth comes from

A rational `p/q` in lowest terms costs about `bits(q)` to represent, for values
of bounded magnitude. The problem is that arithmetic adds those bits:

- `a/q + b/s = (as + bq)/(qs)`, reduced by `gcd`
- `(a/q)(b/s) = ab/(qs)`

For rationals that do not share factors — the generic case — nothing cancels
and denominator bits simply add. Growth is therefore not a property of any one
operator but of *chain length*.

The worst offender is the contraction inside `MATMUL`. A dot product of length
`K` sums `K` products, each with its own denominator, so the sum's denominator
is generically the product of all `2K` operand denominators:

```
bits ≈ Σ (bits(qᵢ) + bits(sᵢ)) ≈ 2·K·b
```

At a hidden width of 512 with 16-bit denominators, one layer produces ~16,000-bit
denominators. The second layer takes those as input and produces ~16 million
bits *per element*. This is why exact arithmetic is usually dismissed for
machine learning: not because it gives wrong answers, but because it stops
giving answers at all.

Measured, on `[1,K]·[K,1]` contractions of unit fractions over distinct primes:

| K | denominator bits |
| ---: | ---: |
| 2 | 14 |
| 8 | 70 |
| 32 | 221 |

Linear in `K`, exactly as predicted.

## The strategy

### 1. A shared grid removes the dependence on reduction length

Suppose every element of both operands lies on the grid `(1/d)ℤ` — that is,
every denominator divides `d`. Write `aᵢ = αᵢ/d` and `bᵢ = βᵢ/d` with integer
`α, β`. Then

```
Σᵢ aᵢbᵢ  =  (Σᵢ αᵢβᵢ) / d²
```

The products already share the denominator `d²`, so the sum has denominator
`d²` — **whatever `K` is**. The reduction length drops out completely. The
numerator is an ordinary integer sum.

This is the important result, and note what it does *not* depend on: the grid
need not be a power of two. Any common `d` works. Sharing the denominator is
what matters; the grid's fineness is a separate, independent choice.

Measured, at `d = 256` (so the `d²` bound is 17 bits):

| K | gridded | ungridded |
| ---: | ---: | ---: |
| 2 | 16 | 14 |
| 8 | 14 | 70 |
| 32 | 13 | 221 |
| 64 | 13 | — |

The gridded column does not grow. It slightly *falls*, because longer sums
offer more opportunities for common factors to cancel.

### 2. Regridding per layer removes the dependence on depth

A grid bounds one contraction, but chaining squares the denominator each time:
`d → d² → d⁴ → …`, which is `d^(2^L)` at depth `L`. So the grid alone is not
enough.

`REGRID` — round every element back onto the grid `1/d` — restores the
invariant after each layer, so the cost per layer is `d → d² → d` and the
representation size becomes constant in both `K` and `L`.

This is the tensor lifting of Core's `QUANTIZE` word, whose documentation
already states the principle: *"an exact number carries its whole history in
its denominator, so an iterative method grows one without bound; quantizing
each step keeps the representation the size of the answer rather than the size
of the computation."* The tensor operator carries a different name only because
profile operators must not collide with frozen Core vocabulary; the tie rule
(halves away from zero) is inherited so the language does not hold two.

Measured over 8 chained `[2,2]` matmuls on a `d = 16` grid, the denominator
without regridding runs `5, 6, 10, 11, 15, 16, 20, 21, 25` bits and keeps
climbing; with a `REGRID` after each layer it holds at 5.

The climb is milder than the `d^(2^L)` worst case — about 2.5 bits per layer
rather than a doubling — because a power-of-two grid lets sums cancel trailing
zeros. Milder is not bounded: nothing in that sequence stops it, and the cost is
still a function of depth rather than of the answer.

### 3. The denominator is a budgeted resource

Exact arithmetic's characteristic failure is not a wrong answer but an
unbounded one: the program does not fail, it stops finishing. That is the worst
possible failure mode, because it is indistinguishable from slowness.

`TensorMemoryBudget::max_denominator_bits` makes the denominator a checked
resource like element count and byte count. It is verified in `Tensor::new`,
which every operator publishes its result through, so no exact tensor can exist
without having passed it. A graph that does not regrid often enough now reports
which node overflowed instead of disappearing into bignum arithmetic.

This follows the profile's existing rule that resource exhaustion is a declared
outcome rather than a silent one.

## What exactness costs, stated honestly

- **Error.** Regridding to `1/d` moves each element by at most `1/(2d)`. This
  is a hard bound, not a statistical expectation, but it is a real
  approximation: an exact graph with `REGRID` nodes is a fixed-point scheme
  whose rounding is *specified* rather than delegated to hardware. Exactness
  buys reproducibility, not freedom from approximation.
- **Numerators.** Bounding denominators does not bound magnitude. A sum of `K`
  products of size `M` reaches `K·M²`, and depth compounds it. Normalization is
  what bounds the numerator, so `REGRID` and RMSNorm are complementary: one
  holds the denominator, the other the numerator.
- **Speed.** Bignum arithmetic is far slower than hardware floating point even
  at a bounded size. The exact path is for cases where a reproducible answer is
  worth more than throughput — which is the syntax front end's situation, not
  every model's.

## What is not solved

**Transcendentals.** `EXP`, `LOG`, and `RSQRT` are irrational for essentially
all rational inputs, so there is no exact result to return. They are declared
`dtypeDomain: "approximate"` and reject `q` rather than silently returning an
approximation, which would be the implicit conversion the profile forbids.

This is the blocking gap for a fully exact Transformer, because attention needs
softmax. Closing it requires an operator contract carrying a *declared
precision* — for example, "the best rational approximation of `exp(x)` with
denominator at most `d`, by a specified series with a proven truncation bound".
That is a well-posed problem and it is the natural next piece of work, but it
is a new contract rather than an extension of an existing one, and Profile 0.1
does not define it.

Worth noting for the free-syntax goal specifically: the *decode* step needs
only `argmax` over logits, which is exact over `q` and needs no transcendental
at all. It is the softmax inside attention, not the choice of output token,
that currently forces the approximate domain.

**Training.** `REGRID` is declared non-differentiable (`kind: "none"`), because
rounding to a grid has zero derivative almost everywhere. The straight-through
estimator that quantization-aware training normally uses is a *different*
function, and giving it to `REGRID` implicitly would make the gradient graph
disagree with the forward graph. If it is wanted, it belongs in its own
declared contract.

## Status

| Piece | State |
| --- | --- |
| `q` exact dtype, NIL and denominator boundaries | implemented |
| Exact ADD/SUB/MUL/DIV, MATMUL, REDUCE_SUM/MAX, WHERE | implemented |
| `REGRID` (`tensor.regrid.v1`) | implemented |
| Denominator budget enforcement | implemented |
| Declared `dtypeDomain` per operator, gated in validator and backend | implemented |
| Exact transcendentals with declared precision | not defined |
| Exact `argmax` for decoding | not implemented |
| Straight-through gradient contract | not defined |
