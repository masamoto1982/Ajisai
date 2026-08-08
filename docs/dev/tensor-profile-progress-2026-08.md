# Tensor Profile implementation progress — 2026-08

This is a development-status note, not a normative source. Normative profile
semantics remain under `spec/`.

## Current end-to-end path

The repository can now deserialize a typed graph, validate SSA ordering and
Tensor types, bind symbolic dimensions from concrete f32/f64 inputs, and execute
`MATMUL`, broadcast arithmetic, `EXP`, `LOG`, `RSQRT`, `REDUCE_SUM`,
`REDUCE_MAX`, and `WHERE` on the deterministic reference CPU backend. Shape
products and output bytes are checked before allocation. The Core kernel and its
exact Scalar domain remain unchanged.

`WHERE` introduced the profile's `bool` predicate dtype. It is deliberately not
one of the numeric dtypes: the graph validator and the reference backend both
reject a predicate wherever an operator's contract ranges over `f32`/`f64`, so
"no implicit casts" is enforced rather than assumed. With `RSQRT` and `WHERE`
in place, masked softmax and RMS normalization are executable graph
compositions, which is the property the profile claims about NN operations.

## Milestone progress

Percentages indicate implementation readiness for the stated milestone, not a
claim that the overall LLM system is that complete.

| Milestone | Progress | Completed | Remaining critical work |
| --- | ---: | --- | --- |
| Tensor Profile 0.1 semantics | 70% | value boundary, f32/f64, bool predicate dtype, numeric classes, operator schema, explicit RNG contract | complete CAST rules, NaN payload/rounding details, conformance tolerances |
| Checked Tensor runtime | 55% | immutable buffers, checked shape/strides, element and byte budgets | views/layouts, reshape/permute/slice, temporary/peak budgets |
| Typed Graph IR | 45% | serde exchange, SSA validation, symbolic dimensions, semantic identity, operator-aware inference, dtype-domain enforcement | constants/artifacts loading, graph normalization, effects/resource analysis, stable canonical encoding |
| Reference tensor algebra | 55% | MATMUL, broadcast ADD/SUB/MUL/DIV, EXP, LOG, RSQRT, REDUCE_SUM, REDUCE_MAX, WHERE | reshape/permute, gather, concat |
| Tiny Transformer inference | 25% | projections, masked stable softmax, and RMS normalization are executable graph compositions | embedding/gather, RoPE, tokenizer/weights, sampling |
| Reverse-mode autodiff | 0% | VJP identifiers exist only as contracts | graph transform, gradient graph validation, VJP conformance |
| Accelerator backend | 0% | backend boundary is specified | lowering, receipts, bounded-error conformance |

## Next implementation sequence

1. Add reshape/permute so multi-head attention layouts are expressible without
   materializing a transposed copy per node.
2. Add gather so token embedding — currently the only step of a tiny
   Transformer with no expressible form at all — becomes a graph node.
3. Define canonical graph encoding and execution receipts before introducing an
   optimized backend.
4. Implement VJP-driven reverse-mode transformation only after forward graph
   contracts cover the minimal Transformer algebra.

At this point Ajisai has a validated executable Tensor graph slice covering the
pointwise and reduction algebra a Transformer block needs, but it is not yet a
usable LLM inference runtime. The remaining dependencies are structural Tensor
algebra (reshape/permute/gather) and artifact-backed weights.
