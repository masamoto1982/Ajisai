# Tensor Profile implementation progress — 2026-08

This is a development-status note, not a normative source. Normative profile
semantics remain under `spec/`.

## Current end-to-end path

The repository can now deserialize a typed graph, validate SSA ordering and
Tensor types, bind symbolic dimensions from concrete f32/f64 inputs, and execute
`MATMUL`, broadcast arithmetic, `EXP`, `LOG`, `REDUCE_SUM`, and `REDUCE_MAX` on the deterministic
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
| Reference tensor algebra | 40% | MATMUL, broadcast ADD/SUB/MUL/DIV, EXP, LOG, REDUCE_SUM, REDUCE_MAX | RSQRT, WHERE, reshape/permute, gather, concat |
| Tiny Transformer inference | 15% | projections and numerically stable softmax are executable graph compositions | embedding/gather, normalization, RoPE, tokenizer/weights, sampling |
| Reverse-mode autodiff | 0% | VJP identifiers exist only as contracts | graph transform, gradient graph validation, VJP conformance |
| Accelerator backend | 0% | backend boundary is specified | lowering, receipts, bounded-error conformance |

## Next implementation sequence

1. Add `RSQRT` and `WHERE` for normalization and masks.
2. Add reshape/permute and gather so embedding and attention layouts are
   expressible.
3. Define canonical graph encoding and execution receipts before introducing an
   optimized backend.
4. Implement VJP-driven reverse-mode transformation only after forward graph
   contracts cover the minimal Transformer algebra.

At this point Ajisai has a validated executable Tensor graph slice, but it is
not yet a usable LLM inference runtime. The largest missing dependency for a
first tiny Transformer is structural Tensor algebra and artifact-backed weights.
