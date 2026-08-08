# Tensor Profile implementation progress — 2026-08

This is a development-status note, not a normative source. Normative profile
semantics remain under `spec/`.

## Current end-to-end path

The repository can now deserialize a typed graph, validate SSA ordering and
Tensor types, bind symbolic dimensions from concrete f32/f64 inputs, and execute
`MATMUL`, `EXP`, `LOG`, `REDUCE_SUM`, and `REDUCE_MAX` on the deterministic
reference CPU backend. Shape products and output bytes are checked before
allocation. The Core kernel and its exact Scalar domain remain unchanged.

## Milestone progress

Percentages indicate implementation readiness for the stated milestone, not a
claim that the overall LLM system is that complete.

| Milestone | Progress | Completed | Remaining critical work |
| --- | ---: | --- | --- |
| Tensor Profile 0.1 semantics | 65% | value boundary, f32/f64, numeric classes, operator schema, explicit RNG contract | complete CAST rules, NaN payload/rounding details, conformance tolerances |
| Checked Tensor runtime | 55% | immutable buffers, checked shape/strides, element and byte budgets | views/layouts, reshape/permute/slice, temporary/peak budgets |
| Typed Graph IR | 40% | serde exchange, SSA validation, symbolic dimensions, semantic identity, operator-aware inference | constants/artifacts loading, graph normalization, effects/resource analysis, stable canonical encoding |
| Reference tensor algebra | 30% | MATMUL, EXP, LOG, REDUCE_SUM, REDUCE_MAX | elementwise arithmetic, RSQRT, WHERE, reshape/permute, gather, concat |
| Tiny Transformer inference | 10% | enough primitives to exercise projections and reduction foundations | broadcast arithmetic, stable softmax, embedding/gather, RoPE, tokenizer/weights, sampling |
| Reverse-mode autodiff | 0% | VJP identifiers exist only as contracts | graph transform, gradient graph validation, VJP conformance |
| Accelerator backend | 0% | backend boundary is specified | lowering, receipts, bounded-error conformance |

## Next implementation sequence

1. Add broadcast elementwise arithmetic and `RSQRT`/`WHERE`.
2. Compose and test numerically stable softmax from MAX, subtraction, EXP, SUM,
   and division rather than adding SOFTMAX to Core.
3. Add reshape/permute and gather so embedding and attention layouts are
   expressible.
4. Define canonical graph encoding and execution receipts before introducing an
   optimized backend.
5. Implement VJP-driven reverse-mode transformation only after forward graph
   contracts cover the minimal Transformer algebra.

At this point Ajisai has a validated executable Tensor graph slice, but it is
not yet a usable LLM inference runtime. The largest missing dependency for a
first tiny Transformer is broadcast elementwise algebra, followed by structural
Tensor operations and artifact-backed weights.
