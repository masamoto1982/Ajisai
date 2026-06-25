# Pure HOF kernel memoization (direction B)

Status: **implemented** (MAP). A value-model-neutral runtime optimization: it
changes how many times a higher-order kernel runs, never the observable result.
The canonical language definition is `SPECIFICATION.html`.

## Idea

When Ajisai runs, calling a kernel on each element of a vector is a *search*
repeated per element. For a **pure** kernel the per-element application is a pure
function `(kernel, element) -> result`, so when a vector carries repeated
elements the same result can be reused instead of recomputed. This activates the
previously dormant pure-result cache (`elastic::CacheManager`) on a real
execution path.

This is the sound, high-yield slice of the "wire the result cache into the pure
path" plan. A *general* pure-word memoization is a poor fit for a stack VM:
`WordDefinition` carries no arity, so the only sound key is the whole stack — low
hit-rate and O(depth) to compute. A HOF kernel sidesteps this: its arity is
**exactly one element**, and `MAP` runs it against an **isolated, element-only
stack** (`map.rs` swaps the stack out, `clear()`s it, pushes one element), so the
result provably depends only on that element.

## Where

`interpreter/higher_order/map.rs`, the `StackTop` per-element loop, the
`QuantizedBlock` kernel arm. The memo wraps the existing
`execute_hedged_map_kernel` call: on a hit it pushes the cached result and skips
the kernel; on a miss it runs the kernel and stores the result. All other arms
(`WordName`, non-quantizable `CodeBlock`) and the bulk-tensor fast path are
unchanged. `interpreter/higher_order/memo.rs` holds the key construction and the
`elastic_cache` fetch/store wrappers.

## Soundness

Three independent guards; if any fails for an element, that element runs the
kernel normally — there is never a false hit:

1. **Pure kernel only.** Engages only when the quantized kernel is
   `QuantizedPurity::Pure`: no host effects, deterministic result.
2. **Canonical element identity.** The element key is built only for bare
   rational scalars (`Value::as_scalar` is `Some` solely for
   `ValueData::Scalar(Fraction)`; irrational `ExactScalar`, Booleans, tensors,
   vectors, records, NIL all yield `None` and fall through). A `Fraction` is
   stored reduced, so `numerator/denominator` is canonical and equal values map
   to equal keys. The element's interpretation role is folded into the key so a
   numeric `3` and a differently-roled `3` never collide.
3. **Definition-change invalidation.** The backing store is `elastic_cache`,
   which `invalidate_execution_artifacts` flushes on every dictionary/module
   epoch bump (`DEF`/`DEL`/import). A redefinition that changes the kernel's
   meaning therefore cannot serve a stale result. The cache key additionally
   embeds both epochs as a second, key-level guard.

The kernel key is the kernel's serialized token stream used **directly** (not a
hash), so distinct kernels can never collide. Hedged modes disable the memo
(like the bulk fast path) so the quantized/plain race still observes every
per-element event.

## Gating, metrics, default

Default on; toggle via `Interpreter::set_hof_memo_enabled(bool)` or the
`AJISAI_NO_HOF_MEMO` environment switch, exactly like the D1 scalar fast path.
`RuntimeMetrics` gains `hof_memo_hit_count` / `hof_memo_miss_count` /
`hof_memo_store_count` (observational only). Because the default observable
result is byte-identical, the toggle exists for A/B measurement and the
differential tests, not as a semantic knob.

## Verification

`memo_tests.rs`:
- **Differential ON vs OFF** stacks are byte-identical across repeated elements,
  distinct elements, a user-word kernel, division-by-zero bubbles, and
  non-rational (collection) elements that fall through.
- **Engagement counters**: `[ 3 3 3 5 ]` with a pure two-op kernel yields exactly
  two hits / two misses / two stores; all-distinct yields zero hits; disabled
  yields no cache work.
- **Invalidation**: redefining a kernel's helper word is never served a stale
  cached result.

`cargo test --lib` / `--tests` all green (1414 lib). A synthetic 600-element MAP
with ~6 distinct values and a moderately heavy pure kernel runs **~4.1x** faster
with the memo on (≈3.43 ms → ≈0.84 ms/run, 594/600 applications served from
cache). The payoff is workload-dependent: it scales with element repetition and
kernel cost, and is neutral (a cheap key probe per element) when elements are
distinct or non-rational.

## Scope and non-goals

- **MAP only** for now. `FILTER`/`ANY`/`ALL`/`COUNT`/fold-family kernels run
  against the same isolated-element discipline and are natural follow-ups; they
  are intentionally out of this first step.
- The bulk-tensor fast path (1-D dense tensor + fast-unary kernel) still wins
  where it applies and is untouched — the memo targets the per-element loop that
  bulk-ineligible kernels (e.g. multi-op blocks, user-word-calling kernels) take.
- Invalidation is coarse (epoch-bump flush). Finer dependency-scoped
  invalidation using `collect_transitive_dependents` (added in the prior step)
  is possible but unnecessary here: redefinitions mid-loop are rare and the
  flush is already sound and cheap.
